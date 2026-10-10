use super::AnimationState;
use game_engine_core::m2;

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

#[test]
fn aura312_replaces_stand_from_metamorphosis_row2463() {
    // Retail 12.1.0.69933 AnimReplacement row2463, set536, spell187827.
    let mut unit = player(&[0, 25]);
    unit.set_replacements([(0, 25)].into()).unwrap();
    unit.select_animation_id(0, true).unwrap();
    assert_eq!(unit.sequences[unit.current].id, 25);
}

#[test]
fn aura312_replaces_run_from_spectral_sight_row2349() {
    // Retail AnimReplacement row2349, set499, matrix spell1251417.
    let mut unit = player(&[0, 5, 223]);
    unit.set_replacements([(5, 223)].into()).unwrap();
    unit.update_locomotion(5, false, true).unwrap();
    assert_eq!(unit.sequences[unit.current].id, 223);
    assert_eq!(unit.transition.as_ref().unwrap().duration_ms, 200.0);
}

#[test]
fn aura312_removal_restores_source_selection() {
    let mut unit = player(&[0, 25]);
    unit.set_replacements([(0, 25)].into()).unwrap();
    unit.select_animation_id(0, true).unwrap();
    assert_eq!(unit.sequences[unit.current].id, 25);
    unit.set_replacements(Default::default()).unwrap();
    assert_eq!(unit.sequences[unit.current].id, 0);
    assert!(unit.transition.is_some());
}

#[test]
fn aura312_missing_destination_plays_source_not_destination_fallback() {
    let mut unit = player(&[0, 5]);
    unit.set_replacements([(5, 223)].into()).unwrap();
    unit.update_locomotion(5, false, true).unwrap();
    assert_eq!(unit.sequences[unit.current].id, 5);
}
