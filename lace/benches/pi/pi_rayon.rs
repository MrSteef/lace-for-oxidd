use super::{c_double, rng_refmut, RAND_MAX};
use std::cell::RefCell;

thread_local! {
    static SEED: RefCell<u32> = RefCell::new(0);
}

fn pi_mc(start: usize, cnt: usize) -> u64 {
    if cnt == 1 {
        SEED.with(|seed| {
            let mut seed = seed.borrow_mut();
            if *seed == 0 {
                *seed = 123;
            }
            let x: c_double =
                (rng_refmut(&mut seed, RAND_MAX) as c_double) / (RAND_MAX as c_double);
            let y: c_double =
                (rng_refmut(&mut seed, RAND_MAX) as c_double) / (RAND_MAX as c_double);
            if (x * x + y * y).sqrt() < 1.0 {
                1
            } else {
                0
            }
        })
    } else {
        let (a, b) = rayon::join(
            || pi_mc(start, cnt / 2),
            || pi_mc(start + cnt / 2, (cnt + 1) / 2),
        );
        a + b
    }
}

pub fn run(n: usize) -> c_double {
    let x = pi_mc(0, n);
    let pi: c_double = 4.0 * (x as c_double / n as c_double);
    pi
}
