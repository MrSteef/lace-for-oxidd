use super::{ok, SmallVec, BOUND};

fn nqueens(n: usize, j: usize, board: SmallVec<[u8; BOUND]>) -> usize {
    if j == n {
        return 1;
    }
    let mut count = 0;
    for i in 0..(n as u8) {
        let mut tmp = board.clone();
        tmp.push(i);
        if ok(j + 1, tmp.as_ptr()) {
            count += nqueens(n, j + 1, tmp);
        }
    }
    count
}

pub fn run(n: usize) {
    nqueens(n, 0, SmallVec::new());
}
