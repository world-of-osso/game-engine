use std::path::Path;

use bevy::mesh::VertexAttributeValues;

#[test]
fn torch_glow_bone_uses_spherical_billboard_flag() {
    let path = Path::new("data/models/club_1h_torch_a_01.m2");
    if !path.exists() {
        return;
    }

    let model = game_engine::asset::m2::load_m2_uncached(path, &[0, 0, 0]).unwrap();
    let glow_bone = model.bones.get(1).expect("torch glow bone 1 should exist");
    assert_ne!(glow_bone.flags & 0x8, 0, "torch glow bone should billboard");
}

#[test]
fn torch_glow_batch_is_a_rigid_single_joint_quad() {
    let path = Path::new("data/models/club_1h_torch_a_01.m2");
    if !path.exists() {
        return;
    }

    let model = game_engine::asset::m2::load_m2_uncached(path, &[0, 0, 0]).unwrap();
    let glow_batch = model
        .batches
        .iter()
        .find(|batch| batch.texture_fdid == Some(198077))
        .expect("torch glow batch should use glowwhite32");

    assert_eq!(glow_batch.mesh.count_vertices(), 4);
    let tris = match glow_batch.mesh.indices() {
        Some(bevy::mesh::Indices::U16(v)) => v.len() / 3,
        Some(bevy::mesh::Indices::U32(v)) => v.len() / 3,
        None => 0,
    };
    assert_eq!(tris, 2, "torch glow should remain a quad");

    let Some(VertexAttributeValues::Uint16x4(joints)) = glow_batch
        .mesh
        .attribute(bevy::mesh::Mesh::ATTRIBUTE_JOINT_INDEX)
    else {
        panic!("torch glow batch should have joint indices");
    };
    let Some(VertexAttributeValues::Float32x4(weights)) = glow_batch
        .mesh
        .attribute(bevy::mesh::Mesh::ATTRIBUTE_JOINT_WEIGHT)
    else {
        panic!("torch glow batch should have joint weights");
    };

    for (joint_set, weight_set) in joints.iter().zip(weights.iter()) {
        assert_eq!(joint_set[0], 1, "torch glow quad should bind to bone 1");
        assert_eq!(joint_set[1], 0);
        assert_eq!(joint_set[2], 0);
        assert_eq!(joint_set[3], 0);
        assert!((weight_set[0] - 1.0).abs() < 0.001);
        assert!(weight_set[1].abs() < 0.001);
        assert!(weight_set[2].abs() < 0.001);
        assert!(weight_set[3].abs() < 0.001);
    }
}

#[test]
fn torch_item_model_skin_resolution_restores_missing_body_texture() {
    let path = Path::new("data/models/club_1h_torch_a_01.m2");
    if !path.exists() {
        return;
    }

    let bare_model = game_engine::asset::m2::load_m2_uncached(path, &[0, 0, 0]).unwrap();
    assert!(
        bare_model
            .batches
            .iter()
            .any(|batch| batch.texture_fdid.is_none()),
        "standalone torch should currently reproduce the missing-texture path"
    );

    let outfit_data = game_engine::outfit_data::OutfitData::load(Path::new("data"));
    let skin_fdids = outfit_data
        .resolve_item_model_skin_fdids_for_model_path(path)
        .expect("torch item model skin fdids");
    assert_eq!(skin_fdids, [145303, 0, 0]);

    let resolved_model = game_engine::asset::m2::load_m2_uncached(path, &skin_fdids).unwrap();
    assert!(
        resolved_model
            .batches
            .iter()
            .any(|batch| batch.texture_fdid == Some(145303)),
        "resolved torch should bind its body texture"
    );
    assert!(
        resolved_model
            .batches
            .iter()
            .any(|batch| batch.texture_fdid == Some(198077)),
        "resolved torch should still keep its glow batch"
    );
}
