use diam::prelude::*;
use fast_tracer::svg;

fn main() {
    svg("sum.svg", || {
        let s = (0..1000u32)
            .into_par_iter()
            .map(|e| e * 2)
            .log()
            .sum::<u32>();
        assert_eq!(s, 1000 * 999);
    })
    .expect("failed saving log");
}
