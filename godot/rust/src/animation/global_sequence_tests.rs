//! Bone tracks on a global sequence keep their keys in timeline 0 and run on the
//! model's global clock, whichever sequence plays (WebWowViewerCpp animate.h
//! `animateTrack`: `globalSequenceTimes[global_sequence]`, timeline 0 when the
//! track has no timeline for the playing sequence).
use super::AnimationState;
use game_engine_core::m2;
use godot::builtin::Vector3;
use std::{fs, path::PathBuf};

/// creature/boar/boar.m2: bone 0 (root, pivot x -0.2819566) translates x +0.2819566
/// on global sequence 1 (duration 0), one key in timeline 0; 17 sequences.
fn boar() -> m2::Model {
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../data/models");
    let read = |name| fs::read(root.join(name)).expect("authored fixture");
    m2::parse_model(&read("boar.m2"), &read("boar00.skin")).expect("boar")
}

#[test]
fn boar_root_global_translation_applies_in_every_sequence() {
    let model = boar();
    assert_eq!(model.sequences.len(), 17);
    assert_eq!(model.bone_tracks[0].translation.global_sequence, 1);
    let mut player = AnimationState::new(&model).expect("animated boar");
    for index in 0..model.sequences.len() {
        player.select(index, true).expect("boar sequence");
        // Past the crossfade from the previous sequence.
        player.advance(500.0).expect("valid time");
        let root = player.poses()[0].origin;
        assert!(
            root.distance_to(Vector3::ZERO) < 0.0001,
            "sequence {index} (id {}): root at {root}, expected the pivot offset cancelled",
            model.sequences[index].id
        );
    }
}

#[test]
fn global_track_runs_on_global_clock_past_sequence_end() {
    let mut model = boar();
    // Global sequence 2, after the boar's own two.
    model.global_sequences.push(2000);
    let mut tracks = model.bone_tracks.as_ref().clone();
    tracks[0].translation = m2::AnimTrack {
        interpolation_type: 1,
        global_sequence: 2,
        sequences: vec![(vec![0, 2000], vec![[0.0; 3], [20.0, 0.0, 0.0]])],
    };
    model.bone_tracks = std::sync::Arc::new(tracks);
    let pivot = model.bones[0].pivot[0];
    let mut player = AnimationState::new(&model).expect("animated boar");
    // Walk (id 4) lasts 1000 ms; the global sequence keeps counting to 2000 ms.
    let walk = 2;
    assert_eq!(model.sequences[walk].duration, 1000);
    player.select(walk, true).expect("walk");
    player.advance(1234.0).expect("valid time");
    let x = player.poses()[0].origin.x - pivot;
    assert!((x - 12.34).abs() < 0.001, "global translation x={x}");
    player.advance(1000.0).expect("valid time");
    let x = player.poses()[0].origin.x - pivot;
    assert!((x - 2.34).abs() < 0.001, "wrapped global translation x={x}");
}
