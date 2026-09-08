//! Manual fixed-workload extraction characterization, compatible with the upstream fixture.
//!
//! `WOO_SKIN_BENCH_ITERATIONS` selects 1..=10_000 extractions per round (default 1_000).
//! Seven rounds follow 32 warm-up frames for each census-derived rig/workload pair.
//! Timings include `SkinFixture::extract` (AssetPlugin update, MainWorld transfer, and
//! cached production extraction), but exclude joint mutation, setup, validation, and
//! GPU uploads/readback. Per-extraction clock reads remain in the measurement.
//! Staging bytes mean vector length times matrix size, not capacity or GPU allocation.
//! The visible fixture batches are not a claim about native camera-visible counts.

use std::{collections::BTreeSet, env, mem::size_of, time::Instant};

use super::{Duration, Entity, Mat4, SkinFixture};

const ITERATIONS_ENV: &str = "WOO_SKIN_BENCH_ITERATIONS";
const DEFAULT_ITERATIONS: usize = 1_000;
const MAX_ITERATIONS: usize = 10_000;
const ROUNDS: usize = 7;
const WARMUP_FRAMES: usize = 32;
const JOINT_SPACING: f32 = 0.25;

#[derive(Clone, Copy)]
struct Rig {
    name: &'static str,
    joints: usize,
    batches: usize,
}

const RIGS: [Rig; 2] = [
    Rig {
        name: "human_like",
        joints: 216,
        batches: 23,
    },
    Rig {
        name: "wolf_like",
        joints: 66,
        batches: 2,
    },
];

#[derive(Clone, Copy)]
enum Workload {
    Idle,
    ChangedJoints,
}

impl Workload {
    fn name(self) -> &'static str {
        match self {
            Self::Idle => "idle",
            Self::ChangedJoints => "changed_joints",
        }
    }
}

struct RigFixture {
    skin: SkinFixture,
    joints: Vec<Entity>,
    meshes: Vec<Entity>,
    phase: f32,
}

impl RigFixture {
    fn new(rig: Rig) -> Self {
        let mut skin = SkinFixture::new();
        let positions: Vec<_> = (0..rig.joints)
            .map(|index| index as f32 * JOINT_SPACING)
            .collect();
        let joints: Vec<_> = positions.iter().map(|&x| skin.joint(x)).collect();
        let inverse_positions: Vec<_> = positions.iter().map(|&x| -x).collect();
        let bindposes = skin.bindposes(&inverse_positions);
        let meshes = (0..rig.batches)
            .map(|_| skin.mesh(&joints, &bindposes))
            .collect();
        skin.extract();
        Self {
            skin,
            joints,
            meshes,
            phase: 0.0,
        }
    }

    fn prepare_frame(&mut self, workload: Workload) {
        if matches!(workload, Workload::Idle) {
            return;
        }
        self.phase = if self.phase == 1.0 { 2.0 } else { 1.0 };
        for (index, &joint) in self.joints.iter().enumerate() {
            let x = index as f32 * JOINT_SPACING + self.phase;
            self.skin.move_joint(joint, x);
        }
    }

    fn assert_current_matrices(&self) {
        let expected = vec![self.phase; self.joints.len()];
        for &mesh in &self.meshes {
            self.skin.assert_palette(mesh, &expected);
        }
        assert_eq!(self.skin.uniforms().all_skins().count(), self.meshes.len());
    }

    fn distinct_offsets(&self) -> usize {
        self.meshes
            .iter()
            .map(|&mesh| self.skin.offset(mesh))
            .collect::<BTreeSet<_>>()
            .len()
    }

    fn staging_bytes(&self) -> usize {
        self.skin.uniforms().current_staging_buffer.len() * size_of::<Mat4>()
    }
}

#[test]
#[ignore = "manual Vulkan extraction characterization; no timing pass threshold"]
fn characterize_skin_palette_extraction() {
    let iterations = read_iterations();
    for rig in RIGS {
        for workload in [Workload::Idle, Workload::ChangedJoints] {
            characterize_workload(rig, workload, iterations);
        }
    }
}

fn read_iterations() -> usize {
    let iterations = match env::var(ITERATIONS_ENV) {
        Ok(value) => value
            .parse::<usize>()
            .expect("WOO_SKIN_BENCH_ITERATIONS must be a positive integer"),
        Err(env::VarError::NotPresent) => DEFAULT_ITERATIONS,
        Err(error) => panic!("cannot read {ITERATIONS_ENV}: {error}"),
    };
    assert!(
        (1..=MAX_ITERATIONS).contains(&iterations),
        "{ITERATIONS_ENV} must be between 1 and {MAX_ITERATIONS}"
    );
    iterations
}

fn characterize_workload(rig: Rig, workload: Workload, iterations: usize) {
    let mut fixture = RigFixture::new(rig);
    for _ in 0..WARMUP_FRAMES {
        fixture.prepare_frame(workload);
        fixture.skin.extract();
    }
    fixture.assert_current_matrices();
    let timings: Vec<_> = (0..ROUNDS)
        .map(|_| measure_round(&mut fixture, workload, iterations))
        .collect();
    fixture.assert_current_matrices();
    report_result(rig, workload, iterations, &fixture, &timings);
}

fn measure_round(fixture: &mut RigFixture, workload: Workload, iterations: usize) -> f64 {
    let mut elapsed = Duration::ZERO;
    for _ in 0..iterations {
        fixture.prepare_frame(workload);
        let started = Instant::now();
        fixture.skin.extract();
        elapsed += started.elapsed();
    }
    elapsed.as_nanos() as f64 / iterations as f64
}

fn report_result(
    rig: Rig,
    workload: Workload,
    iterations: usize,
    fixture: &RigFixture,
    timings: &[f64],
) {
    let mut ordered = timings.to_vec();
    ordered.sort_by(f64::total_cmp);
    println!(
        "SKIN_BENCH {{\"rig\":\"{}\",\"workload\":\"{}\",\"joints\":{},\"meshes\":{},\"iterations_per_round\":{},\"rounds\":{},\"distinct_offsets\":{},\"staging_matrix_bytes\":{},\"ns_per_extraction\":{:?},\"median_ns_per_extraction\":{}}}",
        rig.name,
        workload.name(),
        rig.joints,
        fixture.meshes.len(),
        iterations,
        ROUNDS,
        fixture.distinct_offsets(),
        fixture.staging_bytes(),
        timings,
        ordered[ROUNDS / 2],
    );
}
