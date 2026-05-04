use super::uts_common::*;
use lace::{lace_task, Worker};
extern crate smallvec;
use smallvec::SmallVec;

#[lace_task]
fn par_tree_search(depth: usize, mut parent: Node) -> TreeSearchResult {
    let mut r = TreeSearchResult {
        maxdepth: depth,
        size: 1,
        leaves: 0,
    };
    let parent_height = parent.height;
    let num_children = parent.num_children();
    let child_type = parent.child_type();
    parent.num_children = num_children;

    if num_children > 0 {
        let mut tokens: SmallVec<[_; 10]> = SmallVec::new();
        // let mut tokens = Vec::with_capacity(num_children as usize);
        for i in 0..num_children {
            let mut child: Node = Node {
                _type: child_type,
                height: parent_height + 1,
                num_children: c_int::MAX,
                state: Default::default(),
            };
            for _ in 0..unsafe { computeGranularity } {
                unsafe { rng_spawn(&mut parent.state, &mut child.state, i) };
            }
            tokens.push(spawn!(par_tree_search(depth + 1, child)));
        }
        while let Some(tkn) = tokens.pop() {
            let c = sync!(tkn);
            if c.maxdepth > r.maxdepth {
                r.maxdepth = c.maxdepth;
            }
            r.size += c.size;
            r.leaves += c.leaves;
        }
    } else {
        r.leaves = 1;
    }
    r
}

#[lace_task]
pub fn run() {
    let root = Node::root(TreeType::GEO);
    call!(par_tree_search(0, root));
}
