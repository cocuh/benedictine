use std::sync::{Mutex, Arc, Condvar};
use std::thread;
use std::collections::HashSet;

#[macro_use]
extern crate log;


pub trait Node: Send + Sync + Clone + Eq + 'static {
    fn is_leaf(&self) -> bool;
}

pub trait BranchIterator<N>: Send + Sync + Clone + 'static {
    fn new(node: &N) -> Self;
    fn next(&mut self) -> Option<N>;
}

pub trait Bounds : Clone {
    fn new() -> Self;
}

pub trait Queue<T> {
    fn new() -> Self;
    fn enqueue(&mut self, elem: T);
    fn dequeue(&mut self) -> Option<T>;
    fn len(&self) -> usize;
}

#[derive(Clone)]
pub struct VoidBounds {}

impl Bounds for VoidBounds {
    fn new() -> Self {
        VoidBounds{}
    }
}

struct FIFOQueue<T> {
    data: Vec<T>,
}

impl<T:Eq> Queue<T> for FIFOQueue<T> {
    fn new()->Self {
        FIFOQueue {data: Vec::new()}
    }

    fn enqueue(&mut self, elem: T) {
        self.data.push(elem);
    }

    fn dequeue(&mut self) -> Option<T> {
        self.data.pop()
    }

    fn len(&self) -> usize {
        self.data.len()
    }
}

#[derive(Debug)]
pub struct Job<N, BI> {
    node: N,
    iterator: Mutex<BI>,
}


impl<N, BI> Job<N, BI>
    where N: Node,
          BI: BranchIterator<N>
{
    fn new(node: N) -> Job<N, BI> {
        let iter = BI::new(&node);
        Job {
            node: node,
            iterator: Mutex::new(iter),
        }
    }
}

impl<N: Eq, BI> std::cmp::PartialEq for Job<N, BI> {
    fn eq(&self, other: &Job<N, BI>) -> bool {
        return self.node == other.node;
    }
}

impl<N: Eq, BI> std::cmp::Eq for Job<N, BI> {}


pub struct Searcher<N, BI, B=VoidBounds> {
    queue: Arc<Mutex<FIFOQueue<Arc<Job<N, BI>>>>>,
    results: Arc<Mutex<Vec<N>>>,
    bounds: Arc<Mutex<B>>,
    waiting_workers: Arc<Mutex<HashSet<usize>>>,
    is_finished: Arc<Mutex<bool>>,
    condvar_worker: Arc<Condvar>,
}

impl<N, BI, B> Searcher<N, BI, B>
    where N: Node,
          BI: BranchIterator<N>,
          B: Bounds,
{
    pub fn new(root:N) -> Searcher<N, BI, B> {
        let mut queue = FIFOQueue::new();
        queue.enqueue(Arc::new(Job::new(root)));
        Searcher {
            queue: Arc::new(Mutex::new(queue)),
            results: Arc::new(Mutex::new(Vec::new())),
            bounds: Arc::new(Mutex::new(B::new())),
            waiting_workers: Arc::new(Mutex::new(HashSet::new())),
            is_finished: Arc::new(Mutex::new(false)),
            condvar_worker: Arc::new(Condvar::new()),
        }
    }

    pub fn run(&self, thread_num: usize) {
        assert!(thread_num >= 1);

        fn push_job<N, BI, Q: Queue<Arc<Job<N, BI>>>>
            (condvar_worker: &Arc<Condvar>,
                            queue: &Arc<Mutex<Q>>,
                            job: Arc<Job<N, BI>>) {
            queue.lock().unwrap().enqueue(job);
            condvar_worker.notify_all();
        };

        fn is_all_worker_waiting(workerids: &HashSet<usize>, thread_num: usize) -> bool {
            return (0..thread_num).all(|id| workerids.contains(&id));
        }

        let workers = (0..thread_num)
            .map(|worker_id| {
                let queue = self.queue.clone();
                let results = self.results.clone();
                let bounds = self.bounds.clone();
                let waiting_workers = self.waiting_workers.clone();
                let is_finished = self.is_finished.clone();
                let condvar_worker = self.condvar_worker.clone();

                let mut builder = thread::Builder::new();

                builder = builder.name(format!("worker {}", worker_id));

                builder.spawn(move || {
                    debug!("[worker {}] start", worker_id);
                    'worker: loop {
                        let job;
                        'get_job: loop {
                            let mut _job = queue.lock().unwrap().dequeue();
                            match _job {
                                Some(j) => {
                                    job = j;
                                    waiting_workers.lock().unwrap().remove(&worker_id);
                                    break 'get_job;
                                }
                                None => {
                                    debug!("[worker {}] no elem in queue", worker_id);
                                    waiting_workers.lock().unwrap().insert(worker_id);
                                    let all_waiting = is_all_worker_waiting(
                                            &*waiting_workers.lock().unwrap(),
                                            thread_num);
                                    if all_waiting {
                                        *is_finished.lock().unwrap() = true;
                                        condvar_worker.notify_all();
                                        debug!("[worker {}] finished", worker_id);
                                        return;
                                    } else {
                                        if *condvar_worker.wait(is_finished.lock().unwrap()).unwrap() {
                                            debug!("[worker {}] finished by notification", worker_id);
                                            return;
                                        } else {
                                            debug!("[worker {}] re-start", worker_id);
                                        }
                                    }
                                }
                            }
                        } // get_job and detect is finished

                        {
                            // leaf process
                            if job.node.is_leaf() {
                                let mut queue = queue.lock().unwrap();
                                results.lock().unwrap().push(job.node.clone());
                                continue 'worker;
                            }
                        }

                        {
                            // pruning here
                        }

                        {
                            // generate child
                            debug!("[worker {}] next", worker_id);
                            let child_node = job.iterator.lock().unwrap().next();
                            match child_node {
                                None => {
                                    // no more child node from this job.node,
                                    // so enumeration ends
                                    let mut queue = queue.lock().unwrap();
                                }
                                Some(node) => {
                                    push_job(&condvar_worker, &queue, job.clone());
                                    let child_job = Job::new(node);
                                    push_job(&condvar_worker, &queue, Arc::new(child_job));
                                }
                            }
                        }
                    }
                }).unwrap()
            })
            .collect::<Vec<_>>();

        for worker in workers.into_iter() {
            worker.join();
        }
    }

    pub fn get_results(&self) -> Vec<N> {
        self.results.lock().unwrap().clone()
    }
}

impl<N, BI, B> std::fmt::Debug for Searcher<N, BI, B> {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        Ok(())
    }
}



