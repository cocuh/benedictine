use std::sync::Arc;

extern crate benedictine_core;
use benedictine_core::*;

extern crate env_logger;

#[derive(Debug,Clone)]
pub struct TreeState {
    data: Vec<usize>,
}

impl TreeState {
    fn new(data: Vec<usize>) -> TreeState {
        TreeState { data: data }
    }
}

impl State for TreeState {
    fn initial() -> TreeState {
        TreeState { data: Vec::new() }
    }

    fn is_leaf(&self) -> bool {
        self.data.len() >= 3
    }
}

impl std::cmp::PartialEq for TreeState {
    fn eq(&self, other: &TreeState) -> bool {
        return self.data == other.data;
    }
}

impl std::cmp::Eq for TreeState {}


#[derive(Debug, Clone)]
pub struct TreeStateIterator {
    current: usize,
}

impl StateIterator<TreeState> for TreeStateIterator {
    fn new() -> TreeStateIterator {
        TreeStateIterator { current: 1 }
    }

    fn next(&mut self, state: &TreeState) -> Option<TreeState> {
        let mut data = state.data.clone();
        match self.current {
            1...3 => {
                data.push(self.current);
                self.current += 1;
                Some(TreeState::new(data))
            }
            _ => None,
        }
    }
}



#[test]
fn test() {
    let _ = env_logger::init();
    let mut searcher = Arc::new(Searcher::<TreeState, TreeStateIterator>::new());
    searcher.run(8);
    let results = searcher.get_results();

    assert_eq!(results.len(), 3*3*3);
}

