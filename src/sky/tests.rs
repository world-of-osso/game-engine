use std::path::{Path, PathBuf};

use bevy::ecs::system::SystemState;

use super::inworld_skybox::{
    active_wmo_local_skybox_wow_path, bevy_to_wow_position, resolve_inworld_map_id,
    should_replace_skybox,
};
use super::*;
use crate::networking::CurrentZone;
use crate::terrain::AdtManager;
use crate::terrain_objects::WmoLocalSkybox;
use game_engine::culling::{Wmo, WmoGroup};

#[test]
fn game_time_to_clock() {
    assert_eq!(format_game_clock(1440.0), "12:00");
    assert_eq!(format_game_clock(720.0), "06:00");
    assert_eq!(format_game_clock(0.0), "00:00");
    assert_eq!(format_game_clock(2880.0), "00:00");
    assert_eq!(format_game_clock(2160.0), "18:00");
    assert_eq!(format_game_clock(780.0), "06:30");
}

#[test]
fn bevy_position_maps_back_to_wow_axes() {
    assert_eq!(
        bevy_to_wow_position(Vec3::new(1.0, 2.0, 3.0)),
        [1.0, -3.0, 2.0]
    );
}

#[test]
fn char_select_fog_is_not_overwritten_by_sky_updates() {
    let mut app = App::new();
    let initial_charselect_fog = Color::srgb(0.18, 0.2, 0.23);
    let initial_world_fog = Color::BLACK;
    let sky_smog = Color::srgb(0.7, 0.8, 0.9);
    let sky_band2 = Color::srgb(0.4, 0.5, 0.6);

    app.insert_resource(GameTime {
        minutes: 100.0,
        speed: 0.0,
    });
    app.insert_resource(LightKeyframes(vec![LightDataRow {
        time: 0.0,
        direct_color: Color::WHITE,
        ambient_color: Color::WHITE,
        sky_top: Color::WHITE,
        sky_middle: Color::WHITE,
        sky_band1: Color::WHITE,
        sky_band2,
        sky_smog,
        fog_color: Color::WHITE,
    }]));
    let charselect_entity = app
        .world_mut()
        .spawn((
            CharSelectScene,
            DistanceFog {
                color: initial_charselect_fog,
                directional_light_color: initial_charselect_fog,
                directional_light_exponent: 8.0,
                falloff: FogFalloff::Linear {
                    start: 140.0,
                    end: 220.0,
                },
            },
        ))
        .id();
    let world_entity = app
        .world_mut()
        .spawn(DistanceFog {
            color: initial_world_fog,
            directional_light_color: initial_world_fog,
            directional_light_exponent: 8.0,
            falloff: FogFalloff::Linear {
                start: 1.0,
                end: 2.0,
            },
        })
        .id();
    app.add_systems(Update, update_fog);

    app.update();

    let charselect_fog = app
        .world()
        .entity(charselect_entity)
        .get::<DistanceFog>()
        .expect("char select fog");
    let world_fog = app
        .world()
        .entity(world_entity)
        .get::<DistanceFog>()
        .expect("world fog");

    assert_eq!(
        charselect_fog.color.to_srgba(),
        initial_charselect_fog.to_srgba()
    );
    assert_eq!(
        charselect_fog.directional_light_color.to_srgba(),
        initial_charselect_fog.to_srgba()
    );
    assert_eq!(world_fog.color.to_srgba(), sky_smog.to_srgba());
    assert_eq!(
        world_fog.directional_light_color.to_srgba(),
        sky_band2.to_srgba()
    );
}

#[test]
fn resolve_inworld_map_id_prefers_map_name_when_present() {
    let mut adt_manager = AdtManager::default();
    adt_manager.map_name = "azeroth".to_string();
    let current_zone = CurrentZone { zone_id: 999 };

    assert_eq!(resolve_inworld_map_id(&adt_manager, &current_zone), 0);
}

#[test]
fn resolve_inworld_map_id_uses_current_zone_when_map_name_is_empty() {
    let adt_manager = AdtManager::default();
    let current_zone = CurrentZone { zone_id: 42 };

    assert_eq!(resolve_inworld_map_id(&adt_manager, &current_zone), 42);
}

#[test]
fn should_replace_skybox_detects_path_changes() {
    let current = Some(PathBuf::from("data/models/skyboxes/current.m2"));
    let desired = Path::new("data/models/skyboxes/current.m2");

    assert!(!should_replace_skybox(current.as_deref(), Some(desired)));

    let desired_change = Path::new("data/models/skyboxes/other.m2");
    assert!(should_replace_skybox(
        current.as_deref(),
        Some(desired_change)
    ));

    assert!(should_replace_skybox(None, Some(desired)));
    assert!(!should_replace_skybox(None, None));
}

#[test]
fn active_wmo_local_skybox_prefers_nearest_containing_wmo() {
    let mut world = World::default();
    let far_wmo = world
        .spawn((
            Wmo,
            GlobalTransform::from_translation(Vec3::new(50.0, 0.0, 0.0)),
            WmoLocalSkybox {
                wow_path: "world/far/far_skybox.m2".to_string(),
            },
        ))
        .id();
    world.spawn((
        WmoGroup {
            group_index: 0,
            bbox_min: Vec3::splat(-100.0),
            bbox_max: Vec3::splat(100.0),
        },
        ChildOf(far_wmo),
    ));

    let near_wmo = world
        .spawn((
            Wmo,
            GlobalTransform::from_translation(Vec3::ZERO),
            WmoLocalSkybox {
                wow_path: "world/near/near_skybox.m2".to_string(),
            },
        ))
        .id();
    world.spawn((
        WmoGroup {
            group_index: 0,
            bbox_min: Vec3::splat(-5.0),
            bbox_max: Vec3::splat(5.0),
        },
        ChildOf(near_wmo),
    ));

    let mut system_state = SystemState::<(
        Query<(Entity, &GlobalTransform, &WmoLocalSkybox), With<Wmo>>,
        Query<(&WmoGroup, &ChildOf)>,
    )>::new(&mut world);
    let (wmo_query, group_query) = system_state.get(&world);

    let skybox =
        active_wmo_local_skybox_wow_path(Vec3::new(1.0, 1.0, 1.0), &wmo_query, &group_query);

    assert_eq!(skybox.as_deref(), Some("world/near/near_skybox.m2"));
}
