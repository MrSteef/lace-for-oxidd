use lace::{lace_task, Worker};

#[lace_task]
pub fn run(n: usize) -> usize {
    if n < 2 {
        n
    } else {
        let (x, y) = join!(run(n - 1), run(n - 2));
        x + y
    }
}
