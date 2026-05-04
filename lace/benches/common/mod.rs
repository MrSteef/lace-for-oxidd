/* Common functionality for all the benchmarks,
 * accessed through a symlink (hencewhy this is mod.rs not common.rs)
 */
use lace::Lace;

pub const TIME_REPS: usize = 20;
#[allow(unused_macros)] // not *actually* unused
macro_rules! time {
    ($stmt:block) => {
        let mut average = std::time::Duration::new(0, 0);
        const N: usize = crate::common::TIME_REPS;
        for i in 0..N {
            let start = std::time::Instant::now();
            std::hint::black_box($stmt);
            let delta = start.elapsed();
            println!(
                "{}/{}: {}",
                i + 1,
                N,
                delta.as_secs() as f64 + delta.subsec_nanos() as f64 / 1_000_000_000.0
            );
            average += delta;
        }
        average /= N as u32;
        println!(
            "average of {N}: {:.3?}",
            average.as_secs() as f64 + average.subsec_nanos() as f64 / 1_000_000_000.0
        );
    };
}
#[allow(unused_imports)] // re-exporting
pub(crate) use time;

const VERSION_SEQ: usize = 1 << 0;
const VERSION_LACE: usize = 1 << 1;
const VERSION_RAYON: usize = 1 << 2;
pub struct Config {
    versions: usize,
    workers: Vec<usize>,
}
use std::str::FromStr;
impl Config {
    pub fn show_help(usage: &str) -> ! {
        println!(
            "usage: {} [-w <workers>] [-seq] [-lace] [-rayon] {}",
            std::env::args()
                .collect::<Vec<String>>()
                .get(0)
                .expect("Could not Get argv[0] while displaying help message"),
            usage
        );
        std::process::exit(1);
    }
    pub fn read(usage: &str, impl_params: Vec<&str>) -> (Vec<String>, Self) {
        let mut cfg = Self {
            versions: 0,
            workers: Vec::new(),
        };
        let argv: Vec<String> = std::env::args().collect();
        let mut leftover: Vec<String> = Vec::new();
        let mut i = 1;
        while i < argv.len() {
            // filter out cargo arguments like "-bench"
            if argv[i].starts_with("--") {
                i += 1;
                continue;
            }
            match argv[i].as_str() {
                "-w" => {
                    i += 1;
                    cfg.workers.push(
                        argv.get(i)
                            .expect("Expected worker count after '-w'")
                            .parse::<usize>()
                            .expect("Expected number for '-w'"),
                    );
                }
                "-seq" => cfg.versions |= VERSION_SEQ,
                "-lace" => cfg.versions |= VERSION_LACE,
                "-rayon" => cfg.versions |= VERSION_RAYON,
                x => {
                    if impl_params.is_empty() {
                        leftover.push(String::from(x));
                    } else if impl_params.contains(&x) {
                        leftover.push(String::from(x));
                        i += 1;
                        if let Some(xval) = argv.get(i) {
                            leftover.push(String::from(xval));
                        }
                    } else {
                        Self::show_help(usage);
                    }
                }
            }
            i += 1;
        }
        // default values
        if cfg.versions == 0 {
            cfg.versions = VERSION_SEQ | VERSION_LACE | VERSION_RAYON;
        }
        if cfg.workers.is_empty() {
            cfg.workers = vec![1, 2, 4, 8];
        }
        (leftover, cfg)
    }
    #[allow(dead_code)] // only unused by *some* of the versions
    pub fn read1_or<T: FromStr>(usage: &str, default: T) -> (Self, T) {
        let (leftover, cfg) = Self::read(usage, vec![]);
        if leftover.len() == 1 {
            if let Ok(x) = leftover[0].parse::<T>() {
                return (cfg, x);
            }
        }
        (cfg, default)
    }

    pub fn seq(&self, name: &String, mut f: impl FnMut()) {
        if (self.versions & VERSION_SEQ) != 0 {
            println!("{name} / Sequential");
            f();
        }
    }
    pub fn lace(&self, name: &String, mut f: impl FnMut(&mut Lace)) {
        if (self.versions & VERSION_LACE) != 0 {
            for &wcount in &self.workers {
                println!("{name} / Lace({wcount})");
                let mut inst = lace::Lace::init(wcount);
                f(&mut inst);
                #[cfg(feature = "metrics")]
                inst.summarize(TIME_REPS);
            }
        }
    }
    pub fn rayon(&self, name: &String, mut f: impl FnMut() + Send) {
        if (self.versions & VERSION_RAYON) != 0 {
            for &wcount in &self.workers {
                extern crate rayon;
                println!("{name} / Rayon({wcount})");
                rayon::ThreadPoolBuilder::new()
                    .num_threads(wcount)
                    .stack_size(1024 * 1024 * 64)
                    .build()
                    .expect("Failed To Create Rayon Thread Pool")
                    .install(|| {
                        f();
                    });
            }
        }
    }
}
