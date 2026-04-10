use super::camera::{
    CHAR_SELECT_CAMERA_GROUND_CLEARANCE, camera_params, char_select_orbit_camera,
    clamp_char_select_eye, orbit_eye, orbit_from_eye_focus, orbit_input_debug_state,
    should_log_orbit_input,
};
use super::*;
use crate::networking_auth::CharacterList;
use bevy::app::App;
use bevy::ecs::message::Messages;
use bevy::ecs::system::RunSystemOnce;
use bevy::input::keyboard::KeyboardInput;
use bevy::input::mouse::{AccumulatedMouseMotion, AccumulatedMouseScroll};
use bevy::state::app::StatesPlugin;
use bevy::window::PrimaryWindow;
use game_engine::ui::automation::UiAutomationPlugin;
use game_engine::ui::event::EventBus;
use game_engine::ui::plugin::UiState;
use game_engine::ui::registry::FrameRegistry;
use shared::components::{CharacterAppearance, EquipmentAppearance};
use shared::protocol::CharacterListEntry;

fn character(character_id: u64, race: u8, sex: u8, name: &str) -> CharacterListEntry {
    CharacterListEntry {
        character_id,
        name: name.to_string(),
        level: 1,
        race,
        class: 1,
        appearance: CharacterAppearance {
            sex,
            ..Default::default()
        },
        equipment_appearance: EquipmentAppearance::default(),
    }
}

#[test]
fn selected_scene_character_id_uses_selected_index() {
    let char_list = CharacterList(vec![
        character(10, 1, 0, "First"),
        character(20, 2, 0, "Second"),
    ]);

    assert_eq!(selected_scene_character_id(&char_list, Some(1)), Some(20));
}

#[test]
fn selected_scene_character_id_falls_back_to_first_character() {
    let char_list = CharacterList(vec![
        character(10, 1, 0, "First"),
        character(20, 2, 0, "Second"),
    ]);

    assert_eq!(selected_scene_character_id(&char_list, None), Some(10));
    assert_eq!(selected_scene_character_id(&char_list, Some(99)), Some(10));
}

#[test]
fn selected_scene_character_identity_uses_selected_character_name_and_id() {
    let char_list = CharacterList(vec![
        character(10, 1, 0, "Theron"),
        character(20, 2, 1, "Elara"),
    ]);

    assert_eq!(
        selected_scene_character_identity(&char_list, Some(1)),
        (Some("Elara".to_string()), Some(20))
    );
}

#[test]
fn selected_scene_character_identity_falls_back_to_first_character() {
    let char_list = CharacterList(vec![
        character(10, 1, 0, "Theron"),
        character(20, 2, 1, "Elara"),
    ]);

    assert_eq!(
        selected_scene_character_identity(&char_list, Some(99)),
        (Some("Theron".to_string()), Some(10))
    );
}

#[test]
fn replace_scene_tree_character_node_updates_selected_character_identity() {
    let old_entity = Entity::from_bits(10);
    let new_entity = Entity::from_bits(20);
    let mut scene_tree = scene_tree::build_scene_tree(vec![
        scene_tree::background_scene_node(old_entity, "ground", 0, vec![]),
        scene_tree::character_scene_node(
            old_entity,
            "humanmale_hd.m2".to_string(),
            "Human".to_string(),
            "Male".to_string(),
            Some("Theron".to_string()),
            Some(6),
        ),
    ]);

    replace_scene_tree_character_node(
        &mut scene_tree,
        scene_tree::character_scene_node(
            new_entity,
            "humanfemale_hd.m2".to_string(),
            "Human".to_string(),
            "Female".to_string(),
            Some("Elara".to_string()),
            Some(7),
        ),
    );

    let character_node = scene_tree
        .root
        .children
        .iter()
        .find(|child| {
            matches!(
                child.props,
                game_engine::scene_tree::NodeProps::Character { .. }
            )
        })
        .expect("character node should exist");
    let game_engine::scene_tree::NodeProps::Character {
        model,
        gender,
        name,
        character_id,
        ..
    } = &character_node.props
    else {
        panic!("expected character node");
    };

    assert_eq!(character_node.entity, Some(new_entity));
    assert_eq!(model, "humanfemale_hd.m2");
    assert_eq!(gender, "Female");
    assert_eq!(name.as_deref(), Some("Elara"));
    assert_eq!(*character_id, Some(7));
}

#[test]
fn authored_char_select_skybox_path_stays_disabled_until_proven() {
    assert!(!should_spawn_authored_char_select_skybox());
}

#[test]
fn char_select_camera_gets_a_sky_dome_child() {
    let mut app = App::new();
    app.init_resource::<Assets<Mesh>>();
    app.init_resource::<Assets<crate::sky_material::SkyMaterial>>();
    app.init_resource::<Assets<Image>>();

    let cloud_maps = {
        let mut images = app.world_mut().resource_mut::<Assets<Image>>();
        crate::sky::cloud_texture::create_procedural_cloud_maps(&mut images)
    };
    app.insert_resource(cloud_maps);

    let camera = app.world_mut().spawn_empty().id();
    let dome = app
        .world_mut()
        .run_system_once(
            move |mut commands: Commands,
                  mut meshes: ResMut<Assets<Mesh>>,
                  mut sky_materials: ResMut<Assets<crate::sky_material::SkyMaterial>>,
                  mut images: ResMut<Assets<Image>>,
                  cloud_maps: Res<crate::sky::cloud_texture::ProceduralCloudMaps>| {
                spawn_char_select_sky_dome(
                    &mut commands,
                    &mut meshes,
                    &mut sky_materials,
                    &mut images,
                    cloud_maps.active_handle(),
                    camera,
                )
            },
        )
        .expect("spawn sky dome");
    app.update();

    let child_of = app.world().get::<ChildOf>(dome).expect("sky dome parent");
    assert_eq!(child_of.parent(), camera);
    assert!(app.world().get::<crate::sky::SkyDome>(dome).is_some());
}

#[test]
fn despawning_model_wrapper_removes_model_root_child() {
    let mut app = App::new();
    let wrapper = app
        .world_mut()
        .spawn((CharSelectModelWrapper, CharSelectScene))
        .id();
    let root = app
        .world_mut()
        .spawn((CharSelectModelRoot, CharSelectModelCharacter(10)))
        .id();
    app.world_mut().entity_mut(wrapper).add_child(root);

    app.world_mut().commands().entity(wrapper).despawn();
    app.update();

    assert!(app.world().get_entity(wrapper).is_err());
    assert!(app.world().get_entity(root).is_err());
}

#[test]
fn appearance_sync_waits_until_model_has_geosets_and_materials() {
    let mut app = App::new();
    app.init_resource::<Assets<Mesh>>();
    app.init_resource::<Assets<StandardMaterial>>();
    let root = app.world_mut().spawn_empty().id();
    let child = app.world_mut().spawn_empty().id();
    app.world_mut().entity_mut(root).add_child(child);

    let ready_without_render_targets = app
        .world_mut()
        .run_system_once(
            move |parent_query: Query<&ChildOf>,
                  geoset_query: Query<(Entity, &crate::m2_spawn::GeosetMesh, &ChildOf)>,
                  material_query: Query<(
                Entity,
                &MeshMaterial3d<StandardMaterial>,
                Option<&crate::m2_spawn::GeosetMesh>,
                Option<&crate::m2_spawn::BatchTextureType>,
                &ChildOf,
            )>| {
                character_root_ready_for_appearance_sync(
                    root,
                    &parent_query,
                    &geoset_query,
                    &material_query,
                )
            },
        )
        .expect("readiness query should run");
    assert!(!ready_without_render_targets);

    let mesh = app
        .world_mut()
        .resource_mut::<Assets<Mesh>>()
        .add(Sphere::new(1.0));
    let material = app
        .world_mut()
        .resource_mut::<Assets<StandardMaterial>>()
        .add(StandardMaterial::default());
    app.world_mut().entity_mut(child).insert((
        crate::m2_spawn::GeosetMesh(101),
        Mesh3d(mesh),
        MeshMaterial3d(material),
    ));

    let ready_with_render_targets = app
        .world_mut()
        .run_system_once(
            move |parent_query: Query<&ChildOf>,
                  geoset_query: Query<(Entity, &crate::m2_spawn::GeosetMesh, &ChildOf)>,
                  material_query: Query<(
                Entity,
                &MeshMaterial3d<StandardMaterial>,
                Option<&crate::m2_spawn::GeosetMesh>,
                Option<&crate::m2_spawn::BatchTextureType>,
                &ChildOf,
            )>| {
                character_root_ready_for_appearance_sync(
                    root,
                    &parent_query,
                    &geoset_query,
                    &material_query,
                )
            },
        )
        .expect("readiness query should run");
    assert!(ready_with_render_targets);
}

#[test]
fn resolve_char_select_model_path_returns_none_when_no_characters_exist() {
    let char_list = CharacterList(Vec::new());

    assert_eq!(resolve_char_select_model_path(&char_list, None), None);
}

#[test]
fn race_model_wow_path_covers_known_playable_races_and_sex() {
    assert_eq!(
        race_model_wow_path(1, 0),
        Some("character/human/male/humanmale_hd.m2")
    );
    assert_eq!(
        race_model_wow_path(2, 0),
        Some("character/orc/male/orcmale_hd.m2")
    );
    assert_eq!(
        race_model_wow_path(10, 1),
        Some("character/bloodelf/female/bloodelffemale_hd.m2")
    );
    assert_eq!(
        race_model_wow_path(10, 0),
        Some("character/bloodelf/male/bloodelfmale_hd.m2")
    );
    assert_eq!(race_model_wow_path(99, 0), None);
}

#[test]
fn camera_params_center_focused_placement_horizontally() {
    let warband = crate::scenes::char_select::warband::WarbandScenes::load();
    let scene = warband
        .scenes
        .iter()
        .find(|scene| scene.id == 1)
        .expect("Adventurer's Rest");
    let placement = selected_scene_placement(&warband, scene).expect("expected placement");
    let (eye, focus, _) =
        camera_params(Some(scene), Some(&placement), ModelPresentation::default());
    let forward = (focus - eye).normalize();
    let right = forward.cross(Vec3::Y).normalize();
    let rel = placement.bevy_position() - eye;

    assert!(
        rel.dot(right).abs() < 0.001,
        "focused placement should sit on the camera centerline"
    );
}

#[test]
fn camera_params_use_tighter_single_character_framing() {
    let warband = crate::scenes::char_select::warband::WarbandScenes::load();
    let scene = warband
        .scenes
        .iter()
        .find(|scene| scene.id == 1)
        .expect("Adventurer's Rest");
    let placement = selected_scene_placement(&warband, scene).expect("expected placement");
    let presentation = ModelPresentation::default();
    let (scene_eye, scene_focus, scene_fov) = camera_params(Some(scene), None, presentation);
    let (eye, focus, fov) = camera_params(Some(scene), Some(&placement), presentation);

    assert!(
        eye.distance(focus) < scene_eye.distance(scene_focus),
        "single-character framing should move the camera closer than the raw warband overview"
    );
    assert!(
        fov < scene_fov,
        "single-character framing should narrow the FOV from the warband overview"
    );
}

#[test]
fn camera_params_use_model_center_height_for_single_character_focus() {
    let warband = crate::scenes::char_select::warband::WarbandScenes::load();
    let scene = warband
        .scenes
        .iter()
        .find(|scene| scene.id == 1)
        .expect("Adventurer's Rest");
    let placement = selected_scene_placement(&warband, scene).expect("expected placement");
    let presentation = ModelPresentation {
        customize_scale: 1.1,
        camera_distance_offset: -0.34,
    };

    let (_, focus, _) = camera_params(Some(scene), Some(&placement), presentation);

    assert!(
        (focus.y - (placement.bevy_position().y + presentation.customize_scale)).abs() < 0.001,
        "single-character focus should target model center height, got focus_y={} placement_y={} scale={}",
        focus.y,
        placement.bevy_position().y,
        presentation.customize_scale
    );
}

#[test]
fn camera_params_preserve_authored_vertical_offset_for_single_character_eye() {
    let warband = crate::scenes::char_select::warband::WarbandScenes::load();
    let scene = warband
        .scenes
        .iter()
        .find(|scene| scene.id == 1)
        .expect("Adventurer's Rest");
    let placement = selected_scene_placement(&warband, scene).expect("expected placement");
    let presentation = ModelPresentation {
        customize_scale: 1.1,
        camera_distance_offset: -0.34,
    };

    let (eye, focus, _) = camera_params(Some(scene), Some(&placement), presentation);
    let authored_vertical = scene.bevy_position().y - scene.bevy_look_at().y;

    assert!(
        ((eye.y - focus.y) - authored_vertical).abs() < 0.001,
        "single-character eye should preserve authored vertical lift, got {} expected {}",
        eye.y - focus.y,
        authored_vertical
    );
}

#[test]
fn character_transform_snaps_character_up_to_warband_terrain() {
    let warband = crate::scenes::char_select::warband::WarbandScenes::load();
    let scene = warband
        .scenes
        .iter()
        .find(|scene| scene.id == 1)
        .expect("Adventurer's Rest");
    let placement = selected_scene_placement(&warband, scene).expect("expected placement");
    let adt_path = crate::scenes::char_select::warband::ensure_warband_terrain(scene)
        .expect("expected warband terrain");
    let data = std::fs::read(&adt_path).expect("expected ADT data");
    let adt = crate::asset::adt::load_adt(&data).expect("expected ADT parse");
    let mut heightmap = TerrainHeightmap::default();
    let (tile_y, tile_x) = scene.tile_coords();
    heightmap.insert_tile(tile_y, tile_x, &adt);

    let transform = character_transform(
        Some(scene),
        Some(&placement),
        Some(&heightmap),
        ModelPresentation::default(),
    );
    let terrain_y = heightmap
        .height_at(transform.translation.x, transform.translation.z)
        .expect("expected terrain at placement");

    assert!(
        (transform.translation.y - terrain_y).abs() < 0.001,
        "character root should sit on terrain, got root_y={} terrain_y={terrain_y}",
        transform.translation.y
    );
}

#[test]
fn clamp_char_select_eye_keeps_camera_above_terrain() {
    let data = std::fs::read("data/terrain/azeroth_32_48.adt")
        .expect("expected test ADT data/terrain/azeroth_32_48.adt");
    let adt = crate::asset::adt::load_adt(&data).expect("expected ADT to parse");
    let mut heightmap = TerrainHeightmap::default();
    heightmap.insert_tile(32, 48, &adt);

    let [bx, _, bz] = crate::asset::m2::wow_to_bevy(-8949.0, -132.0, 83.0);
    let terrain_y = heightmap
        .height_at(bx, bz)
        .expect("expected terrain at sample position");
    let clamped = clamp_char_select_eye(Vec3::new(bx, terrain_y - 3.0, bz), Some(&heightmap));

    assert!(
        (clamped.y - (terrain_y + CHAR_SELECT_CAMERA_GROUND_CLEARANCE)).abs() < 0.001,
        "camera should stay above terrain, got camera_y={} terrain_y={terrain_y}",
        clamped.y
    );
}

#[test]
fn focused_placement_rotation_faces_camera_reasonably() {
    let warband = crate::scenes::char_select::warband::WarbandScenes::load();
    let scene = warband
        .scenes
        .iter()
        .find(|scene| scene.id == 1)
        .expect("Adventurer's Rest");
    let placement = selected_scene_placement(&warband, scene).expect("expected placement");
    let rotation = single_character_rotation(scene, &placement, ModelPresentation::default());
    let (eye, _, _) = camera_params(Some(scene), Some(&placement), ModelPresentation::default());
    let to_camera = (eye - placement.bevy_position()).normalize_or_zero();
    let facing = rotation * Vec3::X;
    let angle = facing.angle_between(to_camera).to_degrees();

    assert!(
        angle < 25.0,
        "focused placement should face mostly toward the camera, got {angle:.2} degrees"
    );
}

#[test]
fn camera_params_apply_model_distance_offset() {
    let warband = crate::scenes::char_select::warband::WarbandScenes::load();
    let scene = warband
        .scenes
        .iter()
        .find(|scene| scene.id == 1)
        .expect("Adventurer's Rest");
    let placement = selected_scene_placement(&warband, scene).expect("expected placement");
    let default_presentation = ModelPresentation::default();
    let human_presentation = ModelPresentation {
        customize_scale: 1.1,
        camera_distance_offset: -0.34,
    };
    let (default_eye, default_focus, _) =
        camera_params(Some(scene), Some(&placement), default_presentation);
    let (eye, focus, _) = camera_params(Some(scene), Some(&placement), human_presentation);

    assert!(eye.distance(focus) < default_eye.distance(default_focus));
}

#[test]
fn orbit_from_eye_focus_preserves_initial_yaw() {
    let eye = Vec3::new(4.0, 3.0, -2.0);
    let focus = Vec3::new(1.5, 1.0, 0.5);

    let orbit = orbit_from_eye_focus(eye, focus);

    assert!(
        orbit_eye(&orbit).distance(eye) < 1e-5,
        "reconstructed orbit eye should match the authored eye position"
    );
}

#[test]
fn char_select_orbit_camera_moves_transform_and_clamps_drag_limits() {
    let mut app = App::new();
    app.insert_resource(crate::client_options::CameraOptions::default());
    app.insert_resource(AccumulatedMouseMotion {
        delta: Vec2::new(500.0, 500.0),
    });
    let mut mouse_buttons = ButtonInput::<MouseButton>::default();
    mouse_buttons.press(MouseButton::Left);
    app.insert_resource(mouse_buttons);

    let orbit = orbit_from_eye_focus(Vec3::new(0.0, 2.0, 6.0), Vec3::new(0.0, 1.0, 0.0));
    let before = Transform::from_translation(orbit_eye(&orbit)).looking_at(orbit.focus, Vec3::Y);
    let entity = app.world_mut().spawn((orbit, before)).id();

    app.world_mut()
        .run_system_once(char_select_orbit_camera)
        .expect("char-select orbit camera should run");

    let orbit = app
        .world()
        .get::<camera::CharSelectOrbit>(entity)
        .expect("char-select orbit");
    let after = app
        .world()
        .get::<Transform>(entity)
        .expect("char-select transform");

    assert!(
        after.translation.distance(before.translation) > 0.1,
        "dragging should move the live camera transform"
    );
    assert!(
        (orbit.yaw + std::f32::consts::FRAC_PI_8).abs() < 0.0001,
        "yaw should clamp at the authored drag limit"
    );
    assert!(
        (orbit.pitch - 0.15).abs() < 0.0001,
        "pitch should clamp at the authored drag limit"
    );
}

#[test]
fn orbit_input_debug_state_reports_live_inputs_and_target_count() {
    let debug = orbit_input_debug_state(true, Vec2::new(12.0, -3.0), 2);

    assert!(debug.left_mouse_pressed);
    assert!(debug.has_mouse_motion);
    assert_eq!(debug.orbit_entity_count, 2);
}

#[test]
fn orbit_input_debug_state_marks_zero_motion_and_missing_targets() {
    let debug = orbit_input_debug_state(false, Vec2::ZERO, 0);

    assert!(!debug.left_mouse_pressed);
    assert!(!debug.has_mouse_motion);
    assert_eq!(debug.orbit_entity_count, 0);
}

#[test]
fn should_log_orbit_input_logs_state_transitions_once() {
    let missing_camera = orbit_input_debug_state(false, Vec2::ZERO, 0);

    assert!(should_log_orbit_input(None, missing_camera));
    assert!(!should_log_orbit_input(
        Some(missing_camera),
        missing_camera
    ));
}

#[test]
fn should_log_orbit_input_always_logs_live_motion() {
    let dragging = orbit_input_debug_state(true, Vec2::new(8.0, 0.0), 1);

    assert!(should_log_orbit_input(Some(dragging), dragging));
}

#[test]
fn debug_orbit_camera_system_ignores_char_select_orbit_entities() {
    let mut app = App::new();
    app.insert_resource(crate::client_options::CameraOptions::default());
    app.insert_resource(AccumulatedMouseMotion {
        delta: Vec2::new(24.0, -6.0),
    });
    app.insert_resource(AccumulatedMouseScroll::default());
    let mut mouse_buttons = ButtonInput::<MouseButton>::default();
    mouse_buttons.press(MouseButton::Left);
    app.insert_resource(mouse_buttons);
    app.add_systems(Update, crate::orbit_camera::orbit_camera_system);

    let orbit = orbit_from_eye_focus(Vec3::new(0.0, 2.0, 6.0), Vec3::new(0.0, 1.0, 0.0));
    let before = Transform::from_translation(orbit_eye(&orbit)).looking_at(orbit.focus, Vec3::Y);
    let entity = app.world_mut().spawn((orbit, before)).id();

    app.update();

    let after = app
        .world()
        .get::<Transform>(entity)
        .expect("char-select transform");
    assert_eq!(
        *after, before,
        "debug orbit camera system should ignore CharSelectOrbit-only entities"
    );
}

#[test]
fn debug_and_char_select_orbit_systems_share_mouse_motion_without_consuming_it() {
    let mut app = App::new();
    app.insert_resource(crate::client_options::CameraOptions::default());
    app.insert_resource(AccumulatedMouseMotion {
        delta: Vec2::new(18.0, -5.0),
    });
    app.insert_resource(AccumulatedMouseScroll::default());
    let mut mouse_buttons = ButtonInput::<MouseButton>::default();
    mouse_buttons.press(MouseButton::Left);
    app.insert_resource(mouse_buttons);
    app.add_systems(
        Update,
        (
            crate::orbit_camera::orbit_camera_system,
            char_select_orbit_camera,
        ),
    );

    let char_select_orbit =
        orbit_from_eye_focus(Vec3::new(0.0, 2.0, 6.0), Vec3::new(0.0, 1.0, 0.0));
    let char_select_transform = Transform::from_translation(orbit_eye(&char_select_orbit))
        .looking_at(char_select_orbit.focus, Vec3::Y);
    let char_select_entity = app
        .world_mut()
        .spawn((char_select_orbit.clone(), char_select_transform))
        .id();

    let debug_orbit = crate::orbit_camera::OrbitCamera::new(Vec3::ZERO, 6.0);
    let debug_transform =
        Transform::from_translation(debug_orbit.eye_position()).looking_at(Vec3::ZERO, Vec3::Y);
    let debug_entity = app.world_mut().spawn((debug_orbit, debug_transform)).id();

    app.update();

    let char_select_orbit = app
        .world()
        .get::<camera::CharSelectOrbit>(char_select_entity)
        .expect("char-select orbit");
    let debug_orbit = app
        .world()
        .get::<crate::orbit_camera::OrbitCamera>(debug_entity)
        .expect("debug orbit");

    assert_ne!(
        char_select_orbit.yaw, 0.0,
        "char-select orbit should still receive the frame's mouse motion"
    );
    assert_ne!(
        debug_orbit.yaw, 0.0,
        "debug orbit camera should also receive the same frame's mouse motion"
    );
}

#[test]
fn char_select_ui_click_handling_does_not_block_orbit_camera() {
    let mut app = App::new();
    app.add_plugins(MinimalPlugins);
    app.add_plugins(StatesPlugin);
    app.add_plugins(UiAutomationPlugin);
    app.add_plugins(crate::scenes::char_select::CharSelectPlugin);
    app.add_message::<KeyboardInput>();
    app.insert_resource(UiState {
        registry: FrameRegistry::new(0.0, 0.0),
        event_bus: EventBus::new(),
        focused_frame: None,
    });
    app.insert_resource(ButtonInput::<MouseButton>::default());
    app.insert_resource(CharacterList(vec![character(1, 1, 0, "Elara")]));
    app.insert_resource(crate::client_options::CameraOptions::default());
    app.insert_resource(AccumulatedMouseMotion {
        delta: Vec2::new(18.0, -5.0),
    });
    app.insert_state(crate::game_state::GameState::CharSelect);

    let window_entity = app
        .world_mut()
        .spawn((Window::default(), PrimaryWindow))
        .id();
    app.update();
    app.world_mut()
        .run_system_once(
            |windows: Query<&Window, With<PrimaryWindow>>, mut ui: ResMut<UiState>| {
                ui_toolkit::plugin::sync_registry_to_primary_window(&mut ui.registry, &windows);
                ui_toolkit::layout::recompute_layouts(&mut ui.registry);
            },
        )
        .expect("char-select UI layout should resolve");

    let create_button_center = {
        let ui = app.world().resource::<UiState>();
        let create_id = ui
            .registry
            .get_by_name("CreateChar")
            .expect("CreateChar frame should exist");
        let layout = ui
            .registry
            .get(create_id)
            .and_then(|frame| frame.layout_rect.as_ref())
            .expect("CreateChar should have resolved layout");
        Vec2::new(
            layout.x + layout.width * 0.5,
            layout.y + layout.height * 0.5,
        )
    };
    app.world_mut()
        .entity_mut(window_entity)
        .get_mut::<Window>()
        .expect("primary window")
        .set_cursor_position(Some(create_button_center));

    let mut click_cursor = app
        .world()
        .resource::<Messages<crate::scenes::char_select::input::CharSelectClickEvent>>()
        .get_cursor_current();
    app.world_mut()
        .resource_mut::<ButtonInput<MouseButton>>()
        .press(MouseButton::Left);

    let orbit = orbit_from_eye_focus(Vec3::new(0.0, 2.0, 6.0), Vec3::new(0.0, 1.0, 0.0));
    let before = Transform::from_translation(orbit_eye(&orbit)).looking_at(orbit.focus, Vec3::Y);
    let entity = app.world_mut().spawn((orbit, before)).id();

    app.world_mut()
        .run_system_once(crate::scenes::char_select::input::char_select_mouse_input)
        .expect("char-select mouse input should run");
    let click_count = {
        let messages = app
            .world()
            .resource::<Messages<crate::scenes::char_select::input::CharSelectClickEvent>>();
        click_cursor.read(messages).count()
    };
    assert_eq!(
        click_count, 1,
        "char-select mouse input should still register the UI click"
    );

    app.world_mut()
        .run_system_once(char_select_orbit_camera)
        .expect("char-select orbit camera should run");

    let orbit = app
        .world()
        .get::<camera::CharSelectOrbit>(entity)
        .expect("char-select orbit");
    assert_ne!(
        orbit.yaw, 0.0,
        "orbit camera should still receive the same frame's mouse motion"
    );
}

#[test]
fn sync_char_select_model_leaves_camera_unchanged_when_character_is_already_displayed() {
    let mut app = App::new();
    app.init_resource::<Assets<Mesh>>();
    app.init_resource::<Assets<StandardMaterial>>();
    app.init_resource::<Assets<crate::sky_material::SkyMaterial>>();
    app.init_resource::<Assets<crate::m2_effect_material::M2EffectMaterial>>();
    app.init_resource::<Assets<crate::skybox_m2_material::SkyboxM2Material>>();
    app.init_resource::<Assets<crate::terrain_material::TerrainMaterial>>();
    app.init_resource::<Assets<crate::water_material::WaterMaterial>>();
    app.init_resource::<Assets<Image>>();
    app.init_resource::<Assets<bevy::mesh::skinning::SkinnedMeshInverseBindposes>>();
    app.insert_resource(crate::creature_display::CreatureDisplayMap);
    app.insert_resource(game_engine::customization_data::CustomizationDb::load(
        std::path::Path::new("data"),
    ));
    app.insert_resource(crate::terrain_heightmap::TerrainHeightmap::default());

    let character_id = 42;
    app.insert_resource(CharacterList(vec![character(character_id, 1, 0, "Elara")]));
    app.insert_resource(crate::scenes::char_select::SelectedCharIndex(Some(0)));
    app.insert_resource(scene_types::DisplayedCharacterId(Some(character_id)));

    let warband = crate::scenes::char_select::warband::WarbandScenes::load();
    let scene = warband
        .scenes
        .iter()
        .find(|scene| scene.id == 1)
        .expect("Adventurer's Rest")
        .clone();
    app.insert_resource(warband);
    app.insert_resource(crate::scenes::char_select::warband::SelectedWarbandScene {
        scene_id: scene.id,
    });

    let camera_entity = app
        .world_mut()
        .spawn((
            CharSelectScene,
            camera::orbit_from_eye_focus(Vec3::new(8.0, 5.0, 3.0), Vec3::new(1.0, 2.0, 0.0)),
            Transform::from_xyz(8.0, 5.0, 3.0).looking_at(Vec3::new(1.0, 2.0, 0.0), Vec3::Y),
            Projection::Perspective(PerspectiveProjection::default()),
        ))
        .id();
    let before_orbit = app
        .world()
        .get::<camera::CharSelectOrbit>(camera_entity)
        .expect("camera orbit")
        .clone();
    let before_transform = *app
        .world()
        .get::<Transform>(camera_entity)
        .expect("camera transform");

    app.world_mut()
        .run_system_once(sync_char_select_model)
        .expect("model sync should run");

    let after_orbit = app
        .world()
        .get::<camera::CharSelectOrbit>(camera_entity)
        .expect("camera orbit after sync");
    let after_transform = app
        .world()
        .get::<Transform>(camera_entity)
        .expect("camera transform after sync");

    assert_eq!(after_orbit.yaw, before_orbit.yaw);
    assert_eq!(after_orbit.base_yaw, before_orbit.base_yaw);
    assert_eq!(after_orbit.pitch, before_orbit.pitch);
    assert_eq!(after_orbit.focus, before_orbit.focus);
    assert_eq!(after_orbit.distance, before_orbit.distance);
    assert_eq!(after_orbit.base_pitch, before_orbit.base_pitch);
    assert_eq!(
        *after_transform, before_transform,
        "steady-state model sync should not rewrite the camera transform"
    );
}

#[test]
fn model_sync_debug_state_skips_respawn_when_displayed_matches_selected_character() {
    let debug_state = model_sync_debug_state(Some(42), Some(42));

    assert_eq!(debug_state.displayed_id, Some(42));
    assert_eq!(debug_state.desired_id, Some(42));
    assert!(
        !debug_state.should_respawn(),
        "matching displayed and desired ids should not respawn the char-select model"
    );
}

#[test]
fn model_sync_debug_state_requests_respawn_when_selected_character_changes() {
    let debug_state = model_sync_debug_state(Some(42), Some(77));

    assert_eq!(debug_state.displayed_id, Some(42));
    assert_eq!(debug_state.desired_id, Some(77));
    assert!(
        debug_state.should_respawn(),
        "different displayed and desired ids should respawn the char-select model"
    );
}

#[test]
fn focused_placement_rotation_faces_camera_tightly() {
    let warband = crate::scenes::char_select::warband::WarbandScenes::load();
    let scene = warband
        .scenes
        .iter()
        .find(|scene| scene.id == 1)
        .expect("Adventurer's Rest");
    let placement = selected_scene_placement(&warband, scene).expect("expected placement");
    let rotation = single_character_rotation(scene, &placement, ModelPresentation::default());
    let (eye, _, _) = camera_params(Some(scene), Some(&placement), ModelPresentation::default());
    let to_camera = (eye - placement.bevy_position()).normalize_or_zero();
    let to_camera = Vec3::new(to_camera.x, 0.0, to_camera.z).normalize_or_zero();
    let facing = rotation * Vec3::X;
    let facing = Vec3::new(facing.x, 0.0, facing.z).normalize_or_zero();
    let angle = facing.angle_between(to_camera).to_degrees();

    assert!(
        angle < 1.0,
        "single-character rotation should face the camera horizontally, got {angle:.2} degrees"
    );
}

#[test]
fn char_select_ambient_brightness_matches_scene_lighting_budget() {
    assert_eq!(lighting::CHAR_SELECT_AMBIENT_BRIGHTNESS, 150.0);
}
