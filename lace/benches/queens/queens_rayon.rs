use super::{ok, SmallVec, BOUND};
use rayon::prelude::*;

fn nqueens(n: usize, j: usize, board: SmallVec<[u8; BOUND]>) -> usize {
    if j == n {
        return 1;
    }
    (0..(n as u8))
        .into_par_iter()
        .map(|i| {
            let mut tmp = board.clone();
            tmp.push(i);
            tmp
        })
        .filter(|tmp| ok(j + 1, tmp.as_ptr()))
        .map(|tmp| nqueens(n, j + 1, tmp))
        .sum()
}

pub fn run(n: usize) {
    nqueens(n, 0, SmallVec::new());
}
