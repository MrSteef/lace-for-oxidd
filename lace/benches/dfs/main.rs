mod dfs_lace;
mod dfs_rayon;
mod dfs_seq;

mod common;
use common::{time, Config};

extern crate smallvec;
pub use smallvec::SmallVec;

pub fn main() {
    // parse CLI arguments
    let usage_str = "[-D <depth>] [-W <width>] [-G <granularity>]";
    let (args, cfg) = Config::read(usage_str, vec!["-D", "-W", "-G"]);

    let mut depth: usize = 10;
    let mut width: usize = 5;
    let mut grain: usize = 100;
    let mut i = 0;
    while i < args.len() {
        match args[i].as_str() {
            "-D" => {
                if let Some(n) = args.get(i + 1) {
                    if let Ok(n) = n.parse::<usize>() {
                        i += 1;
                        depth = n;
                    } else {
                        println!("Expected <depth> number after '-d'");
                        Config::show_help(usage_str)
                    }
                } else {
                    println!("Expected <depth> after '-d'");
                    Config::show_help(usage_str)
                }
            }
            "-W" => {
                if let Some(n) = args.get(i + 1) {
                    if let Ok(n) = n.parse::<usize>() {
                        i += 1;
                        width = n;
                    } else {
                        println!("Expected <width> number after '-w'");
                        Config::show_help(usage_str)
                    }
                } else {
                    println!("Expected <width> after '-w'");
                    Config::show_help(usage_str)
                }
            }
            "-G" => {
                if let Some(n) = args.get(i + 1) {
                    if let Ok(n) = n.parse::<usize>() {
                        i += 1;
                        grain = n;
                    } else {
                        println!("Expected <granularity> number after '-g'");
                        Config::show_help(usage_str)
                    }
                } else {
                    println!("Expected <granularity> after '-g'");
                    Config::show_help(usage_str)
                }
            }
            _ => panic!("Internal Error: superfluous parameter passed through"),
        }
        i += 1;
    }

    let group_name = format!("DFS d:{}, w:{}, g:{}", depth, width, grain);
    unsafe {
        WIDTH = width;
        GRAIN = grain;
    }

    cfg.seq(&group_name, || {
        time!({
            dfs_seq::run(depth);
        });
    });
    cfg.lace(&group_name, |lace_inst| {
        time!({
            lace::lace_run!(lace_inst, dfs_lace::run(depth));
        });
    });
    cfg.rayon(&group_name, || {
        time!({
            dfs_rayon::run(depth);
        });
    });
}

// common between the three versions
pub static mut WIDTH: usize = 0;
#[no_mangle]
pub static mut GRAIN: usize = 0;

extern crate cty;
use cty::c_int;
#[link(name = "dfs_c", kind = "static")]
extern "C" {
    fn leaf_loop() -> c_int;
}
