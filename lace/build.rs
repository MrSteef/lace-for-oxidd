// Example custom build script.
fn main() {
    // the C bindings are only needed when benchmarking,
    // there doesn't yet seem to be a good way to detect "cargo bench"
    // mode so we could turn off compilation for this...

    let files = vec!["benches/uts/c/uts.c", "benches/uts/c/rng/brg_sha1.c"];
    for file in &files {
        println!("cargo::rerun-if-changed={file}");
    }
    cc::Build::new()
        .files(files)
        .define("BRG_C99_TYPES", None)
        .define("BRG_RNG", None)
        .static_flag(true)
        .compile("uts_c");

    let files = vec!["benches/dfs/leaf_loop.c"];
    for file in &files {
        println!("cargo::rerun-if-changed={file}");
    }
    cc::Build::new()
        .files(files)
        .static_flag(true)
        .compile("dfs_c");

    let files = vec!["benches/queens/ok.c"];
    for file in &files {
        println!("cargo::rerun-if-changed={file}");
    }
    cc::Build::new()
        .files(files)
        .static_flag(true)
        .compile("queens_c");

    let files = vec!["benches/matmul/task_body.c"];
    for file in &files {
        println!("cargo::rerun-if-changed={file}");
    }
    cc::Build::new()
        .files(files)
        .static_flag(true)
        .compile("matmul_c");
}
