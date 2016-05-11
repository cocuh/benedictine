use std::sync::Arc;

extern crate benedictine_core;
use benedictine_core::*;

extern crate env_logger;

#[derive(Debug,Clone)]
pub struct TreeNode {
    data: Vec<usize>,
}

impl TreeNode {
    fn new(data: Vec<usize>) -> TreeNode {
        TreeNode { data: data }
    }
}

impl Node for TreeNode {
    fn root() -> TreeNode {
        TreeNode { data: Vec::new() }
    }

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


#[derive(Debug, Clone)]
pub struct TreeBranchIterator {
    current: usize,
}

impl BranchIterator<TreeNode> for TreeBranchIterator {
    fn new() -> Self {
        TreeBranchIterator { current: 1 }
    }

    fn next(&mut self, state: &TreeNode) -> Option<TreeNode> {
        let mut data = state.data.clone();
        match self.current {
            1...3 => {
                data.push(self.current);
                self.current += 1;
                Some(TreeNode::new(data))
            }
            _ => None,
        }
    }
}



#[test]
fn test() {
    let _ = env_logger::init();
    let mut searcher = Arc::new(Searcher::<TreeNode, TreeBranchIterator>::new());
    searcher.run(8);
    let results = searcher.get_results();

    assert_eq!(results.len(), 3*3*3);
}

