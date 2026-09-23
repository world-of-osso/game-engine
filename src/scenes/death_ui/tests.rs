#[path = "../../ui/screens/menu_character_layout_test_support.rs"]
mod layout_support;

use std::sync::mpsc;

use game_engine::network_runtime::messages::ConnectionSender;
use game_engine::network_runtime::worker::NetworkCommand;
use game_engine::status::MapStatusSnapshot;
use lightyear::prelude::{Message as NetworkMessage, MessageSender as TransportSender};
use shared::protocol::{ReleaseSpirit, ResurrectAtCorpse};

use super::*;
use crate::scenes::static_popup::StaticPopupPlugin;

fn death_app() -> (App, mpsc::Receiver<NetworkCommand>) {
    let mut app = App::new();
    app.add_plugins((MinimalPlugins, bevy::state::app::StatesPlugin));
    app.insert_resource(ButtonInput::<MouseButton>::default());
    app.insert_resource(ButtonInput::<KeyCode>::default());
    app.init_state::<GameState>();
    app.insert_state(GameState::InWorld);
    app.insert_resource(UiState {
        registry: game_engine::ui::registry::FrameRegistry::new(1920.0, 1080.0),
        event_bus: game_engine::ui::event::EventBus::new(),
        focused_frame: None,
    });
    app.init_resource::<MapStatusSnapshot>();
    let (sender, commands) = mpsc::channel();
    app.insert_resource(ConnectionSender::new(Some(sender)));
    app.add_plugins((
        StaticPopupPlugin,
        game_engine::death::DeathPlugin,
        DeathUiPlugin,
    ));
    app.world_mut().spawn((
        Window {
            resolution: (1920, 1080).into(),
            ..default()
        },
        PrimaryWindow,
    ));
    app.world_mut()
        .spawn((LocalPlayer, Transform::from_xyz(0.0, 0.0, 0.0)));
    app.update();
    game_engine::network_events::dispatch_outgoing(app.world_mut());
    drain(&commands);
    (app, commands)
}

fn drain(commands: &mpsc::Receiver<NetworkCommand>) -> Vec<NetworkCommand> {
    commands.try_iter().collect()
}

fn corpse_at(x: f32, z: f32) -> DeathPositionEntry {
    DeathPositionEntry {
        map_id: 0,
        x,
        y: 0.0,
        z,
    }
}

fn set_death_state(app: &mut App, state: DeathStateEntry, corpse: Option<DeathPositionEntry>) {
    let mut snapshot = app.world_mut().resource_mut::<DeathStatusSnapshot>();
    snapshot.state = Some(state);
    snapshot.corpse = corpse;
    app.update();
}

fn move_player(app: &mut App, x: f32, z: f32) {
    let mut players = app
        .world_mut()
        .query_filtered::<&mut Transform, With<LocalPlayer>>();
    players.single_mut(app.world_mut()).unwrap().translation = Vec3::new(x, 0.0, z);
    app.update();
}

fn frame_text(app: &App, name: &str) -> Option<String> {
    let ui = app.world().resource::<UiState>();
    let id = ui.registry.get_by_name(name)?;
    match &ui.registry.get(id)?.widget_data {
        Some(game_engine::ui::frame::WidgetData::FontString(fs)) => Some(fs.text.clone()),
        Some(game_engine::ui::frame::WidgetData::Button(button)) => Some(button.text.clone()),
        _ => None,
    }
}

fn frame_visible(app: &App, name: &str) -> bool {
    let ui = app.world().resource::<UiState>();
    ui.registry
        .get_by_name(name)
        .and_then(|id| ui.registry.get(id))
        .is_some_and(|frame| !frame.hidden)
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
    // Popup results are read on the following frame.
    app.update();
}

/// Runs a queued network command against a worker world holding only an `M` sender; a
/// command for any other message type panics. Asserts `M` was buffered for transport.
fn assert_sent<M: NetworkMessage>(command: NetworkCommand) {
    let NetworkCommand::Apply(apply) = command else {
        panic!("expected a queued message, not worker shutdown");
    };
    let mut worker = World::new();
    let sender = worker.spawn(TransportSender::<M>::default()).id();
    worker.flush();
    worker.clear_trackers();
    apply(&mut worker);
    assert!(
        worker
            .entity(sender)
            .get_ref::<TransportSender<M>>()
            .unwrap()
            .is_changed(),
        "expected {} to be sent",
        std::any::type_name::<M>()
    );
}

fn popup_keys(app: &App) -> Vec<String> {
    app.world()
        .resource::<PopupStack>()
        .visible()
        .into_iter()
        .map(|entry| entry.spec.key)
        .collect()
}

#[test]
fn death_shows_release_spirit_popup_without_cancel() {
    let (mut app, _commands) = death_app();
    set_death_state(&mut app, DeathStateEntry::Dead, Some(corpse_at(0.0, 0.0)));
    app.update();

    assert_eq!(
        frame_text(&app, "StaticPopup1Text").as_deref(),
        Some("You have died. Release spirit to the nearest graveyard?")
    );
    assert_eq!(
        frame_text(&app, "StaticPopup1Button1").as_deref(),
        Some("Release Spirit")
    );
    assert!(!frame_visible(&app, "StaticPopup1Button2"));
}

#[test]
fn accepting_death_popup_sends_release_spirit() {
    let (mut app, commands) = death_app();
    set_death_state(&mut app, DeathStateEntry::Dead, Some(corpse_at(0.0, 0.0)));
    app.update();

    click_frame(&mut app, "StaticPopup1Button1");
    game_engine::network_events::dispatch_outgoing(app.world_mut());

    let mut sent = drain(&commands);
    assert_eq!(sent.len(), 1);
    assert_sent::<ReleaseSpirit>(sent.remove(0));
    assert!(popup_keys(&app).is_empty(), "popup waits for the server");
}

#[test]
fn escape_does_not_dismiss_death_popup() {
    let (mut app, _commands) = death_app();
    set_death_state(&mut app, DeathStateEntry::Dead, None);
    app.world_mut().resource_mut::<PopupStack>().cancel_top();
    app.update();
    app.update();

    assert_eq!(popup_keys(&app), [DEATH_POPUP]);
}

#[test]
fn ghost_sees_hint_far_from_corpse_and_recover_popup_in_range() {
    let (mut app, commands) = death_app();
    move_player(&mut app, 100.0, 0.0);
    set_death_state(&mut app, DeathStateEntry::Ghost, Some(corpse_at(0.0, 0.0)));

    assert!(popup_keys(&app).is_empty());
    assert!(frame_visible(&app, "GhostHintFrame"));
    assert_eq!(
        frame_text(&app, "GhostHintFrameText").as_deref(),
        Some("Return to your corpse to resurrect (100 yds)")
    );

    move_player(&mut app, 0.0, 29.0);
    app.update();
    assert_eq!(popup_keys(&app), [RECOVER_CORPSE_POPUP]);
    assert_eq!(
        frame_text(&app, "StaticPopup1Text").as_deref(),
        Some("Resurrect now?")
    );
    assert!(!frame_visible(&app, "GhostHintFrame"));

    click_frame(&mut app, "StaticPopup1Button1");
    game_engine::network_events::dispatch_outgoing(app.world_mut());
    let mut sent = drain(&commands);
    assert_eq!(sent.len(), 1);
    assert_sent::<ResurrectAtCorpse>(sent.remove(0));
}

#[test]
fn leaving_corpse_range_hides_recover_popup() {
    let (mut app, _commands) = death_app();
    set_death_state(&mut app, DeathStateEntry::Ghost, Some(corpse_at(0.0, 10.0)));
    assert_eq!(popup_keys(&app), [RECOVER_CORPSE_POPUP]);

    move_player(&mut app, 0.0, 60.0);
    assert!(popup_keys(&app).is_empty());
    assert!(
        app.world_mut()
            .resource_mut::<Messages<PopupResult>>()
            .drain()
            .next()
            .is_none(),
        "hiding is not an answer"
    );
}

#[test]
fn ghost_greys_the_world_until_resurrected() {
    let (mut app, _commands) = death_app();
    let camera = app.world_mut().spawn(WowCamera::default()).id();
    set_death_state(
        &mut app,
        DeathStateEntry::Ghost,
        Some(corpse_at(0.0, 100.0)),
    );
    app.update();
    let saturation = |app: &App| {
        app.world()
            .get::<ColorGrading>(camera)
            .map(|grading| grading.global.post_saturation)
    };
    assert_eq!(saturation(&app), Some(GHOST_POST_SATURATION));

    set_death_state(&mut app, DeathStateEntry::Alive, None);
    assert_eq!(saturation(&app), Some(1.0));
}

#[test]
fn resurrection_clears_death_popups_and_hint() {
    let (mut app, _commands) = death_app();
    set_death_state(&mut app, DeathStateEntry::Ghost, Some(corpse_at(0.0, 5.0)));
    assert_eq!(popup_keys(&app), [RECOVER_CORPSE_POPUP]);

    set_death_state(&mut app, DeathStateEntry::Alive, None);
    app.update();

    assert!(popup_keys(&app).is_empty());
    assert!(!frame_visible(&app, "StaticPopup1"));
    assert!(!frame_visible(&app, "GhostHintFrame"));
}

#[test]
fn leaving_world_clears_death_popup_and_state() {
    let (mut app, _commands) = death_app();
    set_death_state(&mut app, DeathStateEntry::Dead, None);
    assert_eq!(popup_keys(&app), [DEATH_POPUP]);

    app.insert_state(GameState::Login);
    app.update();

    assert!(!app.world().resource::<PopupStack>().is_open());
    assert_eq!(app.world().resource::<DeathStatusSnapshot>().state, None);
}

#[test]
fn server_error_reopens_death_popup_and_shows_error() {
    let (mut app, _commands) = death_app();
    set_death_state(&mut app, DeathStateEntry::Dead, None);
    app.update();
    click_frame(&mut app, "StaticPopup1Button1");
    assert!(popup_keys(&app).is_empty());

    app.world_mut()
        .resource_mut::<DeathStatusSnapshot>()
        .last_error = Some("no graveyard".into());
    app.update();

    assert_eq!(popup_keys(&app), [DEATH_POPUP]);
    assert_eq!(
        app.world().resource::<UiErrors>().lines[0].text,
        "no graveyard"
    );
}
