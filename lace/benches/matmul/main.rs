mod matmul_lace;
mod matmul_rayon;
mod matmul_seq;

mod common;
use common::{time, Config};

pub type REAL = f32;
pub fn main() {
    // parse CLI arguments
    let (cfg, n) = Config::read1_or::<usize>("<n>", 2048);
    let group_name = format!("Matmul {}", n);
    let a: Vec<REAL> = vec![1.0; n * n];
    let b: Vec<REAL> = vec![1.0; n * n];
    let mut output: Vec<REAL> = vec![0.0; n * n];

    cfg.seq(&group_name, || {
        time!({
            matmul_seq::run(n, &a, &b, &mut output);
        });
    });
    cfg.lace(&group_name, |lace_inst| {
        time!({
            lace::lace_run!(lace_inst, matmul_lace::run(n, &a, &b, &mut output));
        });
    });
    cfg.rayon(&group_name, || {
        time!({
            matmul_rayon::run(n, &a, &b, &mut output);
        });
    });
}

// common between the three versions
extern crate cty;
use cty::{c_float, c_int};
#[link(name = "matmul_c", kind = "static")]
extern "C" {
    fn task_body_add(
        a: *const c_float,
        b: *const c_float,
        c: *mut c_float,
        m: c_int,
        n: c_int,
        p: c_int,
        ld: c_int,
    );
    fn task_body_noadd(
        a: *const c_float,
        b: *const c_float,
        c: *mut c_float,
        m: c_int,
        n: c_int,
        p: c_int,
        ld: c_int,
    );
}
#[inline(always)]
fn task_body(
    a: &[REAL],
    b: &[REAL],
    c: *mut REAL,
    m: usize,
    n: usize,
    p: usize,
    ld: usize,
    add: bool,
) {
    if add {
        unsafe {
            task_body_add(
                a.as_ptr(),
                b.as_ptr(),
                c,
                m as c_int,
                n as c_int,
                p as c_int,
                ld as c_int,
            );
        }
    } else {
        unsafe {
            task_body_noadd(
                a.as_ptr(),
                b.as_ptr(),
                c,
                m as c_int,
                n as c_int,
                p as c_int,
                ld as c_int,
            );
        }
    }
}
