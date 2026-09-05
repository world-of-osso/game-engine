use std::time::Duration;

use bevy::asset::uuid_handle;
use bevy::prelude::*;

use crate::sky::SkyEnvMapHandle;

use super::super::TerrainMaterialFreezeAfter;
use super::{
    TerrainMaterial, sync_terrain_environment_map, terrain_material_updates_enabled,
    update_terrain_animation_time,
};

const FIRST_ENVIRONMENT: Handle<Image> = uuid_handle!("00000000-0000-0000-0000-000000000001");
const SECOND_ENVIRONMENT: Handle<Image> = uuid_handle!("00000000-0000-0000-0000-000000000002");

#[test]
fn terrain_material_freeze_stops_updates_at_deadline() {
    let (mut app, material) = terrain_material_test_app(Some(Duration::from_secs(2)));

    advance_time(&mut app, Duration::from_secs(1));
    app.update();
    assert_material_updated(&app, &material, 1.0, FIRST_ENVIRONMENT.clone());

    app.world_mut()
        .insert_resource(SkyEnvMapHandle(SECOND_ENVIRONMENT.clone()));
    advance_time(&mut app, Duration::from_secs(1));
    app.update();
    assert_material_updated(&app, &material, 1.0, FIRST_ENVIRONMENT.clone());
}

#[test]
fn terrain_material_freeze_is_opt_in() {
    let (mut app, material) = terrain_material_test_app(None);

    advance_time(&mut app, Duration::from_secs(1));
    app.update();
    assert_material_updated(&app, &material, 1.0, FIRST_ENVIRONMENT.clone());

    app.world_mut()
        .insert_resource(SkyEnvMapHandle(SECOND_ENVIRONMENT.clone()));
    advance_time(&mut app, Duration::from_secs(1));
    app.update();
    assert_material_updated(&app, &material, 2.0, SECOND_ENVIRONMENT.clone());
}

fn terrain_material_test_app(freeze_after: Option<Duration>) -> (App, Handle<TerrainMaterial>) {
    let mut app = App::new();
    app.insert_resource(Time::<()>::default());
    app.insert_resource(Time::<Real>::default());
    app.init_resource::<Assets<TerrainMaterial>>();
    app.insert_resource(SkyEnvMapHandle(FIRST_ENVIRONMENT.clone()));
    if let Some(freeze_after) = freeze_after {
        app.insert_resource(TerrainMaterialFreezeAfter(freeze_after));
    }
    app.add_systems(
        Update,
        (update_terrain_animation_time, sync_terrain_environment_map)
            .run_if(terrain_material_updates_enabled),
    );

    let material = app
        .world_mut()
        .resource_mut::<Assets<TerrainMaterial>>()
        .add(test_material());
    (app, material)
}

fn advance_time(app: &mut App, elapsed: Duration) {
    app.world_mut().resource_mut::<Time>().advance_by(elapsed);
    app.world_mut()
        .resource_mut::<Time<Real>>()
        .advance_by(elapsed);
}

fn assert_material_updated(
    app: &App,
    handle: &Handle<TerrainMaterial>,
    expected_time: f32,
    expected_environment: Handle<Image>,
) {
    let material = app
        .world()
        .resource::<Assets<TerrainMaterial>>()
        .get(handle)
        .expect("test terrain material");
    assert_eq!(material.settings.config.w, expected_time);
    assert_eq!(material.environment_map, expected_environment);
}

fn test_material() -> TerrainMaterial {
    TerrainMaterial {
        settings: super::super::TerrainMaterialSettings {
            config: Vec4::ZERO,
            surface: Vec4::ZERO,
            layer_params_0: Vec4::ZERO,
            layer_params_1: Vec4::ZERO,
            layer_params_2: Vec4::ZERO,
            layer_params_3: Vec4::ZERO,
            animation_params_0: Vec4::ZERO,
            animation_params_1: Vec4::ZERO,
            animation_params_2: Vec4::ZERO,
            animation_params_3: Vec4::ZERO,
        },
        ground_0: Handle::default(),
        ground_1: Handle::default(),
        ground_2: Handle::default(),
        ground_3: Handle::default(),
        height_0: Handle::default(),
        height_1: Handle::default(),
        height_2: Handle::default(),
        height_3: Handle::default(),
        alpha_packed: Handle::default(),
        shadow_map: Handle::default(),
        environment_map: Handle::default(),
    }
}
