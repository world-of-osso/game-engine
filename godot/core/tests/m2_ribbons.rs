//! Authenticated Retail1417024 (Prismatic Bauble), not a layout-shaped mock.
//! M2Ribbon field semantics: WebWowViewerCpp M2FileHeader.h and CRibbonEmitter.
use game_engine_core::asset::m2_format::m2_ribbon::parse_ribbon_emitters;
use std::path::Path;

fn source_ribbon() -> game_engine_core::asset::m2_format::m2_ribbon::M2RibbonEmitter {
    let path = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../data/models/1417024.m2");
    let bytes = std::fs::read(&path).unwrap_or_else(|error| panic!("{}: {error}", path.display()));
    let size = u32::from_le_bytes(bytes[4..8].try_into().unwrap()) as usize;
    let ribbons = parse_ribbon_emitters(&bytes[8..8 + size]).unwrap();
    assert_eq!(ribbons.len(), 1);
    ribbons.into_iter().next().unwrap()
}

fn source_model(fdid: u32) -> game_engine_core::m2::Model {
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../data/models");
    let model = std::fs::read(root.join(format!("{fdid}.m2"))).unwrap();
    let skin = std::fs::read(root.join(format!("{fdid}00.skin"))).unwrap();
    game_engine_core::m2::parse_model(&model, &skin).unwrap()
}

#[test]
fn toyfx26_real_animated_ribbon_uv_lookup_samples_global_translation() {
    use game_engine_core::{
        m2_material::{self, MaterialTime, MaterialTracks},
        m2_ribbons,
    };
    let model = source_model(1865728);
    let source = &model.ribbon_emitters[0];
    assert_ne!(model.flags & 0x20000, 0);
    assert_eq!(source.texture_transform_lookup, 0);
    let index = m2_ribbons::texture_transform_index(&model, source, 0)
        .unwrap()
        .unwrap();
    assert_eq!(index, 1);
    assert_eq!(
        m2_ribbons::texture_transform_index(&model, source, 1).unwrap(),
        None
    );
    assert_eq!(
        m2_ribbons::texture_transform_index(&model, source, 2).unwrap(),
        Some(0)
    );
    let tracks = MaterialTracks::of(&model);
    let track = &tracks.transforms[index];
    assert_eq!(track.translation.global_sequence, 1);
    assert_eq!(
        track.translation.sequences[0],
        (vec![0, 1267], vec![[0.0, 0.0, 0.0], [0.0, -1.0, 0.0]])
    );
    let first = m2_material::texture_matrix(track, &tracks, MaterialTime::default());
    let middle = m2_material::texture_matrix(
        track,
        &tracks,
        MaterialTime {
            global_ms: 633,
            ..Default::default()
        },
    );
    assert_eq!(first, m2_material::IDENTITY_MATRIX);
    assert!((middle[5] + 633.0 / 1267.0).abs() < 0.0001);
    assert_eq!(&middle[..5], &first[..5]);
}

fn frame_at(x: f32) -> game_engine_core::m2_ribbons::RibbonFrame {
    use glam::Vec3;
    game_engine_core::m2_ribbons::RibbonFrame {
        position: Vec3::new(x, 0.0, 0.0),
        direction: Vec3::X,
        vertical: Vec3::Y,
    }
}

fn source_values() -> game_engine_core::m2_ribbons::RibbonValues {
    game_engine_core::m2_ribbons::sample(
        &source_ribbon(),
        &game_engine_core::m2::AnimTime {
            sequence: 0,
            time_ms: 0,
            global_ms: 0.0,
            global_sequences: &[],
        },
    )
}

#[test]
fn toyfx26_ribbon_rounds_fractional_rate_and_clamps_short_lifetime() {
    use game_engine_core::m2_ribbons::RibbonTrail;
    let mut source = source_ribbon();
    source.edges_per_second = 2.5;
    source.edge_lifetime = 0.1;
    let mut trail = RibbonTrail::new(&source).unwrap();
    trail
        .advance(0.0, frame_at(0.0), source_values(), true)
        .unwrap();
    trail
        .advance(0.15, frame_at(1.0), source_values(), true)
        .unwrap();
    assert!(
        !trail.triangles(1.0).is_empty(),
        "initial edge survives the source minimum 0.25s lifetime"
    );
    source.edge_lifetime = 1.0;
    let mut trail = RibbonTrail::new(&source).unwrap();
    trail
        .advance(0.0, frame_at(0.0), source_values(), true)
        .unwrap();
    trail
        .advance(0.35, frame_at(1.0), source_values(), true)
        .unwrap();
    assert_eq!(
        trail.triangles(1.0).len(),
        12,
        "ceil(2.5)=3 emits the next edge by 0.35s"
    );
}

#[test]
fn toyfx26_ribbon_long_frame_clamps_aging_to_lifetime() {
    use game_engine_core::m2_ribbons::RibbonTrail;
    let source = source_ribbon();
    let mut trail = RibbonTrail::new(&source).unwrap();
    trail
        .advance(0.0, frame_at(0.0), source_values(), true)
        .unwrap();
    trail
        .advance(0.25, frame_at(1.0), source_values(), true)
        .unwrap();
    trail
        .advance(2.0, frame_at(2.0), source_values(), true)
        .unwrap();
    let minimum_x = trail
        .triangles(1.0)
        .iter()
        .map(|vertex| vertex.position[0])
        .fold(f32::INFINITY, f32::min);
    assert!(
        minimum_x < 1.1,
        "clamped update interpolates surviving samples along the entire 1→2 segment, got {minimum_x}"
    );
}

#[test]
fn toyfx26_ribbon_animated_slot_updates_existing_strip_uvs() {
    use game_engine_core::m2_ribbons::RibbonTrail;
    let mut source = source_ribbon();
    source.texture_rows = 2;
    let mut trail = RibbonTrail::new(&source).unwrap();
    let mut values = source_values();
    trail.advance(0.0, frame_at(0.0), values, true).unwrap();
    trail.advance(0.1, frame_at(1.0), values, true).unwrap();
    values.slot = 1;
    trail.advance(0.01, frame_at(1.1), values, true).unwrap();
    assert!(
        trail
            .triangles(1.0)
            .iter()
            .all(|vertex| vertex.uv[1] >= 0.5),
        "all retained edges switch to the current tile"
    );
}

#[test]
fn toyfx26_rainbow_ribbon_has_source_fifty_edges_and_half_second_lifetime() {
    let ribbon = source_ribbon();
    assert_eq!(ribbon.bone_index, 1);
    assert_eq!(ribbon.position, [0.0, 0.0, 0.0]);
    assert_eq!(ribbon.edges_per_second, 50.0);
    assert_eq!(ribbon.edge_lifetime, 0.5);
    assert_eq!(ribbon.gravity, 0.0);
    assert_eq!((ribbon.texture_rows, ribbon.texture_cols), (1, 1));
    assert_eq!(ribbon.texture_indices, [0]);
    assert_eq!(ribbon.material_indices, [0]);
    assert_eq!(ribbon.color.sequences[0].1, [[1.0; 3]]);
    assert_eq!(ribbon.height_above.sequences[0].1, [0.5555556]);
    assert_eq!(ribbon.height_below.sequences[0].1, [0.5555556]);
    assert_eq!(ribbon.texture_slot.sequences[0].1, [0]);
    assert!(ribbon.visibility.sequences.is_empty());
}

#[test]
fn toyfx26_ribbon_world_edges_preserve_source_width_and_expire_after_half_second() {
    use game_engine_core::{
        m2::AnimTime,
        m2_ribbons::{self, RibbonFrame, RibbonTrail},
    };
    use glam::Vec3;
    let source = source_ribbon();
    let time = AnimTime {
        sequence: 0,
        time_ms: 0,
        global_ms: 0.0,
        global_sequences: &[],
    };
    let values = m2_ribbons::sample(&source, &time);
    assert_eq!(values.color, [1.0; 4]);
    assert!(
        values.enabled,
        "source unauthored visibility defaults to enabled"
    );
    let frame = |x| RibbonFrame {
        position: Vec3::new(x, 0.0, 0.0),
        direction: Vec3::X,
        vertical: Vec3::Y,
    };
    let mut trail = RibbonTrail::new(&source).unwrap();
    trail.advance(0.0, frame(0.0), values, true).unwrap();
    trail.advance(0.48, frame(3.5), values, true).unwrap();
    let vertices = trail.triangles(1.0);
    assert!(!vertices.is_empty());
    let minimum_x = vertices
        .iter()
        .map(|v| v.position[0])
        .fold(f32::INFINITY, f32::min);
    let maximum_x = vertices
        .iter()
        .map(|v| v.position[0])
        .fold(f32::NEG_INFINITY, f32::max);
    assert!((minimum_x - 0.0).abs() < 0.0001);
    assert!((maximum_x - 3.5).abs() < 0.0001);
    assert!(
        vertices
            .iter()
            .all(|v| (v.position[1].abs() - 0.5555556).abs() < 0.0001)
    );
    assert!(vertices.iter().all(|v| v.color == [1.0; 4]));
    trail.advance(0.03, frame(3.57), values, false).unwrap();
    assert!(
        trail.triangles(1.0).iter().all(|v| v.position[0] > 0.01),
        "old world edge expires; remaining edges do not move with their owner"
    );
    trail.advance(0.51, frame(100.0), values, false).unwrap();
    assert!(
        trail.triangles(1.0).is_empty(),
        "stopped emission leaves no edge beyond source lifetime"
    );
}
