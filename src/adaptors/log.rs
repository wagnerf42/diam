//! Provides logging for parallel iterators.
use rayon::iter::plumbing::*;
use rayon::iter::*;
use std::cell::Cell;
use tracing::span::{EnteredSpan, Id};

/// `Logged` is an iterator that logs all tasks created.
#[must_use = "iterator adaptors are lazy and do nothing unless consumed"]
pub struct Logged<I: ParallelIterator> {
    base: I,
}

impl<I: ParallelIterator> Logged<I> {
    /// Create a new `Logged` iterator.
    pub(crate) fn new(base: I) -> Logged<I>
    where
        I: ParallelIterator,
    {
        Logged { base }
    }
}

impl<T, I> ParallelIterator for Logged<I>
where
    I: ParallelIterator<Item = T>,
    T: Send,
{
    type Item = T;

    fn drive_unindexed<C>(self, consumer: C) -> C::Result
    where
        C: UnindexedConsumer<Self::Item>,
    {
        let start_span = tracing::span!(tracing::Level::TRACE, "drive");
        let father_id = start_span.id();
        let logged_consumer = LoggedConsumer {
            left: false,
            base: consumer,
            father_id: Cell::new(father_id.map(|id| id.into_u64())),
        };
        let _enter = start_span.enter();
        self.base.drive_unindexed(logged_consumer)
    }

    fn opt_len(&self) -> Option<usize> {
        self.base.opt_len()
    }
}

struct LoggedReducer<R> {
    base: R,
    _par_span: EnteredSpan,
    _seq_span: EnteredSpan,
}

impl<Result, R: Reducer<Result>> Reducer<Result> for LoggedReducer<R> {
    fn reduce(self, left: Result, right: Result) -> Result {
        self.base.reduce(left, right)
    }
}

/// ////////////////////////////////////////////////////////////////////////
/// Consumer implementation

struct LoggedConsumer<C> {
    base: C,
    left: bool,
    father_id: Cell<Option<u64>>,
}

impl<T, C> Consumer<T> for LoggedConsumer<C>
where
    C: Consumer<T>,
    T: Send,
{
    type Folder = LoggedFolder<C::Folder>;
    type Reducer = LoggedReducer<C::Reducer>;
    type Result = C::Result;

    fn split_at(self, index: usize) -> (Self, Self, Self::Reducer) {
        let sequential_span = if self.left {
            tracing::span!(parent: self.father_id.get().map(Id::from_u64), tracing::Level::TRACE, "left")
        } else {
            tracing::span!(parent: self.father_id.get().map(Id::from_u64), tracing::Level::TRACE, "right")
        };
        let parallel_span =
            tracing::span!(parent: sequential_span.id(), tracing::Level::TRACE, "parallel");
        let entered_sequential_span = sequential_span.entered();
        let (left, right, reducer) = self.base.split_at(index);
        (
            LoggedConsumer {
                base: left,
                left: true,
                father_id: Cell::new(parallel_span.id().map(|id| id.into_u64())),
            },
            LoggedConsumer {
                base: right,
                left: false,
                father_id: Cell::new(parallel_span.id().map(|id| id.into_u64())),
            },
            LoggedReducer {
                base: reducer,
                _seq_span: entered_sequential_span,
                _par_span: parallel_span.entered(),
            },
        )
    }

    fn into_folder(self) -> LoggedFolder<C::Folder> {
        let sequential_span = if self.left {
            tracing::span!(parent: self.father_id.get().map(Id::from_u64), tracing::Level::TRACE, "left")
        } else {
            tracing::span!(parent: self.father_id.get().map(Id::from_u64), tracing::Level::TRACE, "right")
        };
        let folder_span =
            tracing::span!(parent: sequential_span.id(), tracing::Level::TRACE, "fold");
        let entered_seq = sequential_span.entered();
        LoggedFolder {
            base: self.base.into_folder(),
            span: folder_span.entered(),
            outer_span: entered_seq,
        }
    }

    fn full(&self) -> bool {
        self.base.full()
    }
}

impl<T, C> UnindexedConsumer<T> for LoggedConsumer<C>
where
    C: UnindexedConsumer<T>,
    T: Send,
{
    fn split_off_left(&self) -> Self {
        LoggedConsumer {
            left: true,
            base: self.base.split_off_left(),
            father_id: self.father_id.clone(),
        }
    }
    fn to_reducer(&self) -> LoggedReducer<C::Reducer> {
        let sequential_span = if self.left {
            tracing::span!(parent: self.father_id.get().map(Id::from_u64), tracing::Level::TRACE, "left")
        } else {
            tracing::span!(parent: self.father_id.get().map(Id::from_u64), tracing::Level::TRACE, "right")
        };
        let entered_sequential_span = sequential_span.entered();
        let parallel_span =
            tracing::span!(parent: entered_sequential_span.id(), tracing::Level::TRACE, "parallel");
        self.father_id
            .set(parallel_span.id().map(|id| id.into_u64()));
        LoggedReducer {
            base: self.base.to_reducer(),
            _seq_span: entered_sequential_span,
            _par_span: parallel_span.entered(),
        }
    }
}

/// ////////////////////////////////////////////////////////////////////////
/// Folder implementation

struct LoggedFolder<F> {
    base: F,
    span: EnteredSpan,
    outer_span: EnteredSpan,
}

impl<T, F> Folder<T> for LoggedFolder<F>
where
    F: Folder<T>,
    T: Send,
{
    type Result = F::Result;

    fn consume(self, item: T) -> Self {
        LoggedFolder {
            base: self.base.consume(item),
            span: self.span,
            outer_span: self.outer_span,
        }
    }

    fn complete(self) -> F::Result {
        self.base.complete()
    }

    fn full(&self) -> bool {
        self.base.full()
    }
}
