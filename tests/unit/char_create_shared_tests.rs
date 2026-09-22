use super::*;
use bevy::ecs::system::{RunSystemOnce, SystemState};
use game_engine::ui::frame::WidgetData;
use game_engine::ui::screens::char_create_component::CustomizationOptionUi;
use std::path::Path;
use ui_toolkit::screen::SharedContext;

struct Fixture {
    world: World,
    registry: FrameRegistry,
    state: CharCreateState,
    db: CustomizationDb,
}

impl Fixture {
    fn new() -> Self {
        let mut world = World::new();
        world.insert_resource(CharCreateScreenWrap(CharCreateScreenRes {
            screen: Screen::new(char_create_screen),
            shared: SharedContext::new(),
        }));
        Self {
            world,
            registry: FrameRegistry::new(1920.0, 1080.0),
            state: CharCreateState::default(),
            db: CustomizationDb::default(),
        }
    }

    fn with_catalog(race: u8, sex: u8, class: u8) -> Self {
        let mut fixture = Self::new();
        fixture.db = CustomizationDb::try_load(Path::new("data"))
            .expect("local customization catalog must load");
        fixture.state.selected_race = race;
        fixture.state.selected_sex = sex;
        fixture.state.selected_class = class;
        fixture.state.appearance.sex = sex;
        fixture
    }

    fn show_option(&mut self, option_id: u32) {
        self.state.mode = CharCreateMode::Customize;
        self.state.selected_category = self
            .db
            .option_by_id(self.state.selected_race, self.state.selected_sex, option_id)
            .expect("authored option must exist")
            .category_id;
    }

    fn sync(&mut self, focused: bool) {
        let mut system = SystemState::<Option<ResMut<CharCreateScreenWrap>>>::new(&mut self.world);
        let mut screen = system.get_mut(&mut self.world).unwrap();
        sync_screen_state(
            &mut screen,
            &mut self.registry,
            &self.state,
            &self.db,
            focused,
        );
    }

    fn generation(&self) -> u64 {
        self.world
            .resource::<CharCreateScreenWrap>()
            .0
            .shared
            .generation::<CharCreateUiState>()
    }

    fn shared(&self) -> &CharCreateUiState {
        self.world
            .resource::<CharCreateScreenWrap>()
            .0
            .shared
            .get::<CharCreateUiState>()
            .unwrap()
    }

    fn option(&self, id: u32) -> &CustomizationOptionUi {
        self.shared()
            .options
            .iter()
            .find(|option| option.id == id)
            .expect("option must reach the current view")
    }

    fn text(&self, name: &str) -> &str {
        let id = self
            .registry
            .get_by_name(name)
            .expect("expected text frame");
        match &self.registry.get(id).unwrap().widget_data {
            Some(WidgetData::FontString(text)) => &text.text,
            _ => panic!("expected font string: {name}"),
        }
    }

    fn assert_settled(&mut self, focused: bool) {
        let generation = self.generation();
        self.registry.render_dirty.clear();
        self.sync(focused);
        assert_eq!(self.generation(), generation);
        assert!(self.registry.render_dirty.is_empty());
    }
}

#[test]
fn char_create_shared_stable_state_and_fresh_context() {
    let mut fixture = Fixture::new();
    assert_eq!(fixture.generation(), 0);
    fixture.sync(false);
    assert!(fixture.generation() > 0);
    assert!(fixture.registry.get_by_name(CHAR_CREATE_ROOT.0).is_some());
    fixture.assert_settled(false);

    fixture
        .world
        .insert_resource(CharCreateScreenWrap(CharCreateScreenRes {
            screen: Screen::new(char_create_screen),
            shared: SharedContext::new(),
        }));
    fixture.registry = FrameRegistry::new(1920.0, 1080.0);
    fixture.sync(false);
    assert!(fixture.registry.get_by_name(CHAR_CREATE_ROOT.0).is_some());
    fixture.assert_settled(false);
}

#[test]
fn char_create_shared_mode_appearance_error_and_focus_changes_propagate() {
    let mut fixture = Fixture::with_catalog(1, 0, 1);
    fixture.sync(false);
    fixture.show_option(10); // Human Face; Hair Style shares the authored category.
    let previous = fixture.generation();
    fixture.sync(false);
    assert_eq!(fixture.generation(), previous + 1);
    let input = fixture.registry.get_by_name(CREATE_NAME_INPUT.0).unwrap();
    helpers::insert_char_into_editbox(&mut fixture.registry, input, "Theron");
    fixture.sync(false);
    assert_eq!(fixture.shared().name, "Theron");
    fixture.assert_settled(false);

    fixture.state.appearance.face = 2;
    fixture.state.appearance.hair_style = 3;
    fixture.state.error_text = Some("Name unavailable".into());
    let previous = fixture.generation();
    fixture.sync(true);
    assert_eq!(fixture.generation(), previous + 1);
    assert_eq!(fixture.option(10).selected_choice_id, 22);
    assert_eq!(fixture.option(11).selected_choice_id, 47);
    assert_eq!(fixture.text("OptionValue_10_Text"), "3");
    assert_eq!(fixture.text("OptionValue_11_Text"), "Monk");
    assert!(fixture.shared().name_input_focused);
    assert_eq!(fixture.text(ERROR_TEXT.0), "Name unavailable");
    assert_eq!(
        helpers::get_editbox_text(&fixture.registry, input),
        "Theron"
    );
    fixture.assert_settled(true);
    let previous = fixture.generation();
    fixture.sync(false);
    assert_eq!(fixture.generation(), previous + 1);
    assert!(!fixture.shared().name_input_focused);
    fixture.assert_settled(false);
}

#[test]
fn char_create_shared_compares_category_labels_choices_and_swatches() {
    let mut fixture = Fixture::with_catalog(1, 0, 1);
    fixture.show_option(463); // Eye color: real authored palette choices.
    fixture.sync(false);
    let expected = fixture.shared().clone();
    assert!(!expected.categories.is_empty());
    assert!(!expected.options[0].choices.is_empty());
    let changes: [fn(&mut CharCreateUiState); 7] = [
        |state| state.categories[0].label = "Old category".into(),
        |state| state.options[0].label = "Old option".into(),
        |state| state.options[0].choices[0].label = "Old choice".into(),
        |state| state.options[0].choices[0].swatch = Some([64, 32, 16]),
        |state| state.options[0].choices[0].secondary_swatch = Some([96, 48, 24]),
        |state| state.options[0].choices[0].enabled = !state.options[0].choices[0].enabled,
        |state| state.options[0].enabled = !state.options[0].enabled,
    ];
    for change in changes {
        let mut stale = expected.clone();
        change(&mut stale);
        {
            let mut screen = fixture.world.resource_mut::<CharCreateScreenWrap>();
            screen.0.shared.insert(stale);
            let inner = &mut screen.0;
            inner.screen.sync(&inner.shared, &mut fixture.registry);
        }
        let previous = fixture.generation();
        fixture.sync(false);
        assert_eq!(fixture.generation(), previous + 1);
        assert_eq!(fixture.shared(), &expected);
        assert_eq!(fixture.text("OptionLabel_463"), "Eye Color");
        fixture.assert_settled(false);
    }
}

#[test]
fn char_create_shared_name_survives_category_and_dropdown_rebuilds() {
    let mut fixture = Fixture::with_catalog(1, 0, 1);
    fixture.show_option(10);
    fixture.sync(false);
    let input = fixture.registry.get_by_name(CREATE_NAME_INPUT.0).unwrap();
    helpers::insert_char_into_editbox(&mut fixture.registry, input, "Théron");
    fixture.sync(true);

    for option_id in [8789, 890, 463, 10] {
        fixture.show_option(option_id);
        fixture.state.open_dropdown = Some(option_id);
        fixture.sync(true);
        assert_eq!(
            fixture.registry.get_by_name(CREATE_NAME_INPUT.0),
            Some(input)
        );
        assert_eq!(
            helpers::get_editbox_text(&fixture.registry, input),
            "Théron"
        );
        assert_eq!(fixture.shared().name, "Théron");
        assert!(fixture.shared().name_input_focused);
        assert!(
            fixture
                .registry
                .get_by_name(&format!("Option_{option_id}"))
                .is_some()
        );
        fixture.assert_settled(true);
    }
}

#[test]
fn char_create_shared_eye_ear_eyebrow_and_jewelry_choices_reach_registry_controls() {
    for (race, sex, option_id, label) in [
        (1, 0, 463, "Eye Color"),
        (1, 0, 8789, "Ears"),
        (1, 0, 890, "Eyebrows"),
        (10, 1, 776, "Jewelry Color"),
    ] {
        let mut fixture = Fixture::with_catalog(race, sex, 1);
        fixture.show_option(option_id);
        let choice_id = fixture
            .db
            .choices_for_option(race, sex, 1, option_id)
            .into_iter()
            .rfind(|choice| !choice.has_unsupported_effects)
            .expect("fixture must offer a supported choice")
            .id;
        game_engine::appearance_options::set_choice(
            &fixture.db,
            race,
            sex,
            1,
            &mut fixture.state.appearance,
            option_id,
            choice_id,
        )
        .expect("supported choice must be selectable");
        fixture.state.open_dropdown = Some(option_id);
        fixture.sync(false);
        assert_eq!(fixture.option(option_id).selected_choice_id, choice_id);
        assert_eq!(fixture.text(&format!("OptionLabel_{option_id}")), label);
        let name = format!("OptionChoice_{option_id}_{choice_id}");
        let id = fixture
            .registry
            .get_by_name(&name)
            .expect("choice must be clickable in popup");
        assert_eq!(
            fixture.registry.get(id).unwrap().onclick.as_deref(),
            Some(format!("select_option_choice:{option_id}:{choice_id}").as_str())
        );
        fixture.assert_settled(false);
    }
}

fn run_creation_action(world: &mut World, action: UiAutomationAction) {
    world.resource_mut::<UiAutomationQueue>().push(action);
    world
        .run_system_once(input::char_create_run_automation)
        .expect("creation input system");
    assert!(world.resource::<UiAutomationRunner>().last_error.is_none());
    world
        .run_system_once(char_create_update_visuals)
        .expect("creation view update");
}

fn loopback_creation_request(
    command: game_engine::network_runtime::worker::NetworkCommand,
) -> CreateCharacter {
    use lightyear::prelude::client::ClientPlugins;
    use lightyear::prelude::{
        ChannelRegistry, Connected, Link, Linked, MessageReceiver, MessageSender, PeerId, RemoteId,
        Transport,
    };
    let mut worker = App::new();
    worker.add_plugins(bevy::state::app::StatesPlugin);
    worker.add_plugins(ClientPlugins::default());
    worker.add_plugins(shared::ProtocolPlugin);
    worker.finish();
    worker.cleanup();
    let registry = worker.world().resource::<ChannelRegistry>();
    let mut transport = Transport::default();
    transport.add_sender_from_registry::<AuthChannel>(registry);
    transport.add_receiver_from_registry::<AuthChannel>(registry);
    let peer = worker
        .world_mut()
        .spawn((
            Link::default(),
            transport,
            Linked,
            Connected,
            RemoteId(PeerId::Local(0)),
            MessageSender::<CreateCharacter>::default(),
            MessageReceiver::<CreateCharacter>::default(),
        ))
        .id();
    let game_engine::network_runtime::worker::NetworkCommand::Apply(apply) = command else {
        panic!("expected character request, not worker shutdown");
    };
    apply(worker.world_mut());
    worker.world_mut().run_schedule(PostUpdate);
    {
        let mut entity = worker.world_mut().entity_mut(peer);
        let mut link = entity.get_mut::<Link>().unwrap();
        let packets: Vec<_> = link.send.drain().collect();
        assert!(
            !packets.is_empty(),
            "creation must serialize into actual transport packets"
        );
        for packet in packets {
            link.recv.push_raw(packet);
        }
    }
    worker.world_mut().run_schedule(PreUpdate);
    let requests: Vec<_> = worker
        .world_mut()
        .entity_mut(peer)
        .get_mut::<MessageReceiver<CreateCharacter>>()
        .unwrap()
        .receive()
        .collect();
    assert_eq!(requests.len(), 1);
    requests.into_iter().next().unwrap()
}

#[test]
fn char_create_shared_request_uses_live_name_after_next_and_category_changes() {
    use game_engine::network_runtime::messages::ConnectionSender;
    use game_engine::ui::event::EventBus;
    use shared::components::CustomizationChoiceSelection;
    let mut fixture = Fixture::with_catalog(1, 0, 1);
    fixture.sync(false);
    let startup_ui = CharCreateUi::resolve(&fixture.registry);
    assert!(
        startup_ui.name_input.is_none(),
        "RaceClass startup must reproduce missing cached name input"
    );
    let Fixture {
        mut world,
        registry,
        state,
        db,
    } = fixture;
    world.insert_resource(UiState {
        registry,
        event_bus: EventBus::new(),
        focused_frame: None,
    });
    world.insert_resource(state);
    world.insert_resource(db);
    world.insert_resource(startup_ui);
    world.init_resource::<CharCreateFocus>();
    world.init_resource::<UiAutomationQueue>();
    world.init_resource::<UiAutomationRunner>();
    world.init_resource::<NextState<GameState>>();
    let (sender, requests) = std::sync::mpsc::channel();
    world.insert_resource(ConnectionSender::new(Some(sender)));

    for action in [
        UiAutomationAction::ClickFrame(NEXT_BUTTON.0.to_owned()),
        UiAutomationAction::ClickFrame(CREATE_NAME_INPUT.0.to_owned()),
        UiAutomationAction::TypeText("Theron".to_owned()),
        UiAutomationAction::ClickFrame("Category_23".to_owned()),
        UiAutomationAction::ClickFrame("OptionToggle_8789".to_owned()),
        UiAutomationAction::ClickFrame("OptionChoice_8789_56652".to_owned()),
        UiAutomationAction::ClickFrame("Category_3".to_owned()),
    ] {
        run_creation_action(&mut world, action);
    }
    let expected = world.resource::<CharCreateState>().appearance.clone();
    assert!(
        expected
            .customization_choices
            .contains(&CustomizationChoiceSelection {
                option_id: 8789,
                choice_id: 56652
            })
    );
    assert!(
        world.resource::<CharCreateUi>().name_input.is_none(),
        "stale startup metadata must remain stale in this regression"
    );
    run_creation_action(
        &mut world,
        UiAutomationAction::ClickFrame(CREATE_BUTTON.0.to_owned()),
    );
    assert!(world.resource::<CharCreateState>().error_text.is_none());
    let command = requests
        .try_recv()
        .expect("valid live name must enqueue a creation request");
    assert!(
        requests.try_recv().is_err(),
        "one click must send one request"
    );
    let request = loopback_creation_request(command);
    assert_eq!(request.name, "Theron");
    assert_eq!((request.race, request.class), (1, 1));
    assert_eq!(request.appearance, expected);
}

fn authored_rgb(value: i32) -> Option<[u8; 3]> {
    let [_, red, green, blue] = value.to_be_bytes();
    (value != 0).then_some([red, green, blue])
}

#[test]
fn char_create_shared_catalog_choices_keep_filtered_ids_names_and_swatches_aligned() {
    let db = CustomizationDb::try_load(Path::new("data")).expect("local catalog");
    let mut filtered_options = 0;
    let mut colored_choices = 0;
    let mut unsupported_choices = 0;
    for (race, sex, class) in [(1, 0, 1), (2, 0, 1), (10, 0, 1), (10, 0, 12), (10, 1, 1)] {
        let mut state = CharCreateState {
            selected_race: race,
            selected_sex: sex,
            selected_class: class,
            mode: CharCreateMode::Customize,
            appearance: CharacterAppearance {
                sex,
                ..Default::default()
            },
            ..Default::default()
        };
        let source_options = db.options_for(race, sex).expect("fixture model options");
        assert!(!source_options.is_empty());
        for source in source_options {
            state.selected_category = source.category_id;
            let view = build_ui_state(&state, &db);
            let row = view
                .options
                .iter()
                .find(|option| option.id == source.id)
                .expect("every authored option must reach its category view");
            let category = view
                .categories
                .iter()
                .find(|category| category.id == source.category_id)
                .expect("authored category must be available");
            assert_eq!(category.label, source.category_name);
            assert_eq!(row.label, source.display_name);
            assert_eq!(row.ui_type, source.ui_type);
            let choices = db.choices_for_option(race, sex, class, source.id);
            filtered_options += usize::from(choices.len() != source.choices.len());
            assert_eq!(row.choices.len(), choices.len());
            for (index, (actual, expected)) in row.choices.iter().zip(&choices).enumerate() {
                assert_eq!(actual.id, expected.id);
                let label = if expected.display_name.is_empty() {
                    (index + 1).to_string()
                } else {
                    expected.display_name.clone()
                };
                assert_eq!(actual.label, label);
                assert_eq!(actual.swatch, authored_rgb(expected.swatch_colors[0]));
                assert_eq!(
                    actual.secondary_swatch,
                    authored_rgb(expected.swatch_colors[1])
                );
                colored_choices += usize::from(actual.swatch.is_some());
                if expected.has_unsupported_effects {
                    unsupported_choices += 1;
                    assert!(
                        !actual.enabled,
                        "unsupported effects must not become selectable"
                    );
                }
            }
            if choices.is_empty() || choices.iter().all(|choice| choice.has_unsupported_effects) {
                assert!(!row.enabled);
                assert!(
                    row.disabled_reason
                        .as_ref()
                        .is_some_and(|reason| !reason.is_empty())
                );
            }
        }
    }
    assert!(
        filtered_options > 0,
        "fixtures must exercise actual class filtering"
    );
    assert!(
        colored_choices > 0,
        "fixtures must exercise authored palettes"
    );
    assert!(
        unsupported_choices > 0,
        "fixtures must exercise unsupported effects"
    );
}
