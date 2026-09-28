use std::path::{Path, PathBuf};
use std::time::Duration;

use bevy::ecs::system::SystemState;

use super::cloud_texture::{CLOUD_REGEN_SECONDS, ProceduralCloudMaps};
use super::inworld_skybox::{
    InWorldSkybox, InWorldSkyboxPhase, active_wmo_local_skybox_wow_path, bevy_to_wow_position,
    resolve_inworld_map_id, should_replace_skybox, sync_inworld_skybox_to_camera,
};
use super::*;
use crate::networking::CurrentZone;
use crate::sky_lightdata::interpolate_colors;
use crate::terrain::AdtManager;
use crate::terrain_objects::WmoLocalSkybox;
use game_engine::culling::{Wmo, WmoGroup};

mod inworld_ibl;

#[test]
fn game_clock_shows_in_the_minimap_cluster() {
    use bevy::ecs::system::RunSystemOnce;
    use game_engine::ui::registry::FrameRegistry;
    use game_engine::ui::screens::inworld_hud_component::minimap_screen;
    use ui_toolkit::screen::{Screen, SharedContext};

    let mut registry = FrameRegistry::new(1920.0, 1080.0);
    Screen::new(minimap_screen).sync(&SharedContext::new(), &mut registry);
    let mut app = App::new();
    app.insert_resource(UiState {
        registry,
        event_bus: ui_toolkit::event::EventBus::new(),
        focused_frame: None,
    });
    app.insert_resource(GameTime {
        minutes: 2160.0,
        speed: 1.0,
    });
    app.world_mut()
        .run_system_once(update_time_display)
        .unwrap();

    let ui = app.world().resource::<UiState>();
    let id = ui.registry.get_by_name(MINIMAP_CLOCK.0).unwrap();
    let Some(WidgetData::FontString(text)) = &ui.registry.get(id).unwrap().widget_data else {
        panic!("MinimapClock is not a font string");
    };
    assert_eq!(text.text, "18:00");
}

#[test]
fn water_skybox_isolation_removes_dome_without_removing_camera_or_light() {
    for disabled in [false, true] {
        let mut app = App::new();
        app.add_systems(PostUpdate, remove_disabled_sky_domes);
        if disabled {
            app.insert_resource(SkyboxVisualsDisabled);
        }
        let camera = app.world_mut().spawn(Camera3d::default()).id();
        let light = app.world_mut().spawn(DirectionalLight::default()).id();
        let dome = app.world_mut().spawn(SkyDome).id();
        app.update();

        assert_eq!(app.world().get_entity(dome).is_ok(), !disabled);
        assert!(app.world().get::<Camera3d>(camera).is_some());
        assert!(app.world().get::<DirectionalLight>(light).is_some());
    }
}

#[test]
fn water_skybox_isolation_disables_visual_gate_only() {
    use bevy::ecs::system::RunSystemOnce;

    let mut app = App::new();
    assert!(
        app.world_mut()
            .run_system_once(skybox_visuals_enabled)
            .unwrap()
    );
    app.insert_resource(SkyboxVisualsDisabled);
    assert!(
        !app.world_mut()
            .run_system_once(skybox_visuals_enabled)
            .unwrap()
    );

    let mut time = Time::<()>::default();
    time.advance_by(Duration::from_secs(1));
    app.insert_resource(time);
    app.insert_resource(GameTime {
        minutes: 100.0,
        speed: 1.0,
    });
    app.world_mut().run_system_once(advance_game_time).unwrap();
    assert_eq!(app.world().resource::<GameTime>().minutes, 101.0);
}

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
fn elapsed_cloud_regeneration_preserves_active_texture_while_scroll_advances() {
    let mut app = App::new();
    app.insert_resource(State::new(GameState::InWorld));
    app.insert_resource(Time::<()>::default());
    app.insert_resource(GameTime {
        minutes: 100.0,
        speed: 0.0,
    });
    app.insert_resource(LightKeyframes::for_params(
        12,
        vec![LightDataRow {
            time: 0.0,
            direct_color: Color::WHITE,
            ambient_color: Color::WHITE,
            sky_top: Color::WHITE,
            sky_middle: Color::WHITE,
            sky_band1: Color::WHITE,
            sky_band2: Color::WHITE,
            sky_smog: Color::WHITE,
            fog_color: Color::WHITE,
            sun_color: Color::WHITE,
            sun_halo_color: Color::WHITE,
            cloud_emissive_color: Color::WHITE,
            cloud_layer1_ambient_color: Color::WHITE,
            cloud_layer2_ambient_color: Color::WHITE,
            ocean_close_color: Color::WHITE,
            ocean_far_color: Color::WHITE,
            river_close_color: Color::WHITE,
            river_far_color: Color::WHITE,
            horizon_ambient_color: Color::WHITE,
            ground_ambient_color: Color::WHITE,
            fog_end: 1200.0,
            fog_start: 300.0,
            glow: 1.0,
            cloud_density: 0.0,
            unk1: 0.0,
            unk2: 0.0,
        }],
    ));
    app.insert_resource(RetailSceneLight::from_sky_colors(
        &default_sky_colors(),
        0.0,
    ));
    app.insert_resource(Assets::<Image>::default());
    app.insert_resource(Assets::<SkyMaterial>::default());
    app.insert_resource(Assets::<crate::water_material::WaterMaterial>::default());

    let image = || {
        Image::new_fill(
            Extent3d {
                width: 1,
                height: 1,
                depth_or_array_layers: 1,
            },
            TextureDimension::D2,
            &[17, 34, 51, 255],
            TextureFormat::Rgba8Unorm,
            RenderAssetUsages::default(),
        )
    };
    let cloud_handles = [
        app.world_mut().resource_mut::<Assets<Image>>().add(image()),
        app.world_mut().resource_mut::<Assets<Image>>().add(image()),
        app.world_mut().resource_mut::<Assets<Image>>().add(image()),
    ];
    let material_handle = app
        .world_mut()
        .resource_mut::<Assets<SkyMaterial>>()
        .add(SkyMaterial {
            uniforms: SkyUniforms::default(),
            cloud_texture: cloud_handles[0].clone(),
        });
    app.world_mut()
        .spawn((SkyDome, MeshMaterial3d(material_handle.clone())));
    app.insert_resource(ProceduralCloudMaps {
        handles: cloud_handles,
        active_index: 0,
        next_seed: 3,
        regen_timer: Timer::from_seconds(CLOUD_REGEN_SECONDS, TimerMode::Repeating),
    });
    register_shared_sky_visual_systems(&mut app);

    app.update();

    let (active_handle_before, image_bytes_before, scroll_before) = {
        let world = app.world();
        let cloud_maps = world.resource::<ProceduralCloudMaps>();
        let images = world.resource::<Assets<Image>>();
        let materials = world.resource::<Assets<SkyMaterial>>();
        let active_handle = cloud_maps.active_handle();
        let image_bytes = images
            .get(&active_handle)
            .and_then(|image| image.data.as_ref())
            .expect("active cloud image bytes")
            .clone();
        let scroll = materials
            .get(&material_handle)
            .expect("sky material")
            .uniforms
            .cloud_params;
        (active_handle, image_bytes, scroll)
    };

    app.world_mut().resource_mut::<GameTime>().minutes = 200.0;
    app.world_mut()
        .resource_mut::<Time>()
        .advance_by(Duration::from_secs_f32(CLOUD_REGEN_SECONDS));
    app.update();

    let world = app.world();
    let cloud_maps = world.resource::<ProceduralCloudMaps>();
    let images = world.resource::<Assets<Image>>();
    let materials = world.resource::<Assets<SkyMaterial>>();
    let active_handle_after = cloud_maps.active_handle();
    let image_bytes_after = images
        .get(&active_handle_after)
        .and_then(|image| image.data.as_ref())
        .expect("active cloud image bytes after update");
    let scroll_after = materials
        .get(&material_handle)
        .expect("sky material after update")
        .uniforms
        .cloud_params;

    assert_ne!(scroll_after, scroll_before);
    assert_eq!(active_handle_after, active_handle_before);
    assert_eq!(image_bytes_after, &image_bytes_before);
}

#[test]
fn char_select_fog_is_not_overwritten_by_sky_updates() {
    let mut app = App::new();
    let initial_charselect_fog = Color::srgb(0.18, 0.2, 0.23);
    let initial_world_fog = Color::BLACK;
    let sky_smog = Color::srgb(0.7, 0.8, 0.9);
    let sky_band2 = Color::srgb(0.4, 0.5, 0.6);
    let sky_fog = Color::linear_rgb(0.07, 0.19, 0.27);

    app.insert_resource(GameTime {
        minutes: 100.0,
        speed: 0.0,
    });
    app.insert_resource(LightKeyframes::for_params(
        12,
        vec![LightDataRow {
            time: 0.0,
            direct_color: Color::WHITE,
            ambient_color: Color::WHITE,
            sky_top: Color::WHITE,
            sky_middle: Color::WHITE,
            sky_band1: Color::WHITE,
            sky_band2,
            sky_smog,
            fog_color: sky_fog,
            sun_color: Color::WHITE,
            sun_halo_color: Color::WHITE,
            cloud_emissive_color: Color::WHITE,
            cloud_layer1_ambient_color: Color::WHITE,
            cloud_layer2_ambient_color: Color::WHITE,
            ocean_close_color: Color::WHITE,
            ocean_far_color: Color::WHITE,
            river_close_color: Color::WHITE,
            river_far_color: Color::WHITE,
            horizon_ambient_color: Color::WHITE,
            ground_ambient_color: Color::WHITE,
            fog_end: 1200.0,
            fog_start: 300.0,
            glow: 1.0,
            cloud_density: 0.0,
            unk1: 0.0,
            unk2: 0.0,
        }],
    ));
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
    app.insert_resource(RetailSceneLight::from_sky_colors(
        &default_sky_colors(),
        0.0,
    ));
    app.add_systems(Update, update_scene_light);

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
    // World fog is SkyFogColor without a sun glow; LightData distances are yards × 36.
    assert_eq!(world_fog.color.to_srgba(), sky_fog.to_srgba());
    assert_eq!(world_fog.directional_light_color, Color::NONE);
    assert!(matches!(
        world_fog.falloff,
        FogFalloff::Linear { start, end }
            if (start - 300.0 / 36.0).abs() < 0.01 && (end - 1200.0 / 36.0).abs() < 0.01
    ));
}

#[test]
fn weather_change_updates_world_fog_without_time_advance() {
    let mut app = App::new();
    let sky_smog = Color::srgb(0.7, 0.8, 0.9);
    let sky_band2 = Color::srgb(0.4, 0.5, 0.6);
    let sky_fog = Color::linear_rgb(0.07, 0.19, 0.27);

    app.insert_resource(GameTime {
        minutes: 100.0,
        speed: 0.0,
    });
    app.insert_resource(LightKeyframes::for_params(
        12,
        vec![LightDataRow {
            time: 0.0,
            direct_color: Color::WHITE,
            ambient_color: Color::WHITE,
            sky_top: Color::WHITE,
            sky_middle: Color::WHITE,
            sky_band1: Color::WHITE,
            sky_band2,
            sky_smog,
            fog_color: sky_fog,
            sun_color: Color::WHITE,
            sun_halo_color: Color::WHITE,
            cloud_emissive_color: Color::WHITE,
            cloud_layer1_ambient_color: Color::WHITE,
            cloud_layer2_ambient_color: Color::WHITE,
            ocean_close_color: Color::WHITE,
            ocean_far_color: Color::WHITE,
            river_close_color: Color::WHITE,
            river_far_color: Color::WHITE,
            horizon_ambient_color: Color::WHITE,
            ground_ambient_color: Color::WHITE,
            fog_end: 1200.0,
            fog_start: 300.0,
            glow: 1.0,
            cloud_density: 0.0,
            unk1: 0.0,
            unk2: 0.0,
        }],
    ));
    let world_entity = app
        .world_mut()
        .spawn(DistanceFog {
            color: Color::BLACK,
            directional_light_color: Color::BLACK,
            directional_light_exponent: 8.0,
            falloff: FogFalloff::Linear {
                start: 1.0,
                end: 2.0,
            },
        })
        .id();
    app.insert_resource(RetailSceneLight::from_sky_colors(
        &default_sky_colors(),
        0.0,
    ));
    app.add_systems(Update, update_scene_light);

    app.update();
    let clear_fog = app
        .world()
        .entity(world_entity)
        .get::<DistanceFog>()
        .expect("clear fog")
        .clone();
    assert_eq!(clear_fog.color.to_srgba(), sky_fog.to_srgba());
    assert_eq!(clear_fog.directional_light_color, Color::NONE);
    // The Retail scene light carries the same fog the camera received.
    let scene_light = app.world().resource::<RetailSceneLight>().clone();
    let fog_srgb = sky_fog.to_srgba();
    assert!(
        scene_light
            .fog_color
            .abs_diff_eq(Vec3::new(fog_srgb.red, fog_srgb.green, fog_srgb.blue), 1e-5)
    );
    assert!((scene_light.fog_start - 300.0 / 36.0).abs() < 1e-3);
    assert!((scene_light.fog_end - 1200.0 / 36.0).abs() < 1e-3);

    app.insert_resource(crate::weather::ActiveWeather::preset(
        crate::weather::WeatherKind::Sandstorm,
    ));
    app.update();

    let weather_fog = app
        .world()
        .entity(world_entity)
        .get::<DistanceFog>()
        .expect("weather fog");
    assert_ne!(weather_fog.color.to_srgba(), clear_fog.color.to_srgba());
    assert_ne!(
        weather_fog.directional_light_color.to_srgba(),
        clear_fog.directional_light_color.to_srgba()
    );
    assert!(matches!(
        weather_fog.falloff,
        FogFalloff::Linear { start, end }
        if start < 300.0 / 36.0 && end < 1200.0 / 36.0
    ));
}

#[test]
fn resolve_inworld_map_id_prefers_map_name_when_present() {
    let mut adt_manager = AdtManager::default();
    adt_manager.map_name = "azeroth".to_string();
    let current_zone = CurrentZone {
        zone_id: 999,
        ..Default::default()
    };

    assert_eq!(resolve_inworld_map_id(&adt_manager, &current_zone), 0);
}

#[test]
fn resolve_inworld_map_id_uses_current_zone_when_map_name_is_empty() {
    let adt_manager = AdtManager::default();
    let current_zone = CurrentZone {
        zone_id: 42,
        ..Default::default()
    };

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
            is_exterior: false,
            is_antiportal: false,
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
            is_exterior: false,
            is_antiportal: false,
        },
        ChildOf(near_wmo),
    ));

    let mut system_state = SystemState::<(
        Query<(Entity, &GlobalTransform, &WmoLocalSkybox), With<Wmo>>,
        Query<(&WmoGroup, &ChildOf)>,
    )>::new(&mut world);
    let (wmo_query, group_query) = system_state
        .get(&world)
        .expect("WMO skybox query system state");

    let skybox =
        active_wmo_local_skybox_wow_path(Vec3::new(1.0, 1.0, 1.0), &wmo_query, &group_query);

    assert_eq!(skybox.as_deref(), Some("world/near/near_skybox.m2"));
}

#[test]
fn sync_inworld_skybox_to_camera_moves_skybox_without_query_conflict() {
    let mut app = App::new();
    app.add_systems(Update, sync_inworld_skybox_to_camera);

    app.world_mut().spawn((
        Camera3d::default(),
        Camera {
            is_active: true,
            ..Default::default()
        },
        Transform::from_translation(Vec3::new(4.0, 5.0, 6.0)),
    ));

    let skybox = app
        .world_mut()
        .spawn((
            InWorldSkybox {
                path: PathBuf::from("data/models/skyboxes/test.m2"),
                phase: InWorldSkyboxPhase::Steady,
                elapsed: 0.0,
            },
            Transform::from_translation(Vec3::ZERO),
        ))
        .id();

    app.update();

    let transform = app
        .world()
        .get::<Transform>(skybox)
        .expect("skybox transform");
    assert_eq!(transform.translation, Vec3::new(4.0, 5.0, 6.0));
}

fn read_rgba16f(data: &[u8], face: u32, x: u32, y: u32) -> [f32; 3] {
    let face_bytes = (ENV_MAP_SIZE * ENV_MAP_SIZE) as usize * 8;
    let offset = face as usize * face_bytes + ((y * ENV_MAP_SIZE + x) as usize) * 8;
    std::array::from_fn(|channel| {
        let at = offset + channel * 2;
        half::f16::from_le_bytes([data[at], data[at + 1]]).to_f32()
    })
}

#[test]
fn environment_map_follows_dome_bands_for_noon_light_params_12() {
    let rows = load_light_data("data/LightData.ron", 12);
    let noon = interpolate_colors(&rows, 1440.0);
    let cubemap = build_sky_cubemap(&noon);
    let data = cubemap.data.as_ref().expect("cubemap pixels");
    // +Z face, centre column: rows run from about 44° above to 44° below the horizon.
    for y in [0, 6, 12, 14, 15, 16, 20, 31] {
        let direction = cubemap_direction(4, 16, y);
        let band = super::sky_cubemap_data::sky_band_at_elevation(direction.y.asin());
        let expected = super::sky_gradient::sky_gradient_color(&noon, band);
        let actual = read_rgba16f(data, 4, 16, y);
        let expected = [expected.red, expected.green, expected.blue];
        for channel in 0..3 {
            assert!(
                (actual[channel] - expected[channel]).abs() < 2e-3,
                "row {y} (elevation {:.2}°): {actual:?} vs dome {expected:?}",
                direction.y.asin().to_degrees()
            );
        }
    }
    // Noon sky above the rings is deep blue, not near-white.
    let zenith_side = read_rgba16f(data, 4, 16, 0);
    assert!(
        zenith_side[2] > zenith_side[0] * 3.0,
        "44° noon sky must be blue-dominant: {zenith_side:?}"
    );
}
