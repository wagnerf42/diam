use diam::prelude::*;

fn main() {
    diam::display_svg(|| {
        let s = (0..1_000_000u32)
            .into_par_iter()
            .map(|e| vec![e * 2])
            .log()
            .reduce(Vec::new, |mut a, mut b| {
                a.append(&mut b);
                a
            });
        assert_eq!(s.len(), 1_000_000);
    })
    .expect("failed saving log");
}