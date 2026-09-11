use super::*;
use bevy::ecs::system::SystemState;
use ui_toolkit::screen::SharedContext;

struct Fixture {
    world: World,
    registry: FrameRegistry,
}

impl Fixture {
    fn new() -> Self {
        let mut world = World::new();
        world.insert_resource(LoginScreenResWrap(build_login_screen(
            &LoginStatus::default(),
            "Development".into(),
            true,
        )));
        Self {
            world,
            registry: FrameRegistry::new(1920.0, 1080.0),
        }
    }

    fn sync(&mut self, status: &str, realm: &str, selectable: bool) {
        let mut state = SystemState::<ResMut<LoginScreenResWrap>>::new(&mut self.world);
        let mut screen = state
            .get_mut(&mut self.world)
            .expect("login screen resource");
        sync_login_status(
            &mut self.registry,
            Some(&mut screen),
            &LoginStatus(status.into()),
            realm.into(),
            selectable,
        );
    }

    fn generations(&self) -> [u64; 4] {
        let shared = &self.world.resource::<LoginScreenResWrap>().0.shared;
        [
            shared.generation::<SharedStatusText>(),
            shared.generation::<SharedConnecting>(),
            shared.generation::<SharedRealmText>(),
            shared.generation::<SharedRealmSelectable>(),
        ]
    }

    fn assert_display(&self, status: &str, connecting: bool) {
        let status_id = self.registry.get_by_name(LOGIN_STATUS.0).unwrap();
        let Some(WidgetData::FontString(text)) = &self.registry.get(status_id).unwrap().widget_data
        else {
            panic!("status must be a font string")
        };
        assert_eq!(text.text, status);
        let button_id = self.registry.get_by_name(CONNECT_BUTTON.0).unwrap();
        let Some(WidgetData::Button(button)) = &self.registry.get(button_id).unwrap().widget_data
        else {
            panic!("connect must be a button")
        };
        assert_eq!(
            button.state == ui_toolkit::widgets::button::ButtonState::Disabled,
            connecting
        );
    }
}

#[test]
fn login_shared_unchanged_values_preserve_generations_and_frames() {
    let mut fixture = Fixture::new();
    fixture.sync("Ready", "Development", true);
    let generations = fixture.generations();
    let ids = fixture
        .world
        .resource::<LoginScreenResWrap>()
        .0
        .screen
        .all_frame_ids()
        .to_vec();
    fixture.registry.render_dirty.clear();
    fixture.sync("Ready", "Development", true);
    assert_eq!(fixture.generations(), generations);
    assert_eq!(
        fixture
            .world
            .resource::<LoginScreenResWrap>()
            .0
            .screen
            .all_frame_ids(),
        ids
    );
    assert!(fixture.registry.render_dirty.is_empty());
    fixture.assert_display("Ready", false);
}

#[test]
fn login_shared_changes_update_only_affected_generations() {
    let mut fixture = Fixture::new();
    fixture.sync("Ready", "Development", true);
    let initial = fixture.generations();
    fixture.sync("Invalid password", "Development", true);
    assert_eq!(
        fixture.generations(),
        [initial[0] + 1, initial[1], initial[2], initial[3]]
    );
    fixture.assert_display("Invalid password", false);
    fixture.sync(STATUS_CONNECTING, "Development", true);
    assert_eq!(
        fixture.generations(),
        [initial[0] + 2, initial[1] + 1, initial[2], initial[3]]
    );
    fixture.assert_display(STATUS_CONNECTING, true);
    fixture.sync(STATUS_CONNECTING, "Production", true);
    assert_eq!(
        fixture.generations(),
        [initial[0] + 2, initial[1] + 1, initial[2] + 1, initial[3]]
    );
    fixture.sync(STATUS_CONNECTING, "Production", false);
    assert_eq!(
        fixture.generations(),
        [
            initial[0] + 2,
            initial[1] + 1,
            initial[2] + 1,
            initial[3] + 1
        ]
    );
    let shared = &fixture.world.resource::<LoginScreenResWrap>().0.shared;
    assert_eq!(shared.get::<SharedRealmText>().unwrap().0, "Production");
    assert!(!shared.get::<SharedRealmSelectable>().unwrap().0);
}

#[test]
fn login_shared_initializes_empty_context_and_replacement_screen() {
    let mut fixture = Fixture::new();
    fixture
        .world
        .insert_resource(LoginScreenResWrap(LoginScreenRes {
            screen: Screen::new(login_screen),
            shared: SharedContext::new(),
        }));
    fixture.sync("Ready", "Development", true);
    assert_eq!(fixture.generations(), [1; 4]);
    fixture.assert_display("Ready", false);
    let generations = fixture.generations();
    fixture
        .world
        .resource_mut::<LoginScreenResWrap>()
        .0
        .screen
        .teardown(&mut fixture.registry);
    fixture.world.resource_mut::<LoginScreenResWrap>().0.screen = Screen::new(login_screen);
    assert!(fixture.registry.get_by_name(LOGIN_ROOT.0).is_none());
    fixture.sync("Ready", "Development", true);
    assert_eq!(fixture.generations(), generations);
    assert!(fixture.registry.get_by_name(LOGIN_ROOT.0).is_some());
    fixture.assert_display("Ready", false);
}
