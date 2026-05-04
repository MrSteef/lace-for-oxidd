use super::{c_double, rng, RAND_MAX};

pub fn run(n: usize) -> c_double {
    let mut count = 0;
    let mut seed = 1234321;
    for _ in 0..n {
        let x: c_double = (rng(&mut seed, RAND_MAX) as c_double) / (RAND_MAX as c_double);
        let y: c_double = (rng(&mut seed, RAND_MAX) as c_double) / (RAND_MAX as c_double);
        if (x * x + y * y).sqrt() < 1.0 {
            count += 1;
        }
    }
    4.0 * (count as c_double) / (n as c_double)
}
