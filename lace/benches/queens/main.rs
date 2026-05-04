mod queens_lace;
mod queens_rayon;
mod queens_seq;

mod common;
use common::{time, Config};

extern crate smallvec;
pub use smallvec::SmallVec;
// used for SmallVec size
pub const BOUND: usize = 32;

pub fn main() {
    // parse CLI arguments
    let (cfg, n) = Config::read1_or::<usize>("<n>", 13);
    let group_name = format!("N-Queens {}", n);
    assert!(n < BOUND, "Increase BOUND");

    cfg.seq(&group_name, || {
        time!({
            queens_seq::run(n);
        });
    });
    cfg.lace(&group_name, |lace_inst| {
        time!({
            lace::lace_run!(lace_inst, queens_lace::run(n));
        });
    });
    cfg.rayon(&group_name, || {
        time!({
            queens_rayon::run(n);
        });
    });
}

extern crate cty;
use cty::c_int;
#[link(name = "dfs_c", kind = "static")]
extern "C" {
    fn queens_ok(n: c_int, a: *const u8) -> c_int;
}
#[inline(always)]
fn ok(n: usize, board: *const u8) -> bool {
    1 == unsafe { queens_ok(n as c_int, board) }
}
