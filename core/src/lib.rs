use std::sync::{Mutex, Arc, Condvar};
use std::thread;

#[macro_use]
extern crate log;


pub trait Node: Send + Sync + Clone + Eq + 'static {
    fn is_leaf(&self) -> bool;
}

pub trait BranchIterator<N>: Send + Sync + Clone + 'static {
    fn new() -> Self;
    fn next(&mut self, node: &N) -> Option<N>;
}

pub trait Bounds : Clone {
    fn new() -> Self;
}

pub trait Queue<T> {
    fn new() -> Self;
    fn enqueue(&mut self, elem: T);
    fn dequeue(&mut self) -> Option<T>;
    fn remove(&mut self, elem: &T);
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

    fn remove(&mut self, elem: &T) {
        match self.data.iter().position(|n| *n == *elem) {
            Some(idx) => {
                self.data.remove(idx);
            }
            None => {}
        }
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
        Job {
            node: node,
            iterator: Mutex::new(BI::new()),
        }
    }
}

impl<N: Eq, BI> std::cmp::PartialEq for Job<N, BI> {
    fn eq(&self, other: &Job<N, BI>) -> bool {
        return self.node == other.node;
    }
}

impl<N: Eq, BI> std::cmp::Eq for Job<N, BI> {}


fn is_all_worker_waiting(workerids: &Vec<usize>, thread_num: usize) -> bool {
    return (0..thread_num).all(|id| workerids.contains(&id));
}


pub struct Searcher<N, BI, B=VoidBounds> {
    queue: Arc<Mutex<FIFOQueue<Arc<Job<N, BI>>>>>,
    results: Arc<Mutex<Vec<N>>>,
    bounds: Arc<Mutex<B>>,
    waiting_workers: Arc<Mutex<Vec<usize>>>,
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
            waiting_workers: Arc::new(Mutex::new(Vec::new())),
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

        let workers = (0..thread_num)
            .map(|worker_id| {
                let queue = self.queue.clone();
                let results = self.results.clone();
                let bounds = self.bounds.clone();
                let waiting_workers = self.waiting_workers.clone();
                let is_finished = self.is_finished.clone();
                let condvar_worker = self.condvar_worker.clone();

                thread::spawn(move || {
                    debug!("[worker {}] start", worker_id);
                    'worker: loop {
                        let job;
                        'get_job: loop {
                            let mut _job = queue.lock().unwrap().dequeue();
                            match _job {
                                Some(j) => {
                                    job = j;
                                    break 'get_job;
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
                                        if *condvar_worker.wait(is_finished.lock().unwrap()).unwrap() {
                                            debug!("[worker {}] finished", worker_id);
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
                                queue.remove(&job);
                                results.lock().unwrap().push(job.node.clone());
                                continue 'worker;
                            } else {
                                push_job(&condvar_worker, &queue, job.clone());
                            }
                        }

                        {
                            // pruning here
                        }

                        {
                            // generate child
                            debug!("[worker {}] next", worker_id);
                            let child_node = job.iterator.lock().unwrap().next(&job.node);
                            match child_node {
                                None => {
                                    // no more child node from this job.node,
                                    // so enumeration ends
                                    let mut queue = queue.lock().unwrap();
                                    queue.remove(&job);
                                }
                                Some(node) => {
                                    let child_job = Job::new(node);
                                    push_job(&condvar_worker, &queue, Arc::new(child_job));
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

    pub fn get_results(&self) -> Vec<N> {
        self.results.lock().unwrap().clone()
    }
}

impl<N, BI, B> std::fmt::Debug for Searcher<N, BI, B> {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        Ok(())
    }
}



