//! Background asset loading: keys requested on the main thread load on worker threads
//! (extraction, file reads, parsing), and the main thread collects the results each
//! frame without blocking, so a first use never stalls a frame (retail reads files
//! asynchronously: its `asyncThreadSleep`/`asyncHandlerTimeout` CVars configure an
//! "Async read thread", warcraft.wiki.gg Console_variables/Complete_list).

use std::collections::HashMap;
use std::hash::Hash;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::mpsc::{self, Receiver, Sender};
use std::sync::{Arc, Mutex};
use std::thread;

/// Where one requested key stands.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum LoadState {
    /// Queued or in a worker's hands.
    Loading,
    /// Its result was handed out by `poll`.
    Done,
}

type Job = Box<dyn FnOnce() + Send>;

/// Loads each requested key once with its `load` function on worker threads.
pub struct AssetLoader<K, T> {
    jobs: Option<Sender<Job>>,
    results: Receiver<(K, Result<T, String>)>,
    done: Sender<(K, Result<T, String>)>,
    states: HashMap<K, LoadState>,
    load: Arc<dyn Fn(&K) -> Result<T, String> + Send + Sync>,
    /// Set on drop: queued jobs are skipped, so dropping waits only for jobs in hand.
    closed: Arc<AtomicBool>,
    workers: Vec<thread::JoinHandle<()>>,
}

impl<K, T> AssetLoader<K, T>
where
    K: Clone + Eq + Hash + Send + 'static,
    T: Send + 'static,
{
    /// `workers` threads named `name-{index}`, each running queued jobs in order.
    pub fn new(
        name: &str,
        workers: usize,
        load: impl Fn(&K) -> Result<T, String> + Send + Sync + 'static,
    ) -> Self {
        let (jobs, queue) = mpsc::channel::<Job>();
        let queue = Arc::new(Mutex::new(queue));
        let (done, results) = mpsc::channel();
        let closed = Arc::new(AtomicBool::new(false));
        let workers = (0..workers.max(1))
            .map(|index| {
                let queue = Arc::clone(&queue);
                let closed = Arc::clone(&closed);
                thread::Builder::new()
                    .name(format!("{name}-{index}"))
                    .spawn(move || {
                        // The lock is held only while taking the next job; a closed
                        // queue ends the worker.
                        while let Some(job) = queue.lock().ok().and_then(|queue| queue.recv().ok())
                        {
                            if !closed.load(Ordering::Relaxed) {
                                job();
                            }
                        }
                    })
                    .expect("spawn asset loader worker")
            })
            .collect();
        Self {
            jobs: Some(jobs),
            results,
            done,
            states: HashMap::new(),
            load: Arc::new(load),
            closed,
            workers,
        }
    }

    /// Queue `key` unless it was requested before; `true` when newly queued.
    pub fn request(&mut self, key: K) -> bool {
        if self.states.contains_key(&key) {
            return false;
        }
        self.states.insert(key.clone(), LoadState::Loading);
        let load = Arc::clone(&self.load);
        let done = self.done.clone();
        self.run(move || {
            let loaded = load(&key);
            // A dropped loader waits for nothing.
            let _ = done.send((key, loaded));
        });
        true
    }

    /// Run `job` on a worker, queued behind the loads requested before it.
    pub fn run(&self, job: impl FnOnce() + Send + 'static) {
        if let Some(jobs) = &self.jobs {
            jobs.send(Box::new(job))
                .expect("asset loader workers live as long as the loader");
        }
    }

    pub fn state(&self, key: &K) -> Option<LoadState> {
        self.states.get(key).copied()
    }

    /// Hand out the loads finished since the last call, without blocking.
    pub fn poll(&mut self) -> Vec<(K, Result<T, String>)> {
        let finished: Vec<_> = self.results.try_iter().collect();
        for (key, _) in &finished {
            self.states.insert(key.clone(), LoadState::Done);
        }
        finished
    }

    /// Keys still loading.
    pub fn loading(&self) -> usize {
        self.states
            .values()
            .filter(|&&state| state == LoadState::Loading)
            .count()
    }
}

impl<K, T> Drop for AssetLoader<K, T> {
    fn drop(&mut self) {
        // Closing the queue ends each worker after its current job.
        self.closed.store(true, Ordering::Relaxed);
        self.jobs = None;
        for worker in self.workers.drain(..) {
            let _ = worker.join();
        }
    }
}

#[cfg(test)]
#[path = "asset_loader_tests.rs"]
mod tests;
