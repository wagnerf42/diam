use diam::prelude::*;
use rayon::prelude::*;

fn main() {
    let v = vec![1, 2, 3, 4, 5];
    let h = v
        .par_windows(3)
        .scan(
            || None,
            |state, win| {
                *state = state
                    .map(|s| s / 10 + win[2] * 100)
                    .or_else(|| Some(win[2] * 100 + win[1] * 10 + win[0]));
                *state
            },
        )
        .collect::<Vec<u32>>();
    assert_eq!(h, vec![321, 432, 543]);
}
