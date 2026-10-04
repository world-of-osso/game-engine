use super::*;
use game_engine_ui_model::chat_frame::{HELP_LINES, UNKNOWN_COMMAND_TEXT};
use game_engine_ui_model::hud_layout::{FOREVER, MODERN};
use shared::protocol::{CombatLogKind, MissKind};

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
    assert_eq!(model.state.scroll(), 2);
    model.receive(&server("Bob", "new", ChatType::Say), LOCAL, 2.0);
    model.absorb_new_lines();
    assert_eq!(model.state.scroll(), 3);
    model.scroll(1, true, false);
    assert_eq!(
        model.state.scroll(),
        10,
        "Shift scrolls to the oldest of 11"
    );
    model.click(SCROLL_TO_BOTTOM_ACTION, |_| String::new());
    assert_eq!(model.state.scroll(), 0);
}

const SHOT: CombatLogActor = CombatLogActor {
    name: "Shot",
    unit: CombatLogUnit::Mine,
};
const KOBOLD: CombatLogActor = CombatLogActor {
    name: "Kobold Vermin",
    unit: CombatLogUnit::Hostile,
};
const MINE_GREY: [f32; 4] = [0.7, 0.7, 0.7, 1.0];
const HOSTILE_RED: [f32; 4] = [0.75, 0.05, 0.05, 1.0];

fn combat(
    kind: CombatLogKind,
    spell_id: Option<u32>,
    school_mask: u32,
    amount: i32,
) -> CombatLogEvent {
    CombatLogEvent {
        source: Some(1),
        target: Some(2),
        spell_id,
        school_mask,
        amount,
        overflow: 0,
        absorbed: 0,
        resisted: 0,
        blocked: 0,
        crit: false,
        glancing: false,
        periodic: false,
        kind,
    }
}

/// The text of each message the Modern frame shows, oldest first.
fn shown(model: &ChatModel) -> Vec<String> {
    shown_in(model, MODERN.chat_size)
}

/// The text of each message a chat frame of `chat_size` shows, oldest first.
fn shown_in(model: &ChatModel, chat_size: (f32, f32)) -> Vec<String> {
    let spell_name = |id| format!("Spell {id}");
    chat_frame_view(
        &model.state,
        &model.log,
        &model.combat,
        spell_name,
        chat_size,
    )
    .messages
    .iter()
    .map(|message| {
        message
            .rows
            .iter()
            .flat_map(|row| row.runs.iter().map(|run| run.text.as_str()))
            .collect()
    })
    .collect()
}

#[test]
fn combat_events_become_retail_lines_in_the_combat_log_tab_only() {
    let mut model = ChatModel::default();
    model.receive_combat(
        &combat(CombatLogKind::Damage, Some(35395), 1, 12),
        SHOT,
        KOBOLD,
    );
    model.receive_combat(&combat(CombatLogKind::Damage, None, 1, 3), KOBOLD, SHOT);
    model.receive_combat(&combat(CombatLogKind::Heal, Some(19750), 2, 30), SHOT, SHOT);
    model.receive_combat(&combat(CombatLogKind::Death, None, 0, 0), SHOT, KOBOLD);
    model.receive_combat(&combat(CombatLogKind::Death, None, 0, 0), KOBOLD, SHOT);
    assert_eq!(
        lines(&model, ChatTab::CombatLog),
        [
            (
                "Your Spell 35395 hit Kobold Vermin 12 Physical.".to_string(),
                MINE_GREY
            ),
            (
                "Kobold Vermin Melee hit You 3 Physical.".to_string(),
                HOSTILE_RED
            ),
            (
                "Your Spell 19750 healed You 30 Holy.".to_string(),
                MINE_GREY
            ),
            ("You killed Kobold Vermin.".to_string(), [1.0; 4]),
            ("[You died.]".to_string(), MINE_GREY),
        ]
    );
    assert!(lines(&model, ChatTab::General).is_empty());
    assert!(lines(&model, ChatTab::Whispers).is_empty());
    model.leave_world();
    assert!(lines(&model, ChatTab::CombatLog).is_empty());
}

/// Retail's default quick filters list neither misses nor auras (Blizzard_CombatLog.lua:
/// 229-410; `hideBuffs`/`hideDebuffs`), so they reach no tab.
#[test]
fn a_miss_and_an_aura_application_make_no_line_in_any_tab() {
    let mut model = ChatModel::default();
    let miss = combat(CombatLogKind::Miss(MissKind::Dodge), None, 1, 0);
    model.receive_combat(&miss, SHOT, KOBOLD);
    model.receive_combat(&miss, KOBOLD, SHOT);
    let aura = combat(CombatLogKind::AuraApplied, Some(465), 2, 0);
    model.receive_combat(&aura, SHOT, SHOT);
    for tab in ChatTab::ALL {
        assert!(lines(&model, tab).is_empty(), "{tab:?}");
    }
}

#[test]
fn clicking_a_tab_shows_its_stream_and_keeps_each_tabs_scroll_position() {
    let mut model = ChatModel::default();
    for index in 0..6 {
        model.receive(
            &server("Bob", &format!("chat {index}"), ChatType::Say),
            LOCAL,
            1.0,
        );
        model.receive_combat(
            &combat(CombatLogKind::Damage, None, 1, index + 1),
            SHOT,
            KOBOLD,
        );
    }
    model.absorb_new_lines();
    assert_eq!(shown(&model).last().unwrap(), "[Bob] says: chat 5");
    model.scroll(2, false, false);
    assert_eq!(shown(&model).last().unwrap(), "[Bob] says: chat 3");

    model.click(ChatTab::CombatLog.action(), |_| String::new());
    assert_eq!(model.state.tab, ChatTab::CombatLog);
    assert_eq!(
        shown(&model).last().unwrap(),
        "Your Melee hit Kobold Vermin 6 Physical.",
        "the combat log starts at its own newest line"
    );
    assert_eq!(shown(&model).len(), 6, "no chat line is in the combat log");
    model.scroll(3, false, false);
    assert_eq!(
        shown(&model).last().unwrap(),
        "Your Melee hit Kobold Vermin 3 Physical."
    );

    model.click(ChatTab::General.action(), |_| String::new());
    assert_eq!(
        shown(&model).last().unwrap(),
        "[Bob] says: chat 3",
        "General is still where it was scrolled to"
    );
    // A combat line arriving while General is shown does not move the scrolled-up combat log.
    model.receive_combat(&combat(CombatLogKind::Damage, None, 1, 7), SHOT, KOBOLD);
    model.absorb_new_lines();
    assert_eq!(shown(&model).last().unwrap(), "[Bob] says: chat 3");
    model.click(ChatTab::CombatLog.action(), |_| String::new());
    assert_eq!(
        shown(&model).last().unwrap(),
        "Your Melee hit Kobold Vermin 3 Physical."
    );
    model.click(SCROLL_TO_BOTTOM_ACTION, |_| String::new());
    assert_eq!(
        shown(&model).last().unwrap(),
        "Your Melee hit Kobold Vermin 7 Physical."
    );
}

/// The Forever preset's chat frame is lower than Modern's: it shows the newest lines that
/// fit its own message area, not Modern's count (which drew lines over the tab header).
#[test]
fn the_combat_log_shows_the_lines_that_fit_the_presets_frame() {
    let mut model = ChatModel::default();
    for amount in 1..=20 {
        model.receive_combat(
            &combat(CombatLogKind::Damage, None, 1, amount),
            SHOT,
            KOBOLD,
        );
    }
    model.click(ChatTab::CombatLog.action(), |_| String::new());
    let modern = shown_in(&model, MODERN.chat_size);
    let forever = shown_in(&model, FOREVER.chat_size);
    assert_eq!(modern.len(), 15);
    assert_eq!(forever.len(), 12);
    for lines in [&modern, &forever] {
        assert_eq!(
            lines.last().unwrap(),
            "Your Melee hit Kobold Vermin 20 Physical."
        );
    }
    assert_eq!(forever[0], "Your Melee hit Kobold Vermin 9 Physical.");
}
