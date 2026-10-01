use bevy::ecs::system::RunSystemOnce;
use bevy::input::keyboard::NativeKeyCode;
use game_engine::chat_data::{ChatChannelType, ChatMessage as RuntimeChatMessage};
use game_engine::network_runtime::messages::Inbox;
use game_engine::network_runtime::replication::ReplicationMirrorMap;
use game_engine::status::IgnoreListStatusSnapshot;
use game_engine::ui::chat_frame::{SPELL_LINK_COLOR, local_timestamp};
use game_engine::ui::popup::{PopupOutcome, PopupResult, PopupSpec};
use shared::components::{CharacterAppearance, Npc, Player as NetPlayer};
use shared::protocol::{ChatType, CombatLogEvent, CombatLogKind};

use bevy::input::mouse::AccumulatedMouseScroll;
use game_engine::ui::screens::chat_frame_component::{
    CHAT_COPY_BUTTON, CHAT_SCROLL_TO_BOTTOM_BUTTON, tab_flash_name, tab_name,
};

use super::native_layout_support::compute_layout;
use super::*;
use crate::networking_auth::SelectedCharacterId;
use crate::scenes::static_popup::StaticPopupPlugin;
use crate::ui_input_mode::{UiInputModePlugin, gameplay_keys};

fn chat_app() -> App {
    let mut app = App::new();
    app.add_plugins((MinimalPlugins, bevy::state::app::StatesPlugin));
    app.insert_resource(ButtonInput::<KeyCode>::default());
    app.insert_resource(ButtonInput::<MouseButton>::default());
    app.add_message::<KeyboardInput>();
    app.init_state::<GameState>();
    app.insert_state(GameState::InWorld);
    app.insert_resource(UiState {
        registry: FrameRegistry::new(1920.0, 1080.0),
        event_bus: game_engine::ui::event::EventBus::new(),
        focused_frame: None,
    });
    app.init_resource::<ChatInput>();
    app.init_resource::<EmoteInput>();
    app.init_resource::<ChatState>();
    app.init_resource::<WhisperState>();
    app.init_resource::<game_engine::network_runtime::messages::ConnectionSender>();
    app.add_plugins((UiInputModePlugin, StaticPopupPlugin, ChatFramePlugin));
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

fn window(app: &mut App) -> Entity {
    app.world_mut()
        .query_filtered::<Entity, With<PrimaryWindow>>()
        .single(app.world())
        .unwrap()
}

/// One frame with `key` just pressed and its keyboard event (`text` for printable keys).
fn press(app: &mut App, key: KeyCode, logical_key: Key, text: Option<&str>) {
    let window = window(app);
    {
        let mut keys = app.world_mut().resource_mut::<ButtonInput<KeyCode>>();
        keys.release_all();
        keys.clear();
        keys.press(key);
    }
    app.world_mut().write_message(KeyboardInput {
        key_code: key,
        logical_key,
        state: ButtonState::Pressed,
        text: text.map(Into::into),
        repeat: false,
        window,
    });
    app.update();
}

fn press_enter(app: &mut App) {
    press(app, KeyCode::Enter, Key::Enter, None);
}

fn type_text(app: &mut App, value: &str) {
    for ch in value.chars() {
        let key = if ch == ' ' {
            KeyCode::Space
        } else {
            KeyCode::Unidentified(NativeKeyCode::Unidentified)
        };
        let text = ch.to_string();
        press(app, key, Key::Character(text.as_str().into()), Some(&text));
    }
}

fn editbox(app: &App) -> (u64, String, bool) {
    let ui = app.world().resource::<UiState>();
    let id = ui
        .registry
        .get_by_name(CHAT_EDITBOX.0)
        .expect("chat edit box");
    let frame = ui.registry.get(id).unwrap();
    (id, editbox_text(&ui.registry, id), frame.hidden)
}

fn chat_focused(app: &App) -> bool {
    let (id, ..) = editbox(app);
    focused_editbox(app.world().resource::<UiState>()) == Some(id)
}

/// Text and colour of each shown message row, top to bottom.
fn shown_rows(app: &App) -> Vec<(String, [f32; 4])> {
    let reg = &app.world().resource::<UiState>().registry;
    (0..)
        .map_while(|row| reg.get_by_name(&format!("{CHAT_MESSAGES}Row{row}")))
        .map(|row_id| {
            let runs: Vec<_> = reg
                .children_of(row_id)
                .into_iter()
                .filter_map(|id| match reg.get(id)?.widget_data.as_ref()? {
                    WidgetData::FontString(fs) => Some((fs.text.clone(), fs.color)),
                    _ => None,
                })
                .collect();
            let color = runs.first().map_or([0.0; 4], |run| run.1);
            (runs.into_iter().map(|run| run.0).collect(), color)
        })
        .collect()
}

#[test]
fn enter_opens_focused_editbox_and_switches_to_text_mode() {
    let mut app = chat_app();
    assert!(editbox(&app).2, "edit box starts hidden");

    press_enter(&mut app);
    app.update();

    assert!(chat_focused(&app));
    assert!(!editbox(&app).2, "edit box is shown");
    assert_eq!(*app.world().resource::<UiInputMode>(), UiInputMode::Text);
    assert!(app.world().resource::<ChatFrameState>().input_open);
}

#[test]
fn typed_whisper_command_queues_a_whisper_to_bob_on_enter() {
    let mut app = chat_app();
    press_enter(&mut app);
    type_text(&mut app, "/w Bob hi");
    assert_eq!(editbox(&app).1, "/w Bob hi");

    press_enter(&mut app);

    let queued = app
        .world()
        .resource::<ChatInput>()
        .0
        .clone()
        .expect("queued");
    assert_eq!(queued.channel, ChatType::Whisper("Bob".to_string()));
    assert_eq!(queued.content, "hi");
    assert!(!chat_focused(&app));
    let (_, text, hidden) = editbox(&app);
    assert!(text.is_empty() && hidden, "edit box closes after sending");
    assert_eq!(app.world().resource::<ChatFrameState>().sent, ["/w Bob hi"]);
}

#[test]
fn w_while_typing_goes_to_the_edit_box_not_movement() {
    let mut app = chat_app();
    press_enter(&mut app);

    press(
        &mut app,
        KeyCode::KeyW,
        Key::Character("w".into()),
        Some("w"),
    );

    let mode = *app.world().resource::<UiInputMode>();
    let keys = app.world().resource::<ButtonInput<KeyCode>>();
    assert!(keys.pressed(KeyCode::KeyW));
    assert!(!gameplay_keys(mode, keys).pressed(KeyCode::KeyW));
    assert_eq!(editbox(&app).1, "w");
}

#[test]
fn losing_focus_closes_the_edit_box_without_sending() {
    let mut app = chat_app();
    press_enter(&mut app);
    type_text(&mut app, "hello");
    {
        // What the in-world Escape chain does in Text mode.
        let mut ui = app.world_mut().resource_mut::<UiState>();
        ui.focused_frame = None;
        ui.registry.focused_frame = None;
    }
    app.update();

    assert!(app.world().resource::<ChatInput>().0.is_none());
    let (_, text, hidden) = editbox(&app);
    assert!(text.is_empty() && hidden);
}

#[test]
fn slash_opens_prefilled_and_up_recalls_the_last_sent_line() {
    let mut app = chat_app();
    press(
        &mut app,
        KeyCode::Slash,
        Key::Character("/".into()),
        Some("/"),
    );
    assert_eq!(editbox(&app).1, "/");
    type_text(&mut app, "dance");
    press_enter(&mut app);
    assert_eq!(
        app.world()
            .resource::<EmoteInput>()
            .0
            .as_ref()
            .map(|i| i.emote),
        Some(shared::protocol::EmoteKind::Dance)
    );

    press_enter(&mut app);
    press(&mut app, KeyCode::ArrowUp, Key::ArrowUp, None);
    assert_eq!(editbox(&app).1, "/dance");
}

#[test]
fn r_opens_a_reply_to_the_last_whisper() {
    let mut app = chat_app();
    app.world_mut()
        .resource_mut::<WhisperState>()
        .receive_whisper("Alice");
    press(
        &mut app,
        KeyCode::KeyR,
        Key::Character("r".into()),
        Some("r"),
    );
    assert_eq!(editbox(&app).1, "/w Alice ");
}

#[test]
fn unknown_command_prints_the_retail_help_hint() {
    let mut app = chat_app();
    press_enter(&mut app);
    type_text(&mut app, "/join Trade");
    press_enter(&mut app);
    assert_eq!(
        shown_rows(&app),
        [(
            "Type '/help' for a listing of a few commands.".to_string(),
            [1.0, 1.0, 0.0, 1.0]
        )]
    );
}

fn received(sender: &str, content: &str, channel: ChatType) -> ChatMessage {
    ChatMessage {
        sender: sender.to_string(),
        content: content.to_string(),
        channel,
    }
}

#[test]
fn received_messages_land_in_their_tabs_with_channel_colours() {
    let mut app = chat_app();
    app.init_resource::<crate::networking::ChatLog>();
    app.init_resource::<IgnoreListStatusSnapshot>();
    app.init_resource::<SelectedCharacterId>();
    app.insert_resource(Inbox::new(vec![
        received("Bob", "lfg?", ChatType::Party),
        received("Alice", "psst", ChatType::Whisper("Me".to_string())),
    ]));
    app.world_mut()
        .run_system_once(crate::networking_messages::receive_chat_messages)
        .unwrap();
    app.update();

    assert_eq!(
        shown_rows(&app),
        [
            ("[Party] [Bob]: lfg?".to_string(), [0.67, 0.67, 1.0, 1.0]),
            ("[Alice] whispers: psst".to_string(), [1.0, 0.5, 1.0, 1.0]),
        ]
    );

    app.world_mut().resource_mut::<ChatFrameState>().tab = ChatTab::Whispers;
    app.update();
    assert_eq!(
        shown_rows(&app),
        [("[Alice] whispers: psst".to_string(), [1.0, 0.5, 1.0, 1.0])]
    );
}

#[test]
fn combat_log_tab_shows_resolved_names_and_a_spell_link() {
    let mut app = chat_app();
    let alice = app
        .world_mut()
        .spawn(NetPlayer {
            name: "Alice".to_string(),
            race: 1,
            class: 8,
            appearance: CharacterAppearance::default(),
        })
        .id();
    let kobold = app
        .world_mut()
        .spawn(Npc {
            template_id: 6,
            name: "Kobold Vermin".to_string(),
        })
        .id();
    let mut mirror = ReplicationMirrorMap::default();
    mirror.insert(Entity::from_bits(1001), alice);
    mirror.insert(Entity::from_bits(1002), kobold);
    app.insert_resource(mirror);
    app.init_resource::<crate::networking_messages::CombatTextSource>();
    app.insert_resource(Inbox::new(vec![
        CombatLogEvent {
            source: Some(Entity::from_bits(1001).to_bits()),
            target: Some(Entity::from_bits(1002).to_bits()),
            spell_id: Some(133),
            school_mask: 4,
            amount: 120,
            overflow: 0,
            absorbed: 0,
            resisted: 0,
            blocked: 0,
            crit: false,
            glancing: false,
            periodic: false,
            kind: CombatLogKind::Damage,
        },
        CombatLogEvent {
            source: Some(Entity::from_bits(1002).to_bits()),
            target: Some(Entity::from_bits(4040).to_bits()),
            spell_id: None,
            school_mask: 1,
            amount: 7,
            overflow: 0,
            absorbed: 0,
            resisted: 0,
            blocked: 0,
            crit: false,
            glancing: false,
            periodic: false,
            kind: CombatLogKind::Damage,
        },
    ]));
    app.world_mut()
        .run_system_once(crate::networking_messages::receive_combat_log_events)
        .unwrap();
    app.world_mut().resource_mut::<ChatFrameState>().tab = ChatTab::CombatLog;
    app.update();

    let rows: Vec<String> = shown_rows(&app).into_iter().map(|row| row.0).collect();
    assert_eq!(
        rows,
        [
            "Alice's [Spell #133] hits Kobold Vermin for 120.",
            "Kobold Vermin hits Unknown for 7.",
        ]
    );
    let reg = &app.world().resource::<UiState>().registry;
    let link = reg
        .get_by_name("ChatFrame1Link0_1_133")
        .expect("spell link run");
    let Some(WidgetData::FontString(fs)) = reg.get(link).unwrap().widget_data.as_ref() else {
        panic!("link is a fontstring");
    };
    assert_eq!(fs.color, SPELL_LINK_COLOR);
    assert!(reg.get(link).unwrap().mouse_enabled, "link is hoverable");
}

#[test]
fn enter_with_a_popup_accepts_the_popup_and_does_not_open_chat() {
    let mut app = chat_app();
    let id = app
        .world_mut()
        .resource_mut::<PopupStack>()
        .push(PopupSpec {
            key: "invite".to_string(),
            text: "Bob invites you to a group.".to_string(),
            accept_label: "Accept".to_string(),
            cancel_label: Some("Decline".to_string()),
            timeout: None,
            confirm_text: None,
        });
    app.update();

    press_enter(&mut app);
    app.update();

    let results: Vec<PopupResult> = app
        .world_mut()
        .resource_mut::<Messages<PopupResult>>()
        .drain()
        .collect();
    assert_eq!(
        results,
        [PopupResult {
            id,
            key: "invite".to_string(),
            outcome: PopupOutcome::Accepted,
        }]
    );
    assert!(!chat_focused(&app));
    assert!(!app.world().resource::<ChatFrameState>().input_open);

    press_enter(&mut app);
    assert!(chat_focused(&app), "with no popup, Enter opens chat");
}

#[test]
fn message_rows_arriving_after_login_are_visible() {
    let mut app = chat_app();
    app.init_resource::<crate::networking::ChatLog>();
    app.init_resource::<IgnoreListStatusSnapshot>();
    app.init_resource::<SelectedCharacterId>();
    app.update();
    app.insert_resource(Inbox::new(vec![received(
        "Elara",
        "hello azeroth",
        ChatType::Say,
    )]));
    app.world_mut()
        .run_system_once(crate::networking_messages::receive_chat_messages)
        .unwrap();
    app.update();

    let reg = &app.world().resource::<UiState>().registry;
    let row = reg
        .get_by_name(&format!("{CHAT_MESSAGES}Row0"))
        .expect("row 0");
    let run = reg.children_of(row)[0];
    assert!(reg.get(row).unwrap().visible, "row hidden");
    assert!(reg.get(run).unwrap().visible, "row text hidden");
}

fn add_chat(app: &mut App, channel_type: ChatChannelType, text: &str, timestamp: f64) {
    app.world_mut()
        .resource_mut::<ChatState>()
        .add_message(RuntimeChatMessage {
            channel_type,
            channel_name: String::new(),
            sender: "Bob".to_string(),
            text: text.to_string(),
            timestamp,
        });
    app.update();
}

fn frame_rect(app: &mut App, name: &str) -> game_engine::ui::layout::LayoutRect {
    compute_layout(&mut app.world_mut().resource_mut::<UiState>().registry);
    let reg = &app.world().resource::<UiState>().registry;
    let frame = reg.get(reg.get_by_name(name).expect(name)).unwrap();
    frame.layout_rect.clone().expect("laid out")
}

fn move_cursor_to(app: &mut App, name: &str) {
    let rect = frame_rect(app, name);
    let window = window(app);
    app.world_mut()
        .get_mut::<Window>(window)
        .unwrap()
        .set_cursor_position(Some(Vec2::new(
            rect.x + rect.width / 2.0,
            rect.y + rect.height / 2.0,
        )));
}

fn click(app: &mut App, name: &str) {
    move_cursor_to(app, name);
    let mut mouse = app.world_mut().resource_mut::<ButtonInput<MouseButton>>();
    mouse.release_all();
    mouse.clear();
    mouse.press(MouseButton::Left);
    app.update();
    app.world_mut()
        .resource_mut::<ButtonInput<MouseButton>>()
        .clear();
    app.update();
}

/// One wheel notch up (towards older messages) over the message area.
fn wheel_up(app: &mut App) {
    move_cursor_to(app, CHAT_MESSAGES);
    app.insert_resource(AccumulatedMouseScroll {
        unit: MouseScrollUnit::Line,
        delta: Vec2::new(0.0, 1.0),
    });
    app.update();
    app.insert_resource(AccumulatedMouseScroll::default());
    app.update();
}

fn is_shown(app: &App, name: &str) -> bool {
    let reg = &app.world().resource::<UiState>().registry;
    let id = reg.get_by_name(name).expect(name);
    !reg.get(id).unwrap().hidden
}

fn shown_texts(app: &App) -> Vec<String> {
    shown_rows(app).into_iter().map(|row| row.0).collect()
}

#[test]
fn clicking_a_tab_selects_it_and_lists_only_its_messages() {
    let mut app = chat_app();
    add_chat(&mut app, ChatChannelType::Say, "hello", 10.0);
    add_chat(&mut app, ChatChannelType::Whisper, "psst", 20.0);
    assert_eq!(
        shown_texts(&app),
        ["[Bob] says: hello", "[Bob] whispers: psst"]
    );

    click(&mut app, &tab_name(ChatTab::Whispers.index()));

    assert_eq!(
        app.world().resource::<ChatFrameState>().tab,
        ChatTab::Whispers
    );
    assert_eq!(shown_texts(&app), ["[Bob] whispers: psst"]);
    let reg = &app.world().resource::<UiState>().registry;
    let alpha = |tab: ChatTab| {
        reg.get(reg.get_by_name(&tab_name(tab.index())).unwrap())
            .unwrap()
            .alpha
    };
    assert_eq!(alpha(ChatTab::Whispers), 1.0, "selected tab is opaque");
    assert_eq!(alpha(ChatTab::General), 0.5, "unselected tab is dimmed");
}

#[test]
fn a_message_for_an_unselected_tab_flashes_it_until_a_tab_is_clicked() {
    let mut app = chat_app();
    click(&mut app, &tab_name(ChatTab::Whispers.index()));
    let general_flash = tab_flash_name(ChatTab::General.index());
    let whispers_flash = tab_flash_name(ChatTab::Whispers.index());
    assert!(!is_shown(&app, &general_flash));

    add_chat(&mut app, ChatChannelType::Say, "anyone here?", 10.0);

    assert!(is_shown(&app, &general_flash), "General flashes for a say");
    assert!(!is_shown(&app, &whispers_flash));

    click(&mut app, &tab_name(ChatTab::General.index()));
    assert!(
        !is_shown(&app, &general_flash),
        "selecting a tab stops the flash"
    );
    assert_eq!(shown_texts(&app), ["[Bob] says: anyone here?"]);
}

#[test]
fn a_message_shown_in_the_selected_tab_flashes_nothing() {
    let mut app = chat_app();
    add_chat(&mut app, ChatChannelType::Whisper, "psst", 10.0);
    for tab in ChatTab::ALL {
        assert!(!is_shown(&app, &tab_flash_name(tab.index())), "{tab:?}");
    }
}

#[test]
fn scrolling_up_shows_scroll_to_bottom_and_holds_the_view_until_clicked() {
    let mut app = chat_app();
    for index in 0..40 {
        add_chat(
            &mut app,
            ChatChannelType::Say,
            &format!("line {index}"),
            10.0,
        );
    }
    assert!(!is_shown(&app, CHAT_SCROLL_TO_BOTTOM_BUTTON));
    assert_eq!(shown_texts(&app).last().unwrap(), "[Bob] says: line 39");

    wheel_up(&mut app);

    assert!(is_shown(&app, CHAT_SCROLL_TO_BOTTOM_BUTTON));
    assert_eq!(shown_texts(&app).last().unwrap(), "[Bob] says: line 38");
    add_chat(&mut app, ChatChannelType::Say, "line 40", 10.0);
    assert_eq!(
        shown_texts(&app).last().unwrap(),
        "[Bob] says: line 38",
        "a new line does not move a scrolled-up view"
    );

    click(&mut app, CHAT_SCROLL_TO_BOTTOM_BUTTON);

    assert!(!is_shown(&app, CHAT_SCROLL_TO_BOTTOM_BUTTON));
    assert_eq!(shown_texts(&app).last().unwrap(), "[Bob] says: line 40");
}

#[test]
fn chat_messages_show_their_arrival_time_and_combat_lines_do_not() {
    let mut app = chat_app();
    add_chat(&mut app, ChatChannelType::Say, "hello", 1_700_000_000.0);
    assert_eq!(
        fontstring_text(&app, "ChatFrame1Message0Time"),
        local_timestamp(1_700_000_000.0)
    );

    app.world_mut().resource_mut::<CombatLogChat>().push(
        1_700_000_000.0,
        game_engine::ui::chat_frame::ChatLine::plain([1.0; 4], "Bob dies."),
    );
    click(&mut app, &tab_name(ChatTab::CombatLog.index()));
    assert_eq!(shown_texts(&app), ["Bob dies."]);
    let reg = &app.world().resource::<UiState>().registry;
    assert!(reg.get_by_name("ChatFrame1Message0Time").is_none());
}

fn fontstring_text(app: &App, name: &str) -> String {
    let reg = &app.world().resource::<UiState>().registry;
    match reg
        .get(reg.get_by_name(name).expect(name))
        .unwrap()
        .widget_data
        .as_ref()
    {
        Some(WidgetData::FontString(fs)) => fs.text.clone(),
        other => panic!("{name} is not a fontstring: {other:?}"),
    }
}

#[test]
fn copy_chat_puts_the_selected_tabs_timestamped_lines_on_the_clipboard() {
    let mut app = chat_app();
    let copied = Arc::new(Mutex::new(Vec::<String>::new()));
    let sink = copied.clone();
    app.insert_resource(ChatClipboard(Arc::new(move |text| {
        sink.lock().unwrap().push(text.to_string());
        Ok(())
    })));
    add_chat(&mut app, ChatChannelType::Say, "hello", 1_700_000_000.0);
    add_chat(&mut app, ChatChannelType::Whisper, "psst", 1_700_000_060.0);

    click(&mut app, CHAT_COPY_BUTTON);

    let expected = format!(
        "[{}] [Bob] says: hello\n[{}] [Bob] whispers: psst",
        local_timestamp(1_700_000_000.0),
        local_timestamp(1_700_000_060.0)
    );
    assert_eq!(*copied.lock().unwrap(), [expected]);
}

#[test]
fn a_failed_copy_reports_why_in_chat() {
    let mut app = chat_app();
    app.insert_resource(ChatClipboard(Arc::new(|_| {
        Err("clipboard init: no display".to_string())
    })));

    click(&mut app, CHAT_COPY_BUTTON);

    assert_eq!(
        shown_texts(&app),
        ["Copy Chat failed: clipboard init: no display"]
    );
}
