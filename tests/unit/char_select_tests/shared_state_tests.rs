use super::*;
use bevy::ecs::system::SystemState;
use ui_toolkit::screen::SharedContext;

struct Fixture {
    world: World,
    registry: FrameRegistry,
    characters: CharacterList,
    selected: SelectedCharIndex,
    campsite: CampsitePanelVisible,
    focus: CharSelectFocus,
    delete: DeleteCharacterConfirmationState,
}

impl Fixture {
    fn new() -> Self {
        let mut world = World::new();
        world.insert_resource(fresh_screen());
        Self {
            world,
            registry: test_registry(),
            characters: CharacterList(vec![character(1, "Elara"), character(2, "Theron")]),
            selected: SelectedCharIndex(Some(0)),
            campsite: CampsitePanelVisible(false),
            focus: CharSelectFocus(None),
            delete: DeleteCharacterConfirmationState::default(),
        }
    }

    fn sync(&mut self, with_post_setup: bool) {
        let ui = with_post_setup.then(|| CharSelectUi::resolve(&self.registry));
        let mut state = SystemState::<Option<ResMut<CharSelectScreenWrap>>>::new(&mut self.world);
        let mut screen = state.get_mut(&mut self.world);
        sync_screen_state(
            &mut screen,
            &mut self.registry,
            ui.as_ref(),
            &self.characters,
            &self.selected,
            &self.campsite,
            &mut self.focus,
            &self.delete,
        );
    }

    fn shared(&self) -> &SharedContext {
        &self.world.resource::<CharSelectScreenWrap>().0.shared
    }

    fn generations(&self) -> [u64; 3] {
        [
            self.shared().generation::<CharSelectState>(),
            self.shared().generation::<CampsiteState>(),
            self.shared().generation::<DeleteConfirmUiState>(),
        ]
    }

    fn text(&self, name: &str) -> &str {
        let id = self.registry.get_by_name(name).expect("named text frame");
        let Some(WidgetData::FontString(text)) = &self.registry.get(id).unwrap().widget_data else {
            panic!("{name} is not a font string")
        };
        &text.text
    }

    fn assert_stable(&mut self) {
        let generations = self.generations();
        let ids = self
            .world
            .resource::<CharSelectScreenWrap>()
            .0
            .screen
            .all_frame_ids()
            .to_vec();
        self.registry.render_dirty.clear();
        self.sync(false);
        assert_eq!(self.generations(), generations);
        assert!(self.registry.render_dirty.is_empty());
        assert_eq!(
            self.world
                .resource::<CharSelectScreenWrap>()
                .0
                .screen
                .all_frame_ids(),
            ids
        );
    }
}

fn fresh_screen() -> CharSelectScreenWrap {
    CharSelectScreenWrap(CharSelectScreenRes {
        screen: Screen::new(char_select_screen),
        shared: SharedContext::new(),
    })
}

fn character(character_id: u64, name: &str) -> CharacterListEntry {
    CharacterListEntry {
        character_id,
        name: name.into(),
        level: 10,
        race: 1,
        class: 1,
        appearance: shared::components::CharacterAppearance::default(),
        equipment_appearance: shared::components::EquipmentAppearance::default(),
    }
}

#[test]
fn charselect_shared_stable_inputs_and_replacement_context() {
    let mut fixture = Fixture::new();
    fixture.sync(false);
    assert_eq!(fixture.generations(), [1, 1, 1]);
    assert_eq!(fixture.text(SELECTED_NAME_TEXT.0), "Elara");
    fixture.assert_stable();
    fixture.assert_stable();

    fixture
        .world
        .resource_mut::<CharSelectScreenWrap>()
        .0
        .screen
        .teardown(&mut fixture.registry);
    fixture.world.insert_resource(fresh_screen());
    fixture.sync(false);
    assert_eq!(fixture.generations(), [1, 1, 1]);
    assert_eq!(fixture.text(SELECTED_NAME_TEXT.0), "Elara");
    fixture.assert_stable();
}

#[test]
fn charselect_shared_selection_list_and_campsite_changes_are_independent() {
    let mut fixture = Fixture::new();
    fixture.sync(false);
    fixture.selected.0 = Some(1);
    fixture.sync(false);
    assert_eq!(fixture.generations(), [2, 1, 1]);
    assert_eq!(fixture.text(SELECTED_NAME_TEXT.0), "Theron");
    fixture.assert_stable();

    fixture.characters.0[1].name = "Theron Updated".into();
    fixture.characters.0.push(character(3, "Mira"));
    fixture.sync(false);
    assert_eq!(fixture.generations(), [3, 1, 1]);
    assert_eq!(fixture.text(SELECTED_NAME_TEXT.0), "Theron Updated");
    assert_eq!(
        fixture
            .shared()
            .get::<CharSelectState>()
            .unwrap()
            .characters[2]
            .name,
        "Mira"
    );
    fixture.assert_stable();

    fixture.campsite.0 = true;
    fixture.sync(false);
    assert_eq!(fixture.generations(), [3, 2, 1]);
    assert!(
        fixture
            .shared()
            .get::<CampsiteState>()
            .unwrap()
            .panel_visible
    );
    fixture.assert_stable();
    fixture.campsite.0 = false;
    fixture.sync(false);
    assert_eq!(fixture.generations(), [3, 3, 1]);
    assert!(
        !fixture
            .shared()
            .get::<CampsiteState>()
            .unwrap()
            .panel_visible
    );
}

#[test]
fn charselect_shared_delete_changes_preserve_focus_and_post_setup() {
    let mut fixture = Fixture::new();
    fixture.sync(false);
    fixture.delete.target = Some(DeleteCharacterTarget {
        character_id: 1,
        name: "Elara".into(),
    });
    fixture.sync(true);
    assert_eq!(fixture.generations(), [1, 1, 2]);
    assert_eq!(
        fixture.focus.0,
        fixture.registry.get_by_name(DELETE_CONFIRM_INPUT.0)
    );
    assert!(fixture.focus.0.is_some());
    assert_eq!(
        fixture
            .shared()
            .get::<DeleteConfirmUiState>()
            .unwrap()
            .character_name,
        "Elara"
    );
    fixture.assert_stable();

    fixture.delete.typed_text = "DELETE".into();
    fixture.delete.elapsed_secs = 3.0;
    fixture.sync(true);
    assert_eq!(fixture.generations(), [1, 1, 3]);
    assert!(
        fixture
            .shared()
            .get::<DeleteConfirmUiState>()
            .unwrap()
            .confirm_enabled
    );
    let button_id = fixture
        .registry
        .get_by_name(DELETE_CONFIRM_BUTTON.0)
        .unwrap();
    let Some(WidgetData::Button(button)) = &fixture.registry.get(button_id).unwrap().widget_data
    else {
        panic!("delete confirmation button missing")
    };
    assert_ne!(button.state, ButtonState::Disabled);
    fixture.assert_stable();

    fixture.registry.screen_width = 1600.0;
    fixture.sync(true);
    assert_eq!(fixture.generations(), [1, 1, 3]);
    let root = fixture.registry.get_by_name(CHAR_SELECT_ROOT.0).unwrap();
    assert_eq!(
        fixture.registry.get(root).unwrap().width,
        Dimension::Fixed(1600.0)
    );
    fixture.delete.target = None;
    fixture.sync(false);
    assert_eq!(fixture.generations(), [1, 1, 4]);
    assert!(
        !fixture
            .shared()
            .get::<DeleteConfirmUiState>()
            .unwrap()
            .visible
    );
    fixture.assert_stable();
}
