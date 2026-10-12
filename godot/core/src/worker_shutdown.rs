//! Join detached client workers before the native extension unloads.
//! Owners still drop without waiting; extension shutdown owns the final barrier.

use std::sync::Mutex;
use std::thread::JoinHandle;

type JoinWorker = Box<dyn FnOnce() + Send>;
static WORKERS: Mutex<Vec<JoinWorker>> = Mutex::new(Vec::new());

/// Transfer a worker's join handle to the extension's shutdown barrier.
pub fn track<T: Send + 'static>(worker: JoinHandle<T>) {
    let name = worker.thread().name().unwrap_or("client-worker").to_owned();
    WORKERS
        .lock()
        .expect("worker shutdown registry")
        .push(Box::new(move || {
            if worker.join().is_err() {
                eprintln!("{name}: worker panicked before extension shutdown");
            }
        }));
}

/// Call after client owners have dropped and stopped their request queues.
pub fn join_all() {
    loop {
        let workers = std::mem::take(&mut *WORKERS.lock().expect("worker shutdown registry"));
        if workers.is_empty() {
            return;
        }
        for join in workers {
            join();
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::mpsc;

    #[test]
    fn shutdown_waits_for_detached_workers_and_their_destructors() {
        let (started, start) = mpsc::channel();
        let (release, gate) = mpsc::channel();
        let (finished, finish) = mpsc::channel();
        struct Finished(mpsc::Sender<u32>);
        impl Drop for Finished {
            fn drop(&mut self) {
                self.0.send(4865).unwrap();
            }
        }
        let worker = std::thread::spawn(move || {
            started.send(()).unwrap();
            gate.recv().unwrap();
            Finished(finished)
        });
        start.recv().unwrap();
        track(worker);
        assert!(
            finish.try_recv().is_err(),
            "tracking must not block its owner"
        );
        let releaser = std::thread::spawn(move || release.send(()).unwrap());
        join_all();
        assert_eq!(finish.try_recv().unwrap(), 4865);
        releaser.join().unwrap();
        join_all();
    }
}
