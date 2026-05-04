mod fib_lace;
mod fib_rayon;
mod fib_seq;

mod common;
use common::{time, Config};

pub fn main() {
    // parse CLI arguments
    let (cfg, n) = Config::read1_or::<usize>("<n>", 40);
    let group_name = format!("Fibonacci {}", n);

    // calculate the correct answer
    let mut answer = 1;
    let mut x = 1;
    for _ in 1..n {
        let y = answer + x;
        answer = x;
        x = y;
    }

    cfg.seq(&group_name, || {
        time!({
            let x = fib_seq::run(n);
            assert_eq!(x, answer);
        });
    });
    cfg.lace(&group_name, |lace_inst| {
        time!({
            let x = lace::lace_run!(lace_inst, fib_lace::run(n));
            assert_eq!(x, answer);
        });
    });
    cfg.rayon(&group_name, || {
        time!({
            let x = fib_rayon::run(n);
            assert_eq!(x, answer);
        });
    });
}
