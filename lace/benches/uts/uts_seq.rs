use super::uts_common::*;

fn tree_search(depth: usize, parent: &mut Node) -> TreeSearchResult {
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
            let c = tree_search(depth + 1, &mut child);

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

pub fn run() {
    let mut root = Node::root(TreeType::GEO);
    tree_search(0, &mut root);
}
