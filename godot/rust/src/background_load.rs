//! Data tables loaded on a background thread that the main thread reads without waiting:
//! until the thread is done a reader gets `None` and shows nothing for that data.

use std::thread::JoinHandle;

pub(crate) struct BackgroundLoad<T> {
    worker: Option<JoinHandle<T>>,
    loaded: Option<T>,
}

impl<T: Send + 'static> BackgroundLoad<T> {
    /// Start `load` on a thread named `name`.
    pub(crate) fn start(name: &str, load: impl FnOnce() -> T + Send + 'static) -> Self {
        let worker = std::thread::Builder::new()
            .name(name.into())
            .spawn(load)
            .unwrap_or_else(|error| panic!("Cannot start the {name} loader: {error}"));
        Self {
            worker: Some(worker),
            loaded: None,
        }
    }

    /// Take the thread's result once it is done; `None` while it loads.
    pub(crate) fn poll(&mut self) -> Option<&T> {
        if let Some(worker) = self.worker.take_if(|worker| worker.is_finished()) {
            let loaded = worker
                .join()
                .unwrap_or_else(|panic| std::panic::resume_unwind(panic));
            self.loaded = Some(loaded);
        }
        self.loaded.as_ref()
    }

    /// The result, waiting for the thread when it still loads.
    pub(crate) fn wait(&mut self) -> &T {
        if let Some(worker) = self.worker.take() {
            let loaded = worker
                .join()
                .unwrap_or_else(|panic| std::panic::resume_unwind(panic));
            self.loaded = Some(loaded);
        }
        self.loaded
            .as_ref()
            .expect("a background load has its thread or its result")
    }

    /// The result taken by an earlier `poll`.
    pub(crate) fn loaded(&self) -> Option<&T> {
        self.loaded.as_ref()
    }

    /// Own the result taken by an earlier `poll`.
    pub(crate) fn into_loaded(self) -> Option<T> {
        self.loaded
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::{Duration, Instant};

    /// A read while the thread still loads returns at once; the first read after it is
    /// done takes its result.
    #[test]
    fn reads_never_wait_for_the_loading_thread() {
        let (release, gate) = std::sync::mpsc::channel::<()>();
        let mut table = BackgroundLoad::start("test-table", move || {
            gate.recv().expect("released");
            vec![2589_u32, 4865]
        });
        let started = Instant::now();
        assert_eq!(table.poll(), None, "the thread is still loading");
        assert_eq!(table.loaded(), None);
        assert!(started.elapsed() < Duration::from_millis(50));
        release.send(()).expect("thread waiting");
        let deadline = Instant::now() + Duration::from_secs(10);
        while table.poll().is_none() {
            assert!(Instant::now() < deadline, "the thread never finished");
            std::thread::yield_now();
        }
        assert_eq!(table.loaded(), Some(&vec![2589, 4865]));
    }

    /// `wait` blocks until the thread is done, then keeps its result.
    #[test]
    fn wait_takes_the_result_once_the_thread_is_done() {
        let (release, gate) = std::sync::mpsc::channel::<()>();
        let mut table = BackgroundLoad::start("test-table", move || {
            gate.recv().expect("released");
            vec![2589_u32, 4865]
        });
        // Taken before the releaser starts its sleep, so the wait below is at least 50 ms.
        let started = Instant::now();
        let releaser = std::thread::spawn(move || {
            std::thread::sleep(Duration::from_millis(50));
            release.send(()).expect("thread waiting");
        });
        assert_eq!(table.wait(), &vec![2589, 4865]);
        assert!(started.elapsed() >= Duration::from_millis(50));
        releaser.join().expect("releaser");
        assert_eq!(table.wait(), &vec![2589, 4865]);
        assert_eq!(table.poll(), Some(&vec![2589, 4865]));
    }
}
