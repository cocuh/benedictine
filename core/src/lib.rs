use std::sync::{Mutex, Arc, Condvar};
use std::thread;

#[macro_use]
extern crate log;


pub trait Node: Send + Sync + Clone + Eq + 'static {
    fn initial() -> Self;
    fn is_leaf(&self) -> bool;
}

pub trait StateIterator<S>: Send + Sync + Clone + 'static {
    fn new() -> Self;
    fn next(&mut self, state: &S) -> Option<S>;
}
#[derive(Debug)]
pub struct Job<S, SI> {
    state: S,
    iterator: Mutex<SI>,
}


impl<S, SI> Job<S, SI>
    where S: Node,
          SI: StateIterator<S>
{
    fn new(state: S) -> Job<S, SI> {
        Job {
            state: state,
            iterator: Mutex::new(SI::new()),
        }
    }

    fn next_child(&self) -> Option<Job<S, SI>> {
        let result = self.iterator.lock().unwrap().next(&self.state);
        match result {
            None => None,
            Some(state) => Some(Job::new(state)),
        }
    }

    fn satisfy_bound(&self, bounds: Bounds) -> bool {
        true
    }
}

impl<S: Eq, SI> std::cmp::PartialEq for Job<S, SI> {
    fn eq(&self, other: &Job<S, SI>) -> bool {
        return self.state == other.state;
    }
}

impl<S: Eq, SI> std::cmp::Eq for Job<S, SI> {}


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


impl<S, SI> std::fmt::Debug for Searcher<S, SI> {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        Ok(())
    }
}

pub struct Searcher<S, SI> {
    queue: Arc<Mutex<Vec<Arc<Job<S, SI>>>>>,
    results: Arc<Mutex<Vec<S>>>,
    bounds: Arc<Mutex<Bounds>>,
    waiting_workers: Arc<Mutex<Vec<usize>>>,
    is_finished: Arc<Mutex<bool>>,
    condvar_worker: Arc<Condvar>,
}

impl<S, SI> Searcher<S, SI>
    where S: Node,
          SI: StateIterator<S>
{
    pub fn new() -> Searcher<S, SI> {
        let mut queue = Vec::new();
        queue.push(Arc::new(Job::new(S::initial())));
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

        fn push_job<S, SI>(condvar_worker: &Arc<Condvar>,
                            queue: &Arc<Mutex<Vec<Arc<Job<S, SI>>>>>,
                            job: Arc<Job<S, SI>>) {
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
                            if job.state.is_leaf() {
                                let mut queue = queue.lock().unwrap();
                                remove_item(&mut *queue, &job);
                                results.lock().unwrap().push(job.state.clone());
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

    pub fn get_results(&self) -> Vec<S> {
        self.results.lock().unwrap().clone()
    }
}


