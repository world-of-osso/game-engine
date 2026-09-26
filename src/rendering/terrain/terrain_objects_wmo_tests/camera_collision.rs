use super::*;
use bevy::camera::visibility::{VisibilityPlugin, VisibilitySystems, update_frusta};
use bevy::state::app::StatesPlugin;
use game_engine::camera_control::WowCamera;
use game_engine::culling::{CullingPlugin, Wmo, WmoGroup};
use game_engine::game_state_enum::GameState;

const MAGIC_DISTRICT_ROOT_FDID: u32 = 321999;
/// `sw_magicdistrict` group 58, `BigJailRoom01`: the Stockade entrance stairwell.
const STAIRWELL_GROUP: u16 = 58;

/// Real transform, visibility, frustum, portal culling and camera follow over every
/// `sw_magicdistrict` group, with each group's batch meshes as `WmoCollisionMesh` children.
fn magic_district_camera_app(player_wow: Vec3, camera: WowCamera) -> (App, Entity, Entity) {
    let (placement_transform, root, groups) =
        super::portal_culling::load_tile_30_48_wmo(MAGIC_DISTRICT_ROOT_FDID);
    let mut app = App::new();
    app.add_plugins((
        bevy::app::TaskPoolPlugin::default(),
        AssetPlugin::default(),
        bevy::mesh::MeshPlugin,
        TransformPlugin,
        VisibilityPlugin,
        StatesPlugin,
    ))
    .insert_state(GameState::InWorld)
    .add_plugins(CullingPlugin)
    .init_resource::<Time>()
    .add_systems(Update, crate::rendering::camera::camera_follow_system)
    .add_systems(
        PostUpdate,
        update_frusta.in_set(VisibilitySystems::UpdateFrusta),
    );
    let player = app
        .world_mut()
        .spawn((
            crate::camera::Player,
            Transform::from_translation(wow_to_bevy(player_wow)),
        ))
        .id();
    let camera = app
        .world_mut()
        .spawn((
            Camera3d::default(),
            camera,
            Transform::from_translation(wow_to_bevy(player_wow) + Vec3::Y * 2.0),
        ))
        .id();
    let root_entity = app
        .world_mut()
        .spawn((
            Wmo,
            placement_transform,
            Visibility::default(),
            build_portal_graph(&root),
        ))
        .id();
    let mut commands = app.world_mut().commands();
    for (index, group) in groups.iter().enumerate() {
        let group_entity = spawn_wmo_group_entity(&mut commands, &root, group, index as u16);
        commands.entity(root_entity).add_child(group_entity);
    }
    app.world_mut().flush();
    let group_entities: Vec<(Entity, u16)> = app
        .world_mut()
        .query::<(Entity, &WmoGroup)>()
        .iter(app.world())
        .map(|(entity, group)| (entity, group.group_index))
        .collect();
    for (group_entity, index) in group_entities {
        for batch in &groups[index as usize].batches {
            let mesh = app
                .world_mut()
                .resource_mut::<Assets<Mesh>>()
                .add(batch.mesh.clone());
            app.world_mut().spawn((
                Mesh3d(mesh),
                Transform::default(),
                Visibility::default(),
                crate::collision::WmoCollisionMesh,
                ChildOf(group_entity),
            ));
        }
    }
    (app, player, camera)
}

fn wow_to_bevy(wow: Vec3) -> Vec3 {
    Vec3::new(wow.x, wow.z, -wow.y)
}

fn advance(app: &mut App, seconds: f32) {
    app.world_mut()
        .resource_mut::<Time>()
        .advance_by(std::time::Duration::from_secs_f32(seconds));
    app.update();
}

fn stairwell_visibility(app: &mut App) -> Visibility {
    *app.world_mut()
        .query::<(&WmoGroup, &Visibility)>()
        .iter(app.world())
        .find(|(group, _)| group.group_index == STAIRWELL_GROUP)
        .expect("stairwell group")
        .1
}

/// The user's walk down the Stockade entrance stairs (WoW -8774, 838 to the portal doorway)
/// with the camera low behind the player. Camera smoothing dips the camera a few centimetres
/// under a step, so portal culling finds no stairwell floor below it and hides the stairwell.
/// The camera must still collide with the stairwell walls: portal culling decides what is
/// drawn, not what is solid. Otherwise it swings out through the walls to its full distance
/// and stays outside, where the stairwell stays culled.
#[test]
fn camera_walking_down_the_stockade_stairs_stays_inside_the_stairwell() {
    let start = Vec3::new(-8774.0, 838.0, 92.1);
    let end = Vec3::new(-8766.1, 845.5, 88.0);
    let camera = WowCamera {
        yaw: (-50f32).to_radians(),
        pitch: 10f32.to_radians(),
        distance: 10.0,
        target_distance: 10.0,
        ..default()
    };
    let (mut app, player, camera_entity) = magic_district_camera_app(start, camera);
    for _ in 0..300 {
        advance(&mut app, 1.0 / 60.0);
    }
    assert_eq!(stairwell_visibility(&mut app), Visibility::Visible);

    for step in 0..=90 {
        let walked = start.lerp(end, step as f32 / 90.0);
        app.world_mut()
            .get_mut::<Transform>(player)
            .unwrap()
            .translation = wow_to_bevy(walked);
        advance(&mut app, 1.0 / 60.0);
    }
    for _ in 0..60 {
        advance(&mut app, 1.0 / 60.0);
    }

    let eye = wow_to_bevy(end) + Vec3::Y * 1.8;
    let camera_pos = app
        .world()
        .get::<Transform>(camera_entity)
        .unwrap()
        .translation;
    assert_eq!(
        stairwell_visibility(&mut app),
        Visibility::Visible,
        "camera at WoW ({:.1}, {:.1}, {:.1})",
        camera_pos.x,
        -camera_pos.z,
        camera_pos.y
    );
    assert!(
        camera_pos.distance(eye) < 9.0,
        "the stairwell walls hold the camera in, got {:.1} yd",
        camera_pos.distance(eye)
    );
}
