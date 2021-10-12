mod adaptors;
// pub(crate) use adaptors::Adaptive;
pub(crate) use adaptors::Logged;
pub(crate) use adaptors::Scan;
pub use adaptors::{walk_tree, walk_tree_postfix, walk_tree_prefix};
pub(crate) use adaptors::{ExponentialBlocks, UniformBlocks};
pub mod prelude;

pub fn join<A, B, RA, RB>(oper_a: A, oper_b: B) -> (RA, RB)
where
    A: FnOnce() -> RA + Send,
    B: FnOnce() -> RB + Send,
    RA: Send,
    RB: Send,
{
    let span = tracing::span!(tracing::Level::TRACE, "parallel");
    let father_id = span.id();
    let right_id = father_id.clone();
    let logged_oper_a = move || {
        let a_span = tracing::span!(parent: father_id, tracing::Level::TRACE, "left");
        let _guard = a_span.enter();
        oper_a()
    };
    let logged_oper_b = move || {
        let b_span = tracing::span!(parent: right_id, tracing::Level::TRACE, "right");
        let _guard = b_span.enter();
        oper_b()
    };
    let _guard = span.enter();
    rayon::join(logged_oper_a, logged_oper_b)
}

pub use fast_tracer::{display_svg, svg};
