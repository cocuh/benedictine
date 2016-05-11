use std::sync::{Mutex, Arc, Condvar};
use std::thread;

#[macro_use]
extern crate log;


#[derive(Debug,Clone)]
pub struct State {
    data: Vec<usize>,
}

impl State {
    fn new(data: Vec<usize>) -> State {
        State { data: data }
    }
    fn is_leaf(&self) -> bool {
        self.data.len() >= 3
    }
}

impl std::cmp::PartialEq for State {
    fn eq(&self, other: &State) -> bool {
        return self.data == other.data;
    }
}


#[derive(Debug)]
struct StateIterator {
    current: usize,
}

impl StateIterator {
    fn new() -> StateIterator {
        StateIterator { current: 1 }
    }

    fn next(&mut self, state: &State) -> Option<State> {
        let mut data = state.data.clone();
        match self.current {
            1...3 => {
                data.push(self.current);
                self.current += 1;
                Some(State::new(data))
            }
            _ => None,
        }
    }
}

#[derive(Debug)]
pub struct Node {
    state: State,
    iterator: Mutex<StateIterator>,
}


impl Node {
    fn new(state: State) -> Node {
        let mut node = Node {
            state: state,
            iterator: Mutex::new(StateIterator::new()),
        };
        node
    }

    fn next_child(&self) -> Option<Node> {
        let result = self.iterator.lock().unwrap().next(&self.state);
        match result {
            None => None,
            Some(state) => Some(Node::new(state)),
        }
    }

    fn is_leaf(&self) -> bool {
        self.state.is_leaf()
    }

    fn satisfy_bound(&self, bounds: Bounds) -> bool {
        true
    }
}

impl std::cmp::PartialEq for Node {
    fn eq(&self, other: &Node) -> bool {
        return self.state == other.state;
    }
}

impl std::cmp::Eq for Node {}


#[derive(Debug, Clone)]
struct Bounds {
}

impl Bounds {
    fn new() -> Bounds {
        Bounds {}
    }
}

fn remove_item<T: Eq>(nodes: &mut Vec<Arc<T>>, node: &Arc<T>) {
    match nodes.iter().position(|n| *(*n) == **node) {
        Some(idx) => {
            nodes.remove(idx);
        }
        None => {}
    }
}


fn is_all_worker_waiting(workerids: &Vec<usize>, thread_num: usize) -> bool {
    return (0..thread_num).all(|id| workerids.contains(&id));
}


impl std::fmt::Debug for Searcher {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        Ok(())
    }
}

pub struct Searcher {
    nodes: Arc<Mutex<Vec<Arc<Node>>>>,
    results: Arc<Mutex<Vec<Arc<Node>>>>,
    bounds: Arc<Mutex<Bounds>>,
    waiting_workers: Arc<Mutex<Vec<usize>>>,
    is_finished: Arc<Mutex<bool>>,
    condvar_worker: Arc<Condvar>,
}

impl Searcher {
    fn new() -> Searcher {
        let mut nodes = Vec::new();
        nodes.push(Arc::new(Node::new(State::new(Vec::new()))));
        Searcher {
            nodes: Arc::new(Mutex::new(nodes)),
            results: Arc::new(Mutex::new(Vec::new())),
            bounds: Arc::new(Mutex::new(Bounds::new())),
            waiting_workers: Arc::new(Mutex::new(Vec::new())),
            is_finished: Arc::new(Mutex::new(false)),
            condvar_worker: Arc::new(Condvar::new()),
        }
    }

    fn run(&self, thread_num: usize) {
        assert!(thread_num >= 1);

        fn push_node(condvar_worker: &Arc<Condvar>,
                     nodes: &Arc<Mutex<Vec<Arc<Node>>>>,
                     node: Arc<Node>) {
            nodes.lock().unwrap().push(node);
            condvar_worker.notify_all();
        };

        let workers = (0..thread_num)
            .map(|worker_id| {
                let nodes = self.nodes.clone();
                let results = self.results.clone();
                let bounds = self.bounds.clone();
                let waiting_workers = self.waiting_workers.clone();
                let is_finished = self.is_finished.clone();
                let condvar_worker = self.condvar_worker.clone();

                thread::spawn(move || {
                    debug!("[worker {}] start", worker_id);
                    'worker: loop {
                        let mut node;
                        'get_node: loop {
                            let mut pop_result = nodes.lock().unwrap().pop();
                            match pop_result {
                                Some(n) => {
                                    node = n;
                                    break 'get_node;
                                }
                                None => {
                                    debug!("[worker {}] no elem in queue", worker_id);
                                    waiting_workers.lock().unwrap().push(worker_id);
                                    let all_waiting = is_all_worker_waiting(&*waiting_workers.lock()
                                                                                .unwrap(),
                                                                            thread_num);
                                    if all_waiting {
                                        *is_finished.lock().unwrap() = true;
                                        condvar_worker.notify_all();
                                        debug!("[worker {}] finished first", worker_id);
                                        return;
                                    } else {
                                        condvar_worker.wait(waiting_workers.lock().unwrap());
                                        if *is_finished.lock().unwrap() {
                                            debug!("[worker {}] finished", worker_id);
                                            return;
                                        } else {
                                            debug!("[worker {}] re-start", worker_id);
                                        }
                                    }
                                }
                            }
                        } // get_node and detect is finished

                        {
                            // leaf process
                            if node.is_leaf() {
                                let mut nodes = nodes.lock().unwrap();
                                remove_item(&mut *nodes, &node);
                                results.lock().unwrap().push(node);
                                continue 'worker;
                            } else {
                                push_node(&condvar_worker, &nodes, node.clone());
                            }
                        }

                        {
                            // check bounds
                            let bounds = bounds.lock().unwrap().clone();
                            if !node.satisfy_bound(bounds) {
                                continue;
                            }
                        }

                        {
                            // generate child
                            debug!("[worker {}] next", worker_id);
                            let child = node.next_child();
                            match child {
                                None => {
                                    let mut nodes = nodes.lock().unwrap();
                                    remove_item(&mut *nodes, &node);
                                }
                                Some(child) => {
                                    push_node(&condvar_worker, &nodes, Arc::new(child));
                                }
                            }
                        }
                    }
                })
            })
            .collect::<Vec<_>>();

        for worker in workers.into_iter() {
            worker.join();
        }
    }
}


mod tests {
    use std::sync::{Mutex, Arc, Condvar};
    use super::*;
    extern crate env_logger;

    #[test]
    fn test() {
        let _ = env_logger::init();
        let mut searcher = Arc::new(Searcher::new());
        println!("{:?}", searcher);
        searcher.run(8);
        println!("{:?}", *searcher.results.lock().unwrap());
    }
}
