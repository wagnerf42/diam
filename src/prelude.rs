use crate::Logged;
pub use fast_tracer::svg;
pub use rayon::prelude::*;

pub trait DParallelIterator: ParallelIterator {
    fn log(self, label: &'static str) -> Logged<Self> {
        Logged::new(self, label)
    }
}

impl<I: ParallelIterator> DParallelIterator for I {}
