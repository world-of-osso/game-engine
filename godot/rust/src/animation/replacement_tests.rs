use super::{ActionPriority, AnimationState};
use game_engine_core::{animation_replacements::AnimationReplacementCatalog, m2};

fn player(ids: &[u16]) -> AnimationState {
    let mut unit = super::jump_tests::player(false);
    unit.sequences = ids
        .iter()
        .map(|&id| m2::Sequence {
            id,
            variation_id: 0,
            duration: 1000,
            movespeed: 0.0,
            flags: 0,
            blend_time: 200,
            frequency: 1,
            replay: [0, 0],
            variation_next: -1,
            bounds: [[0.0; 3]; 2],
        })
        .collect();
    unit
}

fn matrix_set(id: u32) -> std::collections::HashMap<u16, u16> {
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../data");
    AnimationReplacementCatalog::load(&root)
        .unwrap()
        .replacements(&[id])
        .unwrap()
}

#[test]
fn aura312_replaces_stand_from_metamorphosis_row2463() {
    // Retail 12.1.0.69933 AnimReplacement row2463, set536, spell187827.
    let mut unit = player(&[0, 25]);
    unit.set_replacements(matrix_set(536)).unwrap();
    unit.select_animation_id(0, true).unwrap();
    assert_eq!(unit.sequences[unit.current].id, 25);
}

#[test]
fn aura312_replaces_run_from_spectral_sight_row2349() {
    // Retail AnimReplacement row2349, set499, matrix spell1251417.
    let mut unit = player(&[0, 5, 223]);
    unit.set_replacements(matrix_set(499)).unwrap();
    unit.update_locomotion(5, false, true).unwrap();
    assert_eq!(unit.sequences[unit.current].id, 223);
    assert_eq!(unit.transition.as_ref().unwrap().duration_ms, 200.0);
}

#[test]
fn aura312_removal_restores_source_selection_without_movement() {
    let mut unit = player(&[0, 25]);
    unit.set_replacements(matrix_set(536)).unwrap();
    assert_eq!(unit.sequences[unit.current].id, 25);
    unit.set_replacements(Default::default()).unwrap();
    assert_eq!(unit.sequences[unit.current].id, 0);
    assert!(unit.transition.is_some());
}

#[test]
fn aura312_missing_destination_plays_source_not_destination_fallback() {
    let mut unit = player(&[0, 5]);
    unit.set_replacements(matrix_set(499)).unwrap();
    unit.update_locomotion(5, false, true).unwrap();
    assert_eq!(unit.sequences[unit.current].id, 5);
    assert_eq!(unit.resolve_clip(5, &[(223, 0)].into()), Some(5));
}

#[test]
fn aura312_retransition_keeps_sampled_pose_and_minimum_movement_blend() {
    let mut unit = player(&[0, 5, 25]);
    unit.sequences[2].blend_time = 60;
    unit.update_locomotion(5, false, true).unwrap();
    unit.advance(70.0).unwrap();
    let previous = unit.poses();
    unit.set_replacements(matrix_set(536)).unwrap();
    unit.update_locomotion(0, false, false).unwrap();
    assert!(super::npc_pose_tests::pose_distance(&unit.poses(), &previous) < 1e-5);
    assert_eq!(unit.transition.as_ref().unwrap().duration_ms, 150.0);
    unit.advance(40.0).unwrap();
    let previous = unit.poses();
    unit.set_replacements(Default::default()).unwrap();
    assert!(super::npc_pose_tests::pose_distance(&unit.poses(), &previous) < 1e-5);
}

#[test]
fn aura312_jump_progression_uses_source_even_when_two_sources_share_destination() {
    let mut unit = player(&[0, 37, 38, 39, 1604]);
    unit.set_replacements(matrix_set(1013)).unwrap();
    unit.update_locomotion(0, true, false).unwrap();
    assert_eq!(unit.sequences[unit.current].id, 1604);
    unit.advance(1000.0).unwrap();
    unit.update_locomotion(0, true, false).unwrap();
    assert_eq!(unit.source_id, 38);
    unit.update_locomotion(0, false, false).unwrap();
    assert_eq!(unit.source_id, 39);
    unit.advance(1000.0).unwrap();
    unit.update_locomotion(0, false, false).unwrap();
    assert_eq!(unit.sequences[unit.current].id, 0);
}

#[test]
fn aura312_reap_action_remaps_and_restores_without_blend_pop() {
    // Matrix Reap1226019: row4876, set1315, 58→812.
    let mut unit = player(&[0, 58, 812]);
    unit.play_action(58, true, ActionPriority::Spell).unwrap();
    unit.advance(100.0).unwrap();
    let previous = unit.poses();
    unit.set_replacements(matrix_set(1315)).unwrap();
    assert_eq!(unit.action_id(), Some(812));
    assert_eq!(unit.poses(), previous);
    unit.advance(50.0).unwrap();
    let previous = unit.poses();
    unit.set_replacements(Default::default()).unwrap();
    assert_eq!(unit.action_id(), Some(58));
    assert_eq!(unit.poses(), previous);
    unit.set_replacements(matrix_set(1315)).unwrap();
    unit.stop_action(58);
    unit.advance(300.0).unwrap();
    assert_eq!(unit.action_id(), None);
}

#[test]
fn aura312_direct_death_selection_also_translates_source() {
    let mut unit = player(&[0, 1, 25]);
    unit.set_replacements([(1, 25)].into()).unwrap();
    unit.play_death();
    assert_eq!(unit.sequences[unit.current].id, 25);
    assert_eq!(unit.source_id, 1);
}
