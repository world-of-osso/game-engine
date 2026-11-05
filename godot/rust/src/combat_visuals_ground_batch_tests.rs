//! Authored spell ground batches conform to a stepped surface.

use game_engine_core::ground_drape::{
    BatchMesh, GroundDrape, MAX_DRAPED_EDGE_SPAN, MAX_SURFACE_RISE,
};
use glam::{Affine3A, Quat, Vec3};

/// Death and Decay's ground kit (visual 173378, kit 230208) draws two flat quads.
const GROUND_KIT_MODEL: u32 = 1_522_961;

fn data_root() -> std::path::PathBuf {
    std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../data")
}

/// The kit's authored batches in engine space, with the kit's own placement: attach 570339
/// scales by 0.75 and effect name 29394 by 3.0, and offsets the ground model one yard up.
const KIT_SCALE: f32 = 0.75 * 3.0;

/// The kit's authored batches in engine space, with the kit's own placement.
fn kit_batches() -> Vec<(Affine3A, game_engine_core::m2::SubmeshArrays)> {
    let data = data_root();
    let path = data.join("models").join(format!("{GROUND_KIT_MODEL}.m2"));
    let model = crate::assets::read_model_file(&path).expect("ground kit model");
    let batches = game_engine_core::m2::resolve_render_batches(&model, &[0; 3], true, |_| None)
        .expect("resolved batches");
    let placement = Affine3A::from_scale_rotation_translation(
        Vec3::splat(KIT_SCALE),
        Quat::IDENTITY,
        Vec3::new(0.0, 1.0, 0.0),
    );
    batches
        .iter()
        .filter_map(|batch| {
            let submesh = model.submeshes.get(batch.submesh_index)?;
            let arrays = game_engine_core::m2::submesh_arrays(&model, submesh).ok()?;
            Some((placement, arrays))
        })
        .collect()
}

#[test]
fn death_and_decay_ground_batches_follow_a_step() {
    let batches = kit_batches();
    assert_eq!(batches.len(), 3, "the kit draws three batches");
    let drape = GroundDrape::default();
    let mut ground_batches = 0;
    for (placement, arrays) in &batches {
        let indices: Vec<u32> = arrays.indices.iter().map(|&index| index as u32).collect();
        let mesh = BatchMesh {
            positions: &arrays.positions,
            uvs: &arrays.uv,
            uvs2: Some(&arrays.uv2),
            colors: None,
            indices: &indices,
        };
        if !drape.is_ground_batch(&mesh, *placement, 0.0) {
            continue;
        }
        ground_batches += 1;
        // The authored quad is 7.85 yd at scale 1: 17.66 yd across at the kit's own scale.
        let authored_extent = (arrays
            .positions
            .iter()
            .map(|point| point[0])
            .fold(f32::NEG_INFINITY, f32::max)
            - arrays
                .positions
                .iter()
                .map(|point| point[0])
                .fold(f32::INFINITY, f32::min))
            * KIT_SCALE;
        assert!(
            (authored_extent - 17.66).abs() < 0.05,
            "kit disc extent {authored_extent}"
        );
        // Ground under the placement, with a one-yard step at x = 0.
        let step = |x: f32| if x > 0.0 { 1.0 } else { 0.0 };
        let draped = drape
            .drape_batch(
                &mesh,
                *placement,
                0.0,
                MAX_SURFACE_RISE,
                |x, _| Some(step(x)),
                |x, _, _, _| Some(step(x)),
            )
            .expect("ground batch drapes");
        assert!(draped.dropped > 0, "no vertex found the ground");
        // The authored quad is planar; on a step its world heights must split into two
        // levels (draped positions are model space, so the placement scales them back).
        let world_y = |point: &[f32; 3]| placement.transform_point3(Vec3::from_array(*point)).y;
        let lowest = draped
            .positions
            .iter()
            .fold(f32::INFINITY, |low, point| low.min(world_y(point)));
        let highest = draped
            .positions
            .iter()
            .fold(f32::NEG_INFINITY, |high, point| high.max(world_y(point)));
        assert!(
            highest - lowest > 0.5,
            "draped heights span {}",
            highest - lowest
        );
        // No kept triangle bridges the step: every edge stays inside one step height.
        for triangle in draped.indices.chunks_exact(3) {
            let mut heights = [0.0f32; 3];
            for (height, index) in heights.iter_mut().zip(triangle) {
                *height = world_y(&draped.positions[*index as usize]);
            }
            let span = heights.iter().copied().fold(f32::NEG_INFINITY, f32::max)
                - heights.iter().copied().fold(f32::INFINITY, f32::min);
            assert!(
                span <= MAX_DRAPED_EDGE_SPAN,
                "kept triangle spans {span}: {heights:?}"
            );
        }
        // UVs stay inside the authored mapping, so the rune keeps its texture.
        let (uv_min, uv_max) = arrays.uv.iter().fold(
            ([f32::INFINITY; 2], [f32::NEG_INFINITY; 2]),
            |(mut low, mut high), uv| {
                for axis in 0..2 {
                    low[axis] = low[axis].min(uv[axis]);
                    high[axis] = high[axis].max(uv[axis]);
                }
                (low, high)
            },
        );
        for uv in &draped.uvs {
            for axis in 0..2 {
                assert!(
                    uv[axis] >= uv_min[axis] - 1e-3 && uv[axis] <= uv_max[axis] + 1e-3,
                    "uv {uv:?} outside authored {uv_min:?}..{uv_max:?}"
                );
            }
        }
    }
    assert_eq!(
        ground_batches, 2,
        "two of the kit's batches are ground quads"
    );
}
