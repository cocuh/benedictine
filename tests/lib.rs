use std::sync::{Arc, Mutex};
use std::collections::BinaryHeap;

extern crate benedictine;
use benedictine::*;

extern crate env_logger;

#[derive(Debug,Clone)]
struct TreeBounds {
}

impl Bounds for TreeBounds {
    fn new() -> Self {
        TreeBounds{}
    }
}


#[derive(Debug,Clone)]
struct TreeNode {
    data: Vec<usize>,
    depth: usize,
}

impl TreeNode {
    fn new(data: Vec<usize>, depth: usize) -> TreeNode {
        TreeNode { data: data, depth: depth}
    }

    fn root() -> TreeNode {
        TreeNode { data: Vec::new(), depth: 0 }
    }
}

impl Node for TreeNode {
    fn is_leaf(&self) -> bool {
        self.data.len() >= 3
    }
}

impl std::cmp::PartialEq for TreeNode {
    fn eq(&self, other: &TreeNode) -> bool {
        return self.data == other.data;
    }
}

impl std::cmp::Eq for TreeNode {}

impl std::cmp::PartialOrd for TreeNode {
    fn partial_cmp(&self, other:&Self) -> Option<std::cmp::Ordering> {
        None
    }
}

impl std::cmp::Ord for TreeNode {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        self.depth.cmp(&other.depth)
    }
}


#[derive(Debug, Clone)]
pub struct TreeBranchIterator {
    current: usize,
    node: TreeNode,
}

impl BranchIterator<TreeNode> for TreeBranchIterator {
    fn new(node: &TreeNode) -> Self {
        TreeBranchIterator {
            current: 1,
            node: node.clone(),
        }
    }

    fn next(&mut self) -> Option<TreeNode> {
        let mut data = self.node.data.clone();
        match self.current {
            1...8 => {
                data.push(self.current);
                self.current += 1;
                Some(TreeNode::new(data, self.node.depth + 1))
            }
            _ => None,
        }
    }
}



#[test]
fn test() {
    let _ = env_logger::init();
    let mut searcher = Arc::new(Searcher::<TreeNode, TreeBranchIterator, TreeBounds>::new(TreeNode::root()));
    searcher.run(8);
    let results = searcher.get_results();

    assert_eq!(results.len(), 8*8*8);
}

