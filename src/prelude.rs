pub use crate::{walk_tree, walk_tree_postfix, walk_tree_prefix};
use crate::{ExponentialBlocks, Logged, Scan, UniformBlocks};
pub use rayon::prelude::*;

pub trait DParallelIterator: ParallelIterator {
    /// Log each task with the `tracing` crate.
    /// You can use the `svg` function to
    /// generate a graphical display of an iterator's
    /// execution. Be careful that nested iterators need
    /// to both be logged for the display to work.
    ///
    /// # Example
    ///
    /// ```no_run
    /// use diam::prelude::*;
    /// svg("collect.svg",||(0..1000).into_par_iter().log("collect").collect::<Vec<_>>());
    /// ```
    fn log(self) -> Logged<Self> {
        Logged::new(self)
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

pub trait DIndexedParallelIterator: IndexedParallelIterator {
    /// Normally, parallel iterators are recursively divided into tasks in parallel.
    /// This adaptor changes the default behavior by splitting the iterator into a **sequence**
    /// of parallel iterators of increasing sizes.
    /// Sizes grow exponentially in order to avoid creating
    /// too many blocks. This also allows to balance the current block with all previous ones.
    ///
    /// This can have many applications but the most notable ones are:
    /// - better performances with [`find_first()`]
    /// - more predictable performances with [`find_any()`] or any interruptible computation
    ///
    /// [`find_first()`]: trait.ParallelIterator.html#method.find_first
    /// [`find_any()`]: trait.ParallelIterator.html#method.find_any
    ///
    /// # Examples
    ///
    /// ```
    /// use diam::prelude::*;
    /// assert_eq!((0..10_000).into_par_iter()
    ///                       .by_exponential_blocks()
    ///                       .find_first(|&e| e==4_999), Some(4_999))
    /// ```
    ///
    /// In this example, without blocks, rayon will split the initial range into two but all work
    /// on the right hand side (from 5,000 onwards) is **useless** since the sequential algorithm
    /// never goes there. This means that if two threads are used there will be **no** speedup **at
    /// all**.
    /// `by_exponential_blocks` on the other hand will start with the leftmost range from 0 to `p` (threads number), continue
    /// with p to 3p, the 3p to 7p...
    /// Each subrange is treated in parallel, while all subranges are treated sequentially.
    /// We therefore ensure a logarithmic number of blocks (and overhead) while guaranteeing
    /// we stop at the first block containing the searched data.
    fn by_exponential_blocks(self) -> ExponentialBlocks<Self> {
        ExponentialBlocks::new(self)
    }

    /// Normally, parallel iterators are recursively divided into tasks in parallel.
    /// This adaptor changes the default behavior by splitting the iterator into a **sequence**
    /// of parallel iterators of given `blocks_size`.
    /// The main application is to obtain better
    /// memory locality (especially if the reduce operation re-use folded data).
    /// # Example
    /// ```
    /// use diam::prelude::*;
    /// // during most reductions v1 and v2 fit the cache
    /// let v = (0u32..10_000_000)
    ///     .into_par_iter()
    ///     .by_uniform_blocks(1_000_000)
    ///     .fold(Vec::new, |mut v, e| { v.push(e); v})
    ///     .reduce(Vec::new, |mut v1, mut v2| { v1.append(&mut v2); v1});
    /// assert_eq!(v, (0u32..10_000_000).collect::<Vec<u32>>());
    /// ```
    fn by_uniform_blocks(self, blocks_size: usize) -> UniformBlocks<Self> {
        UniformBlocks::new(self, blocks_size)
    }
}

impl<I: IndexedParallelIterator> DIndexedParallelIterator for I {}
