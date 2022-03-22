use diam::prelude::*;
use rayon::prelude::*;

fn main() {
    svg("sum.svg", || {
        let s = (0..1_000u32)
            .into_par_iter()
            .map(|e| e * 2)
            .log("sum")
            .sum::<u32>();
        assert_eq!(s, 1000 * 999);
    })
    .expect("failed saving log");
}
