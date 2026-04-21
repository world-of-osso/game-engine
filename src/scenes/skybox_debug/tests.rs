use super::{
    ResolvedDebugSkybox, SkyboxDebugOverride, SkyboxDebugScene, SkyboxDebugSetup,
    SkyboxDebugSkybox, SkyboxDebugViewMode, camera_scene_node, debug_scene_camera_bundle,
    resolve_debug_skybox, skybox_debug_composition, spawn_debug_scene_environment,
    spawn_skybox_debug_reference_objects, sync_skybox_to_camera, sync_skyboxdebug_camera_fov,
};
use crate::client_options::CameraOptions;
use crate::orbit_camera::OrbitCamera;
use bevy::ecs::system::RunSystemOnce;
use bevy::prelude::*;
use game_engine::scene_tree::NodeProps;
use std::path::PathBuf;

#[test]
fn debug_override_resolves_light_skybox_id() {
    let resolved = resolve_debug_skybox(None, Some(SkyboxDebugOverride::LightSkyboxId(653)))
        .expect("resolved light skybox override");
    assert!(
        resolved
            .path
            .ends_with("data/models/skyboxes/11xp_cloudsky01.m2"),
        "unexpected resolved path: {}",
        resolved.path.display()
    );
    assert_eq!(resolved.source, "forced LightSkyboxID=653");
    assert_eq!(
        resolved.light_skybox_flags,
        Some(
            crate::light_lookup::LightSkyboxFlags::FULL_DAY_SKYBOX
                | crate::light_lookup::LightSkyboxFlags::COMBINE_PROCEDURAL_AND_SKYBOX
                | crate::light_lookup::LightSkyboxFlags::PROCEDURAL_FOG_COLOR_BLEND
                | crate::light_lookup::LightSkyboxFlags::FORCE_SUNSHAFTS
        )
    );
}

#[test]
fn debug_override_resolves_skybox_fdid() {
    let resolved =
        resolve_debug_skybox(None, Some(SkyboxDebugOverride::SkyboxFileDataId(5_412_968)))
            .expect("resolved skybox fdid override");
    assert!(
        resolved
            .path
            .ends_with("data/models/skyboxes/11xp_cloudsky01.m2"),
        "unexpected resolved path: {}",
        resolved.path.display()
    );
    assert_eq!(resolved.source, "forced SkyboxFileDataID=5412968");
}

#[test]
fn default_debug_scene_uses_shared_campsite_fallback_when_first_scene_has_no_local_authored_skybox()
{
    let scene = crate::scenes::char_select::warband::WarbandScenes::load()
        .scenes
        .into_iter()
        .find(|scene| scene.id == 1)
        .expect("known scene");
    let resolved = resolve_debug_skybox(Some(&scene), None).expect("resolved default skybox");

    assert!(
        resolved
            .path
            .ends_with("data/models/skyboxes/costalislandskybox.m2"),
        "unexpected resolved path: {}",
        resolved.path.display()
    );
    assert_eq!(resolved.source, "warband scene 1 (Adventurer's Rest)");
}

#[test]
fn debug_skybox_sync_uses_orbit_focus() {
    let mut app = App::new();
    app.add_systems(Update, sync_skybox_to_camera);

    app.world_mut().spawn((
        SkyboxDebugScene,
        Camera3d::default(),
        OrbitCamera::new(Vec3::new(3.0, 4.0, 5.0), 7.5),
        Transform::from_translation(Vec3::new(30.0, 40.0, 50.0)),
    ));
    let skybox = app
        .world_mut()
        .spawn((
            SkyboxDebugScene,
            SkyboxDebugSkybox,
            Transform::from_translation(Vec3::ZERO),
        ))
        .id();

    app.update();

    let transform = app
        .world()
        .get::<Transform>(skybox)
        .expect("skybox transform");
    assert_eq!(transform.translation, Vec3::new(3.0, 4.0, 5.0));
}

#[test]
fn default_mode_spawns_procedural_baseline_sky_and_fog() {
    let mut app = App::new();
    app.init_resource::<Assets<Mesh>>();
    app.init_resource::<Assets<StandardMaterial>>();
    app.init_resource::<Assets<crate::sky_material::SkyMaterial>>();
    app.init_resource::<Assets<Image>>();

    let _ = app.world_mut().run_system_once(
        |mut commands: Commands,
         mut meshes: ResMut<Assets<Mesh>>,
         mut sky_materials: ResMut<Assets<crate::sky_material::SkyMaterial>>,
         mut images: ResMut<Assets<Image>>| {
            let cloud_maps = crate::sky::cloud_texture::create_procedural_cloud_maps(&mut images);
            let setup = SkyboxDebugSetup {
                scene: None,
                focus: Vec3::new(0.0, 1.0, 0.0),
                eye: Vec3::new(0.0, 1.0, 7.5),
            };
            spawn_debug_scene_environment(
                &mut commands,
                &mut meshes,
                &mut sky_materials,
                &mut images,
                cloud_maps.active_handle(),
                &setup,
                CameraOptions::default().fov_degrees,
                skybox_debug_composition(SkyboxDebugViewMode::Default, None),
            );
        },
    );

    let dome_count = {
        let world = app.world_mut();
        let mut query =
            world.query_filtered::<Entity, (With<crate::sky::SkyDome>, With<SkyboxDebugScene>)>();
        query.iter(world).count()
    };
    let fog_count = {
        let world = app.world_mut();
        let mut query = world
            .query_filtered::<Entity, (With<Camera3d>, With<DistanceFog>, With<SkyboxDebugScene>)>(
            );
        query.iter(world).count()
    };

    assert_eq!(dome_count, 1);
    assert_eq!(fog_count, 1);
}

#[test]
fn verification_mode_skips_debug_reference_objects() {
    let mut app = App::new();
    app.init_resource::<Assets<Mesh>>();
    app.init_resource::<Assets<StandardMaterial>>();
    app.init_resource::<Assets<Image>>();

    let _ = app.world_mut().run_system_once(
        |mut commands: Commands,
         mut meshes: ResMut<Assets<Mesh>>,
         mut materials: ResMut<Assets<StandardMaterial>>,
         mut images: ResMut<Assets<Image>>| {
            spawn_skybox_debug_reference_objects(
                &mut commands,
                &mut meshes,
                &mut materials,
                &mut images,
                SkyboxDebugViewMode::AuthoredOnlyVerification,
            );
        },
    );
    let ground_plane_count = {
        let world = app.world_mut();
        let mut query = world.query::<&Name>();
        query
            .iter(world)
            .filter(|name| name.as_str() == "SkyboxDebugGroundPlane")
            .count()
    };

    assert_eq!(ground_plane_count, 0);
}

#[test]
fn default_mode_spawns_grass_ground_plane() {
    let mut app = App::new();
    app.init_resource::<Assets<Mesh>>();
    app.init_resource::<Assets<StandardMaterial>>();
    app.init_resource::<Assets<Image>>();

    let _ = app.world_mut().run_system_once(
        |mut commands: Commands,
         mut meshes: ResMut<Assets<Mesh>>,
         mut materials: ResMut<Assets<StandardMaterial>>,
         mut images: ResMut<Assets<Image>>| {
            spawn_skybox_debug_reference_objects(
                &mut commands,
                &mut meshes,
                &mut materials,
                &mut images,
                SkyboxDebugViewMode::Default,
            );
        },
    );
    let ground_plane_count = {
        let world = app.world_mut();
        let mut query = world.query::<(&Name, Entity)>();
        query
            .iter(world)
            .filter(|(name, _)| name.as_str() == "SkyboxDebugGroundPlane")
            .count()
    };

    assert_eq!(ground_plane_count, 1);
}

#[test]
fn skyboxdebug_camera_uses_requested_fov_in_projection_and_scene_tree() {
    let setup = SkyboxDebugSetup {
        scene: None,
        focus: Vec3::new(0.0, 1.0, 0.0),
        eye: Vec3::new(0.0, 1.0, 7.5),
    };
    let fov_degrees = 117.0;
    let mut app = App::new();
    app.world_mut().spawn(debug_scene_camera_bundle(
        &setup,
        fov_degrees,
        skybox_debug_composition(SkyboxDebugViewMode::Default, None),
    ));

    let world = app.world_mut();
    let mut camera_query = world.query::<&Projection>();
    let projection = camera_query.single(world).expect("camera projection");
    let Projection::Perspective(perspective) = projection else {
        panic!("expected perspective projection");
    };
    assert!((perspective.fov.to_degrees() - fov_degrees).abs() < 0.001);

    let node = camera_scene_node(fov_degrees);
    match node.props {
        NodeProps::Camera { fov } => {
            assert!((fov - fov_degrees).abs() < 0.001);
        }
        props => panic!("expected camera node props, got {props:?}"),
    }
}

#[test]
fn skyboxdebug_camera_sync_updates_projection_from_camera_options() {
    let mut app = App::new();
    app.insert_resource(CameraOptions {
        fov_degrees: 103.0,
        ..default()
    });
    app.add_systems(Update, sync_skyboxdebug_camera_fov);
    app.world_mut().spawn((
        SkyboxDebugScene,
        Camera3d::default(),
        OrbitCamera::new(Vec3::ZERO, 7.5),
        Projection::Perspective(PerspectiveProjection {
            fov: 90.0_f32.to_radians(),
            ..default()
        }),
    ));

    app.update();

    let world = app.world_mut();
    let mut camera_query = world.query::<&Projection>();
    let projection = camera_query.single(world).expect("camera projection");
    let Projection::Perspective(perspective) = projection else {
        panic!("expected perspective projection");
    };
    assert!((perspective.fov.to_degrees() - 103.0).abs() < 0.001);
}

#[test]
fn verification_mode_spawns_black_background_without_procedural_sky() {
    let mut app = App::new();
    app.init_resource::<Assets<Mesh>>();
    app.init_resource::<Assets<StandardMaterial>>();
    app.init_resource::<Assets<crate::sky_material::SkyMaterial>>();
    app.init_resource::<Assets<Image>>();

    let _ = app.world_mut().run_system_once(
        |mut commands: Commands,
         mut meshes: ResMut<Assets<Mesh>>,
         mut sky_materials: ResMut<Assets<crate::sky_material::SkyMaterial>>,
         mut images: ResMut<Assets<Image>>| {
            let cloud_maps = crate::sky::cloud_texture::create_procedural_cloud_maps(&mut images);
            let setup = SkyboxDebugSetup {
                scene: None,
                focus: Vec3::new(0.0, 1.0, 0.0),
                eye: Vec3::new(0.0, 1.0, 7.5),
            };
            spawn_debug_scene_environment(
                &mut commands,
                &mut meshes,
                &mut sky_materials,
                &mut images,
                cloud_maps.active_handle(),
                &setup,
                CameraOptions::default().fov_degrees,
                skybox_debug_composition(SkyboxDebugViewMode::AuthoredOnlyVerification, None),
            );
        },
    );

    let dome_count = {
        let world = app.world_mut();
        let mut query =
            world.query_filtered::<Entity, (With<crate::sky::SkyDome>, With<SkyboxDebugScene>)>();
        query.iter(world).count()
    };
    let fog_count = {
        let world = app.world_mut();
        let mut query = world
            .query_filtered::<Entity, (With<Camera3d>, With<DistanceFog>, With<SkyboxDebugScene>)>(
            );
        query.iter(world).count()
    };
    let clear_color = app.world().resource::<ClearColor>().0;

    assert_eq!(dome_count, 0);
    assert_eq!(fog_count, 0);
    assert_eq!(clear_color, Color::BLACK);
}

#[test]
fn light_params_hide_flags_suppress_procedural_celestial_baseline() {
    let resolved = ResolvedDebugSkybox {
        path: PathBuf::from("data/models/skyboxes/11xp_cloudsky01.m2"),
        source: "test".into(),
        light_params_id: Some(42),
        light_params_flags: Some(
            crate::light_lookup::LightParamsFlags::HIDE_SUN
                | crate::light_lookup::LightParamsFlags::HIDE_MOON
                | crate::light_lookup::LightParamsFlags::HIDE_STARS,
        ),
        light_skybox_id: Some(653),
        light_skybox_flags: Some(
            crate::light_lookup::LightSkyboxFlags::COMBINE_PROCEDURAL_AND_SKYBOX
                | crate::light_lookup::LightSkyboxFlags::PROCEDURAL_FOG_COLOR_BLEND,
        ),
    };

    let composition = skybox_debug_composition(SkyboxDebugViewMode::Default, Some(&resolved));

    assert!(!composition.shows_procedural_visible_baseline);
    assert!(composition.shows_procedural_fog);
}

#[test]
fn light_params_dont_inherit_skybox_suppresses_procedural_baseline() {
    let resolved = ResolvedDebugSkybox {
        path: PathBuf::from("data/models/skyboxes/11xp_cloudsky01.m2"),
        source: "test".into(),
        light_params_id: Some(42),
        light_params_flags: Some(crate::light_lookup::LightParamsFlags::DONT_INHERIT_SKYBOX),
        light_skybox_id: Some(653),
        light_skybox_flags: Some(
            crate::light_lookup::LightSkyboxFlags::COMBINE_PROCEDURAL_AND_SKYBOX,
        ),
    };

    let composition = skybox_debug_composition(SkyboxDebugViewMode::Default, Some(&resolved));

    assert!(!composition.shows_procedural_visible_baseline);
}

#[test]
fn light_params_height_fog_above_plane_enables_fog_without_skybox_blend() {
    let resolved = ResolvedDebugSkybox {
        path: PathBuf::from("data/models/skyboxes/11xp_cloudsky01.m2"),
        source: "test".into(),
        light_params_id: Some(42),
        light_params_flags: Some(crate::light_lookup::LightParamsFlags::HEIGHT_FOG_ABOVE_PLANE),
        light_skybox_id: Some(653),
        light_skybox_flags: Some(crate::light_lookup::LightSkyboxFlags::empty()),
    };

    let composition = skybox_debug_composition(SkyboxDebugViewMode::Default, Some(&resolved));

    assert!(composition.shows_procedural_fog);
}
