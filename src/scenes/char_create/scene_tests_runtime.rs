use super::*;

#[test]
fn runtime_race_click_updates_displayed_models_through_full_scheduler() {
    use bevy::input::ButtonInput;
    use bevy::state::app::StatesPlugin;
    use bevy::window::PrimaryWindow;
    use game_engine::asset::char_texture::CharTextureData;
    use game_engine::ui::automation::UiAutomationPlugin;
    use game_engine::ui::plugin::UiState;

    let mut app = App::new();
    app.add_plugins(MinimalPlugins);
    app.add_plugins(bevy::transform::TransformPlugin);
    app.add_plugins(StatesPlugin);
    app.add_plugins(bevy::asset::AssetPlugin::default());
    app.add_plugins(bevy::text::TextPlugin::default());
    app.add_plugins(UiAutomationPlugin);
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
    app.add_plugins(ui_toolkit::plugin::UiPlugin);
    app.add_plugins(crate::scenes::char_create::CharCreatePlugin);
    app.add_plugins(CharCreateScenePlugin);
    app.add_message::<bevy::input::keyboard::KeyboardInput>();
    app.insert_state(crate::game_state::GameState::CharCreate);

    let window_entity = app
        .world_mut()
        .spawn((Window::default(), PrimaryWindow))
        .id();
    app.update();
    app.update();

    let initial_displayed = app.world().resource::<DisplayedModels>();
    assert_eq!(initial_displayed.race, Some(1));
    let initial_models = initial_displayed.models.clone();
    assert!(
        initial_models.len() >= 2,
        "initial scene should spawn both sex variants, got {}",
        initial_models.len()
    );

    let race_2_center = {
        let ui = app.world().resource::<UiState>();
        let race_2_id = ui
            .registry
            .get_by_name("Race_2")
            .expect("Race_2 frame should exist");
        let layout = ui
            .registry
            .get(race_2_id)
            .and_then(|f| f.layout_rect.as_ref())
            .expect("Race_2 should have layout");
        Vec2::new(
            layout.x + layout.width / 2.0,
            layout.y + layout.height / 2.0,
        )
    };

    app.world_mut()
        .entity_mut(window_entity)
        .get_mut::<Window>()
        .unwrap()
        .set_cursor_position(Some(race_2_center));
    app.world_mut()
        .resource_mut::<ButtonInput<MouseButton>>()
        .press(MouseButton::Left);

    app.update();

    let new_race = app.world().resource::<CharCreateState>().selected_race;
    assert_eq!(new_race, 2, "Race_2 click should update selected_race");

    let displayed = app.world().resource::<DisplayedModels>();
    assert_eq!(
        displayed.race,
        Some(2),
        "sync_model should observe the changed race during scheduled runtime update"
    );
    assert_eq!(displayed.active_sex, 0);
    assert!(
        displayed.models.len() >= 2,
        "respawned scene should still track both sex variants, got {}",
        displayed.models.len()
    );
    assert_ne!(
        displayed.models, initial_models,
        "runtime race switch should replace the tracked model entities"
    );
}
