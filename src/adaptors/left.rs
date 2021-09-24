use rayon::iter::plumbing::*;
use rayon::iter::*;

pub struct LeanLeft<I> {
    base: I,
    depth: usize,
}

impl<I> LeanLeft<I> {
    pub(crate) fn new(base: I, depth: usize) -> Self {
        LeanLeft { base, depth }
    }
}

impl<I: ParallelIterator> ParallelIterator for LeanLeft<I> {
    type Item = I::Item;
    fn drive_unindexed<C: UnindexedConsumer<Self::Item>>(self, consumer: C) -> C::Result {
        unimplemented!()
    }
}
