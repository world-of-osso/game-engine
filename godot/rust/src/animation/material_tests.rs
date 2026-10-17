//! Actual base transition ordering, including an interrupted bone snapshot.
use super::AnimationState;
use game_engine_core::{
    m2,
    m2_material::{self, MaterialTime, MaterialTracks},
};
use std::{fs, path::PathBuf};

fn wings() -> m2::Model {
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../data/models");
    let read = |name: &str| fs::read(root.join(name)).expect("authentic wings fixture");
    m2::parse_model(&read("165644.m2"), &read("16564400.skin")).expect("classic wings")
}

#[test]
fn material_base_transition_samples_authentic_wings_at_each_sequence_clock() {
    let model = wings();
    let tracks = MaterialTracks::of(&model);
    let binding = m2_material::batch_binding(&model, &model.batches[0], &[0; 3]).unwrap();
    let mut player = AnimationState::new(&model).unwrap();
    player.advance(100.0).unwrap();
    player.select(1, true).unwrap();
    player.advance(75.0).unwrap();
    let from = m2_material::sample_material(
        &tracks,
        &binding,
        MaterialTime {
            sequence: 0,
            time_ms: 175,
            global_ms: 900,
        },
    );
    let to = m2_material::sample_material(
        &tracks,
        &binding,
        MaterialTime {
            sequence: 1,
            time_ms: 75,
            global_ms: 900,
        },
    );
    assert_ne!(from.texture_weights[0], to.texture_weights[0]);
    let actual = player.sample_material(&binding, 900);
    assert!(
        (actual.texture_weights[0] - (from.texture_weights[0] + to.texture_weights[0]) * 0.5).abs()
            < 1e-6,
        "from={from:?} to={to:?} actual={actual:?}"
    );
    assert_eq!(
        actual.mesh_color, to.mesh_color,
        "step gold tint remains held"
    );
}

#[test]
fn material_base_retransition_has_no_zero_elapsed_pop() {
    let model = wings();
    let binding = m2_material::batch_binding(&model, &model.batches[0], &[0; 3]).unwrap();
    let mut player = AnimationState::new(&model).unwrap();
    player.advance(100.0).unwrap();
    player.select(1, true).unwrap();
    player.advance(60.0).unwrap();
    let before = player.sample_material(&binding, 900);
    player.select(0, true).unwrap();
    let after = player.sample_material(&binding, 900);
    assert_eq!(before, after);
    player.advance(40.0).unwrap();
    let before = player.sample_material(&binding, 900);
    player.select(2, true).unwrap();
    assert_eq!(before, player.sample_material(&binding, 900));
}
