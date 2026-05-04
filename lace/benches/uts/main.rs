mod uts_lace;
mod uts_rayon;
mod uts_seq;

pub mod uts_common;

mod common;
use common::{time, Config};

pub fn main() {
    // parse CLI arguments
    let (mut args, cfg) = Config::read("", vec![]);
    if args.len() == 1 {
        // presets taken from the C implementation
        let new_args = match args[0].as_str() {
            "t1" => vec!["-t", "1", "-a", "3", "-d", "10", "-b", "4", "-r", "19"],
            "t2" => vec!["-t", "1", "-a", "2", "-d", "16", "-b", "6", "-r", "502"],
            "t3" => vec![
                "-t", "0", "-b", "2000", "-q", "0.124875", "-m", "8", "-r", "42",
            ],
            "t4" => vec![
                "-t", "2", "-a", "0", "-d", "16", "-b", "6", "-r", "1", "-q", "0.234375", "-m",
                "4", "-r", "1",
            ],
            "t5" => vec!["-t", "1", "-a", "0", "-d", "20", "-b", "4", "-r", "34"],
            "t1l" => vec!["-t", "1", "-a", "3", "-d", "13", "-b", "4", "-r", "29"],
            "t2l" => vec!["-t", "1", "-a", "2", "-d", "23", "-b", "7", "-r", "220"],
            "t3l" => vec![
                "-t", "0", "-b", "2000", "-q", "0.200014", "-m", "5", "-r", "7",
            ],
            "t1xl" => vec!["-t", "1", "-a", "3", "-d", "15", "-b", "4", "-r", "29"],
            "t1xxl" => vec!["-t", "1", "-a", "3", "-d", "15", "-b", "4", "-r", "19"],
            "t2xxl" => vec![
                "-t",
                "0",
                "-b",
                "2000",
                "-q",
                "0.499999995",
                "-m",
                "2",
                "-r",
                "0",
            ],
            "t3xxl" => vec![
                "-t", "0", "-b", "2000", "-q", "0.499995", "-m", "2", "-r", "316",
            ],
            "t1wl" => vec!["-t", "1", "-a", "3", "-d", "18", "-b", "4", "-r", "19"],
            "t2wl" => vec![
                "-t",
                "0",
                "-b",
                "2000",
                "-q",
                "0.4999999995",
                "-m 2",
                "-r 559",
            ],
            "t3wl" => vec![
                "-t",
                "0",
                "-b",
                "2000",
                "-q",
                "0.4999995",
                "-m",
                "2",
                "-r",
                "559",
            ],
            _ => vec![],
        };
        if !new_args.is_empty() {
            args = new_args.into_iter().map(|s| String::from(s)).collect();
        }
    }
    let group_name = format!("UTS {}", args.join(" "));
    // add a fake argv[0] back in so the C code handling this doesn't skip an argument
    args.insert(0, "FAKE_ARGV_0".to_string());
    uts_common::parse_args(args);

    cfg.seq(&group_name, || {
        time!({
            uts_seq::run();
        });
    });
    cfg.lace(&group_name, |lace_inst| {
        time!({
            lace::lace_run!(lace_inst, uts_lace::run());
        });
    });
    cfg.rayon(&group_name, || {
        time!({
            uts_rayon::run();
        });
    });
}
