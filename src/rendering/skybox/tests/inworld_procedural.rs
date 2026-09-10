use super::*;
use crate::sky::{SkyDome, SkyMaterial, SkyboxVisualsDisabled};
use bevy::ecs::system::RunSystemOnce;

fn sky_app(active: bool) -> (App, Entity) {
    let mut app = App::new();
    let mut terrain = AdtManager::default();
    terrain.map_name = "azeroth".into();
    app.add_plugins((bevy::app::TaskPoolPlugin::default(), TransformPlugin))
        .init_resource::<Assets<Mesh>>()
        .init_resource::<Assets<StandardMaterial>>()
        .init_resource::<Assets<M2EffectMaterial>>()
        .init_resource::<Assets<SkyboxM2Material>>()
        .init_resource::<Assets<SkyMaterial>>()
        .init_resource::<Assets<Image>>()
        .init_resource::<Assets<SkinnedMeshInverseBindposes>>()
        .init_resource::<creature_display::CreatureDisplayMap>()
        .init_resource::<CurrentZone>()
        .insert_resource(terrain)
        .add_systems(
            Update,
            sync_inworld_authored_skybox.run_if(crate::sky::skybox_visuals_enabled),
        )
        .add_systems(PostUpdate, crate::sky::remove_disabled_sky_domes);
    let clouds = crate::sky::cloud_texture::create_procedural_cloud_maps(
        &mut app.world_mut().resource_mut::<Assets<Image>>(),
    );
    app.insert_resource(clouds);
    let position = Vec3::new(-8977.593, 81.04212, 179.76495);
    app.world_mut().spawn((
        crate::camera::Player,
        LocalPlayer,
        Transform::from_translation(position),
    ));
    let camera = app
        .world_mut()
        .spawn((
            Camera3d::default(),
            Camera {
                is_active: active,
                ..default()
            },
            Transform::from_translation(position + Vec3::Y * 2.0),
        ))
        .id();
    (app, camera)
}

fn dome_entities(world: &mut World) -> Vec<Entity> {
    world
        .query_filtered::<Entity, With<SkyDome>>()
        .iter(world)
        .collect()
}

#[test]
fn inworld_procedural_sky_spawns_at_camera_once_and_tracks_movement() {
    let (mut app, camera) = sky_app(true);
    app.update();
    let domes = dome_entities(app.world_mut());
    assert_eq!(
        domes.len(),
        1,
        "Azeroth LightParams12 requires its normal sky dome"
    );
    let dome = domes[0];
    assert_eq!(app.world().get::<ChildOf>(dome).unwrap().parent(), camera);
    for translation in [
        Vec3::new(-8977.0, 83.0, 180.0),
        Vec3::new(-8950.0, 95.0, 200.0),
    ] {
        app.world_mut()
            .get_mut::<Transform>(camera)
            .unwrap()
            .translation = translation;
        app.update();
        assert_eq!(dome_entities(app.world_mut()), vec![dome]);
        assert!(
            app.world()
                .get::<GlobalTransform>(dome)
                .unwrap()
                .translation()
                .abs_diff_eq(translation, 0.001)
        );
    }
    let material = app
        .world()
        .get::<MeshMaterial3d<SkyMaterial>>(dome)
        .unwrap();
    assert!(
        app.world()
            .resource::<Assets<SkyMaterial>>()
            .get(&material.0)
            .is_some()
    );
    app.world_mut()
        .run_system_once(teardown_inworld_skybox)
        .unwrap();
    assert!(dome_entities(app.world_mut()).is_empty());
}

#[test]
fn inworld_procedural_sky_waits_for_active_camera_and_respects_disable() {
    let (mut app, camera) = sky_app(false);
    app.update();
    assert!(dome_entities(app.world_mut()).is_empty());
    app.world_mut().get_mut::<Camera>(camera).unwrap().is_active = true;
    app.insert_resource(SkyboxVisualsDisabled);
    app.update();
    assert!(dome_entities(app.world_mut()).is_empty());
    app.world_mut().remove_resource::<SkyboxVisualsDisabled>();
    app.update();
    assert_eq!(dome_entities(app.world_mut()).len(), 1);
    app.insert_resource(SkyboxVisualsDisabled);
    app.update();
    assert!(dome_entities(app.world_mut()).is_empty());
}
