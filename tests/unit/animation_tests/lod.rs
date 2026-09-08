use super::*;
use crate::animation::lod::{AnimationLod, animation_lod};
use crate::networking_npc::NpcVisualRoot;
use crate::rendering::camera::WowCamera;
use bevy::camera::visibility::ViewVisibility;

/// M2 track translation and its Bevy-space joint translation (x, z, -y).
const M2_TRACK: [f32; 3] = [1.0, 2.0, 3.0];
const JOINT_POSE: Vec3 = Vec3::new(1.0, 3.0, -2.0);

#[test]
fn lod_thresholds_follow_distance_and_visibility() {
    assert_eq!(animation_lod(10.0, true), AnimationLod::Full);
    assert_eq!(animation_lod(30.0, true), AnimationLod::Full);
    assert_eq!(animation_lod(30.1, true), AnimationLod::Half);
    assert_eq!(animation_lod(60.0, true), AnimationLod::Half);
    assert_eq!(animation_lod(60.1, true), AnimationLod::Frozen);
    assert_eq!(animation_lod(10.0, false), AnimationLod::Frozen);
}

#[test]
fn half_rate_samples_alternate_frames() {
    let owner = Entity::from_raw_u32(7).unwrap();
    let samples: Vec<bool> = (0..4)
        .map(|frame| AnimationLod::Half.samples_frame(frame, owner))
        .collect();
    assert_eq!(samples, vec![false, true, false, true]);
    assert!(AnimationLod::Full.samples_frame(3, owner));
    assert!(!AnimationLod::Frozen.samples_frame(2, owner));
}

fn inworld_app() -> App {
    let mut app = App::new();
    app.add_plugins((
        MinimalPlugins,
        bevy::state::app::StatesPlugin,
        bevy::transform::TransformPlugin,
    ));
    app.insert_state(GameState::InWorld);
    app.add_plugins(AnimationPlugin);
    app
}

fn spawn_camera(app: &mut App) {
    app.world_mut()
        .spawn((WowCamera::default(), Transform::IDENTITY));
}

/// Returns the model owner and its single joint. The joint starts at identity.
fn spawn_model(app: &mut App, parent: Entity, view: ViewVisibility) -> (Entity, Entity) {
    let world = app.world_mut();
    let joint = world
        .spawn((Transform::IDENTITY, BonePivot(Vec3::ZERO)))
        .id();
    let mesh = world.spawn((Transform::IDENTITY, view)).id();
    let model = world
        .spawn((
            Transform::IDENTITY,
            ChildOf(parent),
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
                bone_tracks: vec![stationary_bone(M2_TRACK)],
                joint_entities: vec![joint],
            },
        ))
        .id();
    world.entity_mut(mesh).insert(ChildOf(model));
    world.entity_mut(joint).insert(ChildOf(model));
    (model, joint)
}

fn spawn_npc(app: &mut App, distance: f32, view: ViewVisibility) -> (Entity, Entity) {
    let world = app.world_mut();
    let root = world
        .spawn(Transform::from_translation(Vec3::new(0.0, 0.0, distance)))
        .id();
    let visual_root = world
        .spawn((NpcVisualRoot, Transform::IDENTITY, ChildOf(root)))
        .id();
    spawn_model(app, visual_root, view)
}

/// Two updates bind the Bevy player and propagate `GlobalTransform` before measuring.
fn warm_up(app: &mut App, joint: Entity) {
    app.update();
    app.update();
    reset_joint(app, joint);
}

fn reset_joint(app: &mut App, joint: Entity) {
    *app.world_mut().get_mut::<Transform>(joint).unwrap() = Transform::IDENTITY;
}

fn joint_written(app: &App, joint: Entity) -> bool {
    app.world().get::<Transform>(joint).unwrap().translation == JOINT_POSE
}

#[test]
fn near_visible_npc_samples_every_frame() {
    let mut app = inworld_app();
    spawn_camera(&mut app);
    let (_, joint) = spawn_npc(&mut app, 10.0, ViewVisibility::VISIBLE);
    warm_up(&mut app, joint);

    for _ in 0..2 {
        app.update();
        assert!(joint_written(&app, joint));
        reset_joint(&mut app, joint);
    }
}

#[test]
fn far_visible_npc_is_frozen() {
    let mut app = inworld_app();
    spawn_camera(&mut app);
    let (model, joint) = spawn_npc(&mut app, 80.0, ViewVisibility::VISIBLE);
    warm_up(&mut app, joint);

    app.update();
    app.update();
    assert_eq!(
        app.world().get::<AnimationLod>(model),
        Some(&AnimationLod::Frozen)
    );
    assert!(!joint_written(&app, joint));
}

#[test]
fn near_offscreen_npc_is_frozen() {
    let mut app = inworld_app();
    spawn_camera(&mut app);
    let (_, joint) = spawn_npc(&mut app, 10.0, ViewVisibility::HIDDEN);
    warm_up(&mut app, joint);

    app.update();
    app.update();
    assert!(!joint_written(&app, joint));
}

#[test]
fn mid_range_npc_samples_every_other_frame() {
    let mut app = inworld_app();
    spawn_camera(&mut app);
    let (model, joint) = spawn_npc(&mut app, 45.0, ViewVisibility::VISIBLE);
    warm_up(&mut app, joint);

    let mut written = Vec::new();
    for _ in 0..4 {
        app.update();
        written.push(joint_written(&app, joint));
        reset_joint(&mut app, joint);
    }
    assert_eq!(
        app.world().get::<AnimationLod>(model),
        Some(&AnimationLod::Half)
    );
    assert!(
        written == [true, false, true, false] || written == [false, true, false, true],
        "unexpected sampling pattern {written:?}"
    );
}

#[test]
fn model_outside_npc_root_keeps_full_rate() {
    let mut app = inworld_app();
    spawn_camera(&mut app);
    let parent = app
        .world_mut()
        .spawn(Transform::from_translation(Vec3::new(0.0, 0.0, 80.0)))
        .id();
    let (model, joint) = spawn_model(&mut app, parent, ViewVisibility::HIDDEN);
    warm_up(&mut app, joint);

    app.update();
    assert!(app.world().get::<AnimationLod>(model).is_none());
    assert!(joint_written(&app, joint));
}

#[test]
fn npc_without_camera_keeps_full_rate() {
    let mut app = inworld_app();
    let (model, joint) = spawn_npc(&mut app, 80.0, ViewVisibility::HIDDEN);
    warm_up(&mut app, joint);

    app.update();
    assert!(app.world().get::<AnimationLod>(model).is_none());
    assert!(joint_written(&app, joint));
}
