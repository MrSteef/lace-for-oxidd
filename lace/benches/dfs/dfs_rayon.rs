use super::{leaf_loop, WIDTH};
use rayon::prelude::*;

pub fn run(d: usize) {
    if d > 0 {
        let _ = (0..unsafe { WIDTH })
            .into_par_iter()
            .for_each(|_| run(d - 1));
    } else {
        unsafe { leaf_loop() };
    }
}
