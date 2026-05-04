use super::{ok, SmallVec, BOUND};
use lace::{lace_task, Worker};

#[lace_task]
fn nqueens(n: usize, j: usize, board: SmallVec<[u8; BOUND]>) -> usize {
    if j == n {
        return 1;
    }
    let mut tokens: SmallVec<[_; BOUND]> = SmallVec::new();
    for i in 0..(n as u8) {
        let mut tmp = board.clone();
        tmp.push(i);
        if ok(j + 1, tmp.as_ptr()) {
            let tkn = spawn!(nqueens(n, j + 1, tmp));
            tokens.push(tkn);
        }
    }
    let mut count = 0;
    while let Some(tkn) = tokens.pop() {
        count += sync!(tkn);
    }
    count
}

#[lace_task]
pub fn run(n: usize) {
    call!(nqueens(n, 0, SmallVec::new()));
}
