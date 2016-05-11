use std::sync::{Mutex, Arc, Condvar};
use std::thread;

#[macro_use]
extern crate log;


pub trait Node: Send + Sync + Clone + Eq + 'static {
    fn root() -> Self;
    fn is_leaf(&self) -> bool;
}

pub trait BranchIterator<N>: Send + Sync + Clone + 'static {
    fn new() -> Self;
    fn next(&mut self, node: &N) -> Option<N>;
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

    fn next_child(&self) -> Option<Job<N, BI>> {
        let result = self.iterator.lock().unwrap().next(&self.node);
        match result {
            None => None,
            Some(node) => Some(Job::new(node)),
        }
    }

    fn satisfy_bound(&self, bounds: Bounds) -> bool {
        true
    }
}

impl<N: Eq, BI> std::cmp::PartialEq for Job<N, BI> {
    fn eq(&self, other: &Job<N, BI>) -> bool {
        return self.node == other.node;
    }
}

impl<N: Eq, BI> std::cmp::Eq for Job<N, BI> {}


#[derive(Debug, Clone)]
struct Bounds {
}

impl Bounds {
    fn new() -> Bounds {
        Bounds {}
    }
}

fn remove_item<T: Eq>(jobs: &mut Vec<Arc<T>>, job: &Arc<T>) {
    match jobs.iter().position(|n| *(*n) == **job) {
        Some(idx) => {
            jobs.remove(idx);
        }
        None => {}
    }
}


fn is_all_worker_waiting(workerids: &Vec<usize>, thread_num: usize) -> bool {
    return (0..thread_num).all(|id| workerids.contains(&id));
}


impl<N, BI> std::fmt::Debug for Searcher<N, BI> {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        Ok(())
    }
}

pub struct Searcher<N, BI> {
    queue: Arc<Mutex<Vec<Arc<Job<N, BI>>>>>,
    results: Arc<Mutex<Vec<N>>>,
    bounds: Arc<Mutex<Bounds>>,
    waiting_workers: Arc<Mutex<Vec<usize>>>,
    is_finished: Arc<Mutex<bool>>,
    condvar_worker: Arc<Condvar>,
}

impl<N, BI> Searcher<N, BI>
    where N: Node,
          BI: BranchIterator<N>
{
    pub fn new() -> Searcher<N, BI> {
        let mut queue = Vec::new();
        queue.push(Arc::new(Job::new(N::root())));
        Searcher {
            queue: Arc::new(Mutex::new(queue)),
            results: Arc::new(Mutex::new(Vec::new())),
            bounds: Arc::new(Mutex::new(Bounds::new())),
            waiting_workers: Arc::new(Mutex::new(Vec::new())),
            is_finished: Arc::new(Mutex::new(false)),
            condvar_worker: Arc::new(Condvar::new()),
        }
    }

    pub fn run(&self, thread_num: usize) {
        assert!(thread_num >= 1);

        fn push_job<N, BI>(condvar_worker: &Arc<Condvar>,
                            queue: &Arc<Mutex<Vec<Arc<Job<N, BI>>>>>,
                            job: Arc<Job<N, BI>>) {
            queue.lock().unwrap().push(job);
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
                        let mut job;
                        'get_job: loop {
                            let mut _job = queue.lock().unwrap().pop();
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
                        } // get_job and detect is finished

                        {
                            // leaf process
                            if job.node.is_leaf() {
                                let mut queue = queue.lock().unwrap();
                                remove_item(&mut *queue, &job);
                                results.lock().unwrap().push(job.node.clone());
                                continue 'worker;
                            } else {
                                push_job(&condvar_worker, &queue, job.clone());
                            }
                        }

                        {
                            // check bounds
                            let bounds = bounds.lock().unwrap().clone();
                            if !job.satisfy_bound(bounds) {
                                continue;
                            }
                        }

                        {
                            // generate child
                            debug!("[worker {}] next", worker_id);
                            let child = job.next_child();
                            match child {
                                None => {
                                    let mut queue = queue.lock().unwrap();
                                    remove_item(&mut *queue, &job);
                                }
                                Some(child) => {
                                    push_job(&condvar_worker, &queue, Arc::new(child));
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


