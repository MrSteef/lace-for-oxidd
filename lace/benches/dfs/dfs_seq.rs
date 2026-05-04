use super::{leaf_loop, WIDTH};
use std::hint::black_box;

pub fn run(d: usize) {
    if d > 0 {
        for _ in 0..unsafe { WIDTH } {
            run(d - 1);
        }
    } else {
        black_box(unsafe { leaf_loop() });
    }
}
