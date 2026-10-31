use super::{AnimationState, npc_pose_tests};
use game_engine_core::m2;
use std::{fs, path::PathBuf};

fn kokia() -> m2::Model {
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../data/products/wow/88d5e8ef05ec5a8446cdec93d142d7c6/models");
    let read = |name: &str| fs::read(root.join(name)).expect("Kokia asset");
    let bytes = read("1630402.m2");
    let mut model = m2::parse_model_with_skeleton(
        &bytes,
        &read("1630584.skin"),
        Some(&read("1834492.skel")),
        |_| None,
    )
    .unwrap();
    model.parent_animation =
        Some(m2::parse_parent_animation(&bytes, &read("1830371.skel"), |_| None).unwrap());
    model
}

#[test]
fn kokia_selects_child_stand_and_animates_parent_death() {
    let model = kokia();
    let mut state = AnimationState::new(&model).unwrap();
    assert!(state.current < model.sequences.len());
    state.select_animation_id(1, false).unwrap();
    assert_eq!(state.sequences[state.current].id, 1);
    assert!(state.current >= model.sequences.len());
    let start = state.sample_sequence(state.current, 0.0);
    let end = state.sample_sequence(state.current, 1900.0);
    assert!(
        start
            .iter()
            .zip(&end)
            .any(|(a, b)| a.position.distance_to(b.position) > 0.01
                || a.rotation.dot(b.rotation).abs() < 0.99)
    );
    state.advance(2200.0).unwrap();
    assert_eq!(state.sequences[state.current].id, 1);
    assert_eq!(state.time_ms, 1900.0);
}

#[test]
fn parent_pivot_offset_uses_local_nonuniform_scale_before_rotation() {
    let mut model = kokia();
    model.bones[0].pivot = [1.0, 0.0, 0.0];
    let parent = model.parent_animation.as_mut().unwrap();
    parent.bones[0].pivot = [0.0; 3];
    let death = parent
        .sequences
        .iter()
        .position(|clip| clip.id == 1 && clip.variation_id == 0)
        .unwrap();
    let track = &mut std::sync::Arc::make_mut(&mut parent.bone_tracks)[0];
    track.translation.global_sequence = -1;
    track.rotation.global_sequence = -1;
    track.scale.global_sequence = -1;
    track.translation.sequences = vec![(Vec::new(), Vec::new()); parent.sequences.len()];
    track.rotation.sequences = vec![(Vec::new(), Vec::new()); parent.sequences.len()];
    track.scale.sequences = vec![(Vec::new(), Vec::new()); parent.sequences.len()];
    track.rotation.sequences[death] = (vec![0], vec![[32767, 32767, -9597, -9597]]);
    track.scale.sequences[death] = (vec![0], vec![[2.0, 4.0, 3.0]]);
    let mut state = AnimationState::new(&model).unwrap();
    state.select_animation_id(1, false).unwrap();
    let root = state.sample_sequence(state.current, 0.0)[0];
    assert!(
        root.position
            .distance_to(godot::builtin::Vector3::new(0.0, 0.0, -2.0))
            < 0.001,
        "{:?}",
        root.position
    );
}

#[test]
fn different_hierarchy_keeps_child_animations_available() {
    let mut model = kokia();
    let parent = model.parent_animation.as_mut().unwrap();
    parent.bones[0].parent_bone_id = 1;
    let mut state = AnimationState::new(&model).unwrap();
    assert!(!state.select_animation_id(0, true).unwrap());
    assert!(state.select_animation_id(1, false).is_err());
    state.advance(100.0).unwrap();
    assert_eq!(state.sequences[state.current].id, 0);
}

#[test]
fn different_parent_affine_flags_keep_child_animations_available() {
    let mut model = kokia();
    let parent = model.parent_animation.as_mut().unwrap();
    parent.bones[1].flags ^= 1;
    let mut state = AnimationState::new(&model).unwrap();
    assert!(!state.select_animation_id(0, true).unwrap());
    assert!(state.select_animation_id(1, false).is_err());
    state.advance(100.0).unwrap();
    assert_eq!(state.sequences[state.current].id, 0);
}

#[test]
fn blacklist_blocks_parent_but_not_child_and_child_only_still_plays() {
    let mut model = kokia();
    model
        .parent_animation
        .as_mut()
        .unwrap()
        .blacklist
        .extend([0, 1]);
    let mut state = AnimationState::new(&model).unwrap();
    assert!(!state.select_animation_id(0, true).unwrap());
    assert!(state.select_animation_id(1, false).is_err());
    let mut child_only = AnimationState::new(npc_pose_tests::human_male_model()).unwrap();
    child_only.select_animation_id(97, true).unwrap();
    child_only.advance(100.0).unwrap();
    assert_eq!(child_only.sequences[child_only.current].id, 97);
}
