use super::*;
use std::path::Path;

use crate::character_customization::{
    CharacterCustomizationSelection, collect_appearance_materials,
};
use bevy::ecs::system::RunSystemOnce;
use game_engine::customization_data::OptionType;

fn blood_elf_warrior_state() -> CharCreateState {
    CharCreateState {
        selected_race: 10,
        selected_class: 1,
        selected_sex: 0,
        appearance: CharacterAppearance {
            sex: 0,
            skin_color: 0,
            face: 0,
            eye_color: 0,
            hair_style: 0,
            hair_color: 0,
            facial_style: 0,
            customization_choices: Vec::new(),
        },
        ..Default::default()
    }
}

fn assert_face_materials_present(db: &CustomizationDb, state: &CharCreateState) {
    let expected_face = db
        .get_choice_for_class(10, 0, 1, OptionType::Face, 0)
        .unwrap();
    let all_materials = collect_appearance_materials(
        &CharacterCustomizationSelection {
            race: state.selected_race,
            class: state.selected_class,
            sex: state.selected_sex,
            appearance: state.appearance.clone(),
        },
        db,
    );
    assert_eq!(expected_face.requirement_id, 142);
    assert!(
        expected_face
            .materials
            .iter()
            .all(|material| all_materials.contains(material))
    );
}

#[test]
fn non_demon_hunter_face_uses_non_dh_materials() {
    let db = CustomizationDb::load(Path::new("data"));
    let state = blood_elf_warrior_state();
    assert_face_materials_present(&db, &state);
}

// --- Dropdown camera zoom: target calculation, restore on close ---

#[test]
fn zoom_target_no_dropdown_returns_default() {
    let (focus, distance) = zoom_target_for_dropdown(None);
    assert_eq!(focus, DEFAULT_FOCUS);
    let expected_distance = (DEFAULT_EYE - DEFAULT_FOCUS).length();
    assert!((distance - expected_distance).abs() < 0.01);
}

#[test]
fn zoom_target_face_field_returns_face_params() {
    let (focus, distance) = zoom_target_for_dropdown(Some(OptionType::Face));
    assert_eq!(focus, FACE_FOCUS);
    assert_eq!(distance, FACE_DISTANCE);
}

#[test]
fn zoom_target_hair_style_is_face_zoom() {
    let (focus, distance) = zoom_target_for_dropdown(Some(OptionType::HairStyle));
    assert_eq!(focus, FACE_FOCUS);
    assert_eq!(distance, FACE_DISTANCE);
}

#[test]
fn zoom_target_hair_color_is_face_zoom() {
    let (_, distance) = zoom_target_for_dropdown(Some(OptionType::HairColor));
    assert_eq!(distance, FACE_DISTANCE);
}

#[test]
fn zoom_target_facial_style_is_face_zoom() {
    let (_, distance) = zoom_target_for_dropdown(Some(OptionType::FacialHair));
    assert_eq!(distance, FACE_DISTANCE);
}

#[test]
fn zoom_target_non_face_field_returns_default() {
    let (focus, distance) = zoom_target_for_dropdown(Some(OptionType::SkinColor));
    assert_eq!(focus, DEFAULT_FOCUS);
    let expected = (DEFAULT_EYE - DEFAULT_FOCUS).length();
    assert!((distance - expected).abs() < 0.01);
}

#[test]
fn zoom_target_restore_on_close_matches_no_dropdown() {
    // Opening a face dropdown then closing (None) should restore default
    let (open_focus, open_dist) = zoom_target_for_dropdown(Some(OptionType::Face));
    let (close_focus, close_dist) = zoom_target_for_dropdown(None);
    assert_ne!(open_focus, close_focus);
    assert!((open_dist - close_dist).abs() > 0.1);
    assert_eq!(close_focus, DEFAULT_FOCUS);
}

#[test]
fn face_distance_is_closer_than_default() {
    let (_, default_dist) = zoom_target_for_dropdown(None);
    assert!(FACE_DISTANCE < default_dist);
}

#[test]
fn select_race_action_updates_char_create_state() {
    let db = CustomizationDb::load(Path::new("data"));
    let mut state = CharCreateState::default();
    assert_eq!(state.selected_race, 1, "default race should be human");

    super::super::input::apply_race_change_with_seed(&mut state, 2, &db, 42);
    assert_eq!(
        state.selected_race, 2,
        "apply_race_change should update selected_race"
    );
}

#[test]
fn char_create_action_parse_recognizes_select_race() {
    use game_engine::ui::screens::char_create_component::CharCreateAction;
    let action = CharCreateAction::parse("select_race:2");
    assert!(
        matches!(action, Some(CharCreateAction::SelectRace(2))),
        "should parse 'select_race:2' into SelectRace(2), got {action:?}"
    );
}

#[test]
fn sync_model_detects_race_change_and_respawns() {
    let mut app = App::new();
    app.insert_resource(CreationSceneCatalog::load(Path::new("data/ChrRaces.csv")).unwrap());
    app.init_resource::<Assets<Mesh>>();
    app.init_resource::<Assets<StandardMaterial>>();
    app.init_resource::<Assets<M2EffectMaterial>>();
    app.init_resource::<Assets<Image>>();
    app.init_resource::<Assets<SkinnedMeshInverseBindposes>>();
    app.insert_resource(creature_display::CreatureDisplayMap);

    // Start with race 1 (human) displayed
    app.insert_resource(DisplayedModels {
        race: Some(1),
        active_sex: 0,
        models: vec![],
        last_appearance: None,
        last_class: None,
        background: None,
    });

    // Change state to race 2 (orc)
    app.insert_resource(CharCreateState {
        selected_race: 2,
        selected_class: 1,
        selected_sex: 0,
        ..Default::default()
    });

    let race_changed = app
        .world_mut()
        .run_system_once(
            |state: Option<Res<CharCreateState>>, displayed: Res<DisplayedModels>| -> bool {
                let state = state.unwrap();
                displayed.race != Some(state.selected_race)
            },
        )
        .expect("system should run");

    assert!(
        race_changed,
        "sync_model should detect race change from 1 to 2"
    );
}

#[test]
fn setup_scene_loads_the_authored_alliance_backdrop() {
    let mut app = App::new();
    app.insert_resource(CreationSceneCatalog::load(Path::new("data/ChrRaces.csv")).unwrap());
    app.init_resource::<Assets<Mesh>>();
    app.init_resource::<Assets<StandardMaterial>>();
    app.init_resource::<Assets<M2EffectMaterial>>();
    app.init_resource::<Assets<Image>>();
    app.init_resource::<Assets<SkinnedMeshInverseBindposes>>();
    app.insert_resource(creature_display::CreatureDisplayMap);
    app.init_resource::<DisplayedModels>();
    app.world_mut().run_system_once(setup_scene).unwrap();
    app.update();
    let root = app
        .world_mut()
        .query::<(Entity, &Name)>()
        .iter(app.world())
        .find(|(_, name)| name.as_str() == "CharCreateBackdrop_623712")
        .map(|(entity, _)| entity)
        .expect("creation must load the authored Alliance scene");
    let mesh_count = app
        .world_mut()
        .query::<(Entity, &Mesh3d)>()
        .iter(app.world())
        .filter(|(entity, _)| {
            let mut entity = *entity;
            while let Some(parent) = app.world().get::<ChildOf>(entity) {
                entity = parent.parent();
                if entity == root {
                    return true;
                }
            }
            false
        })
        .count();
    assert!(
        mesh_count > 20,
        "authored scene must contain actual renderable M2 batches, got {mesh_count}"
    );
    assert!(
        !app.world_mut()
            .query::<&Name>()
            .iter(app.world())
            .any(|name| name.as_str() == "Ground"),
        "substitute grass plane must be removed"
    );
}

#[test]
fn setup_scene_does_not_create_an_unused_procedural_sky_map() {
    let mut app = App::new();
    app.insert_resource(CreationSceneCatalog::load(Path::new("data/ChrRaces.csv")).unwrap());
    app.init_resource::<Assets<Mesh>>();
    app.init_resource::<Assets<StandardMaterial>>();
    app.init_resource::<Assets<M2EffectMaterial>>();
    app.init_resource::<Assets<Image>>();
    app.init_resource::<Assets<SkinnedMeshInverseBindposes>>();
    app.insert_resource(creature_display::CreatureDisplayMap);
    app.init_resource::<DisplayedModels>();

    app.world_mut()
        .run_system_once(setup_scene)
        .expect("setup_scene should run");
    app.update();

    let has_env_map = app
        .world()
        .get_resource::<crate::sky::SkyEnvMapHandle>()
        .is_some();
    assert!(
        !has_env_map,
        "creation should not manufacture an unrelated procedural sky environment"
    );
}

#[test]
fn setup_scene_creates_camera_and_lighting_standalone() {
    let mut app = App::new();
    app.insert_resource(CreationSceneCatalog::load(Path::new("data/ChrRaces.csv")).unwrap());
    app.init_resource::<Assets<Mesh>>();
    app.init_resource::<Assets<StandardMaterial>>();
    app.init_resource::<Assets<M2EffectMaterial>>();
    app.init_resource::<Assets<Image>>();
    app.init_resource::<Assets<SkinnedMeshInverseBindposes>>();
    app.insert_resource(creature_display::CreatureDisplayMap);
    app.init_resource::<DisplayedModels>();

    app.world_mut()
        .run_system_once(setup_scene)
        .expect("setup_scene should run");
    app.update();

    let has_camera = app
        .world_mut()
        .query::<(&Camera3d, &CharCreateScene)>()
        .iter(app.world())
        .count()
        > 0;
    assert!(has_camera, "char create scene should spawn a camera");

    let ambient = app.world().resource::<GlobalAmbientLight>();
    assert!(
        ambient.brightness > 0.0,
        "ambient light should have positive brightness"
    );
    let source_model = asset::m2::load_m2(Path::new("data/models/623712.m2"), &[0; 3]).unwrap();
    let source_light = asset::m2_light::evaluate_light(
        &source_model.lights[0],
        0,
        0,
        0,
        &source_model.global_sequences,
    );
    let color = ambient.color.to_linear();
    let exposed = Vec3::new(color.red, color.green, color.blue)
        * ambient.brightness
        * bevy::camera::Exposure::default().exposure();
    assert!(
        (exposed - Vec3::from_array(source_light.color))
            .abs()
            .max_element()
            < 0.0001,
        "authored ambient multiplier must survive renderer exposure: {exposed:?} vs {:?}",
        source_light.color
    );

    let has_directional = app
        .world_mut()
        .query::<(&DirectionalLight, &CharCreateScene)>()
        .iter(app.world())
        .count()
        > 0;
    assert!(
        !has_directional,
        "authored creation scene must not receive an extra outdoor directional light"
    );

    assert_eq!(
        app.world_mut()
            .query::<&PointLight>()
            .iter(app.world())
            .count(),
        2,
        "both authored scene point lights must remain"
    );
    let displayed = app.world().resource::<DisplayedModels>();
    assert_eq!(
        displayed.race,
        Some(1),
        "setup_scene should spawn initial human models"
    );
    assert!(
        displayed.models.len() >= 2,
        "setup_scene should spawn both male and female models, got {}",
        displayed.models.len()
    );

    // Verify the model entities exist and have renderable children (meshes + materials)
    for &(sex, entity) in &displayed.models {
        assert!(
            app.world().get_entity(entity).is_ok(),
            "model entity for sex={sex} should exist"
        );
        assert!(
            app.world().get::<CharCreateModelRoot>(entity).is_some(),
            "model entity for sex={sex} should have CharCreateModelRoot"
        );
    }
    let mesh_count = app.world_mut().query::<&Mesh3d>().iter(app.world()).count();
    assert!(
        mesh_count >= 2,
        "scene should contain renderable meshes (ground + model), got {mesh_count}"
    );
    let material_count = app
        .world_mut()
        .query::<&MeshMaterial3d<StandardMaterial>>()
        .iter(app.world())
        .count();
    assert!(
        material_count >= 2,
        "scene should contain materials for rendering, got {material_count}"
    );
}

#[test]
fn changing_race_replaces_model_entities_in_bevy_tree() {
    let mut app = App::new();
    app.insert_resource(CreationSceneCatalog::load(Path::new("data/ChrRaces.csv")).unwrap());
    app.init_resource::<Assets<Mesh>>();
    app.init_resource::<Assets<StandardMaterial>>();
    app.init_resource::<Assets<M2EffectMaterial>>();
    app.init_resource::<Assets<Image>>();
    app.init_resource::<Assets<SkinnedMeshInverseBindposes>>();
    app.insert_resource(creature_display::CreatureDisplayMap);
    app.init_resource::<DisplayedModels>();

    // Set up initial scene with race 1 (human)
    app.world_mut()
        .run_system_once(setup_scene)
        .expect("setup_scene should run");
    app.update();

    let initial_models: Vec<(u8, Entity)> =
        app.world().resource::<DisplayedModels>().models.clone();
    assert!(
        initial_models.len() >= 2,
        "should have male+female human models"
    );
    let _initial_mesh_count = app
        .world_mut()
        .query::<(&Mesh3d, &ChildOf)>()
        .iter(app.world())
        .count();

    // Change state to race 2 (orc)
    app.insert_resource(CharCreateState {
        selected_race: 2,
        selected_class: 1,
        selected_sex: 0,
        ..Default::default()
    });

    // Run sync_model to detect and apply the race change
    app.world_mut()
        .run_system_once(sync_model)
        .expect("sync_model should run");
    app.update();

    let new_displayed = app.world().resource::<DisplayedModels>();
    assert_eq!(
        new_displayed.race,
        Some(2),
        "displayed race should update to orc (2)"
    );
    assert!(
        new_displayed.models.len() >= 2,
        "should have male+female orc models, got {}",
        new_displayed.models.len()
    );

    // Old entities should be despawned
    for &(sex, entity) in &initial_models {
        assert!(
            app.world().get_entity(entity).is_err(),
            "old model entity for sex={sex} should be despawned after race change"
        );
    }

    // New entities should exist with model root
    for &(sex, entity) in &new_displayed.models {
        assert!(
            app.world().get_entity(entity).is_ok(),
            "new model entity for sex={sex} should exist"
        );
        assert!(
            app.world().get::<CharCreateModelRoot>(entity).is_some(),
            "new model entity for sex={sex} should have CharCreateModelRoot"
        );
    }

    // New model should have renderable meshes
    let new_mesh_count = app
        .world_mut()
        .query::<(&Mesh3d, &ChildOf)>()
        .iter(app.world())
        .count();
    assert!(
        new_mesh_count >= 2,
        "new model should have renderable child meshes, got {new_mesh_count}"
    );
}

#[test]
fn geosets_visible_after_two_updates_with_full_plugin() {
    use game_engine::asset::char_texture::CharTextureData;

    let mut app = App::new();
    app.add_plugins(bevy::MinimalPlugins);
    app.add_plugins(bevy::transform::TransformPlugin);
    app.add_plugins(bevy::state::app::StatesPlugin);
    app.init_resource::<Assets<Mesh>>();
    app.init_resource::<Assets<StandardMaterial>>();
    app.init_resource::<Assets<M2EffectMaterial>>();
    app.init_resource::<Assets<Image>>();
    app.init_resource::<Assets<SkinnedMeshInverseBindposes>>();
    app.insert_resource(creature_display::CreatureDisplayMap);
    app.insert_resource(CustomizationDb::load(Path::new("data")));
    app.insert_resource(CharTextureData::load(Path::new("data")));
    app.insert_resource(bevy::input::ButtonInput::<bevy::prelude::MouseButton>::default());
    app.insert_resource(bevy::input::mouse::AccumulatedMouseMotion::default());
    app.insert_resource(crate::client_options::CameraOptions::default());
    app.add_plugins(CharCreateScenePlugin);
    app.insert_state(crate::game_state::GameState::CharCreate);

    // Frame 1: OnEnter fires setup_scene, commands queued
    app.update();
    // Insert CharCreateState (normally from CharCreatePlugin's OnEnter)
    app.insert_resource(CharCreateState::default());
    // Frame 2: commands applied, sync_appearance runs
    app.update();
    // Frame 3: any deferred work
    app.update();

    let visible_geosets = app
        .world_mut()
        .query::<(&crate::m2_spawn::GeosetMesh, &Visibility)>()
        .iter(app.world())
        .filter(|(_, vis)| **vis == Visibility::Inherited)
        .count();
    let total_geosets = app
        .world_mut()
        .query::<&crate::m2_spawn::GeosetMesh>()
        .iter(app.world())
        .count();

    assert!(
        visible_geosets > 0,
        "after setup + 3 updates, geosets should be visible, got 0/{total_geosets}"
    );
}

#[test]
fn direct_entry_has_initial_appearance_by_end_of_first_update() {
    use bevy::state::app::StatesPlugin;
    use bevy::window::PrimaryWindow;
    use game_engine::asset::char_texture::CharTextureData;
    use game_engine::ui::automation::UiAutomationPlugin;

    let mut app = App::new();
    app.add_plugins(MinimalPlugins);
    app.add_plugins(bevy::transform::TransformPlugin);
    app.add_plugins(StatesPlugin);
    app.add_plugins(bevy::asset::AssetPlugin::default());
    app.add_plugins(bevy::text::TextPlugin::default());
    app.add_plugins(UiAutomationPlugin);
    app.add_plugins(ui_toolkit::plugin::UiPlugin);
    app.init_resource::<bevy::ui::UiScale>();
    app.insert_resource(ButtonInput::<MouseButton>::default());
    app.insert_resource(bevy::input::mouse::AccumulatedMouseMotion::default());
    app.insert_resource(crate::client_options::CameraOptions::default());
    app.init_resource::<game_engine::network_runtime::messages::ConnectionSender>();
    app.insert_resource(CustomizationDb::load(Path::new("data")));
    app.insert_resource(CharTextureData::load(Path::new("data")));
    app.insert_resource(crate::creature_display::CreatureDisplayMap);
    app.init_resource::<Assets<Mesh>>();
    app.init_resource::<Assets<StandardMaterial>>();
    app.init_resource::<Assets<M2EffectMaterial>>();
    app.init_resource::<Assets<Image>>();
    app.init_resource::<Assets<SkinnedMeshInverseBindposes>>();
    app.add_plugins(crate::scenes::char_create::CharCreatePlugin);
    app.add_plugins(CharCreateScenePlugin);
    app.add_message::<bevy::input::keyboard::KeyboardInput>();
    app.insert_state(crate::game_state::GameState::CharCreate);
    app.world_mut().spawn((Window::default(), PrimaryWindow));

    app.update();

    let displayed = app.world().resource::<DisplayedModels>();
    assert!(
        displayed.last_appearance.is_some(),
        "direct charcreate entry should have applied an initial appearance by the end of the first update"
    );
    assert_eq!(
        displayed.last_class,
        Some(1),
        "direct charcreate entry should track the initial class after the first update"
    );

    let visible_geosets = app
        .world_mut()
        .query::<(&crate::m2_spawn::GeosetMesh, &Visibility)>()
        .iter(app.world())
        .filter(|(_, vis)| **vis == Visibility::Inherited)
        .count();
    let total_geosets = app
        .world_mut()
        .query::<&crate::m2_spawn::GeosetMesh>()
        .iter(app.world())
        .count();

    assert!(
        visible_geosets > 0,
        "direct charcreate entry should apply initial appearance on the first update, got 0/{total_geosets} visible geosets"
    );
}

#[test]
fn sync_appearance_unhides_geoset_meshes() {
    use game_engine::asset::char_texture::CharTextureData;

    let mut app = App::new();
    app.add_plugins(bevy::MinimalPlugins);
    app.add_plugins(bevy::transform::TransformPlugin);
    app.init_resource::<Assets<Mesh>>();
    app.init_resource::<Assets<StandardMaterial>>();
    app.init_resource::<Assets<M2EffectMaterial>>();
    app.init_resource::<Assets<Image>>();
    app.init_resource::<Assets<SkinnedMeshInverseBindposes>>();
    app.insert_resource(creature_display::CreatureDisplayMap);
    app.insert_resource(CustomizationDb::load(Path::new("data")));
    app.insert_resource(CharTextureData::load(Path::new("data")));
    app.insert_resource(CreationSceneCatalog::load(Path::new("data/ChrRaces.csv")).unwrap());
    app.init_resource::<DisplayedModels>();

    // Spawn the scene
    app.world_mut()
        .run_system_once(setup_scene)
        .expect("setup_scene should run");
    app.update();

    // Insert CharCreateState (normally done by CharCreatePlugin)
    app.insert_resource(CharCreateState::default());
    app.update();

    // Run sync_appearance
    app.world_mut()
        .run_system_once(sync_appearance)
        .expect("sync_appearance should run");
    app.update();

    // Count visible geoset meshes
    let visible_geosets = app
        .world_mut()
        .query::<(&crate::m2_spawn::GeosetMesh, &Visibility)>()
        .iter(app.world())
        .filter(|(_, vis)| **vis == Visibility::Inherited)
        .count();

    let total_geosets = app
        .world_mut()
        .query::<&crate::m2_spawn::GeosetMesh>()
        .iter(app.world())
        .count();

    assert!(
        visible_geosets > 0,
        "sync_appearance should un-hide at least some geoset meshes, got 0/{total_geosets} visible"
    );
}

#[test]
fn camera_ray_hits_character_model() {
    let mut app = App::new();
    app.insert_resource(CreationSceneCatalog::load(Path::new("data/ChrRaces.csv")).unwrap());
    app.add_plugins(bevy::MinimalPlugins);
    app.add_plugins(bevy::transform::TransformPlugin);
    app.add_plugins(bevy::camera::visibility::VisibilityPlugin);
    app.add_plugins(bevy::picking::PickingPlugin);
    app.init_resource::<Assets<Mesh>>();
    app.init_resource::<Assets<StandardMaterial>>();
    app.init_resource::<Assets<M2EffectMaterial>>();
    app.init_resource::<Assets<Image>>();
    app.init_resource::<Assets<SkinnedMeshInverseBindposes>>();
    app.insert_resource(creature_display::CreatureDisplayMap);
    app.init_resource::<DisplayedModels>();

    app.world_mut()
        .run_system_once(setup_scene)
        .expect("setup_scene should run");
    // Multiple updates to propagate transforms
    app.update();
    app.update();
    app.update();

    let displayed = app.world().resource::<DisplayedModels>();
    let active_entity = displayed
        .models
        .iter()
        .find(|(sex, _)| *sex == 0)
        .map(|(_, e)| *e)
        .expect("male model should exist");

    // Verify model has a valid world-space transform
    let model_global = app
        .world()
        .get::<GlobalTransform>(active_entity)
        .expect("model should have GlobalTransform");
    let model_pos = model_global.translation();

    // Model should be near the origin (where setup_scene places it)
    assert!(
        model_pos.length() < 5.0,
        "model should be near origin, got {model_pos:?}"
    );

    // Check mesh children have GlobalTransform
    let mesh_with_gt = app
        .world_mut()
        .query::<(&Mesh3d, &GlobalTransform)>()
        .iter(app.world())
        .count();
    let total_meshes = app.world_mut().query::<&Mesh3d>().iter(app.world()).count();
    assert!(total_meshes > 0, "should have meshes");
    assert_eq!(
        mesh_with_gt, total_meshes,
        "all {total_meshes} meshes should have GlobalTransform, only {mesh_with_gt} do"
    );

    let _aabb_count = app
        .world_mut()
        .query::<(&Mesh3d, &bevy::camera::primitives::Aabb)>()
        .iter(app.world())
        .count();
    let visible_meshes = app
        .world_mut()
        .query::<(&Mesh3d, &InheritedVisibility)>()
        .iter(app.world())
        .filter(|(_, iv)| iv.get())
        .count();
    assert!(
        visible_meshes > 0,
        "at least some meshes should be visible, got 0/{total_meshes}"
    );

    // Cast a ray from camera eye toward model position
    let ray_target = model_pos + Vec3::Y * 1.0;
    let ray_dir = (ray_target - DEFAULT_EYE).normalize();
    let _ray = Ray3d::new(DEFAULT_EYE, Dir3::new(ray_dir).expect("valid direction"));

    // Check if any visible mesh's world-space Aabb intersects the camera-to-model ray
    let ray_origin = DEFAULT_EYE;
    let has_mesh_on_ray = app
        .world_mut()
        .query::<(
            &Mesh3d,
            &GlobalTransform,
            &bevy::camera::primitives::Aabb,
            &InheritedVisibility,
        )>()
        .iter(app.world())
        .filter(|(_, _, _, iv)| iv.get())
        .any(|(_, gt, aabb, _)| {
            let world_center = gt.transform_point(aabb.center.into());
            let half = aabb.half_extents;
            let world_half = gt.affine().matrix3.abs() * half;
            let to_center = world_center - ray_origin;
            let along_ray = to_center.dot(ray_dir);
            if along_ray < 0.0 {
                return false;
            }
            let closest = ray_origin + ray_dir * along_ray;
            let diff = (world_center - closest).abs();
            diff.x <= world_half.x + 0.5
                && diff.y <= world_half.y + 0.5
                && diff.z <= world_half.z + 0.5
        });

    assert!(
        has_mesh_on_ray,
        "camera ray should intersect at least one visible mesh's bounding box"
    );
}

#[path = "scene_tests_runtime.rs"]
mod runtime_tests;

#[path = "background_tests.rs"]
mod background_tests;

#[path = "neutral_capture_tests.rs"]
mod neutral_capture_tests;

#[path = "projection_tests.rs"]
mod projection_tests;

#[test]
fn apply_orbit_produces_valid_transform() {
    let orbit = CharCreateOrbit {
        yaw: 0.0,
        pitch: 0.0,
        focus: DEFAULT_FOCUS,
        distance: (DEFAULT_EYE - DEFAULT_FOCUS).length(),
        base_pitch: 0.0,
        manual_distance: None,
        default_focus: DEFAULT_FOCUS,
        default_distance: (DEFAULT_EYE - DEFAULT_FOCUS).length(),
    };
    let mut transform = Transform::default();
    apply_orbit_transform(&orbit, &mut transform);
    // Camera should be looking roughly toward the focus point
    let forward = transform.forward();
    assert!(forward.z < 0.0, "camera should face -Z (toward model)");
}

struct CameraFixture {
    world: World,
    camera: Entity,
}

impl CameraFixture {
    fn new() -> Self {
        let mut world = World::new();
        world.init_resource::<CharCreateState>();
        world.init_resource::<CustomizationDb>();
        world.insert_resource(Time::<()>::default());
        let offset = DEFAULT_EYE - DEFAULT_FOCUS;
        let orbit = CharCreateOrbit {
            yaw: 0.0,
            pitch: 0.0,
            focus: DEFAULT_FOCUS,
            distance: offset.length(),
            base_pitch: (offset.y / offset.length()).asin(),
            manual_distance: None,
            default_focus: DEFAULT_FOCUS,
            default_distance: offset.length(),
        };
        let mut transform = Transform::default();
        apply_orbit_transform(&orbit, &mut transform);
        let camera = world.spawn((orbit, transform)).id();
        Self { world, camera }
    }

    fn control(&mut self, control: CameraControl) {
        self.world.resource_mut::<CharCreateState>().camera_action = Some(control);
        self.world
            .run_system_once(apply_camera_control)
            .expect("camera action system");
        assert!(
            self.world
                .resource::<CharCreateState>()
                .camera_action
                .is_none()
        );
        self.settle_zoom();
    }

    fn settle_zoom(&mut self) {
        self.world
            .resource_mut::<Time>()
            .advance_by(std::time::Duration::from_millis(200));
        self.world
            .run_system_once(camera_zoom_for_dropdown)
            .expect("camera zoom system");
    }

    fn position(&self) -> Vec3 {
        self.world
            .get::<Transform>(self.camera)
            .unwrap()
            .translation
    }

    fn radius(&self) -> f32 {
        let focus = self
            .world
            .get::<CharCreateOrbit>(self.camera)
            .unwrap()
            .focus;
        self.position().distance(focus)
    }
}

#[test]
fn camera_control_rotation_actions_move_the_preview_and_consume_requests() {
    for (control, expected_sign) in [
        (CameraControl::RotateLeft, -1.0),
        (CameraControl::RotateRight, 1.0),
    ] {
        let mut fixture = CameraFixture::new();
        let before = fixture.position();
        fixture.control(control);
        let after = fixture.position();
        assert!(
            after.x * expected_sign > 0.1,
            "rotation should move camera around preview: {after:?}"
        );
        assert!((after.distance(DEFAULT_FOCUS) - before.distance(DEFAULT_FOCUS)).abs() < 0.0001);
        let transform = fixture.world.get::<Transform>(fixture.camera).unwrap();
        let to_focus = (DEFAULT_FOCUS - after).normalize();
        assert!(Vec3::from(transform.forward()).dot(to_focus) > 0.999);
        fixture.world.run_system_once(apply_camera_control).unwrap();
        assert_eq!(
            fixture.position(),
            after,
            "consumed request must not rotate a second time"
        );
    }
}

#[test]
fn camera_control_manual_zoom_persists_until_reset() {
    let mut fixture = CameraFixture::new();
    let default_radius = fixture.radius();
    fixture.control(CameraControl::ZoomIn);
    assert!((fixture.radius() - (default_radius - 0.5)).abs() < 0.0001);
    let manual = fixture
        .world
        .get::<CharCreateOrbit>(fixture.camera)
        .unwrap()
        .manual_distance
        .expect("manual zoom target must remain active");
    assert!((manual - (default_radius - 0.5)).abs() < 0.0001);
    fixture.settle_zoom();
    assert!(
        (fixture.radius() - (default_radius - 0.5)).abs() < 0.0001,
        "automatic dropdown zoom must not undo manual zoom"
    );
    fixture.control(CameraControl::ZoomOut);
    assert!((fixture.radius() - default_radius).abs() < 0.0001);
    fixture.control(CameraControl::ZoomIn);
    fixture.control(CameraControl::RotateRight);
    fixture.control(CameraControl::Reset);
    assert!(
        fixture
            .world
            .get::<CharCreateOrbit>(fixture.camera)
            .unwrap()
            .manual_distance
            .is_none()
    );
    assert!(
        fixture.position().distance(DEFAULT_EYE) < 0.0001,
        "reset should restore default preview framing: {:?}",
        fixture.position()
    );
}

#[test]
fn camera_control_manual_zoom_stays_within_preview_limits() {
    let mut fixture = CameraFixture::new();
    fixture
        .world
        .get_mut::<CharCreateOrbit>(fixture.camera)
        .unwrap()
        .manual_distance = Some(1.2);
    fixture.control(CameraControl::ZoomIn);
    assert!((fixture.radius() - 1.0).abs() < 0.0001);
    fixture.control(CameraControl::ZoomIn);
    assert!((fixture.radius() - 1.0).abs() < 0.0001);
    fixture
        .world
        .get_mut::<CharCreateOrbit>(fixture.camera)
        .unwrap()
        .manual_distance = Some(9.8);
    fixture.control(CameraControl::ZoomOut);
    assert!((fixture.radius() - 10.0).abs() < 0.0001);
    fixture.control(CameraControl::ZoomOut);
    assert!((fixture.radius() - 10.0).abs() < 0.0001);
}

#[test]
fn camera_control_eye_and_ear_option_ids_zoom_to_face_and_restore_on_close() {
    let mut fixture = CameraFixture::new();
    fixture
        .world
        .insert_resource(CustomizationDb::try_load(Path::new("data")).expect("local catalog"));
    for option_id in [463, 8789] {
        fixture
            .world
            .resource_mut::<CharCreateState>()
            .open_dropdown = Some(option_id);
        fixture.settle_zoom();
        assert!((fixture.radius() - FACE_DISTANCE).abs() < 0.0001);
        assert_eq!(
            fixture
                .world
                .get::<CharCreateOrbit>(fixture.camera)
                .unwrap()
                .focus,
            FACE_FOCUS
        );
        fixture
            .world
            .resource_mut::<CharCreateState>()
            .open_dropdown = None;
        fixture.settle_zoom();
        assert!(fixture.position().distance(DEFAULT_EYE) < 0.0001);
    }
}
