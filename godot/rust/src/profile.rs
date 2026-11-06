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

/// Scheduled CPU time for short Linux rendering spans, excluding host runqueue waits.
#[cfg(target_os = "linux")]
pub(crate) struct CpuNodeSpan {
    name: &'static str,
    started: Duration,
}

#[cfg(target_os = "linux")]
pub(crate) fn cpu_node_span(name: &'static str) -> Option<CpuNodeSpan> {
    thread_cpu_time().map(|started| CpuNodeSpan { name, started })
}

#[cfg(target_os = "linux")]
fn thread_cpu_time() -> Option<Duration> {
    let mut time = libc::timespec {
        tv_sec: 0,
        tv_nsec: 0,
    };
    // SAFETY: `time` is a valid writable timespec and the thread clock needs no handle.
    if unsafe { libc::clock_gettime(libc::CLOCK_THREAD_CPUTIME_ID, &mut time) } != 0 {
        godot::prelude::godot_error!("Thread CPU clock: {}", std::io::Error::last_os_error());
        return None;
    }
    let (Ok(seconds), Ok(nanoseconds)) = (u64::try_from(time.tv_sec), u32::try_from(time.tv_nsec))
    else {
        godot::prelude::godot_error!("Thread CPU clock returned a negative value");
        return None;
    };
    if nanoseconds >= 1_000_000_000 {
        godot::prelude::godot_error!("Thread CPU clock returned invalid nanoseconds {nanoseconds}");
        return None;
    }
    Some(Duration::new(seconds, nanoseconds))
}

/// The calling thread's CPU time in milliseconds, so benchmarks can split a frame
/// into a thread's work and its waiting on Linux and Windows.
#[cfg(any(target_os = "linux", windows))]
pub(crate) fn thread_cpu_ms() -> Option<f64> {
    thread_cpu_time().map(|time| time.as_secs_f64() * 1000.0)
}

#[cfg(not(any(target_os = "linux", windows)))]
pub(crate) fn thread_cpu_ms() -> Option<f64> {
    None
}

/// Kernel plus user time of the calling thread. Windows charges it per scheduler tick
/// (about 15.6 ms), so read it over many frames rather than per frame.
#[cfg(windows)]
fn thread_cpu_time() -> Option<Duration> {
    use std::ffi::c_void;
    #[repr(C)]
    #[derive(Default)]
    struct FileTime {
        low: u32,
        high: u32,
    }
    #[link(name = "kernel32")]
    unsafe extern "system" {
        fn GetCurrentThread() -> *mut c_void;
        fn GetThreadTimes(
            thread: *mut c_void,
            creation: *mut FileTime,
            exit: *mut FileTime,
            kernel: *mut FileTime,
            user: *mut FileTime,
        ) -> i32;
    }
    let mut creation = FileTime::default();
    let mut exit = FileTime::default();
    let mut kernel = FileTime::default();
    let mut user = FileTime::default();
    // SAFETY: the pseudo handle names the calling thread and all four outputs are
    // valid writable FILETIMEs.
    let ok = unsafe {
        GetThreadTimes(
            GetCurrentThread(),
            &mut creation,
            &mut exit,
            &mut kernel,
            &mut user,
        )
    };
    if ok == 0 {
        godot::prelude::godot_error!("Thread CPU times: {}", std::io::Error::last_os_error());
        return None;
    }
    let ticks = |time: &FileTime| (u64::from(time.high) << 32) | u64::from(time.low);
    Some(Duration::from_nanos((ticks(&kernel) + ticks(&user)) * 100))
}

#[cfg(target_os = "linux")]
impl Drop for CpuNodeSpan {
    fn drop(&mut self) {
        let Some(end) = thread_cpu_time() else {
            return;
        };
        let Some(duration) = end.checked_sub(self.started) else {
            godot::prelude::godot_error!("Thread CPU clock moved backwards");
            return;
        };
        NODE_TIMES.with(|times| *times.borrow_mut().entry(self.name).or_default() += duration);
    }
}

#[cfg(all(test, target_os = "linux"))]
mod tests {
    use super::{cpu_node_span, take_node_times};
    use std::time::{Duration, Instant};

    #[test]
    fn cpu_span_excludes_sleep_and_records_work() {
        take_node_times();
        let wall = Instant::now();
        let span = cpu_node_span("test.cpu").expect("Linux thread CPU clock");
        std::thread::sleep(Duration::from_millis(50));
        let mut sum = 0_u64;
        for value in 0..100_000 {
            sum = sum.wrapping_add(std::hint::black_box(value));
        }
        std::hint::black_box(sum);
        drop(span);
        let cpu_ms = take_node_times()
            .into_iter()
            .find_map(|(name, milliseconds)| (name == "test.cpu").then_some(milliseconds))
            .expect("CPU sample");
        assert!(cpu_ms > 0.0);
        assert!(
            cpu_ms < wall.elapsed().as_secs_f64() * 500.0,
            "CPU {cpu_ms} ms should exclude sleep"
        );
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
