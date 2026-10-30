//! Opt-in main-thread timing: with `GAME_PROFILE_MS=<threshold>` set, each span that
//! lasts at least the threshold prints `PROFILE <label> ms=<elapsed> cpu=<on-CPU ms>`
//! when it ends; `cpu` is the thread's scheduled run time (`/proc/thread-self/schedstat`),
//! so it excludes the time the thread waited for a CPU on a loaded host. Nested spans
//! print innermost first.

use std::{
    cell::RefCell,
    collections::BTreeMap,
    sync::OnceLock,
    time::{Duration, Instant},
};

thread_local! {
    static NODE_TIMES: RefCell<BTreeMap<&'static str, Duration>> = const { RefCell::new(BTreeMap::new()) };
}

/// Wall time accumulated per node callback since the last client frame boundary.
pub(crate) fn take_node_times() -> Vec<(&'static str, f64)> {
    NODE_TIMES.with(|times| {
        times
            .borrow_mut()
            .iter_mut()
            .filter_map(|(&name, duration)| {
                let value = std::mem::take(duration);
                (value != Duration::ZERO).then_some((name, value.as_secs_f64() * 1000.0))
            })
            .collect()
    })
}

pub(crate) struct NodeSpan {
    name: &'static str,
    started: Instant,
}

pub(crate) fn node_span(name: &'static str) -> NodeSpan {
    NodeSpan {
        name,
        started: Instant::now(),
    }
}

impl Drop for NodeSpan {
    fn drop(&mut self) {
        let duration = self.started.elapsed();
        NODE_TIMES.with(|times| *times.borrow_mut().entry(self.name).or_default() += duration);
    }
}

fn threshold_ms() -> Option<f64> {
    static THRESHOLD: OnceLock<Option<f64>> = OnceLock::new();
    *THRESHOLD.get_or_init(|| std::env::var("GAME_PROFILE_MS").ok()?.parse().ok())
}

/// A running span; `label` is only evaluated when the span is printed.
pub(crate) struct Span<F: FnOnce() -> String> {
    label: Option<F>,
    started: Instant,
    started_cpu_ns: Option<u64>,
    threshold: f64,
}

/// The calling thread's on-CPU time in nanoseconds, where the kernel reports it.
fn thread_cpu_ns() -> Option<u64> {
    std::fs::read_to_string("/proc/thread-self/schedstat")
        .ok()
        .and_then(|stat| stat.split_whitespace().next()?.parse().ok())
}

pub(crate) fn span<F: FnOnce() -> String>(label: F) -> Option<Span<F>> {
    let threshold = threshold_ms()?;
    Some(Span {
        label: Some(label),
        started: Instant::now(),
        started_cpu_ns: thread_cpu_ns(),
        threshold,
    })
}

impl<F: FnOnce() -> String> Drop for Span<F> {
    fn drop(&mut self) {
        let elapsed = self.started.elapsed().as_secs_f64() * 1000.0;
        if elapsed >= self.threshold
            && let Some(label) = self.label.take()
        {
            let cpu = match (self.started_cpu_ns, thread_cpu_ns()) {
                (Some(started), Some(now)) => format!("{:.3}", (now - started) as f64 / 1e6),
                _ => "unreported".to_owned(),
            };
            println!("PROFILE {} ms={elapsed:.3} cpu={cpu}", label());
        }
    }
}
