use super::*;
use bevy::ecs::system::RunSystemOnce;
use game_engine::network_runtime::messages::ConnectionSender;
use game_engine::ui::event::EventBus;
use game_engine::ui::screens::char_create_component::{RANDOM_NAME_BUTTON, char_create_screen};
use ui_toolkit::screen::SharedContext;

#[test]
fn clicked_name_button_changes_draft_and_editbox_without_changing_customization() {
    let catalog =
        name_catalog::NameCatalog::load(std::path::Path::new("data/NameGen.csv")).unwrap();
    let mut state = CharCreateState {
        mode: CharCreateMode::Customize,
        selected_race: 1,
        selected_sex: 0,
        selected_category: 17,
        open_dropdown: Some(31),
        name: "Previous".to_string(),
        ..Default::default()
    };
    let db = CustomizationDb::load(std::path::Path::new("data"));
    let mut view = build_ui_state(&state, &db);
    view.random_name_available = true;
    let mut shared = SharedContext::new();
    shared.insert(view);
    let mut registry = FrameRegistry::new(1920.0, 1080.0);
    let mut screen = Screen::new(char_create_screen);
    screen.sync(&shared, &mut registry);
    let button = registry
        .get(registry.get_by_name(RANDOM_NAME_BUTTON.0).unwrap())
        .unwrap();
    let action = CharCreateAction::parse(button.onclick.as_deref().unwrap());
    assert_eq!(action, Some(CharCreateAction::RandomizeName));
    let original_appearance = state.appearance.clone();

    input::apply_random_name_with_seed(&mut state, &mut registry, &catalog, 123);

    assert_ne!(state.name, "Previous");
    assert_eq!(
        get_editbox_text(
            &registry,
            registry.get_by_name(CREATE_NAME_INPUT.0).unwrap()
        ),
        state.name
    );
    assert_eq!(state.appearance, original_appearance);
    assert_eq!(state.selected_category, 17);
    assert_eq!(state.open_dropdown, Some(31));
    assert_eq!(state.mode, CharCreateMode::Customize);
    state.mode = CharCreateMode::RaceClass;
    state.mode = CharCreateMode::Customize;
    assert_eq!(
        state.name,
        get_editbox_text(
            &registry,
            registry.get_by_name(CREATE_NAME_INPUT.0).unwrap()
        )
    );
}

#[test]
fn automation_click_updates_live_editbox_without_sending_create_request() {
    let mut world = World::new();
    let db = CustomizationDb::load(std::path::Path::new("data"));
    let state = CharCreateState {
        mode: CharCreateMode::Customize,
        selected_category: 3,
        open_dropdown: Some(8789),
        ..Default::default()
    };
    let mut view = build_ui_state(&state, &db);
    view.random_name_available = true;
    let mut shared = SharedContext::new();
    shared.insert(view);
    let mut registry = FrameRegistry::new(1920.0, 1080.0);
    let mut screen = Screen::new(char_create_screen);
    screen.sync(&shared, &mut registry);
    let cc = CharCreateUi::resolve(&registry);
    world.insert_resource(CharCreateScreenWrap(CharCreateScreenRes { screen, shared }));
    world.insert_resource(UiState {
        registry,
        event_bus: EventBus::new(),
        focused_frame: None,
    });
    world.insert_resource(state);
    world.insert_resource(cc);
    world.insert_resource(db);
    world.insert_resource(NameCatalogResource(NameCatalog::load(
        std::path::Path::new("data/NameGen.csv"),
    )));
    world.init_resource::<CharCreateFocus>();
    world.init_resource::<UiAutomationQueue>();
    world.init_resource::<UiAutomationRunner>();
    world.init_resource::<NextState<GameState>>();
    let (sender, requests) = std::sync::mpsc::channel();
    world.insert_resource(ConnectionSender::new(Some(sender)));

    let mut previous = String::new();
    for _ in 0..2 {
        world
            .resource_mut::<UiAutomationQueue>()
            .push(UiAutomationAction::ClickFrame(
                RANDOM_NAME_BUTTON.0.to_owned(),
            ));
        world
            .run_system_once(input::char_create_run_automation)
            .unwrap();
        assert!(world.resource::<UiAutomationRunner>().last_error.is_none());
        let current = &world.resource::<CharCreateState>().name;
        assert_ne!(current, &previous);
        previous = current.clone();
        let ui = world.resource::<UiState>();
        let input = ui.registry.get_by_name(CREATE_NAME_INPUT.0).unwrap();
        assert_eq!(&get_editbox_text(&ui.registry, input), current);
        assert_eq!(
            world.resource::<CharCreateState>().open_dropdown,
            Some(8789)
        );
    }
    assert!(requests.try_recv().is_err());
}

#[test]
fn unsupported_race_does_not_replace_existing_name() {
    let catalog =
        name_catalog::NameCatalog::load(std::path::Path::new("data/NameGen.csv")).unwrap();
    let mut registry = FrameRegistry::new(1920.0, 1080.0);
    let mut state = CharCreateState {
        selected_race: 99,
        name: "Previous".to_owned(),
        ..Default::default()
    };
    input::apply_random_name_with_seed(&mut state, &mut registry, &catalog, 123);
    assert_eq!(state.name, "Previous");
    assert!(state.error_text.as_deref().unwrap().contains("No authored"));
}
