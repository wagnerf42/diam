use rayon::iter::plumbing::{bridge, Producer, ProducerCallback};
use rayon::prelude::*;
use std::marker::PhantomData;

pub trait HomogeneousTuples: TupleCollect {}
impl<T: TupleCollect> HomogeneousTuples for T {}

pub struct Tuples<I, T> {
    pub(crate) base: I,
    pub(crate) phantom: PhantomData<T>,
}

pub trait TupleCollect: Send + Sized {
    type Item: Sized;
    const SIZE: usize;
    fn next_from_iter<I: Iterator<Item = Self::Item>>(iterator: &mut I) -> Option<Self>;
    fn next_back_from_iter<I: DoubleEndedIterator<Item = Self::Item>>(
        iterator: &mut I,
    ) -> Option<Self>;
}

impl<A, I, T> ParallelIterator for Tuples<I, T>
where
    I: IndexedParallelIterator<Item = A>,
    T: TupleCollect<Item = A>,
{
    type Item = T;

    fn drive_unindexed<C>(self, consumer: C) -> C::Result
    where
        C: rayon::iter::plumbing::UnindexedConsumer<Self::Item>,
    {
        bridge(self, consumer)
    }

    fn opt_len(&self) -> Option<usize> {
        Some(self.len())
    }
}

impl<A, I, T> IndexedParallelIterator for Tuples<I, T>
where
    I: IndexedParallelIterator<Item = A>,
    T: TupleCollect<Item = A> + Send,
{
    fn len(&self) -> usize {
        self.base.len() / T::SIZE
    }

    fn drive<C: rayon::iter::plumbing::Consumer<Self::Item>>(self, consumer: C) -> C::Result {
        bridge(self, consumer)
    }

    fn with_producer<CB: rayon::iter::plumbing::ProducerCallback<Self::Item>>(
        self,
        callback: CB, // this callback is called on the tuples
    ) -> CB::Output {
        return self.base.with_producer(Callback {
            callback,
            phantom: PhantomData,
        });

        struct Callback<CB, TUPLE> {
            callback: CB,
            phantom: PhantomData<TUPLE>,
        }

        impl<TUPLE: TupleCollect<Item = I>, I, CB> ProducerCallback<I> for Callback<CB, TUPLE>
        where
            CB: ProducerCallback<TUPLE>,
        {
            type Output = CB::Output;
            fn callback<P>(self, base: P) -> CB::Output
            where
                P: Producer<Item = I>,
            {
                let producer = TuplesProducer {
                    base,
                    phantom: PhantomData,
                };
                self.callback.callback(producer)
            }
        }
    }
}

struct TuplesProducer<P, T> {
    base: P,
    phantom: PhantomData<T>,
}

impl<A, P: Producer<Item = A>, T: TupleCollect<Item = A>> Producer for TuplesProducer<P, T> {
    type Item = T;

    type IntoIter = SeqTuples<P::IntoIter, T>;

    fn into_iter(self) -> Self::IntoIter {
        let mut base_iter = self.base.into_iter();
        let base_size = base_iter.len();
        let extra_elements = base_size % T::SIZE;
        // eat all extra elements
        for _ in 0..extra_elements {
            base_iter.next_back();
        }
        SeqTuples {
            base: base_iter,
            phantom: PhantomData,
        }
    }

    fn split_at(self, index: usize) -> (Self, Self) {
        let (left_producer, right_producer) = self.base.split_at(index * T::SIZE);
        (
            TuplesProducer {
                base: left_producer,
                phantom: PhantomData,
            },
            TuplesProducer {
                base: right_producer,
                phantom: PhantomData,
            },
        )
    }
}

struct SeqTuples<I, T> {
    base: I,
    phantom: PhantomData<T>,
}

impl<A, I: Iterator<Item = A>, T: TupleCollect<Item = A>> Iterator for SeqTuples<I, T> {
    type Item = T;

    fn next(&mut self) -> Option<Self::Item> {
        T::next_from_iter(&mut self.base)
    }
}

impl<A, I: Iterator<Item = A>, T: TupleCollect<Item = A>> ExactSizeIterator for SeqTuples<I, T> {}
impl<A, I: DoubleEndedIterator<Item = A>, T: TupleCollect<Item = A>> DoubleEndedIterator
    for SeqTuples<I, T>
{
    fn next_back(&mut self) -> Option<Self::Item> {
        T::next_back_from_iter(&mut self.base)
    }
}

impl<A: Send + Sized> TupleCollect for (A, A) {
    type Item = A;

    const SIZE: usize = 2;

    fn next_from_iter<I: Iterator<Item = Self::Item>>(iterator: &mut I) -> Option<Self> {
        iterator
            .next()
            .and_then(|n| iterator.next().map(|n2| (n, n2)))
    }

    fn next_back_from_iter<I: DoubleEndedIterator<Item = Self::Item>>(
        iterator: &mut I,
    ) -> Option<Self> {
        iterator
            .next_back()
            .and_then(|n2| iterator.next_back().map(|n| (n, n2)))
    }
}

impl<A: Send + Sized> TupleCollect for (A, A, A) {
    type Item = A;

    const SIZE: usize = 3;

    fn next_from_iter<I: Iterator<Item = Self::Item>>(iterator: &mut I) -> Option<Self> {
        iterator
            .next()
            .map(|n| (n, iterator.next().unwrap(), iterator.next().unwrap()))
    }

    fn next_back_from_iter<I: DoubleEndedIterator<Item = Self::Item>>(
        iterator: &mut I,
    ) -> Option<Self> {
        iterator.next_back().map(|c| {
            let b = iterator.next_back().unwrap();
            let a = iterator.next_back().unwrap();
            (a, b, c)
        })
    }
}
