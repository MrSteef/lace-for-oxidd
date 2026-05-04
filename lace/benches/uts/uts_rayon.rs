use super::uts_common::*;
use rayon::prelude::*;

fn par_tree_search(depth: usize, parent: &mut Node) -> TreeSearchResult {
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
        let mut children = Vec::with_capacity(num_children as usize);
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
            children.push(child);
        }
        let (maxdepth, size, leaves) = children
            .into_par_iter()
            .map(|mut c| {
                let r = par_tree_search(depth + 1, &mut c);
                (r.maxdepth, r.size, r.leaves)
            })
            .reduce(
                || (0, 0, 0),
                |(x, y, z), (a, b, c)| (x.max(a), y + b, z + c),
            );
        r.maxdepth = r.maxdepth.max(maxdepth);
        r.size += size;
        r.leaves += leaves;
    } else {
        r.leaves = 1;
    }
    r
}

pub fn run() {
    let mut root = Node::root(TreeType::GEO);
    par_tree_search(0, &mut root);
}
