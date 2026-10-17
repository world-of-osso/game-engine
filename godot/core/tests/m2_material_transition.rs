use game_engine_core::{
    m2,
    m2_material::{self, BatchBinding, MaterialTime, MaterialTracks},
};

fn material_track<T>(sequences: Vec<(Vec<u32>, Vec<T>)>) -> m2::AnimTrack<T> {
    m2::AnimTrack {
        interpolation_type: 1,
        global_sequence: -1,
        sequences,
    }
}

fn transition_fixture() -> (MaterialTracks, BatchBinding) {
    use game_engine_core::asset::m2_format::m2_anim::{ColorAnimTracks, TextureAnimTracks};
    let tracks = MaterialTracks {
        global_sequences: vec![1000],
        stand_ms: 1000,
        colors: vec![ColorAnimTracks {
            color: material_track(vec![
                (vec![0, 1000], vec![[1.0, 0.0, 0.0], [0.0, 1.0, 0.0]]),
                (vec![0, 1000], vec![[0.0, 0.0, 1.0], [0.0, 0.0, 0.0]]),
            ]),
            opacity: material_track(vec![(vec![], vec![]), (vec![0], vec![0])]),
        }],
        weights: vec![material_track(vec![
            (vec![0], vec![32767]),
            (vec![0], vec![0]),
        ])],
        transforms: vec![TextureAnimTracks {
            translation: material_track(vec![
                (vec![0], vec![[0.0; 3]]),
                (vec![0], vec![[1.0, 0.0, 0.0]]),
            ]),
            scale: material_track(vec![(vec![], vec![]), (vec![0], vec![[3.0; 3]])]),
            rotation: material_track(vec![
                (vec![], vec![]),
                (vec![0], vec![[0.0, 0.0, 1.0, 0.0]]),
            ]),
        }],
    };
    let binding = BatchBinding {
        pixel_shader: 1,
        vertex_shader: 0,
        textures: vec![],
        texture_types: vec![],
        texture_wrap: 0,
        texture_transforms: [Some(0), None],
        texture_weights: [Some(0), None, None],
        color: Some(0),
        apply_weight: true,
    };
    (tracks, binding)
}

#[test]
fn material_transition_uses_two_clocks_defaults_and_quaternion_rotation() {
    use m2_material::{MaterialSource, MaterialTransition};
    let (tracks, binding) = transition_fixture();
    let time = MaterialTime {
        sequence: 1,
        time_ms: 500,
        global_ms: 900,
    };
    let from = MaterialTime {
        sequence: 0,
        time_ms: 250,
        global_ms: 900,
    };
    let sample = m2_material::sample_material_transition(
        &tracks,
        &binding,
        time,
        Some(MaterialTransition {
            source: MaterialSource::Sequence(from),
            weight: 0.5,
        }),
    );
    assert_eq!(sample.mesh_color, [0.375, 0.125, 0.25]);
    assert_eq!(sample.texture_weights, [0.5, 1.0, 1.0]);
    assert_eq!(
        sample.opacity, 0.25,
        "default alpha blended BEFORE multiplication"
    );
    // Half identity -> 180deg is 90deg; linear matrix blending would collapse the axes.
    let expected = [0.0, 2.0, -2.0, 0.0, 1.5, 0.5];
    for (actual, expected) in sample.texture_matrices[0].iter().zip(expected) {
        assert!(
            (actual - expected).abs() < 1e-5,
            "{:?}",
            sample.texture_matrices[0]
        );
    }
}

#[test]
fn material_transition_excludes_step_and_global_tracks() {
    use m2_material::{MaterialSource, MaterialTransition};
    let (mut tracks, binding) = transition_fixture();
    tracks.colors[0].color.interpolation_type = 0;
    tracks.weights[0].global_sequence = 0;
    tracks.weights[0].sequences = vec![(vec![0, 1000], vec![0, 32767])];
    let time = MaterialTime {
        sequence: 1,
        time_ms: 500,
        global_ms: 250,
    };
    let from = MaterialTime {
        sequence: 0,
        time_ms: 750,
        global_ms: 250,
    };
    let sample = m2_material::sample_material_transition(
        &tracks,
        &binding,
        time,
        Some(MaterialTransition {
            source: MaterialSource::Sequence(from),
            weight: 0.5,
        }),
    );
    assert_eq!(sample.mesh_color, [0.0, 0.0, 1.0]);
    assert!((sample.texture_weights[0] - 0.25).abs() < 1.0 / 32767.0);
}

#[test]
fn material_snapshot_retains_interrupted_channel_pose_without_matrix_blending() {
    use m2_material::{MaterialSnapshot, MaterialSource, MaterialTransition};
    let (tracks, binding) = transition_fixture();
    let outgoing = MaterialTime {
        sequence: 0,
        time_ms: 250,
        global_ms: 900,
    };
    let current = MaterialTime {
        sequence: 1,
        time_ms: 500,
        global_ms: 900,
    };
    let mut snapshot = MaterialSnapshot::new(&tracks);
    let half = Some(MaterialTransition {
        source: MaterialSource::Sequence(outgoing),
        weight: 0.5,
    });
    let before = m2_material::sample_material_transition(&tracks, &binding, current, half);
    snapshot.capture(&tracks, current, Some(outgoing), 0.5);
    let after = m2_material::sample_material_transition(
        &tracks,
        &binding,
        outgoing,
        Some(MaterialTransition {
            source: MaterialSource::Snapshot(&snapshot),
            weight: 0.0,
        }),
    );
    assert_eq!(before, after);
    // Repeated interruption captures from the existing snapshot, not the former sequence.
    let quarter = Some(MaterialTransition {
        source: MaterialSource::Snapshot(&snapshot),
        weight: 0.25,
    });
    let before = m2_material::sample_material_transition(&tracks, &binding, outgoing, quarter);
    snapshot.capture(&tracks, outgoing, None, 0.25);
    let after = m2_material::sample_material_transition(
        &tracks,
        &binding,
        current,
        Some(MaterialTransition {
            source: MaterialSource::Snapshot(&snapshot),
            weight: 0.0,
        }),
    );
    assert_eq!(before, after);
}

// Count only this test thread, excluding fixtures, the test runner and other tests.
thread_local! {
    static ALLOCATIONS: std::cell::Cell<Option<usize>> = const { std::cell::Cell::new(None) };
}
struct CountingAllocator;
#[global_allocator]
static ALLOCATOR: CountingAllocator = CountingAllocator;

fn count_allocation() {
    ALLOCATIONS.with(|count| {
        if let Some(value) = count.get() {
            count.set(Some(value + 1));
        }
    });
}

unsafe impl std::alloc::GlobalAlloc for CountingAllocator {
    unsafe fn alloc(&self, layout: std::alloc::Layout) -> *mut u8 {
        count_allocation();
        unsafe { std::alloc::GlobalAlloc::alloc(&std::alloc::System, layout) }
    }
    unsafe fn dealloc(&self, pointer: *mut u8, layout: std::alloc::Layout) {
        unsafe { std::alloc::GlobalAlloc::dealloc(&std::alloc::System, pointer, layout) }
    }
    unsafe fn realloc(&self, pointer: *mut u8, layout: std::alloc::Layout, size: usize) -> *mut u8 {
        count_allocation();
        unsafe { std::alloc::GlobalAlloc::realloc(&std::alloc::System, pointer, layout, size) }
    }
}

#[test]
fn material_sampling_and_repeated_snapshot_capture_allocate_nothing_after_load() {
    use m2_material::{MaterialSnapshot, MaterialSource, MaterialTransition};
    let (tracks, binding) = transition_fixture();
    let mut snapshot = MaterialSnapshot::new(&tracks);
    let previous = MaterialTime {
        sequence: 0,
        time_ms: 250,
        global_ms: 900,
    };
    ALLOCATIONS.with(|count| count.set(Some(0)));
    for time_ms in 0..100 {
        let time = MaterialTime {
            sequence: 1,
            time_ms,
            global_ms: 900,
        };
        let transition = Some(MaterialTransition {
            source: MaterialSource::Sequence(previous),
            weight: 0.5,
        });
        std::hint::black_box(m2_material::sample_material_transition(
            &tracks, &binding, time, transition,
        ));
        snapshot.capture(&tracks, time, Some(previous), 0.5);
        snapshot.capture(&tracks, time, None, 0.25);
        let transition = Some(MaterialTransition {
            source: MaterialSource::Snapshot(&snapshot),
            weight: 0.25,
        });
        std::hint::black_box(m2_material::sample_material_transition(
            &tracks, &binding, time, transition,
        ));
    }
    let count = ALLOCATIONS.with(|count| count.replace(None).unwrap());
    assert_eq!(
        count, 0,
        "material frame sampling and interrupted transitions must not allocate"
    );
}

// Synthetic multi-value global fixture: the five bounded real-asset controls have
// only single-table globals. This establishes the approved selection rule, not a
// measured target-client multi-value oracle.
fn global_value_fixture() -> (MaterialTracks, BatchBinding) {
    let (mut tracks, binding) = transition_fixture();
    tracks.colors[0].color.global_sequence = 0;
    tracks.colors[0].color.sequences = vec![
        (vec![0, 1000], vec![[0.0; 3], [1.0; 3]]),
        (vec![100, 2100], vec![[1.0; 3], [0.0; 3]]),
        (vec![0, 1000], vec![]),
    ];
    tracks.weights[0].global_sequence = 0;
    tracks.weights[0].sequences = vec![
        (vec![0, 1000], vec![0, 32767]),
        (vec![], vec![32767, 0]),
        (vec![0, 1000], vec![]),
    ];
    (tracks, binding)
}

#[test]
fn global_material_timestamps_and_values_select_independently() {
    let (tracks, binding) = global_value_fixture();
    for (sequence, expected) in [(1, 0.75), (2, 1.0), (9, 0.25)] {
        let sample = m2_material::sample_material(
            &tracks,
            &binding,
            MaterialTime {
                sequence,
                time_ms: 990,
                global_ms: 1250,
            },
        );
        assert_eq!(sample.mesh_color, [expected; 3], "sequence {sequence}");
        assert!(
            (sample.texture_weights[0] - expected).abs() < 1.0 / 32767.0,
            "sequence {sequence}: {:?}",
            sample.texture_weights
        );
    }
}

#[test]
fn multi_value_globals_ignore_sequence_and_snapshot_transition_sources() {
    use m2_material::{MaterialSnapshot, MaterialSource, MaterialTransition};
    let (tracks, binding) = global_value_fixture();
    let snapshot = MaterialSnapshot::new(&tracks);
    let current = MaterialTime {
        sequence: 1,
        time_ms: 990,
        global_ms: 1250,
    };
    let outgoing = MaterialTime {
        sequence: 0,
        time_ms: 500,
        global_ms: 1250,
    };
    for source in [
        MaterialSource::Sequence(outgoing),
        MaterialSource::Snapshot(&snapshot),
    ] {
        let sample = m2_material::sample_material_transition(
            &tracks,
            &binding,
            current,
            Some(MaterialTransition {
                source,
                weight: 0.5,
            }),
        );
        assert_eq!(sample.mesh_color, [0.75; 3]);
        assert!((sample.texture_weights[0] - 0.75).abs() < 1.0 / 32767.0);
    }
}

#[test]
fn global_step_and_zero_duration_clock_keep_selected_value_timeline() {
    let (mut tracks, binding) = global_value_fixture();
    let time = MaterialTime {
        sequence: 1,
        time_ms: 990,
        global_ms: 1250,
    };
    tracks.weights[0].interpolation_type = 0;
    assert_eq!(
        m2_material::sample_material(&tracks, &binding, time).texture_weights[0],
        1.0
    );
    tracks.global_sequences[0] = 0;
    tracks.weights[0].interpolation_type = 1;
    let sample = m2_material::sample_material(&tracks, &binding, time);
    assert_eq!(sample.texture_weights[0], 1.0);
    assert_eq!(sample.mesh_color, [1.0; 3]);
}
