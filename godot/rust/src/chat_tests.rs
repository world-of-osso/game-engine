use super::*;
use game_engine_ui_model::chat_frame::{HELP_LINES, UNKNOWN_COMMAND_TEXT};
use shared::protocol::CombatLogKind;

const LOCAL: Option<&str> = Some("Fbchat");

fn server(sender: &str, content: &str, channel: ChatType) -> ChatMessage {
    ChatMessage {
        sender: sender.into(),
        content: content.into(),
        channel,
    }
}

/// `(text, rgba)` of every line the tab lists, oldest first.
fn lines(model: &ChatModel, tab: ChatTab) -> Vec<(String, [f32; 4])> {
    tab_entries(tab, &model.log, &model.combat)
        .into_iter()
        .map(|entry| {
            (
                entry.line.plain_text(|id| format!("Spell {id}")),
                entry.line.color,
            )
        })
        .collect()
}

#[test]
fn server_lines_get_retail_wording_colours_and_tabs() {
    let mut model = ChatModel::default();
    model.receive(&server("Fbchat", "hello", ChatType::Say), LOCAL, 1.0);
    model.receive(&server("Bob", "run!", ChatType::Yell), LOCAL, 2.0);
    model.receive(
        &server("Bob", "psst", ChatType::Whisper("Fbchat".into())),
        LOCAL,
        3.0,
    );
    model.receive(
        &server("Fbchat", "ok", ChatType::Whisper("Bob".into())),
        LOCAL,
        4.0,
    );
    model.receive(&server("", "Welcome to Osso", ChatType::System), LOCAL, 5.0);
    model.receive(
        &server("Hogger", "Grr", ChatType::MonsterSay(9)),
        LOCAL,
        6.0,
    );
    model.receive(&server("Bob", "waves.", ChatType::Emote), LOCAL, 7.0);

    let white = [1.0, 1.0, 1.0, 1.0];
    let pink = [1.0, 0.5, 1.0, 1.0];
    assert_eq!(
        lines(&model, ChatTab::General),
        [
            ("[Fbchat] says: hello".to_string(), white),
            ("[Bob] yells: run!".to_string(), [1.0, 0.25, 0.25, 1.0]),
            ("[Bob] whispers: psst".to_string(), pink),
            ("To [Bob]: ok".to_string(), pink),
            ("Welcome to Osso".to_string(), [1.0, 1.0, 0.0, 1.0]),
            ("Hogger says: Grr".to_string(), [1.0, 1.0, 0.624, 1.0]),
            ("Bob waves.".to_string(), [1.0, 0.5, 0.25, 1.0]),
        ]
    );
    assert_eq!(
        lines(&model, ChatTab::Whispers),
        [
            ("[Bob] whispers: psst".to_string(), pink),
            ("To [Bob]: ok".to_string(), pink),
        ]
    );
    assert!(lines(&model, ChatTab::CombatLog).is_empty());
}

#[test]
fn submitted_lines_become_say_yell_whisper_and_emote_requests() {
    let mut model = ChatModel::default();
    let send = |channel, content: &str| {
        Some(ChatRequest::Send {
            channel,
            content: content.into(),
        })
    };
    model.open();
    assert_eq!(
        model.submit("hello there"),
        send(ChatType::Say, "hello there")
    );
    assert!(!model.state.input_open);
    assert_eq!(model.submit("/y RUN"), send(ChatType::Yell, "RUN"));
    assert_eq!(
        model.submit("/w Bob hi you"),
        send(ChatType::Whisper("Bob".into()), "hi you")
    );
    assert_eq!(model.submit("/e sighs"), send(ChatType::Emote, "sighs"));
    assert_eq!(
        model.submit("/dance"),
        Some(ChatRequest::Emote(EmoteKind::Dance))
    );
    // Recalled newest first, then back to an empty box.
    assert_eq!(model.state.history_prev(), Some("/dance"));
    assert_eq!(model.state.history_prev(), Some("/e sighs"));
    assert_eq!(model.state.history_next(), Some("/dance"));
    assert_eq!(model.state.history_next(), Some(""));
}

#[test]
fn local_commands_print_system_lines_and_send_nothing() {
    let mut model = ChatModel::default();
    assert_eq!(model.submit("/help"), None);
    assert_eq!(model.submit("/join Trade"), None);
    assert_eq!(model.submit("/who Bob"), None);
    let shown: Vec<String> = lines(&model, ChatTab::General)
        .into_iter()
        .map(|(text, _)| text)
        .collect();
    let mut expected: Vec<String> = HELP_LINES.map(String::from).to_vec();
    expected.push(UNKNOWN_COMMAND_TEXT.into());
    expected.push(WHO_UNAVAILABLE_TEXT.into());
    assert_eq!(shown, expected);
}

/// `/invite` and friends go to the server's group backend instead of a local line.
#[test]
fn group_commands_become_group_requests() {
    let mut model = ChatModel::default();
    for (line, command) in [
        ("/invite Bob", GroupCommand::Invite("Bob".into())),
        ("/inv Bob", GroupCommand::Invite("Bob".into())),
        ("/uninvite Bob", GroupCommand::Uninvite("Bob".into())),
        ("/promote Bob", GroupCommand::Promote("Bob".into())),
        ("/leave", GroupCommand::Leave),
        ("/readycheck", GroupCommand::StartReadyCheck),
    ] {
        assert_eq!(
            model.submit(line),
            Some(ChatRequest::Group(command)),
            "{line}"
        );
    }
    assert!(lines(&model, ChatTab::General).is_empty());
}

#[test]
fn reply_opens_only_after_an_incoming_whisper() {
    let mut model = ChatModel::default();
    assert_eq!(model.open_prefill(ChatOpenKey::Reply), None);
    assert_eq!(model.open_prefill(ChatOpenKey::Slash).as_deref(), Some("/"));
    assert_eq!(model.open_prefill(ChatOpenKey::Enter).as_deref(), Some(""));
    model.receive(
        &server("Fbchat", "x", ChatType::Whisper("Carol".into())),
        LOCAL,
        1.0,
    );
    assert_eq!(model.open_prefill(ChatOpenKey::Reply), None);
    model.receive(
        &server("Bob", "psst", ChatType::Whisper("Fbchat".into())),
        LOCAL,
        2.0,
    );
    assert_eq!(
        model.open_prefill(ChatOpenKey::Reply).as_deref(),
        Some("/w Bob ")
    );
    assert_eq!(
        model.submit("/r thanks"),
        Some(ChatRequest::Send {
            channel: ChatType::Whisper("Bob".into()),
            content: "thanks".into(),
        })
    );
}

#[test]
fn unseen_whispers_flash_unselected_tabs_until_a_tab_is_clicked() {
    let mut model = ChatModel::default();
    model.receive(
        &server("Bob", "psst", ChatType::Whisper("Fbchat".into())),
        LOCAL,
        1.0,
    );
    model.absorb_new_lines();
    // General lists the whisper and is selected: nothing flashes.
    assert!(model.state.flashing.is_empty());

    model.click(ChatTab::CombatLog.action(), |_| String::new());
    model.receive(
        &server("Bob", "again", ChatType::Whisper("Fbchat".into())),
        LOCAL,
        2.0,
    );
    model.absorb_new_lines();
    assert_eq!(model.state.flashing, [ChatTab::General, ChatTab::Whispers]);

    model.click(ChatTab::Whispers.action(), |_| String::new());
    assert!(model.state.flashing.is_empty());
    assert_eq!(model.state.tab, ChatTab::Whispers);
}

#[test]
fn scrolled_up_view_holds_while_new_lines_arrive_and_scroll_to_bottom_returns() {
    let mut model = ChatModel::default();
    for index in 0..10 {
        model.receive(
            &server("Bob", &format!("line {index}"), ChatType::Say),
            LOCAL,
            1.0,
        );
    }
    model.absorb_new_lines();
    model.scroll(2, false, false);
    assert_eq!(model.state.scroll, 2);
    model.receive(&server("Bob", "new", ChatType::Say), LOCAL, 2.0);
    model.absorb_new_lines();
    assert_eq!(model.state.scroll, 3);
    model.scroll(1, true, false);
    assert_eq!(model.state.scroll, 10, "Shift scrolls to the oldest of 11");
    model.click(SCROLL_TO_BOTTOM_ACTION, |_| String::new());
    assert_eq!(model.state.scroll, 0);
}

#[test]
fn combat_log_lines_list_only_in_the_combat_log_tab() {
    let mut model = ChatModel::default();
    let event = CombatLogEvent {
        source: Some(1),
        target: Some(2),
        spell_id: Some(133),
        school_mask: 4,
        amount: 42,
        overflow: 0,
        absorbed: 0,
        resisted: 0,
        blocked: 0,
        crit: false,
        glancing: false,
        periodic: false,
        kind: CombatLogKind::Damage,
    };
    model.receive_combat(&event, "Fbchat", UNKNOWN_NAME);
    assert_eq!(
        lines(&model, ChatTab::CombatLog),
        [(
            "Fbchat's [Spell 133] hits Unknown for 42.".to_string(),
            [1.0; 4]
        )]
    );
    assert!(lines(&model, ChatTab::General).is_empty());
    model.leave_world();
    assert!(lines(&model, ChatTab::CombatLog).is_empty());
}
