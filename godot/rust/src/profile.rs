//! Opt-in main-thread timing: with `GAME_PROFILE_MS=<threshold>` set, each span that
//! lasts at least the threshold prints `PROFILE <label> ms=<elapsed>` when it ends.
//! Nested spans print innermost first.

use std::{sync::OnceLock, time::Instant};

fn threshold_ms() -> Option<f64> {
    static THRESHOLD: OnceLock<Option<f64>> = OnceLock::new();
    *THRESHOLD.get_or_init(|| std::env::var("GAME_PROFILE_MS").ok()?.parse().ok())
}

/// A running span; `label` is only evaluated when the span is printed.
pub(crate) struct Span<F: FnOnce() -> String> {
    label: Option<F>,
    started: Instant,
    threshold: f64,
}

pub(crate) fn span<F: FnOnce() -> String>(label: F) -> Option<Span<F>> {
    let threshold = threshold_ms()?;
    Some(Span {
        label: Some(label),
        started: Instant::now(),
        threshold,
    })
}

impl<F: FnOnce() -> String> Drop for Span<F> {
    fn drop(&mut self) {
        let elapsed = self.started.elapsed().as_secs_f64() * 1000.0;
        if elapsed >= self.threshold
            && let Some(label) = self.label.take()
        {
            println!("PROFILE {} ms={elapsed:.1}", label());
        }
    }
}
