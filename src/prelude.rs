use crate::{Logged, Scan};
pub use fast_tracer::svg;
pub use rayon::prelude::*;

pub trait DParallelIterator: ParallelIterator {
    /// Log each task with the `tracing` crate.
    /// You can use the `svg` function to
    /// generate a graphical display of an iterator's
    /// execution. Be careful that nested iterators need
    /// to both be logged for the display to work.
    fn log(self, label: &'static str) -> Logged<Self> {
        Logged::new(self, label)
    }

    /// Create a scan iterator.
    ///
    /// # Example
    ///
    /// ```
    /// use diam::prelude::*;
    /// let v = vec![1, 2, 3, 4, 5];
    /// let h = v
    ///     .par_windows(3)
    ///     .scan(
    ///         || None,
    ///         |state, win| {
    ///             *state = state
    ///                 .map(|s| s / 10 + win[2] * 100)
    ///                 .or_else(|| Some(win[2] * 100 + win[1] * 10 + win[0]));
    ///             *state
    ///         },
    ///     )
    ///     .collect::<Vec<u32>>();
    /// assert_eq!(h, vec![321, 432, 543]);
    /// ```
    fn scan<ID, F, U, T>(self, identity: ID, scan_op: F) -> Scan<Self, ID, F>
    where
        F: Fn(&mut U, Self::Item) -> Option<T> + Sync + Send,
        ID: Fn() -> U + Sync + Send,
        U: Send,
        T: Send,
    {
        Scan::new(self, identity, scan_op)
    }
}

impl<I: ParallelIterator> DParallelIterator for I {}
