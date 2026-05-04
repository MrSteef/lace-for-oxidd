mod pi_lace;
mod pi_rayon;
mod pi_seq;

mod common;
use common::{time, Config};
use std::hint::black_box;

pub fn main() {
    // parse CLI arguments
    let (cfg, n) = Config::read1_or::<usize>("<n>", 100000000);
    let group_name = format!("Pi {}", n);

    cfg.seq(&group_name, || {
        time!({
            black_box(pi_seq::run(n));
        });
    });
    cfg.lace(&group_name, |lace_inst| {
        time!({
            lace::lace_run!(lace_inst, pi_lace::run(n));
        });
    });
    cfg.rayon(&group_name, || {
        time!({
            pi_rayon::run(n);
        });
    });
}

// common values
extern crate cty;
pub use cty::c_double;
use cty::c_int;
const RAND_MAX: c_int = 2147483647;
#[inline(always)]
pub fn rng(seed: &mut u32, max: c_int) -> u32 {
    let mut next: u32 = *seed;
    next *= 1103515245;
    next += 12345;
    *seed = next;
    next % (max as u32)
}
#[inline(always)]
pub fn rng_refmut(seed: &mut std::cell::RefMut<u32>, max: c_int) -> u32 {
    let mut next: u32 = **seed;
    next *= 1103515245;
    next += 12345;
    **seed = next;
    next % (max as u32)
}
