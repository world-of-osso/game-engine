use super::*;

#[test]
fn apply_animation_updates_each_model_with_its_own_data() {
    let mut app = App::new();
    app.add_plugins(MinimalPlugins);
    app.add_systems(Update, apply_animation);

    let joint_a = app
        .world_mut()
        .spawn((Transform::IDENTITY, BonePivot(Vec3::ZERO)))
        .id();
    let joint_b = app
        .world_mut()
        .spawn((Transform::IDENTITY, BonePivot(Vec3::ZERO)))
        .id();

    let player_a = M2AnimPlayer {
        current_seq_idx: 0,
        time_ms: 0.0,
        looping: true,
        transition: None,
    };
    let player_b = M2AnimPlayer {
        current_seq_idx: 0,
        time_ms: 0.0,
        looping: true,
        transition: None,
    };
    let data_a = M2AnimData {
        bones: single_root_bone(),
        spherical_billboards: vec![false],
        sequences: vec![stand_sequence()],
        bone_tracks: vec![stationary_bone([1.0, 2.0, 3.0])],
        joint_entities: vec![joint_a],
    };
    let data_b = M2AnimData {
        bones: single_root_bone(),
        spherical_billboards: vec![false],
        sequences: vec![stand_sequence()],
        bone_tracks: vec![stationary_bone([4.0, 5.0, 6.0])],
        joint_entities: vec![joint_b],
    };

    app.world_mut().spawn((player_a, data_a));
    app.world_mut().spawn((player_b, data_b));

    app.update();

    let transform_a = app.world().get::<Transform>(joint_a).unwrap();
    let transform_b = app.world().get::<Transform>(joint_b).unwrap();
    assert_eq!(transform_a.translation, Vec3::new(1.0, 3.0, -2.0));
    assert_eq!(transform_b.translation, Vec3::new(4.0, 6.0, -5.0));
}

fn assert_animation_plugin_runs_in_state(state: GameState, message: &str) {
    let mut app = App::new();
    app.add_plugins((MinimalPlugins, bevy::state::app::StatesPlugin));
    app.init_state::<GameState>();
    app.insert_state(state);
    app.add_plugins(AnimationPlugin);

    let joint = app
        .world_mut()
        .spawn((Transform::IDENTITY, BonePivot(Vec3::ZERO)))
        .id();
    app.world_mut().spawn((
        M2AnimPlayer {
            current_seq_idx: 0,
            time_ms: 0.0,
            looping: true,
            transition: None,
        },
        M2AnimData {
            bones: single_root_bone(),
            spherical_billboards: vec![false],
            sequences: vec![stand_sequence()],
            bone_tracks: vec![stationary_bone([1.0, 2.0, 3.0])],
            joint_entities: vec![joint],
        },
    ));

    app.update();

    let transform = app.world().get::<Transform>(joint).unwrap();
    assert_eq!(
        transform.translation,
        Vec3::new(1.0, 3.0, -2.0),
        "{message}"
    );
}

#[test]
fn animation_plugin_runs_on_char_select() {
    assert_animation_plugin_runs_in_state(
        GameState::CharSelect,
        "char-select models should sample their idle pose instead of staying in rest pose",
    );
}

#[test]
fn animation_plugin_runs_on_char_create() {
    assert_animation_plugin_runs_in_state(
        GameState::CharCreate,
        "char-create models should sample their idle pose instead of staying in rest pose",
    );
}

#[test]
fn animation_plugin_runs_on_inworld_selection_debug() {
    assert_animation_plugin_runs_in_state(
        GameState::InWorldSelectionDebug,
        "in-world selection debug models should sample their idle pose",
    );
}

#[test]
fn looping_stand_advances_into_next_authored_variant() {
    let mut player = M2AnimPlayer {
        current_seq_idx: 0,
        time_ms: 900.0,
        looping: true,
        transition: None,
    };
    let data = M2AnimData {
        bones: vec![],
        spherical_billboards: vec![],
        sequences: vec![
            stand_sequence_with_next(1000, 0, 1),
            stand_sequence_with_next(2000, 1, -1),
        ],
        bone_tracks: vec![],
        joint_entities: vec![],
    };

    advance_player_time(&mut player, &data, 200.0);

    assert_eq!(
        player.current_seq_idx, 1,
        "should follow the authored idle chain"
    );
    assert_eq!(
        player.time_ms, 100.0,
        "should preserve overflow into the next variant"
    );
    assert!(
        player.transition.is_some(),
        "switching stand variants should crossfade"
    );
}
