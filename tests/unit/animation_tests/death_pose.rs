use super::*;
use shared::components::{Health as NetHealth, Npc};

fn anim_model() -> (M2AnimPlayer, M2AnimData) {
    (
        M2AnimPlayer {
            current_seq_idx: 0,
            time_ms: 0.0,
            looping: true,
            transition: None,
        },
        M2AnimData {
            bones: vec![],
            spherical_billboards: vec![],
            sequences: vec![
                sequence(ANIM_STAND, 1000),
                sequence(crate::animation::death::ANIM_DEATH, 1200),
            ],
            bone_tracks: vec![].into(),
            joint_entities: vec![],
        },
    )
}

fn npc_with_model(app: &mut App, current: f32) -> (Entity, Entity) {
    let model = app.world_mut().spawn(anim_model()).id();
    let root = app
        .world_mut()
        .spawn(Transform::default())
        .add_child(model)
        .id();
    let npc = app
        .world_mut()
        .spawn((
            Npc {
                template_id: 6,
                name: "Kobold Vermin".into(),
            },
            NetHealth {
                current,
                max: 100.0,
            },
        ))
        .add_child(root)
        .id();
    (npc, model)
}

fn death_app() -> App {
    let mut app = App::new();
    app.add_plugins(MinimalPlugins);
    app.add_systems(
        Update,
        (
            crate::animation::death::clear_revived_unit_models,
            crate::animation::death::mark_dead_unit_models,
            switch_animation,
            crate::animation::death::play_death_animation,
        )
            .chain(),
    );
    app
}

#[test]
fn a_dead_npc_plays_death_once_and_keeps_its_last_frame() {
    let mut app = death_app();
    let (npc, model) = npc_with_model(&mut app, 100.0);
    app.update();
    assert!(
        app.world()
            .get::<crate::animation::death::DeathPose>(model)
            .is_none(),
        "alive"
    );

    app.world_mut().get_mut::<NetHealth>(npc).unwrap().current = 0.0;
    app.update();
    app.update();

    let player = app.world().get::<M2AnimPlayer>(model).unwrap();
    assert_eq!(player.current_seq_idx, 1, "Death is playing");
    assert!(!player.looping, "and stops on its last frame");
}

#[test]
fn stand_and_movement_leave_a_corpse_alone() {
    let mut app = death_app();
    let (_, model) = npc_with_model(&mut app, 0.0);
    app.update();
    app.world_mut()
        .get_mut::<M2AnimPlayer>(model)
        .unwrap()
        .time_ms = 1200.0;
    app.update();
    let player = app.world().get::<M2AnimPlayer>(model).unwrap();
    assert_eq!(player.current_seq_idx, 1);
    assert_eq!(player.time_ms, 1200.0);
}

#[test]
fn a_dead_player_lies_as_a_corpse_until_revived() {
    let mut app = death_app();
    // A player's animated model is the player entity itself.
    let (player_anim, data) = anim_model();
    let player = app
        .world_mut()
        .spawn((
            shared::components::Player {
                name: "Theron".into(),
                race: 1,
                class: 1,
                appearance: Default::default(),
            },
            NetHealth {
                current: 0.0,
                max: 100.0,
            },
            player_anim,
            data,
        ))
        .id();
    app.update();
    app.update();
    let anim = app.world().get::<M2AnimPlayer>(player).unwrap();
    assert_eq!(anim.current_seq_idx, 1, "a dead player plays Death");
    assert!(!anim.looping);

    app.world_mut()
        .get_mut::<NetHealth>(player)
        .unwrap()
        .current = 50.0;
    app.update();
    app.update();
    let anim = app.world().get::<M2AnimPlayer>(player).unwrap();
    assert_eq!(anim.current_seq_idx, 0, "a revived player stands again");
}
