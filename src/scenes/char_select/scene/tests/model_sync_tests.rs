use super::*;

#[test]
fn clicking_non_selected_character_switches_model_and_highlights_card() {
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
    app.init_resource::<scene_types::DisplayedCharacterId>();
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

    let first_id = 6;
    let second_id = 7;
    app.insert_resource(CharacterList(vec![
        character(first_id, 1, 0, "Theron"),
        character(second_id, 1, 1, "Elara"),
    ]));
    app.insert_state(crate::game_state::GameState::CharSelect);

    let mut window = Window::default();
    window.resolution.set(1280.0, 720.0);
    let window_entity = app.world_mut().spawn((window, PrimaryWindow)).id();

    app.update();
    app.world_mut()
        .run_system_once(
            |windows: Query<&Window, With<PrimaryWindow>>, mut ui: ResMut<UiState>| {
                ui_toolkit::plugin::sync_registry_to_primary_window(&mut ui.registry, &windows);
                ui_toolkit::layout::recompute_layouts(&mut ui.registry);
            },
        )
        .expect("char-select UI layout should resolve");

    app.world_mut()
        .run_system_once(sync_char_select_model)
        .expect("initial model sync should run");

    let initial_root = {
        let world = app.world_mut();
        let mut root_query = world
            .query_filtered::<(Entity, &CharSelectModelCharacter), With<CharSelectModelRoot>>();
        let roots: Vec<_> = root_query
            .iter(world)
            .map(|(entity, character)| (entity, character.0))
            .collect();
        assert_eq!(roots.len(), 1, "expected one initial model root");
        assert_eq!(
            roots[0].1, first_id,
            "initial model sync should spawn the first selected character"
        );
        roots[0].0
    };
    let background_entity = app.world_mut().spawn_empty().id();
    let (initial_race, initial_gender, initial_model) =
        scene_systems::char_info_strings(&app.world().resource::<CharacterList>(), Some(0));
    app.insert_resource(scene_tree::build_scene_tree(vec![
        scene_tree::background_scene_node(background_entity, "ground", 0, vec![]),
        scene_tree::character_scene_node(
            initial_root,
            initial_model,
            initial_race,
            initial_gender,
            Some("Theron".to_string()),
            Some(first_id),
        ),
    ]));

    let second_card_center = {
        let ui = app.world().resource::<UiState>();
        let card = ui
            .registry
            .get_by_name("CharCard_1")
            .expect("CharCard_1 frame should exist");
        let layout = ui
            .registry
            .get(card)
            .and_then(|frame| frame.layout_rect.as_ref())
            .expect("CharCard_1 should have resolved layout");
        Vec2::new(
            layout.x + layout.width * 0.5,
            layout.y + layout.height * 0.5,
        )
    };
    app.world_mut()
        .entity_mut(window_entity)
        .get_mut::<Window>()
        .expect("primary window")
        .set_cursor_position(Some(second_card_center));
    app.world_mut()
        .resource_mut::<ButtonInput<MouseButton>>()
        .press(MouseButton::Left);

    app.world_mut()
        .run_system_once(crate::scenes::char_select::input::char_select_mouse_input)
        .expect("mouse click dispatch should run");
    app.world_mut()
        .run_system_once(crate::scenes::char_select::input::dispatch_char_select_action)
        .expect("char-select action dispatch should run");
    app.world_mut()
        .run_system_once(crate::scenes::char_select::char_select_update_visuals)
        .expect("visual sync should run");
    app.world_mut()
        .run_system_once(sync_char_select_model)
        .expect("model sync after click should run");

    assert_eq!(
        app.world()
            .resource::<crate::scenes::char_select::SelectedCharIndex>()
            .0,
        Some(1),
        "clicking the second character card should switch SelectedCharIndex"
    );
    assert_eq!(
        app.world()
            .resource::<scene_types::DisplayedCharacterId>()
            .0,
        Some(second_id),
        "model sync should switch the displayed character id after the click"
    );

    let displayed_roots = {
        let world = app.world_mut();
        let mut root_query = world
            .query_filtered::<(Entity, &CharSelectModelCharacter), With<CharSelectModelRoot>>();
        root_query
            .iter(world)
            .map(|(entity, character)| (entity, character.0))
            .collect::<Vec<_>>()
    };
    assert_eq!(
        displayed_roots.len(),
        1,
        "expected one displayed model root"
    );
    assert_eq!(
        displayed_roots[0].1, second_id,
        "clicking a different card should respawn the displayed model for that character"
    );

    let ui = app.world().resource::<UiState>();
    let selected_name_id = ui
        .registry
        .get_by_name("CharSelectCharacterName")
        .expect("CharSelectCharacterName");
    let Some(game_engine::ui::frame::WidgetData::FontString(selected_name)) = ui
        .registry
        .get(selected_name_id)
        .and_then(|frame| frame.widget_data.as_ref())
    else {
        panic!("CharSelectCharacterName should be a fontstring");
    };
    assert_eq!(selected_name.text, "Elara");

    let card0_selected = ui
        .registry
        .get(
            ui.registry
                .get_by_name("CharCard_0Selected")
                .expect("CharCard_0Selected"),
        )
        .expect("CharCard_0Selected frame");
    let card1_selected = ui
        .registry
        .get(
            ui.registry
                .get_by_name("CharCard_1Selected")
                .expect("CharCard_1Selected"),
        )
        .expect("CharCard_1Selected frame");
    assert!(
        card0_selected.hidden,
        "first card highlight should hide after clicking the second card"
    );
    assert!(
        !card1_selected.hidden,
        "second card highlight should show after clicking the second card"
    );

    let scene_tree = app.world().resource::<game_engine::scene_tree::SceneTree>();
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
        .expect("scene tree should contain a character node");
    let game_engine::scene_tree::NodeProps::Character {
        name, character_id, ..
    } = &character_node.props
    else {
        panic!("expected character node");
    };
    assert_eq!(character_node.entity, Some(displayed_roots[0].0));
    assert_eq!(name.as_deref(), Some("Elara"));
    assert_eq!(*character_id, Some(second_id));
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
fn sync_char_select_model_respawns_when_selected_character_changes() {
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

    let first_id = 42;
    let second_id = 77;
    app.insert_resource(CharacterList(vec![
        character(first_id, 1, 0, "Elara"),
        character(second_id, 1, 1, "Theron"),
    ]));
    app.insert_resource(crate::scenes::char_select::SelectedCharIndex(Some(0)));
    app.insert_resource(scene_types::DisplayedCharacterId(None));

    app.world_mut()
        .run_system_once(sync_char_select_model)
        .expect("initial model sync should run");

    let (first_wrapper, first_root) = {
        let world = app.world_mut();
        let mut wrapper_query = world.query_filtered::<Entity, With<CharSelectModelWrapper>>();
        let wrappers: Vec<_> = wrapper_query.iter(world).collect();
        assert_eq!(wrappers.len(), 1, "expected one char-select model wrapper");

        let mut root_query = world
            .query_filtered::<(Entity, &CharSelectModelCharacter), With<CharSelectModelRoot>>();
        let roots: Vec<_> = root_query
            .iter(world)
            .map(|(entity, character)| (entity, character.0))
            .collect();
        assert_eq!(roots.len(), 1, "expected one char-select model root");
        assert_eq!(
            roots[0].1, first_id,
            "initial sync should spawn the first character"
        );

        (wrappers[0], roots[0].0)
    };

    app.world_mut()
        .resource_mut::<crate::scenes::char_select::SelectedCharIndex>()
        .0 = Some(1);
    app.world_mut()
        .run_system_once(sync_char_select_model)
        .expect("respawn model sync should run");

    assert!(
        app.world().get_entity(first_wrapper).is_err(),
        "changing selection should despawn the old model wrapper"
    );
    assert!(
        app.world().get_entity(first_root).is_err(),
        "changing selection should despawn the old model root"
    );

    let new_root = {
        let world = app.world_mut();
        let mut wrapper_query = world.query_filtered::<Entity, With<CharSelectModelWrapper>>();
        let wrappers: Vec<_> = wrapper_query.iter(world).collect();
        assert_eq!(
            wrappers.len(),
            1,
            "respawn should leave exactly one char-select model wrapper"
        );

        let mut root_query = world
            .query_filtered::<(Entity, &CharSelectModelCharacter), With<CharSelectModelRoot>>();
        let roots: Vec<_> = root_query
            .iter(world)
            .map(|(entity, character)| (entity, character.0))
            .collect();
        assert_eq!(
            roots.len(),
            1,
            "respawn should leave exactly one model root"
        );
        assert_eq!(
            roots[0].1, second_id,
            "respawned model root should belong to the newly selected character"
        );
        roots[0].0
    };

    let displayed = app.world().resource::<scene_types::DisplayedCharacterId>();
    assert_eq!(
        displayed.0,
        Some(second_id),
        "displayed character id should update to the newly selected character"
    );
    assert_ne!(
        new_root, first_root,
        "changing selection should spawn a new model root entity"
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
