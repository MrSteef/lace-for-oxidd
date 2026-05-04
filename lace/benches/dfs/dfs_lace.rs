use super::{leaf_loop, SmallVec, WIDTH};
use lace::{lace_task, Worker};

#[lace_task]
pub fn run(d: usize) {
    if d > 0 {
        let mut tokens: SmallVec<[_; 16]> = SmallVec::new();
        for _ in 0..unsafe { WIDTH } {
            tokens.push(spawn!(run(d - 1)));
        }
        while let Some(tkn) = tokens.pop() {
            sync!(tkn);
        }
    } else {
        unsafe { leaf_loop() };
    }
}
