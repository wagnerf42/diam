//! Provides logging for parallel iterators.
use rayon::iter::plumbing::*;
use rayon::iter::*;
use tracing::span::{EnteredSpan, Id};

/// `Logged` is an iterator that logs all tasks created.
#[must_use = "iterator adaptors are lazy and do nothing unless consumed"]
pub struct Logged<I: ParallelIterator> {
    base: I,
    label: &'static str,
}

impl<I: ParallelIterator> Logged<I> {
    /// Create a new `Logged` iterator.
    pub(crate) fn new(base: I, label: &'static str) -> Logged<I>
    where
        I: ParallelIterator,
    {
        Logged { base, label }
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
        let start_span = tracing::span!(tracing::Level::TRACE, "parallel");
        let father_id = start_span.id();
        let logged_consumer = LoggedConsumer {
            base: consumer,
            label: self.label,
            father_id,
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
    _span: EnteredSpan,
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
    label: &'static str,
    father_id: Option<Id>,
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
        let parallel_span =
            tracing::span!(parent: self.father_id, tracing::Level::TRACE, "parallel");
        let (left, right, reducer) = self.base.split_at(index);
        (
            LoggedConsumer {
                base: left,
                label: self.label,
                father_id: parallel_span.id(),
            },
            LoggedConsumer {
                base: right,
                label: self.label,
                father_id: parallel_span.id(),
            },
            LoggedReducer {
                base: reducer,
                _span: parallel_span.entered(),
            },
        )
    }

    fn into_folder(self) -> LoggedFolder<C::Folder> {
        let folder_span = tracing::span!(parent: self.father_id, tracing::Level::TRACE, "foo");
        LoggedFolder {
            base: self.base.into_folder(),
            span: folder_span.entered(),
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
            base: self.base.split_off_left(),
            label: self.label,
            father_id: self.father_id.clone(),
        }
    }
    fn to_reducer(&self) -> LoggedReducer<C::Reducer> {
        let parallel_span =
            tracing::span!(parent: self.father_id.clone(), tracing::Level::TRACE, "parallel");
        LoggedReducer {
            base: self.base.to_reducer(),
            _span: parallel_span.entered(),
        }
    }
}

/// ////////////////////////////////////////////////////////////////////////
/// Folder implementation

struct LoggedFolder<F> {
    base: F,
    span: EnteredSpan,
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
        }
    }

    fn complete(self) -> F::Result {
        self.base.complete()
    }

    fn full(&self) -> bool {
        self.base.full()
    }
}
