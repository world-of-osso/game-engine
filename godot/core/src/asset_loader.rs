//! Background asset loading: keys requested on the main thread load on worker threads
//! (extraction, file reads, parsing), and the main thread collects the results each
//! frame without blocking, so a first use never stalls a frame (retail reads files
//! asynchronously: its `asyncThreadSleep`/`asyncHandlerTimeout` CVars configure an
//! "Async read thread", warcraft.wiki.gg Console_variables/Complete_list).
//!
//! Loads needed now go ahead of prefetches: a prefetch queued before a cast must not
//! delay the cast's own assets. Loads the loading screen waits for go ahead of both.

use std::collections::{HashMap, VecDeque};
use std::hash::Hash;
use std::sync::mpsc::{self, Receiver, Sender};
use std::sync::{Arc, Condvar, Mutex};
use std::thread;

/// Where one requested key stands.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum LoadState {
    /// Queued or in a worker's hands.
    Loading,
    /// Its result was handed out by `poll`.
    Done,
}

/// How soon a load is needed.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Priority {
    /// The loading screen waits for it: ahead of every other load.
    First,
    /// Something is waiting for it.
    Now,
    /// A prefetch: loaded when nothing needed now is queued.
    Later,
}

type Job = Box<dyn FnOnce() + Send>;

enum Task<K> {
    Load(K),
    Run(Job),
}

struct Queues<K> {
    now: VecDeque<Task<K>>,
    later: VecDeque<Task<K>>,
    /// Set on drop: workers stop taking tasks.
    closed: bool,
}

type Shared<K> = Arc<(Mutex<Queues<K>>, Condvar)>;

/// Loads each requested key once with its `load` function on worker threads.
pub struct AssetLoader<K, T> {
    queues: Shared<K>,
    results: Receiver<(K, Result<T, String>)>,
    states: HashMap<K, LoadState>,
}

impl<K, T> AssetLoader<K, T>
where
    K: Clone + Eq + Hash + Send + 'static,
    T: Send + 'static,
{
    /// `workers` threads named `name-{index}`, each taking the next task needed now,
    /// else the next prefetch.
    pub fn new(
        name: &str,
        workers: usize,
        load: impl Fn(&K) -> Result<T, String> + Send + Sync + 'static,
    ) -> Self {
        let queues: Shared<K> = Arc::new((
            Mutex::new(Queues {
                now: VecDeque::new(),
                later: VecDeque::new(),
                closed: false,
            }),
            Condvar::new(),
        ));
        let (done, results) = mpsc::channel();
        let load: Arc<LoadFn<K, T>> = Arc::new(load);
        for index in 0..workers.max(1) {
            let worker = Worker {
                queues: Arc::clone(&queues),
                load: Arc::clone(&load),
                done: done.clone(),
            };
            // Detached: a dropped loader never waits for the task in hand.
            thread::Builder::new()
                .name(format!("{name}-{index}"))
                .spawn(move || worker.run())
                .expect("spawn asset loader worker");
        }
        Self {
            queues,
            results,
            states: HashMap::new(),
        }
    }

    /// Queue `key` unless it was requested before; `true` when newly queued. A key
    /// still queued moves up to `priority`: a prefetch when it is needed now, any key
    /// to the front when the loading screen waits for it.
    pub fn request(&mut self, key: K, priority: Priority) -> bool {
        if self.states.contains_key(&key) {
            if priority != Priority::Later {
                self.promote(&key, priority);
            }
            return false;
        }
        self.states.insert(key.clone(), LoadState::Loading);
        self.push(Task::Load(key), priority);
        true
    }

    fn promote(&self, key: &K, priority: Priority) {
        let mut queues = self.queues.0.lock().expect("asset loader queue");
        let is_key = |task: &Task<K>| matches!(task, Task::Load(queued) if queued == key);
        let task = match queues.now.iter().position(is_key) {
            Some(index) if priority == Priority::First => queues.now.remove(index),
            Some(_) => None,
            None => queues
                .later
                .iter()
                .position(is_key)
                .and_then(|index| queues.later.remove(index)),
        };
        if let Some(task) = task {
            enqueue(&mut queues, task, priority);
        }
    }

    /// Run `job` on a worker, queued behind the tasks of its priority requested before.
    pub fn run(&self, job: impl FnOnce() + Send + 'static, priority: Priority) {
        self.push(Task::Run(Box::new(job)), priority);
    }

    fn push(&self, task: Task<K>, priority: Priority) {
        let (lock, wake) = &*self.queues;
        let mut queues = lock.lock().expect("asset loader queue");
        enqueue(&mut queues, task, priority);
        wake.notify_one();
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

fn enqueue<K>(queues: &mut Queues<K>, task: Task<K>, priority: Priority) {
    match priority {
        Priority::First => queues.now.push_front(task),
        Priority::Now => queues.now.push_back(task),
        Priority::Later => queues.later.push_back(task),
    }
}

type LoadFn<K, T> = dyn Fn(&K) -> Result<T, String> + Send + Sync;

/// One worker thread's handles.
struct Worker<K, T> {
    queues: Shared<K>,
    load: Arc<LoadFn<K, T>>,
    done: Sender<(K, Result<T, String>)>,
}

impl<K, T> Worker<K, T> {
    /// Take tasks until the loader is dropped.
    fn run(self) {
        while let Some(task) = next_task(&self.queues) {
            match task {
                Task::Load(key) => {
                    let loaded = (self.load)(&key);
                    // A dropped loader waits for nothing.
                    let _ = self.done.send((key, loaded));
                }
                Task::Run(job) => job(),
            }
        }
    }
}

/// The next task, waiting for one; `None` once the loader is dropped.
fn next_task<K>(queues: &Shared<K>) -> Option<Task<K>> {
    let (lock, wake) = &**queues;
    let mut queues = lock.lock().ok()?;
    loop {
        if queues.closed {
            return None;
        }
        if let Some(task) = queues.now.pop_front().or_else(|| queues.later.pop_front()) {
            return Some(task);
        }
        queues = wake.wait(queues).ok()?;
    }
}

impl<K, T> Drop for AssetLoader<K, T> {
    fn drop(&mut self) {
        // Workers finish the task in hand and skip the queued ones; nothing waits for them.
        let (lock, wake) = &*self.queues;
        if let Ok(mut queues) = lock.lock() {
            queues.closed = true;
        }
        wake.notify_all();
    }
}

#[cfg(test)]
#[path = "asset_loader_tests.rs"]
mod tests;
