use super::*;
use std::path::Path;

#[test]
fn startup_mode_can_open_customize_directly() {
    let db = CustomizationDb::load(Path::new("data"));
    let state =
        initial_char_create_state(Some(StartupCharCreateMode(CharCreateMode::Customize)), &db);
    assert_eq!(state.mode, CharCreateMode::Customize);
}

#[test]
fn default_startup_mode_stays_on_race_class() {
    let db = CustomizationDb::load(Path::new("data"));
    let state = initial_char_create_state(None, &db);
    assert_eq!(state.mode, CharCreateMode::RaceClass);
}

#[test]
fn randomized_appearance_stays_within_valid_choice_ranges() {
    let db = CustomizationDb::load(Path::new("data"));
    let mut state = CharCreateState {
        selected_race: 1,
        selected_class: 1,
        selected_sex: 0,
        ..Default::default()
    };

    randomize_appearance_with_seed(&mut state, &db, 0x1234_5678_9abc_def0);

    assert_eq!(state.appearance.sex, 0);
    assert!(
        state.appearance.skin_color < db.choice_count_for_class(1, 0, 1, OptionType::SkinColor)
    );
    assert!(state.appearance.face < db.choice_count_for_class(1, 0, 1, OptionType::Face));
    assert!(
        state.appearance.hair_style < db.choice_count_for_class(1, 0, 1, OptionType::HairStyle)
    );
    assert!(
        state.appearance.hair_color < db.choice_count_for_class(1, 0, 1, OptionType::HairColor)
    );
    assert!(
        state.appearance.facial_style < db.choice_count_for_class(1, 0, 1, OptionType::FacialHair)
    );
}

fn skin_choice_ids(
    db: &CustomizationDb,
    race: u8,
    sex: u8,
    class: u8,
) -> std::collections::HashSet<u32> {
    let count = db.choice_count_for_class(race, sex, class, OptionType::SkinColor);
    (0..count)
        .filter_map(|index| {
            db.get_choice_for_class(race, sex, class, OptionType::SkinColor, index)
                .map(|choice| choice.id)
        })
        .collect()
}

fn face_is_compatible_with_skin(
    db: &CustomizationDb,
    race: u8,
    sex: u8,
    class: u8,
    skin_color: u8,
    face: u8,
) -> bool {
    let skin_ids = skin_choice_ids(db, race, sex, class);
    let Some(selected_skin_id) = db
        .get_choice_for_class(race, sex, class, OptionType::SkinColor, skin_color)
        .map(|choice| choice.id)
    else {
        return false;
    };
    let Some(choice) = db.get_choice_for_class(race, sex, class, OptionType::Face, face) else {
        return false;
    };
    let related_skin_ids: std::collections::HashSet<_> = choice
        .related_materials
        .iter()
        .map(|material| material.related_choice_id)
        .chain(
            choice
                .related_geosets
                .iter()
                .map(|geoset| geoset.related_choice_id),
        )
        .filter(|choice_id| skin_ids.contains(choice_id))
        .collect();
    related_skin_ids.is_empty() || related_skin_ids.contains(&selected_skin_id)
}

#[test]
fn randomized_face_stays_compatible_with_selected_skin_color() {
    let db = CustomizationDb::load(Path::new("data"));
    let mut state = CharCreateState {
        selected_race: 1,
        selected_class: 1,
        selected_sex: 0,
        ..Default::default()
    };

    for seed in 0..256_u64 {
        randomize_appearance_with_seed(&mut state, &db, seed);
        assert!(
            face_is_compatible_with_skin(
                &db,
                state.selected_race,
                state.selected_sex,
                state.selected_class,
                state.appearance.skin_color,
                state.appearance.face,
            ),
            "seed {seed:#x} produced incompatible face {} for skin {}",
            state.appearance.face,
            state.appearance.skin_color
        );
    }
}

#[test]
fn changing_skin_color_reclamps_face_to_compatible_set() {
    let db = CustomizationDb::load(Path::new("data"));
    let mut state = CharCreateState {
        selected_race: 1,
        selected_class: 1,
        selected_sex: 0,
        appearance: CharacterAppearance {
            sex: 0,
            skin_color: 0,
            face: 10,
            ..Default::default()
        },
        ..Default::default()
    };

    input::adjust_appearance(&mut state, AppearanceField::SkinColor, 1, &db);

    assert!(face_is_compatible_with_skin(
        &db,
        state.selected_race,
        state.selected_sex,
        state.selected_class,
        state.appearance.skin_color,
        state.appearance.face,
    ));
}

#[test]
fn race_class_and_sex_changes_re_randomize_appearance() {
    let db = CustomizationDb::load(Path::new("data"));
    let mut state = CharCreateState::default();

    input::apply_race_change_with_seed(&mut state, 10, &db, 1);
    assert_eq!(state.selected_race, 10);
    assert_ne!(state.appearance, CharacterAppearance::default());

    let race_appearance = state.appearance;
    input::apply_class_change_with_seed(&mut state, 3, &db, 2);
    assert_eq!(state.selected_class, 3);
    assert_ne!(state.appearance, race_appearance);

    let class_appearance = state.appearance;
    input::apply_sex_toggle_with_seed(&mut state, &db, 3);
    assert_eq!(state.selected_sex, 1);
    assert_eq!(state.appearance.sex, 1);
    assert_ne!(state.appearance, class_appearance);
}

#[test]
fn explicit_randomize_re_rolls_appearance_without_changing_selection() {
    let db = CustomizationDb::load(Path::new("data"));
    let mut state = CharCreateState {
        selected_race: 10,
        selected_class: 3,
        selected_sex: 1,
        ..Default::default()
    };

    randomize_appearance_with_seed(&mut state, &db, 11);
    let original = state.appearance;

    input::apply_randomize_with_seed(&mut state, &db, 12);

    assert_eq!(state.selected_race, 10);
    assert_eq!(state.selected_class, 3);
    assert_eq!(state.selected_sex, 1);
    assert_eq!(state.appearance.sex, 1);
    assert_ne!(state.appearance, original);
}

#[test]
fn clicking_race_button_changes_selected_race() {
    use bevy::app::App;
    use bevy::ecs::system::RunSystemOnce;
    use bevy::input::ButtonInput;
    use bevy::prelude::*;
    use bevy::state::app::StatesPlugin;
    use bevy::window::PrimaryWindow;
    use game_engine::customization_data::CustomizationDb;
    use game_engine::ui::automation::UiAutomationPlugin;
    use game_engine::ui::plugin::UiState;
    use game_engine::ui::registry::FrameRegistry;
    use game_engine::ui::event::EventBus;

    let mut app = App::new();
    app.add_plugins(MinimalPlugins);
    app.add_plugins(StatesPlugin);
    app.add_plugins(UiAutomationPlugin);
    app.add_plugins(crate::scenes::char_create::CharCreatePlugin);
    app.add_message::<bevy::input::keyboard::KeyboardInput>();
    app.insert_resource(UiState {
        registry: FrameRegistry::new(1920.0, 1080.0),
        event_bus: EventBus::new(),
        focused_frame: None,
    });
    app.insert_resource(ButtonInput::<MouseButton>::default());
    app.insert_resource(CustomizationDb::load(std::path::Path::new("data")));
    app.insert_state(crate::game_state::GameState::CharCreate);

    let window_entity = app
        .world_mut()
        .spawn((Window::default(), PrimaryWindow))
        .id();
    // Run OnEnter to build the UI
    app.update();
    // Recompute layouts so hit testing works
    app.world_mut()
        .run_system_once(
            |windows: Query<&Window, With<PrimaryWindow>>, mut ui: ResMut<UiState>| {
                ui_toolkit::plugin::sync_registry_to_primary_window(&mut ui.registry, &windows);
                ui_toolkit::layout::recompute_layouts(&mut ui.registry);
            },
        )
        .expect("layout recompute should run");

    // Verify initial state is race 1
    let initial_race = app.world().resource::<CharCreateState>().selected_race;
    assert_eq!(initial_race, 1, "initial race should be human (1)");

    // Find Race_2 button center
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
            .expect("Race_2 should have layout rect");
        Vec2::new(layout.x + layout.width / 2.0, layout.y + layout.height / 2.0)
    };

    // Inject click at Race_2 center
    app.world_mut()
        .entity_mut(window_entity)
        .get_mut::<Window>()
        .unwrap()
        .set_cursor_position(Some(race_2_center));
    app.world_mut()
        .resource_mut::<ButtonInput<MouseButton>>()
        .press(MouseButton::Left);

    // Run the input system
    app.world_mut()
        .run_system_once(input::char_create_mouse_input)
        .expect("char_create_mouse_input should run");

    let new_race = app.world().resource::<CharCreateState>().selected_race;
    assert_eq!(
        new_race, 2,
        "clicking Race_2 should change selected_race from 1 to 2, got {new_race}"
    );
}
