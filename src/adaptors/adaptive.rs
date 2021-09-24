use rayon::iter::plumbing::*;
use std::fmt::Debug;
use std::sync::atomic::{AtomicUsize, Ordering};

use crossbeam::channel::Receiver;
use crossbeam::channel::Sender;
use rayon::current_num_threads;

struct AdaptiveProducer<'f, P> {
    producer: Option<P>,
    sender: Sender<Option<(usize, P)>>,
    receiver: Receiver<Option<(usize, P)>>,
    stealers: &'f AtomicUsize,
    workers: &'f AtomicUsize,
    size: usize,
    block_size: usize,
}

impl<'f, P: Producer> AdaptiveProducer<'f, P> {
    fn stolen(&self) -> bool {
        if self.producer.is_some() && self.stealers.load(Ordering::SeqCst) != 0 {
            let mut stealer_count = self.stealers.load(Ordering::SeqCst);
            while stealer_count != 0 {
                match self.stealers.compare_exchange(
                    stealer_count,
                    stealer_count - 1,
                    Ordering::SeqCst,
                    Ordering::SeqCst,
                ) {
                    Ok(_) => {
                        return true;
                    }
                    Err(new_stealer_count) => stealer_count = new_stealer_count,
                }
            }
            false
        } else {
            false
        }
    }
    fn give_work(&mut self) {
        let mid = self.size / 2;
        let right_size = self.size - mid;
        self.size = mid;
        let (left_producer, right_producer) = self.producer.take().unwrap().split_at(mid);
        self.workers.fetch_add(1, Ordering::SeqCst);
        self.sender.send(Some((right_size, right_producer)));
        self.producer = Some(left_producer);
    }
    fn stop_working(&mut self) {
        // everything done. terminate waiting stealers and return the folder
        let workers = self.workers.fetch_sub(1, Ordering::SeqCst) - 1;
        if workers == 0 {
            for _ in 0..current_num_threads() {
                self.sender.send(None).expect("Failed to send on channel");
            }
        }
    }
    fn fold_block<F>(&mut self, folder: F) -> F
    where
        F: Folder<P::Item>,
    {
        let producer = self.producer.unwrap();
        if self.block_size >= self.size {
            folder = folder.consume_iter(producer.into_iter());
            self.size = 0;
            self.stop_working();
        } else {
            let (left, right) = producer.split_at(self.block_size);
            folder = folder.consume_iter(left.into_iter());
            self.producer = Some(right);
            self.size -= self.block_size;
        }
        return folder;
    }

    fn steal(&mut self) {
        self.stealers.fetch_add(1, Ordering::SeqCst);

        if self.workers.load(Ordering::SeqCst) == 0 {
            return;
        }
        let stolen_task = self.receiver.recv().expect("Failed to receive on channel");
        if let Some((size, producer)) = stolen_task {
            self.size = size;
            self.producer = Some(producer);
        }
    }

    fn all_is_completed(&self) -> bool {
        self.workers.load(Ordering::SeqCst) != 0
    }
}

impl<'f, P: Producer> UnindexedProducer for AdaptiveProducer<'f, P> {
    type Item = P::Item;

    fn split(mut self) -> (Self, Option<Self>) {
        let right = AdaptiveProducer {
            producer: None,
            sender: self.sender.clone(),
            receiver: self.receiver.clone(),
            stealers: self.stealers,
            workers: self.workers,
            size: 0,
            block_size: self.block_size,
        };
        (self, Some(right))
    }

    fn fold_with<F>(mut self, mut folder: F) -> F
    where
        F: Folder<Self::Item>,
    {
        while !self.all_is_completed() {
            if self.producer.is_some() {
                if self.size > 1 && self.stolen() {
                    self.give_work();
                } else {
                    folder = self.fold_block(folder);
                }
            } else {
                self.steal();
            }
        }
        folder
    }
}
