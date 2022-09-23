use rayon::iter::plumbing::{bridge, Producer, ProducerCallback};
use rayon::prelude::*;

pub struct SplitMap<I, O> {
    pub(crate) base: I,
    pub(crate) op: O,
}

impl<A, I, O> ParallelIterator for SplitMap<I, O>
where
    A: Send,
    I: ParallelIterator,
    O: Fn(I::Item) -> (A, A) + Send + Sync,
{
    type Item = A;

    fn drive_unindexed<C>(self, consumer: C) -> C::Result
    where
        C: rayon::iter::plumbing::UnindexedConsumer<Self::Item>,
    {
        todo!()
    }
}

impl<A, I, O> IndexedParallelIterator for SplitMap<I, O>
where
    A: Send,
    I: IndexedParallelIterator,
    O: Fn(I::Item) -> (A, A) + Send + Sync,
{
    fn len(&self) -> usize {
        self.base.len() * 2
    }

    fn drive<C: rayon::iter::plumbing::Consumer<Self::Item>>(self, consumer: C) -> C::Result {
        bridge(self, consumer)
    }

    fn with_producer<CB: ProducerCallback<Self::Item>>(self, callback: CB) -> CB::Output {
        return self.base.with_producer(Callback {
            callback,
            op: &self.op,
        });

        struct Callback<'o, CB, O> {
            callback: CB,
            op: &'o O,
        }

        impl<'o, A, I, O, CB> ProducerCallback<I> for Callback<'o, CB, O>
        where
            A: Send,
            O: Fn(I) -> (A, A) + Send + Sync,
            CB: ProducerCallback<A>,
        {
            type Output = CB::Output;
            fn callback<P>(self, base: P) -> CB::Output
            where
                P: Producer<Item = I>,
            {
                let producer = SplitMapProducer {
                    base,
                    op: self.op,
                    first: None,
                    last: None,
                };
                self.callback.callback(producer)
            }
        }
    }
}

struct SplitMapProducer<'o, O, P, A> {
    base: P,
    op: &'o O,
    first: Option<A>,
    last: Option<A>,
}

impl<'o, T, A, P, O> Producer for SplitMapProducer<'o, O, P, A>
where
    A: Send,
    P: Producer<Item = T>,
    O: Fn(T) -> (A, A) + Sync,
{
    type Item = A;

    type IntoIter = SplitMapIterator<'o, O, P::IntoIter, A>;

    fn into_iter(self) -> Self::IntoIter {
        SplitMapIterator {
            base: self.base.into_iter(),
            op: self.op,
            first: self.first,
            last: self.last,
        }
    }

    fn split_at(self, index: usize) -> (Self, Self) {
        let adjusted_index = index - if self.first.is_some() { 1 } else { 0 };
        let (left_base, right_base) = self.base.split_at(adjusted_index / 2);
        if adjusted_index % 2 == 0 {
            (
                SplitMapProducer {
                    base: left_base,
                    op: self.op,
                    first: self.first,
                    last: None,
                },
                SplitMapProducer {
                    base: right_base,
                    op: self.op,
                    first: None,
                    last: self.last,
                },
            )
        } else {
            let (middle_base, far_right_base) = right_base.split_at(1);
            let mut i = middle_base.into_iter();
            let (last_left, first_right) = (self.op)(i.next().unwrap());
            (
                SplitMapProducer {
                    base: left_base,
                    op: self.op,
                    first: self.first,
                    last: Some(last_left),
                },
                SplitMapProducer {
                    base: far_right_base,
                    op: self.op,
                    last: self.last,
                    first: Some(first_right),
                },
            )
        }
    }
}

struct SplitMapIterator<'o, O, I, A> {
    base: I,
    op: &'o O,
    first: Option<A>,
    last: Option<A>,
}

impl<'o, O, I, A> Iterator for SplitMapIterator<'o, O, I, A>
where
    I: Iterator,
    O: Fn(I::Item) -> (A, A),
{
    type Item = A;

    fn next(&mut self) -> Option<Self::Item> {
        self.first.take().or_else(|| {
            if let Some(next_t) = self.base.next() {
                let (a1, a2) = (self.op)(next_t);
                self.first = Some(a2);
                Some(a1)
            } else {
                self.last.take()
            }
        })
    }
}

impl<'o, O, I, A> ExactSizeIterator for SplitMapIterator<'o, O, I, A>
where
    I: ExactSizeIterator,
    O: Fn(I::Item) -> (A, A),
{
}

impl<'o, O, I, A> DoubleEndedIterator for SplitMapIterator<'o, O, I, A>
where
    I: DoubleEndedIterator,
    O: Fn(I::Item) -> (A, A),
{
    fn next_back(&mut self) -> Option<Self::Item> {
        self.last.take().or_else(|| {
            if let Some(next_t) = self.base.next_back() {
                let (a1, a2) = (self.op)(next_t);
                self.last = Some(a1);
                Some(a2)
            } else {
                self.first.take()
            }
        })
    }
}
