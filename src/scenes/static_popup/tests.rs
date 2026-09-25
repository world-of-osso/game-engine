#[path = "../../ui/screens/menu_character_layout_test_support.rs"]
mod layout_support;

use std::time::Duration;

use bevy::time::TimeUpdateStrategy;
use game_engine::ui::popup::{PopupId, PopupOutcome, PopupSpec};

use super::*;

fn popup_app() -> App {
    let mut app = App::new();
    app.add_plugins((MinimalPlugins, bevy::state::app::StatesPlugin));
    app.insert_resource(ButtonInput::<MouseButton>::default());
    app.insert_resource(ButtonInput::<KeyCode>::default());
    app.insert_resource(TimeUpdateStrategy::ManualDuration(Duration::from_millis(
        250,
    )));
    app.init_state::<GameState>();
    app.insert_state(GameState::InWorld);
    app.insert_resource(UiState {
        registry: game_engine::ui::registry::FrameRegistry::new(1920.0, 1080.0),
        event_bus: game_engine::ui::event::EventBus::new(),
        focused_frame: None,
    });
    app.add_plugins(StaticPopupPlugin);
    app.world_mut().spawn((
        Window {
            resolution: (1920, 1080).into(),
            ..default()
        },
        PrimaryWindow,
    ));
    app.update();
    app
}

fn spec(key: &str) -> PopupSpec {
    PopupSpec {
        key: key.to_string(),
        text: format!("Accept {key}?"),
        accept_label: "Accept".to_string(),
        cancel_label: Some("Decline".to_string()),
        timeout: None,
    }
}

fn push(app: &mut App, spec: PopupSpec) -> PopupId {
    let id = app.world_mut().resource_mut::<PopupStack>().push(spec);
    app.update();
    id
}

fn results(app: &mut App) -> Vec<PopupResult> {
    app.world_mut()
        .resource_mut::<Messages<PopupResult>>()
        .drain()
        .collect()
}

fn frame_text(app: &App, name: &str) -> Option<String> {
    let ui = app.world().resource::<UiState>();
    let id = ui.registry.get_by_name(name)?;
    match &ui.registry.get(id)?.widget_data {
        Some(game_engine::ui::frame::WidgetData::FontString(fs)) => Some(fs.text.clone()),
        _ => None,
    }
}

fn frame_exists(app: &App, name: &str) -> bool {
    app.world()
        .resource::<UiState>()
        .registry
        .get_by_name(name)
        .is_some()
}

fn press_key(app: &mut App, key: KeyCode) {
    app.world_mut()
        .resource_mut::<ButtonInput<KeyCode>>()
        .press(key);
    app.update();
    let mut keys = app.world_mut().resource_mut::<ButtonInput<KeyCode>>();
    keys.release(key);
    keys.clear();
}

fn click_frame(app: &mut App, name: &str) {
    let center = {
        let mut ui = app.world_mut().resource_mut::<UiState>();
        layout_support::compute_layout(&mut ui.registry);
        let id = ui.registry.get_by_name(name).expect(name);
        let rect = ui
            .registry
            .get(id)
            .and_then(|frame| frame.layout_rect.clone())
            .expect("frame layout");
        Vec2::new(rect.x + rect.width / 2.0, rect.y + rect.height / 2.0)
    };
    let mut windows = app
        .world_mut()
        .query_filtered::<&mut Window, With<PrimaryWindow>>();
    windows
        .single_mut(app.world_mut())
        .expect("window")
        .set_cursor_position(Some(center));
    app.world_mut()
        .resource_mut::<ButtonInput<MouseButton>>()
        .press(MouseButton::Left);
    app.update();
    let mut buttons = app.world_mut().resource_mut::<ButtonInput<MouseButton>>();
    buttons.release(MouseButton::Left);
    buttons.clear();
}

#[test]
fn four_pushes_show_three_frames_in_push_order() {
    let mut app = popup_app();
    for key in ["invite", "duel", "rez", "ready"] {
        push(&mut app, spec(key));
    }

    assert_eq!(
        frame_text(&app, "StaticPopup1Text").as_deref(),
        Some("Accept invite?")
    );
    assert_eq!(
        frame_text(&app, "StaticPopup2Text").as_deref(),
        Some("Accept duel?")
    );
    assert_eq!(
        frame_text(&app, "StaticPopup3Text").as_deref(),
        Some("Accept rez?")
    );
    assert!(!frame_exists(&app, "StaticPopup4"));

    let mut ui = app.world_mut().resource_mut::<UiState>();
    layout_support::compute_layout(&mut ui.registry);
    let top = |name: &str| {
        let id = ui.registry.get_by_name(name).expect(name);
        ui.registry.get(id).unwrap().layout_rect.clone().unwrap()
    };
    let (first, second) = (top("StaticPopup1"), top("StaticPopup2"));
    assert_eq!(first.y, 135.0);
    assert_eq!(first.x + first.width / 2.0, 960.0);
    assert_eq!(second.y, first.y + first.height);
}

#[test]
fn same_key_replaces_instead_of_duplicating() {
    let mut app = popup_app();
    let first = push(&mut app, spec("invite"));
    let mut again = spec("invite");
    again.text = "Jaina invites you to a group.".to_string();
    let second = push(&mut app, again);

    assert_eq!(first, second);
    assert_eq!(
        frame_text(&app, "StaticPopup1Text").as_deref(),
        Some("Jaina invites you to a group.")
    );
    assert!(!frame_exists(&app, "StaticPopup2"));
}

#[test]
fn clicking_accept_button_reports_accepted_and_closes() {
    let mut app = popup_app();
    push(&mut app, spec("invite"));
    let duel = push(&mut app, spec("duel"));

    click_frame(&mut app, "StaticPopup2Button1");

    assert_eq!(
        results(&mut app),
        vec![PopupResult {
            id: duel,
            key: "duel".to_string(),
            outcome: PopupOutcome::Accepted,
        }]
    );
    assert!(!frame_exists(&app, "StaticPopup2"));
    assert!(frame_exists(&app, "StaticPopup1"));
}

#[test]
fn clicking_cancel_button_reports_cancelled() {
    let mut app = popup_app();
    let invite = push(&mut app, spec("invite"));

    click_frame(&mut app, "StaticPopup1Button2");

    assert_eq!(
        results(&mut app),
        vec![PopupResult {
            id: invite,
            key: "invite".to_string(),
            outcome: PopupOutcome::Cancelled,
        }]
    );
    assert!(!frame_exists(&app, "StaticPopup1"));
}

#[test]
fn cancel_top_cancels_newest_visible_popup() {
    let mut app = popup_app();
    push(&mut app, spec("invite"));
    let duel = push(&mut app, spec("duel"));

    app.world_mut().resource_mut::<PopupStack>().cancel_top();
    app.update();

    assert_eq!(
        results(&mut app),
        vec![PopupResult {
            id: duel,
            key: "duel".to_string(),
            outcome: PopupOutcome::Cancelled,
        }]
    );
    assert!(app.world().resource::<PopupStack>().is_open());
    assert!(!frame_exists(&app, "StaticPopup2"));
}

#[test]
fn timeout_reports_timed_out_after_time_advances() {
    let mut app = popup_app();
    let mut rez = spec("rez");
    rez.timeout = Some(Duration::from_secs(1));
    let id = push(&mut app, rez);

    // Each update advances 250 ms (the virtual-time max delta).
    app.update();
    app.update();
    assert!(results(&mut app).is_empty());
    assert!(frame_exists(&app, "StaticPopup1"));

    app.update();
    app.update();
    assert_eq!(
        results(&mut app),
        vec![PopupResult {
            id,
            key: "rez".to_string(),
            outcome: PopupOutcome::TimedOut,
        }]
    );
    assert!(!frame_exists(&app, "StaticPopup1"));
}

#[test]
fn enter_accepts_only_the_top_popup() {
    let mut app = popup_app();
    push(&mut app, spec("invite"));
    let duel = push(&mut app, spec("duel"));

    press_key(&mut app, KeyCode::Enter);

    assert_eq!(
        results(&mut app),
        vec![PopupResult {
            id: duel,
            key: "duel".to_string(),
            outcome: PopupOutcome::Accepted,
        }]
    );
    assert_eq!(
        frame_text(&app, "StaticPopup1Text").as_deref(),
        Some("Accept invite?")
    );
    assert!(!frame_exists(&app, "StaticPopup2"));
}

#[test]
fn enter_is_ignored_while_an_editbox_has_focus() {
    let mut app = popup_app();
    push(&mut app, spec("invite"));
    let focused = {
        let ui = app.world().resource::<UiState>();
        ui.registry.get_by_name("StaticPopup1").expect("popup")
    };
    app.world_mut().resource_mut::<UiState>().focused_frame = Some(focused);

    press_key(&mut app, KeyCode::Enter);

    assert!(results(&mut app).is_empty());
    assert!(frame_exists(&app, "StaticPopup1"));
}

#[test]
fn clicks_hit_the_button_under_a_scaled_ui() {
    // A 1280-wide window over the 1920-wide UI (ui_scale 2/3): the window cursor at
    // the button's scaled centre must resolve to that button.
    let mut app = popup_app();
    let duel = push(&mut app, spec("duel"));
    let center = {
        let mut ui = app.world_mut().resource_mut::<UiState>();
        layout_support::compute_layout(&mut ui.registry);
        let id = ui.registry.get_by_name("StaticPopup1Button1").unwrap();
        let rect = ui.registry.get(id).unwrap().layout_rect.clone().unwrap();
        ui.registry.ui_scale = 2.0 / 3.0;
        Vec2::new(rect.x + rect.width / 2.0, rect.y + rect.height / 2.0) * (2.0 / 3.0)
    };
    let mut windows = app
        .world_mut()
        .query_filtered::<&mut Window, With<PrimaryWindow>>();
    windows
        .single_mut(app.world_mut())
        .unwrap()
        .set_cursor_position(Some(center));
    app.world_mut()
        .resource_mut::<ButtonInput<MouseButton>>()
        .press(MouseButton::Left);
    app.update();
    assert_eq!(
        results(&mut app),
        vec![PopupResult {
            id: duel,
            key: "duel".to_string(),
            outcome: PopupOutcome::Accepted,
        }]
    );
}
