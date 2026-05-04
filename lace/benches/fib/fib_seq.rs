pub fn run(n: usize) -> usize {
    if n < 2 {
        n
    } else {
        run(n - 1) + run(n - 2)
    }
}
