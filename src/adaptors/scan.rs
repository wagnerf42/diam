use rayon::iter::plumbing::*;
use rayon::iter::*;

use std::{
    fmt::{self, Debug},
    sync::atomic::{AtomicBool, Ordering},
};

impl<U, I, ID, F, T> Scan<I, ID, F>
where
    I: ParallelIterator,
    F: Fn(&mut U, I::Item) -> Option<T> + Sync + Send,
    ID: Fn() -> U + Sync + Send,
    U: Send,
    T: Send,
{
    pub(crate) fn new(base: I, identity: ID, scan_op: F) -> Self {
        Scan {
            base,
            identity,
            scan_op,
        }
    }
}

/// Scan adaptor for the ParallelIterator trait.
pub struct Scan<I, ID, F> {
    base: I,
    identity: ID,
    scan_op: F,
}

impl<I: ParallelIterator + Debug, ID, F> Debug for Scan<I, ID, F> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Scan").field("base", &self.base).finish()
    }
}

impl<U, I, ID, F, T> ParallelIterator for Scan<I, ID, F>
where
    I: IndexedParallelIterator,
    F: Fn(&mut U, I::Item) -> Option<T> + Sync + Send,
    ID: Fn() -> U + Sync + Send,
    U: Send,
    T: Send,
{
    type Item = T;

    fn drive_unindexed<C>(self, consumer: C) -> C::Result
    where
        C: UnindexedConsumer<Self::Item>,
    {
        let full = AtomicBool::new(false);

        let consumer1 = ScanConsumer {
            base: consumer,
            scan_op: &self.scan_op,
            identity: &self.identity,
            full: &full,
        };
        self.base.drive_unindexed(consumer1)
    }
}

struct ScanConsumer<'f, C, F, ID> {
    base: C,
    scan_op: &'f F,
    identity: &'f ID,
    full: &'f AtomicBool,
}

impl<'f, C, F, ID, T, R, U> Consumer<T> for ScanConsumer<'f, C, F, ID>
where
    C: Consumer<R>,
    F: Fn(&mut U, T) -> Option<R> + Sync,
    ID: Fn() -> U + Sync + Send,
    R: Send,
{
    type Folder = ScanFolder<'f, C::Folder, F, U>;
    type Reducer = C::Reducer;
    type Result = C::Result;

    fn split_at(self, index: usize) -> (Self, Self, Self::Reducer) {
        let (left, right, reducer) = self.base.split_at(index);

        (
            ScanConsumer {
                base: left,
                scan_op: self.scan_op,
                identity: self.identity,
                full: self.full,
            },
            ScanConsumer {
                base: right,
                scan_op: self.scan_op,
                identity: self.identity,
                full: self.full,
            },
            reducer,
        )
    }

    fn into_folder(self) -> Self::Folder {
        ScanFolder {
            base: self.base.into_folder(),
            scan_op: self.scan_op,
            state: (self.identity)(),
            full: self.full,
        }
    }

    fn full(&self) -> bool {
        self.base.full()
    }
}

impl<'f, C, F, ID, R, U, T> UnindexedConsumer<T> for ScanConsumer<'f, C, F, ID>
where
    C: UnindexedConsumer<R>,
    F: Fn(&mut U, T) -> Option<R> + Sync,
    ID: Fn() -> U + Sync + Send,
    R: Send,
{
    fn split_off_left(&self) -> Self {
        ScanConsumer {
            base: self.base.split_off_left(),
            ..*self
        }
    }

    fn to_reducer(&self) -> Self::Reducer {
        self.base.to_reducer()
    }
}

struct ScanFolder<'f, C, F, S> {
    base: C,
    scan_op: &'f F,
    state: S,
    full: &'f AtomicBool,
}

impl<'f, C, F, S, T, R> Folder<T> for ScanFolder<'f, C, F, S>
where
    C: Folder<R>,
    F: Fn(&mut S, T) -> Option<R> + Sync,
{
    type Result = C::Result;

    fn consume(mut self, item: T) -> Self {
        let item_option = (self.scan_op)(&mut self.state, item);

        match item_option {
            Some(item) => self.base = self.base.consume(item),
            None => self.full.store(true, Ordering::Relaxed),
        }

        self
    }

    fn complete(self) -> Self::Result {
        self.base.complete()
    }

    fn full(&self) -> bool {
        self.full.load(Ordering::Relaxed) || self.base.full()
    }
}
