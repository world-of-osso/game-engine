use game_engine_core::m2;
use std::path::PathBuf;

fn fixture(name: &str) -> Vec<u8> {
    let path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../data/models")
        .join(name);
    std::fs::read(&path).unwrap_or_else(|error| panic!("{}: {error}", path.display()))
}

#[test]
fn boar_retains_authored_material_controls_skin_and_collision() {
    let model = m2::parse_model(&fixture("boar.m2"), &fixture("boar00.skin")).unwrap();
    assert_eq!(model.skin_fdids, [473273, 473272]);
    assert_eq!(model.skeleton_fdid, None);
    assert!(!model.uses_texture_combiner_combos);
    assert!(model.texture_unit_lookup.is_empty());
    assert_eq!(model.transparency_lookup, [0]);
    assert_eq!(model.uv_animation_lookup, [-1]);
    assert_eq!(model.transparency_tracks.len(), 1);
    assert!(model.texture_animations.is_empty());
    assert_eq!(model.bounding_box_min, [-2.3635879, -1.342723, -1.1234875]);
    assert_eq!(model.bounding_box_max, [2.999217, 2.1709828, 2.4719868]);
    assert_eq!(model.attachments.len(), 12);
    assert_eq!(model.attachments[0].id, 0);
    assert_eq!(model.attachments[0].bone, 48);
    assert_eq!(&model.attachment_lookup[..4], &[0, 1, 2, -1]);
    let collision = model.collision.expect("authored collision");
    assert_eq!(collision.indices.len(), 36);
    assert_eq!(collision.indices[..6], [0, 1, 2, 2, 3, 0]);
    assert_eq!(collision.vertices.len(), 8);
    assert_eq!(collision.vertices[0], [0.30555555, -0.30555555, 0.0]);
    assert_eq!(collision.bounds_max, [0.30555555, 0.30555555, 2.031278]);
}

#[test]
fn hd_model_retains_external_skin_and_skeleton_references() {
    let model = m2::parse_model_with_skeleton(
        &fixture("humanmale_hd.m2"),
        &fixture("humanmale_hd00.skin"),
        Some(&fixture("humanmale_hd.skel")),
    )
    .unwrap();
    assert_eq!(&model.skin_fdids[..3], &[1012983, 1048729, 1048728]);
    assert!(model.skeleton_fdid.is_some());
    assert_eq!(model.attachments.len(), 45);
    assert_eq!(model.attachment_lookup.len(), 75);
    assert!(!model.texture_animations.is_empty());
    assert!(model.collision.is_some());
}

#[test]
fn malformed_authored_collision_is_rejected_at_model_boundary() {
    let mut bytes = fixture("boar.m2");
    // MD21 chunk header (8 bytes), then MD20 collision index count (0xD8).
    bytes[8 + 0xD8..8 + 0xDC].copy_from_slice(&2_u32.to_le_bytes());
    let error = m2::parse_model(&bytes, &fixture("boar00.skin"))
        .err()
        .expect("invalid collision triangle list");
    assert!(error.contains("M2 collision index count 2 is not a triangle list"));
}
