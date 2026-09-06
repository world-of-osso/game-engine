//! Opt-in thread-CPU attribution for selected Bevy tracing spans.
//!
//! This diagnostic measures CPU consumed by the thread that entered each selected span.
//! It deliberately excludes blocked time and cannot charge uninstrumented worker tasks to
//! their parent systems.

use std::{
    cell::RefCell,
    collections::{BTreeMap, HashMap},
    fs::File,
    io::Write,
    path::PathBuf,
    sync::{
        Arc, Mutex,
        atomic::{AtomicBool, AtomicUsize, Ordering},
    },
    time::{Duration, Instant},
};

use bevy::{app::App, ecs::prelude::*, log, prelude::Last};
use log::tracing;
use log::{
    BoxedLayer,
    tracing::{
        Metadata,
        field::{Field, Visit},
        span::{Attributes, Id},
    },
    tracing_subscriber::{Layer, layer::Context, registry::LookupSpan},
};
use serde::Serialize;

const START_DELAY: Duration = Duration::from_secs(10);
const CAPTURE_DURATION: Duration = Duration::from_secs(5);
const EXPORT_DELAY: Duration = Duration::from_secs(1);

static NEXT_PROFILER_ID: AtomicUsize = AtomicUsize::new(1);

thread_local! {
    static THREAD_PROFILERS: RefCell<HashMap<usize, Arc<Mutex<ThreadReport>>>> = RefCell::new(HashMap::new());
}

/// Adds a bounded Linux thread-CPU profiler when `WOO_CPU_PROFILE_OUTPUT` is set.
pub fn layer(app: &mut App) -> Option<BoxedLayer> {
    let output = std::env::var_os("WOO_CPU_PROFILE_OUTPUT")?;
    if output.is_empty() {
        panic!("WOO_CPU_PROFILE_OUTPUT must name an output file");
    }

    let profiler = Arc::new(SharedProfiler::new(PathBuf::from(output)));
    app.insert_resource(CpuSpanProfileResource(profiler.clone()));
    app.add_systems(Last, export_profile_once);
    Some(Box::new(CpuSpanLayer { profiler }))
}

#[derive(Resource, Clone)]
struct CpuSpanProfileResource(Arc<SharedProfiler>);

struct CpuSpanLayer {
    profiler: Arc<SharedProfiler>,
}

impl<S> Layer<S> for CpuSpanLayer
where
    S: tracing::Subscriber + for<'span> LookupSpan<'span>,
{
    fn on_new_span(&self, attrs: &Attributes<'_>, id: &Id, ctx: Context<'_, S>) {
        let Some(span) = ctx.span(id) else { return };
        let metadata = span.metadata();
        let mut visitor = SpanNameVisitor::default();
        attrs.record(&mut visitor);
        let key = SpanKey::from_metadata(metadata, visitor.name);
        if is_selected_span(metadata.name()) {
            span.extensions_mut().insert(key);
        }
    }

    fn on_enter(&self, id: &Id, ctx: Context<'_, S>) {
        let Some(span) = ctx.span(id) else { return };
        let Some(key) = span.extensions().get::<SpanKey>().cloned() else {
            return;
        };
        let now = match thread_cpu_time_ns() {
            Ok(value) => value,
            Err(error) => {
                self.profiler.record_error(error);
                return;
            }
        };
        let report = self.profiler.thread_report();
        let captured = self.profiler.capture_active();
        report.lock().unwrap().enter(key, now, captured);
    }

    fn on_exit(&self, id: &Id, ctx: Context<'_, S>) {
        let Some(span) = ctx.span(id) else { return };
        if span.extensions().get::<SpanKey>().is_none() {
            return;
        }
        let now = match thread_cpu_time_ns() {
            Ok(value) => value,
            Err(error) => {
                self.profiler.record_error(error);
                return;
            }
        };
        let report = self.profiler.thread_report();
        if let Err(error) = report
            .lock()
            .unwrap()
            .exit(now, self.profiler.capture_active())
        {
            self.profiler.record_error(error);
        }
    }
}

struct SharedProfiler {
    id: usize,
    started: Instant,
    output: PathBuf,
    threads: Mutex<BTreeMap<String, Arc<Mutex<ThreadReport>>>>,
    error: Mutex<Option<String>>,
    exported: AtomicBool,
}

impl SharedProfiler {
    fn new(output: PathBuf) -> Self {
        Self::with_started(output, Instant::now())
    }

    #[cfg(test)]
    fn for_test(output: PathBuf) -> Self {
        Self::with_started(output, Instant::now() - START_DELAY)
    }

    fn with_started(output: PathBuf, started: Instant) -> Self {
        Self {
            id: NEXT_PROFILER_ID.fetch_add(1, Ordering::Relaxed),
            started,
            output,
            threads: Mutex::new(BTreeMap::new()),
            error: Mutex::new(None),
            exported: AtomicBool::new(false),
        }
    }

    fn capture_active(&self) -> bool {
        let elapsed = self.started.elapsed();
        elapsed >= START_DELAY && elapsed < START_DELAY + CAPTURE_DURATION
    }

    fn ready_to_export(&self) -> bool {
        self.started.elapsed() >= START_DELAY + CAPTURE_DURATION + EXPORT_DELAY
    }

    fn thread_report(&self) -> Arc<Mutex<ThreadReport>> {
        THREAD_PROFILERS.with(|reports| {
            if let Some(report) = reports.borrow().get(&self.id) {
                return report.clone();
            }

            let thread = std::thread::current();
            let thread_name = format!("{} ({:?})", thread.name().unwrap_or("unnamed"), thread.id());
            let report = Arc::new(Mutex::new(ThreadReport::new(thread_name.clone())));
            self.threads
                .lock()
                .unwrap()
                .insert(thread_name, report.clone());
            reports.borrow_mut().insert(self.id, report.clone());
            report
        })
    }

    fn record_error(&self, error: impl Into<String>) {
        let mut slot = self.error.lock().unwrap();
        if slot.is_none() {
            *slot = Some(error.into());
        }
    }

    fn export(&self) -> Result<(), String> {
        if let Some(error) = self.error.lock().unwrap().clone() {
            return Err(error);
        }
        let reports = self
            .threads
            .lock()
            .unwrap()
            .values()
            .map(|report| report.lock().unwrap().snapshot())
            .collect();
        let payload = CpuProfileOutput {
            capture_start_after_seconds: START_DELAY.as_secs(),
            capture_duration_seconds: CAPTURE_DURATION.as_secs(),
            export_after_seconds: (START_DELAY + CAPTURE_DURATION + EXPORT_DELAY).as_secs(),
            boundary_policy: "Only spans that enter and exit inside the capture window are counted. boundary_crossing_spans counts entries inside the window that exit afterward, not spans already active at its start. Export waits one second for ordinary exits.".to_string(),
            limitation: "observed_cpu_ns is the first-to-last captured selected-span CPU-clock delta on each thread, not its complete five-second CPU use. Unobserved boundary intervals and threads without selected spans are absent. Blocked time is excluded; uninstrumented work and profiler overhead within the observed intervals remain outside named-span attribution.".to_string(),
            threads: reports,
        };
        let encoded = serde_json::to_vec_pretty(&payload)
            .map_err(|error| format!("serialize CPU span profile: {error}"))?;
        let mut file = File::create(&self.output).map_err(|error| {
            format!("create CPU span profile {}: {error}", self.output.display())
        })?;
        file.write_all(&encoded).map_err(|error| {
            format!("write CPU span profile {}: {error}", self.output.display())
        })?;
        file.write_all(b"\n")
            .map_err(|error| format!("finish CPU span profile {}: {error}", self.output.display()))
    }
}

fn export_profile_once(profile: Res<CpuSpanProfileResource>) {
    if !profile.0.ready_to_export() || profile.0.exported.swap(true, Ordering::AcqRel) {
        return;
    }
    if let Err(error) = profile.0.export() {
        panic!("CPU span profile failed: {error}");
    }
}

#[derive(Clone, Debug, Eq, PartialEq, Ord, PartialOrd)]
struct SpanKey(Arc<str>);

impl SpanKey {
    #[cfg(test)]
    fn new(name: impl Into<Arc<str>>) -> Self {
        Self(name.into())
    }

    fn from_metadata(metadata: &'static Metadata<'static>, field_name: Option<String>) -> Self {
        let name = match field_name {
            Some(name) => format!("{}: {name}", metadata.name()),
            None => metadata.name().to_string(),
        };
        Self(name.into())
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
struct SpanMeasurement {
    calls: u64,
    inclusive_ns: u64,
    self_ns: u64,
}

impl SpanMeasurement {
    #[cfg(test)]
    const fn new(calls: u64, inclusive_ns: u64, self_ns: u64) -> Self {
        Self {
            calls,
            inclusive_ns,
            self_ns,
        }
    }

    fn add(&mut self, inclusive_ns: u64, self_ns: u64) {
        self.calls += 1;
        self.inclusive_ns += inclusive_ns;
        self.self_ns += self_ns;
    }
}

#[derive(Default)]
struct ThreadProfiler {
    stack: Vec<ActiveSpan>,
    measurements: BTreeMap<SpanKey, SpanMeasurement>,
    first_clock_ns: Option<u64>,
    last_clock_ns: Option<u64>,
    boundary_crossing_spans: u64,
}

impl ThreadProfiler {
    fn enter(&mut self, key: SpanKey, clock_ns: u64, captured: bool) {
        self.observe(clock_ns, captured);
        self.stack.push(ActiveSpan {
            key,
            started_ns: clock_ns,
            child_cpu_ns: 0,
            captured,
        });
    }

    fn exit(&mut self, clock_ns: u64, completed_in_window: bool) -> Result<(), String> {
        let active = self
            .stack
            .pop()
            .ok_or_else(|| "selected span exited without matching entry".to_string())?;
        let inclusive_ns = clock_ns
            .checked_sub(active.started_ns)
            .ok_or_else(|| format!("thread CPU clock moved backward for span {}", active.key.0))?;
        if let Some(parent) = self.stack.last_mut() {
            parent.child_cpu_ns += inclusive_ns;
        }
        if active.captured && completed_in_window {
            self.observe(clock_ns, true);
            self.measurements
                .entry(active.key)
                .or_insert(SpanMeasurement {
                    calls: 0,
                    inclusive_ns: 0,
                    self_ns: 0,
                })
                .add(
                    inclusive_ns,
                    inclusive_ns.saturating_sub(active.child_cpu_ns),
                );
        } else if active.captured {
            self.boundary_crossing_spans += 1;
        }
        Ok(())
    }

    fn observe(&mut self, clock_ns: u64, captured: bool) {
        if !captured {
            return;
        }
        self.first_clock_ns.get_or_insert(clock_ns);
        self.last_clock_ns = Some(clock_ns);
    }

    #[cfg(test)]
    fn measurement(&self, name: &str) -> Option<SpanMeasurement> {
        self.measurements.get(&SpanKey::new(name)).copied()
    }
}

#[cfg(test)]
#[derive(Default)]
struct CpuProfiler {
    threads: BTreeMap<String, ThreadProfiler>,
}

#[cfg(test)]
impl CpuProfiler {
    #[cfg(test)]
    fn enter(&mut self, thread: &str, key: SpanKey, clock_ns: u64) {
        self.threads
            .entry(thread.to_string())
            .or_default()
            .enter(key, clock_ns, true);
    }

    #[cfg(test)]
    fn exit(&mut self, thread: &str, clock_ns: u64) -> Result<(), String> {
        self.threads
            .get_mut(thread)
            .ok_or_else(|| format!("unknown thread {thread}"))?
            .exit(clock_ns, true)
    }

    #[cfg(test)]
    fn measurement(&self, thread: &str, name: &str) -> Option<SpanMeasurement> {
        self.threads.get(thread)?.measurement(name)
    }
}

struct ActiveSpan {
    key: SpanKey,
    started_ns: u64,
    child_cpu_ns: u64,
    captured: bool,
}

struct ThreadReport {
    thread_name: String,
    profiler: ThreadProfiler,
}

impl ThreadReport {
    fn new(thread_name: String) -> Self {
        Self {
            thread_name,
            profiler: ThreadProfiler::default(),
        }
    }

    fn enter(&mut self, key: SpanKey, clock_ns: u64, captured: bool) {
        self.profiler.enter(key, clock_ns, captured);
    }

    fn exit(&mut self, clock_ns: u64, completed_in_window: bool) -> Result<(), String> {
        self.profiler.exit(clock_ns, completed_in_window)
    }

    fn snapshot(&self) -> ThreadCpuProfile {
        ThreadCpuProfile {
            thread_name: self.thread_name.clone(),
            observed_cpu_ns: self
                .profiler
                .last_clock_ns
                .zip(self.profiler.first_clock_ns)
                .map_or(0, |(last, first)| last.saturating_sub(first)),
            boundary_crossing_spans: self.profiler.boundary_crossing_spans
                + self
                    .profiler
                    .stack
                    .iter()
                    .filter(|span| span.captured)
                    .count() as u64,
            spans: self
                .profiler
                .measurements
                .iter()
                .map(|(key, measurement)| SpanCpuProfile {
                    name: key.0.to_string(),
                    calls: measurement.calls,
                    inclusive_cpu_ns: measurement.inclusive_ns,
                    self_cpu_ns: measurement.self_ns,
                })
                .collect(),
        }
    }
}

#[derive(Serialize)]
struct CpuProfileOutput {
    capture_start_after_seconds: u64,
    capture_duration_seconds: u64,
    export_after_seconds: u64,
    boundary_policy: String,
    limitation: String,
    threads: Vec<ThreadCpuProfile>,
}

#[derive(Serialize)]
struct ThreadCpuProfile {
    thread_name: String,
    observed_cpu_ns: u64,
    boundary_crossing_spans: u64,
    spans: Vec<SpanCpuProfile>,
}

#[derive(Serialize)]
struct SpanCpuProfile {
    name: String,
    calls: u64,
    inclusive_cpu_ns: u64,
    self_cpu_ns: u64,
}

#[derive(Default)]
struct SpanNameVisitor {
    name: Option<String>,
}

impl Visit for SpanNameVisitor {
    fn record_debug(&mut self, field: &Field, value: &dyn std::fmt::Debug) {
        if field.name() == "name" {
            self.name = Some(format!("{value:?}"));
        }
    }
}

fn is_selected_span(name: &str) -> bool {
    matches!(
        name,
        "system" | "schedule" | "multithreaded executor" | "main_render_schedule"
    )
}

fn thread_cpu_time_ns() -> Result<u64, String> {
    let mut timestamp = libc::timespec {
        tv_sec: 0,
        tv_nsec: 0,
    };
    let result = unsafe { libc::clock_gettime(libc::CLOCK_THREAD_CPUTIME_ID, &mut timestamp) };
    if result != 0 {
        return Err(format!(
            "read CLOCK_THREAD_CPUTIME_ID: {}",
            std::io::Error::last_os_error()
        ));
    }
    let seconds = u64::try_from(timestamp.tv_sec)
        .map_err(|_| "CLOCK_THREAD_CPUTIME_ID returned a negative second count".to_string())?;
    let nanoseconds = u64::try_from(timestamp.tv_nsec)
        .map_err(|_| "CLOCK_THREAD_CPUTIME_ID returned a negative nanosecond count".to_string())?;
    Ok(seconds * 1_000_000_000 + nanoseconds)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn nested_spans_charge_child_cpu_separately_from_parent_self_cpu() {
        let mut profiler = ThreadProfiler::default();
        profiler.enter(SpanKey::new("parent"), 10, true);
        profiler.enter(SpanKey::new("child"), 13, true);
        profiler.exit(18, true).unwrap();
        profiler.exit(23, true).unwrap();

        assert_eq!(
            profiler.measurement("parent"),
            Some(SpanMeasurement::new(1, 13, 8))
        );
        assert_eq!(
            profiler.measurement("child"),
            Some(SpanMeasurement::new(1, 5, 5))
        );
    }

    #[test]
    fn each_thread_keeps_an_independent_span_stack() {
        let mut profiler = CpuProfiler::default();
        profiler.enter("main", SpanKey::new("parent"), 10);
        profiler.enter("render", SpanKey::new("child"), 100);
        profiler.exit("render", 107).unwrap();
        profiler.exit("main", 15).unwrap();

        assert_eq!(
            profiler.measurement("main", "parent"),
            Some(SpanMeasurement::new(1, 5, 5))
        );
        assert_eq!(
            profiler.measurement("render", "child"),
            Some(SpanMeasurement::new(1, 7, 7))
        );
    }

    #[test]
    fn uncaptured_parent_does_not_hide_captured_child_cpu() {
        let mut profiler = ThreadProfiler::default();
        profiler.enter(SpanKey::new("before-window"), 10, false);
        profiler.enter(SpanKey::new("captured-child"), 20, true);
        profiler.exit(25, true).unwrap();
        profiler.exit(30, false).unwrap();

        assert_eq!(
            profiler.measurement("captured-child"),
            Some(SpanMeasurement::new(1, 5, 5))
        );
        assert_eq!(profiler.measurement("before-window"), None);
        assert_eq!(profiler.boundary_crossing_spans, 0);
    }

    #[test]
    fn observed_cpu_is_the_selected_span_interval_including_unattributed_gaps() {
        let mut report = ThreadReport::new("sampled-thread".into());
        report.enter(SpanKey::new("first"), 50, true);
        report.exit(60, true).unwrap();
        report.enter(SpanKey::new("second"), 80, true);
        report.exit(90, true).unwrap();

        let snapshot = report.snapshot();
        assert_eq!(snapshot.observed_cpu_ns, 40);
        let attributed_cpu: u64 = snapshot.spans.iter().map(|span| span.self_cpu_ns).sum();
        assert_eq!(attributed_cpu, 20);
    }

    #[test]
    fn span_crossing_capture_end_is_excluded_and_counted() {
        let mut profiler = ThreadProfiler::default();
        profiler.enter(SpanKey::new("crosses-end"), 10, true);
        profiler.exit(20, false).unwrap();

        assert_eq!(profiler.measurement("crosses-end"), None);
        assert_eq!(profiler.boundary_crossing_spans, 1);
    }

    #[test]
    fn thread_cpu_clock_excludes_blocked_sleep() {
        let wall_start = Instant::now();
        let cpu_start = thread_cpu_time_ns().unwrap();
        std::thread::sleep(Duration::from_millis(30));
        consume_thread_cpu_for(Duration::from_millis(2));
        let cpu_elapsed = thread_cpu_time_ns().unwrap() - cpu_start;

        assert!(wall_start.elapsed() >= Duration::from_millis(25));
        assert!(cpu_elapsed >= Duration::from_millis(2).as_nanos() as u64);
        assert!(cpu_elapsed < wall_start.elapsed().as_nanos() as u64 / 2);
    }

    #[test]
    fn tracing_layer_attributes_concurrent_entries_of_one_span_on_each_real_thread() {
        use log::tracing_subscriber::prelude::*;
        use std::sync::Barrier;

        let output = std::env::temp_dir().join(format!(
            "game-engine-thread-cpu-profile-{}-{}.json",
            std::process::id(),
            NEXT_PROFILER_ID.load(Ordering::Relaxed)
        ));
        let profiler = Arc::new(SharedProfiler::for_test(output.clone()));
        let subscriber = log::tracing_subscriber::registry().with(CpuSpanLayer {
            profiler: profiler.clone(),
        });
        let dispatch = log::tracing::Dispatch::new(subscriber);
        let barrier = Arc::new(Barrier::new(2));

        let worker = log::tracing::dispatcher::with_default(&dispatch, || {
            let shared = log::tracing::info_span!("system", name = "shared");
            let worker_span = shared.clone();
            let worker_dispatch = dispatch.clone();
            let worker_barrier = barrier.clone();
            let worker = std::thread::Builder::new()
                .name("cpu-profile-test-worker".to_string())
                .spawn(move || {
                    log::tracing::dispatcher::with_default(&worker_dispatch, || {
                        run_shared_span_work(worker_span, worker_barrier);
                    });
                })
                .unwrap();

            let _shared = shared.enter();
            barrier.wait();
            consume_thread_cpu_for(Duration::from_millis(2));
            let nested = log::tracing::info_span!("system", name = "nested");
            let _nested = nested.enter();
            consume_thread_cpu_for(Duration::from_millis(2));
            worker
        });
        worker.join().unwrap();

        profiler.export().unwrap();
        let value: serde_json::Value =
            serde_json::from_slice(&std::fs::read(&output).unwrap()).unwrap();
        let threads = value["threads"].as_array().unwrap();
        assert_eq!(threads.len(), 2);
        for thread in threads {
            let observed_cpu_ns = thread["observed_cpu_ns"].as_u64().unwrap();
            assert!(observed_cpu_ns > 0);
            let spans = thread["spans"].as_array().unwrap();
            assert_eq!(span_count(spans, "shared"), 1);
            assert_eq!(span_count(spans, "nested"), 1);
            assert!(spans.iter().all(span_has_positive_cpu));
            let summed_self_cpu_ns = spans
                .iter()
                .map(|span| span["self_cpu_ns"].as_u64().unwrap())
                .sum::<u64>();
            assert!(summed_self_cpu_ns <= observed_cpu_ns);
        }
        std::fs::remove_file(output).unwrap();
    }

    fn run_shared_span_work(span: log::tracing::Span, barrier: Arc<std::sync::Barrier>) {
        let _shared = span.enter();
        barrier.wait();
        consume_thread_cpu_for(Duration::from_millis(2));
        let nested = log::tracing::info_span!("system", name = "nested");
        let _nested = nested.enter();
        consume_thread_cpu_for(Duration::from_millis(2));
    }

    fn consume_thread_cpu_for(duration: Duration) {
        let start = thread_cpu_time_ns().unwrap();
        let target = start + duration.as_nanos() as u64;
        let mut value = 0_u64;
        while thread_cpu_time_ns().unwrap() < target {
            value = value.wrapping_add(1);
        }
        std::hint::black_box(value);
    }

    fn span_count(spans: &[serde_json::Value], expected_name: &str) -> u64 {
        spans
            .iter()
            .find(|span| span["name"].as_str().unwrap().contains(expected_name))
            .unwrap()["calls"]
            .as_u64()
            .unwrap()
    }

    fn span_has_positive_cpu(span: &serde_json::Value) -> bool {
        span["inclusive_cpu_ns"].as_u64().unwrap() > 0 && span["self_cpu_ns"].as_u64().unwrap() > 0
    }
}
