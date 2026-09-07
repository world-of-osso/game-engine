use super::*;

#[test]
#[ignore = "manual profiler CPU overhead measurement"]
fn measure_system_span_cpu_overhead() {
    for span_count in [154, 1031] {
        for round in 0..3 {
            let order = if round % 2 == 0 {
                [false, true]
            } else {
                [true, false]
            };
            for enabled in order {
                measure_span_overhead_case(span_count, round, enabled);
            }
        }
    }
}

fn measure_span_overhead_case(span_count: usize, round: usize, enabled: bool) {
    use log::tracing_subscriber::prelude::*;

    const ENTRIES: u64 = 50_000;
    let profiler = Arc::new(SharedProfiler::for_test(PathBuf::new()));
    let layer = enabled.then(|| CpuSpanLayer {
        profiler: profiler.clone(),
    });
    let subscriber = log::tracing_subscriber::registry().with(layer);
    let cpu_ns = log::tracing::subscriber::with_default(subscriber, || {
        let spans: Vec<_> = (0..span_count)
            .map(|index| {
                let name = format!("overhead_system_{index}");
                log::tracing::info_span!("system", name = ?name)
            })
            .collect();
        let start = thread_cpu_time_ns().unwrap();
        for entry in 0..ENTRIES {
            let _entered = spans[entry as usize % span_count].enter();
            std::hint::black_box(entry);
        }
        thread_cpu_time_ns().unwrap() - start
    });
    assert_overhead_case_accounting(&profiler, enabled, ENTRIES);
    println!(
        "profiler_overhead span_count={span_count} round={round} enabled={enabled} entries={ENTRIES} cpu_ns={cpu_ns}"
    );
}

fn assert_overhead_case_accounting(profiler: &SharedProfiler, enabled: bool, entries: u64) {
    assert!(profiler.error.lock().unwrap().is_none());
    assert!(
        profiler.capture_active(),
        "measurement exceeded capture window"
    );
    let reports = profiler.threads.lock().unwrap();
    if !enabled {
        assert!(reports.is_empty());
        return;
    }
    let calls: u64 = reports
        .values()
        .map(|report| {
            report
                .lock()
                .unwrap()
                .snapshot()
                .spans
                .iter()
                .map(|span| span.calls)
                .sum::<u64>()
        })
        .sum();
    assert_eq!(calls, entries);
}
