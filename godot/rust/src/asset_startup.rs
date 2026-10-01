//! Nonblocking startup boundary for process-wide CASC initialization.
use std::{
    path::PathBuf,
    thread::{self, JoinHandle},
};

pub(crate) struct AssetStartup {
    worker: Option<JoinHandle<Result<(), String>>>,
}

impl AssetStartup {
    pub(crate) fn start(data_root: PathBuf) -> Result<Self, String> {
        Self::spawn(move || crate::assets::creature::local_resolver(&data_root).initialize())
            .map_err(|error| format!("Cannot spawn CASC initialization worker: {error}"))
    }

    fn spawn(
        initialize: impl FnOnce() -> Result<(), String> + Send + 'static,
    ) -> std::io::Result<Self> {
        let worker = thread::Builder::new()
            .name("casc-startup".into())
            .spawn(initialize)?;
        Ok(Self {
            worker: Some(worker),
        })
    }

    pub(crate) fn poll(&mut self) -> Option<Result<(), String>> {
        if !self.worker.as_ref()?.is_finished() {
            return None;
        }
        Some(
            self.worker
                .take()?
                .join()
                .map_err(|_| "CASC initialization worker panicked".to_string())
                .and_then(|result| result),
        )
    }
}

#[cfg(test)]
mod tests {
    use super::AssetStartup;
    use std::{
        sync::mpsc,
        thread,
        time::{Duration, Instant},
    };

    fn finish(startup: &mut AssetStartup) -> Result<(), String> {
        let deadline = Instant::now() + Duration::from_secs(5);
        loop {
            if let Some(result) = startup.poll() {
                return result;
            }
            assert!(Instant::now() < deadline, "startup worker did not finish");
            thread::yield_now();
        }
    }

    #[test]
    fn pending_initialization_does_not_block_caller_or_run_on_its_thread() {
        let caller = thread::current().id();
        let (started, observed) = mpsc::channel();
        let (release, gate) = mpsc::channel();
        let mut startup = AssetStartup::spawn(move || {
            started.send(thread::current().id()).unwrap();
            gate.recv().unwrap();
            Ok(())
        })
        .unwrap();
        let worker = observed.recv_timeout(Duration::from_secs(5));
        // Release even if the assertion fails, so a test failure cannot orphan a worker.
        let pending = startup.poll();
        let _ = release.send(());
        assert_ne!(worker.expect("initializer executes"), caller);
        assert_eq!(pending, None);
        assert_eq!(finish(&mut startup), Ok(()));
        assert_eq!(startup.poll(), None, "completion is delivered once");
    }

    #[test]
    fn initialization_failure_reaches_startup_caller() {
        let mut startup = AssetStartup::spawn(|| Err("CASC open: denied".into())).unwrap();
        assert_eq!(finish(&mut startup), Err("CASC open: denied".into()));
    }

    #[test]
    fn initialization_panic_is_an_explicit_startup_failure() {
        let mut startup = AssetStartup::spawn(|| panic!("broken initialization")).unwrap();
        assert_eq!(
            finish(&mut startup),
            Err("CASC initialization worker panicked".into())
        );
    }
}
