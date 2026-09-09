use super::*;
use bevy::ecs::system::SystemState;
use game_engine::ui::frame::WidgetData;
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
    let mut fixture = Fixture::new();
    fixture.sync(false);
    fixture.state.mode = CharCreateMode::Customize;
    let previous = fixture.generation();
    fixture.sync(false);
    assert_eq!(fixture.generation(), previous + 1);
    let input = fixture.registry.get_by_name(CREATE_NAME_INPUT.0).unwrap();
    if let Some(WidgetData::EditBox(edit)) =
        &mut fixture.registry.get_mut(input).unwrap().widget_data
    {
        edit.text = "Theron".into();
    }
    fixture.assert_settled(false);
    assert_eq!(
        helpers::get_editbox_text(&fixture.registry, input),
        "Theron"
    );

    fixture.state.appearance.face = 2;
    fixture.state.appearance.hair_style = 3;
    fixture.state.error_text = Some("Name unavailable".into());
    let previous = fixture.generation();
    fixture.sync(true);
    assert_eq!(fixture.generation(), previous + 1);
    assert_eq!(fixture.shared().face, 2);
    assert_eq!(fixture.shared().hair_style, 3);
    assert!(fixture.shared().name_input_focused);
    assert_eq!(fixture.text(ERROR_TEXT.0), "Name unavailable");
    fixture.assert_settled(true);
    let previous = fixture.generation();
    fixture.sync(false);
    assert_eq!(fixture.generation(), previous + 1);
    assert!(!fixture.shared().name_input_focused);
    fixture.assert_settled(false);
}

#[test]
fn char_create_shared_compares_labels_swatches_and_name() {
    let mut fixture = Fixture::new();
    fixture.state.mode = CharCreateMode::Customize;
    fixture.sync(false);
    // Each stored field differs independently from the next complete built state.
    let changes: [fn(&mut CharCreateUiState); 6] = [
        |state| state.name = "Old name".into(),
        |state| state.face_label = "Old face".into(),
        |state| state.hair_style_label = "Old hair".into(),
        |state| state.facial_style_label = "Old facial style".into(),
        |state| state.skin_color_swatches = vec![Some([64, 32, 16])],
        |state| state.hair_color_swatches = vec![Some([96, 48, 24])],
    ];
    for change in changes {
        let mut previous_state = build_ui_state(&fixture.state, &fixture.db);
        change(&mut previous_state);
        fixture
            .world
            .resource_mut::<CharCreateScreenWrap>()
            .0
            .shared
            .insert(previous_state);
        let previous = fixture.generation();
        fixture.sync(false);
        assert_eq!(fixture.generation(), previous + 1);
        assert!(fixture.shared().name.is_empty());
        assert!(fixture.shared().face_label.is_empty());
        assert!(fixture.shared().hair_style_label.is_empty());
        assert!(fixture.shared().facial_style_label.is_empty());
        assert!(fixture.shared().skin_color_swatches.is_empty());
        assert!(fixture.shared().hair_color_swatches.is_empty());
        fixture.assert_settled(false);
    }
}
