use super::*;

fn say(body: &str) -> ChatCommand {
    ChatCommand::Send {
        channel: ChatType::Say,
        text: body.to_string(),
    }
}

fn whisper(target: &str, body: &str) -> ChatCommand {
    ChatCommand::Send {
        channel: ChatType::Whisper(target.to_string()),
        text: body.to_string(),
    }
}

#[test]
fn plain_text_is_said_and_channel_commands_pick_their_channel() {
    assert_eq!(parse_chat_input("hello there", None), say("hello there"));
    assert_eq!(parse_chat_input("/s hi", None), say("hi"));
    let cases = [
        ("/y run!", ChatType::Yell, "run!"),
        ("/yell run!", ChatType::Yell, "run!"),
        ("/p pull now", ChatType::Party, "pull now"),
        ("/g hello guild", ChatType::Guild, "hello guild"),
        ("/e waves goodbye", ChatType::Emote, "waves goodbye"),
    ];
    for (line, channel, body) in cases {
        assert_eq!(
            parse_chat_input(line, None),
            ChatCommand::Send {
                channel,
                text: body.to_string()
            },
            "{line}"
        );
    }
}

#[test]
fn whisper_and_reply_target_the_named_player() {
    assert_eq!(
        parse_chat_input("/w Bob hi there", None),
        whisper("Bob", "hi there")
    );
    assert_eq!(
        parse_chat_input("/whisper Bob hi", None),
        whisper("Bob", "hi")
    );
    assert_eq!(
        parse_chat_input("/r thanks", Some("Alice")),
        whisper("Alice", "thanks")
    );
    assert_eq!(parse_chat_input("/r thanks", None), ChatCommand::None);
    assert_eq!(parse_chat_input("/w Bob", None), ChatCommand::None);
}

#[test]
fn emote_who_invite_help_and_unknown_commands() {
    assert_eq!(
        parse_chat_input("/dance", None),
        ChatCommand::Emote(EmoteKind::Dance)
    );
    assert_eq!(
        parse_chat_input("/KNEEL", None),
        ChatCommand::Emote(EmoteKind::Kneel)
    );
    assert_eq!(
        parse_chat_input("/who Elwynn", None),
        ChatCommand::Who("Elwynn".to_string())
    );
    assert_eq!(
        parse_chat_input("/invite Bob", None),
        ChatCommand::Group(GroupCommand::Invite("Bob".to_string()))
    );
    assert_eq!(
        parse_chat_input("/kick Bob extra", None),
        ChatCommand::Group(GroupCommand::Uninvite("Bob".to_string()))
    );
    assert_eq!(
        parse_chat_input("/pr Cid", None),
        ChatCommand::Group(GroupCommand::Promote("Cid".to_string()))
    );
    assert_eq!(
        parse_chat_input("/rc", None),
        ChatCommand::Group(GroupCommand::StartReadyCheck)
    );
    assert_eq!(parse_chat_input("/promote", None), ChatCommand::None);
    assert_eq!(
        parse_chat_input("/leave", None),
        ChatCommand::Group(GroupCommand::Leave)
    );
    let ChatCommand::System(help) = parse_chat_input("/help", None) else {
        panic!("help prints system lines");
    };
    assert!(help[0].contains("/say"));
    for unknown in ["/join Trade", "/leave 2", "/bogus"] {
        assert_eq!(
            parse_chat_input(unknown, None),
            ChatCommand::System(vec![UNKNOWN_COMMAND_TEXT.to_string()]),
            "{unknown}"
        );
    }
}

fn message(channel_type: ChatChannelType, sender: &str, channel_name: &str) -> ChatMessage {
    ChatMessage {
        channel_type,
        channel_name: channel_name.to_string(),
        sender: sender.to_string(),
        text: "hi".to_string(),
        timestamp: 0.0,
    }
}

#[test]
fn received_lines_use_retail_wording_and_channel_colour() {
    let cases = [
        (
            ChatChannelType::Say,
            "",
            "[Bob] says: hi",
            [1.0, 1.0, 1.0, 1.0],
        ),
        (
            ChatChannelType::Yell,
            "",
            "[Bob] yells: hi",
            [1.0, 0.25, 0.25, 1.0],
        ),
        (
            ChatChannelType::Party,
            "",
            "[Party] [Bob]: hi",
            [0.67, 0.67, 1.0, 1.0],
        ),
        (
            ChatChannelType::Guild,
            "",
            "[Guild] [Bob]: hi",
            [0.25, 1.0, 0.25, 1.0],
        ),
        (
            ChatChannelType::Whisper,
            "",
            "[Bob] whispers: hi",
            [1.0, 0.5, 1.0, 1.0],
        ),
        (
            ChatChannelType::Whisper,
            "Carl",
            "To [Carl]: hi",
            [1.0, 0.5, 1.0, 1.0],
        ),
        (ChatChannelType::System, "", "hi", [1.0, 1.0, 0.0, 1.0]),
    ];
    for (channel, name, expected, color) in cases {
        let line = chat_message_line(&message(channel, "Bob", name));
        assert_eq!(line.plain_text(|_| String::new()), expected);
        assert_eq!(line.color, color, "{expected}");
    }
}

#[test]
fn whispers_tab_lists_only_whispers() {
    let whisper = message(ChatChannelType::Whisper, "Bob", "");
    let party = message(ChatChannelType::Party, "Bob", "");
    assert!(ChatTab::Whispers.shows(&whisper));
    assert!(!ChatTab::Whispers.shows(&party));
    assert!(ChatTab::General.shows(&party));
    assert!(!ChatTab::CombatLog.shows(&party));
}

fn event(kind: CombatLogKind, spell_id: Option<u32>) -> CombatLogEvent {
    CombatLogEvent {
        source: Some(1),
        target: Some(2),
        spell_id,
        school_mask: 4,
        amount: 120,
        overflow: 0,
        absorbed: 0,
        resisted: 0,
        blocked: 0,
        crit: false,
        periodic: false,
        kind,
    }
}

fn combat_text(event: &CombatLogEvent) -> String {
    combat_log_line(event, "Alice", "Kobold").plain_text(|id| format!("Spell {id}"))
}

#[test]
fn combat_lines_name_both_units_and_link_the_spell() {
    assert_eq!(
        combat_text(&event(CombatLogKind::Damage, Some(133))),
        "Alice's [Spell 133] hits Kobold for 120."
    );
    let mut crit = event(CombatLogKind::Damage, None);
    crit.crit = true;
    crit.absorbed = 30;
    assert_eq!(
        combat_text(&crit),
        "Alice crits Kobold for 120 (30 Absorbed)."
    );
    let mut hot = event(CombatLogKind::Heal, Some(774));
    hot.periodic = true;
    assert_eq!(
        combat_text(&hot),
        "Kobold gains 120 health from Alice's [Spell 774]."
    );
    assert_eq!(
        combat_text(&event(CombatLogKind::Miss(MissKind::Dodge), None)),
        "Alice attacks. Kobold dodges."
    );
    assert_eq!(
        combat_text(&event(CombatLogKind::Miss(MissKind::Resist), Some(133))),
        "Alice's [Spell 133] was resisted by Kobold."
    );
    assert_eq!(
        combat_text(&event(CombatLogKind::AuraRemoved, Some(1459))),
        "[Spell 1459] fades from Kobold."
    );
    assert_eq!(
        combat_text(&event(CombatLogKind::Death, None)),
        "Kobold dies."
    );
}

#[test]
fn combat_log_keeps_newest_lines() {
    let mut log = CombatLogChat::default();
    for index in 0..MAX_COMBAT_LINES + 5 {
        log.push(0.0, ChatLine::plain(COMBAT_LOG_COLOR, index.to_string()));
    }
    assert_eq!(log.lines.len(), MAX_COMBAT_LINES);
    assert_eq!(log.lines[0].line.plain_text(|_| String::new()), "5");
    assert_eq!(log.received, MAX_COMBAT_LINES as u64 + 5);
}

/// Every character is 10 units wide.
fn fixed_width(value: &str) -> f32 {
    value.chars().count() as f32 * 10.0
}

#[test]
fn wrapping_breaks_between_words_and_keeps_links_whole() {
    let line = ChatLine {
        color: COMBAT_LOG_COLOR,
        spans: vec![text("Al hits "), ChatSpan::SpellLink(133), text(" ok")],
    };
    let rows = wrap_chat_line(&line, |_| "Fireball".to_string(), 140.0, fixed_width);
    let texts: Vec<Vec<&str>> = rows
        .iter()
        .map(|row| row.runs.iter().map(|run| run.text.as_str()).collect())
        .collect();
    assert_eq!(texts, vec![vec!["Al hits "], vec!["[Fireball]", " ok"]]);
    let link = &rows[1].runs[0];
    assert_eq!(link.spell_id, Some(133));
    assert_eq!(link.color, SPELL_LINK_COLOR);
    assert_eq!((link.x, link.width), (0.0, 100.0));
    assert_eq!(rows[1].runs[1].x, 100.0);
}

#[test]
fn sent_history_cycles_up_and_down() {
    let mut state = ChatFrameState::default();
    for line in ["one", "two", "two", "three"] {
        state.remember_sent(line);
    }
    assert_eq!(state.sent, ["one", "two", "three"]);
    assert_eq!(state.history_prev(), Some("three"));
    assert_eq!(state.history_prev(), Some("two"));
    assert_eq!(state.history_prev(), Some("one"));
    assert_eq!(state.history_prev(), Some("one"));
    assert_eq!(state.history_next(), Some("two"));
    assert_eq!(state.history_next(), Some("three"));
    assert_eq!(state.history_next(), Some(""));
    assert_eq!(state.history_next(), None);
}

fn received(channel_type: ChatChannelType, channel_name: &str, text: &str) -> ChatMessage {
    ChatMessage {
        channel_type,
        channel_name: channel_name.to_string(),
        sender: "Bob".to_string(),
        text: text.to_string(),
        timestamp: 0.0,
    }
}

#[test]
fn timestamps_use_the_clock_time_format() {
    let utc_plus_one = chrono::FixedOffset::east_opt(3600).unwrap();
    // 2023-11-14 22:13:20 UTC.
    assert_eq!(format_timestamp(1_700_000_000.7, &utc_plus_one), "23:13:20");
    assert_eq!(format_timestamp(0.0, &chrono::Utc), "00:00:00");
}

#[test]
fn new_messages_flash_matching_tabs_unless_the_selected_tab_shows_them() {
    let say = received(ChatChannelType::Say, "", "hi");
    let whisper = received(ChatChannelType::Whisper, "", "psst");
    let outgoing = received(ChatChannelType::Whisper, "Alice", "hello");
    assert_eq!(
        tabs_to_flash(ChatTab::Whispers, std::slice::from_ref(&say)),
        [ChatTab::General]
    );
    assert_eq!(
        tabs_to_flash(ChatTab::CombatLog, std::slice::from_ref(&whisper)),
        [ChatTab::General, ChatTab::Whispers]
    );
    assert!(tabs_to_flash(ChatTab::General, std::slice::from_ref(&whisper)).is_empty());
    assert!(tabs_to_flash(ChatTab::CombatLog, &[outgoing]).is_empty());
}

#[test]
fn selecting_a_tab_clears_every_flash_and_returns_to_the_newest_message() {
    let mut state = ChatFrameState {
        tab: ChatTab::Whispers,
        flashing: vec![ChatTab::General, ChatTab::CombatLog],
        scroll: 4,
        ..Default::default()
    };
    state.select_tab(ChatTab::General);
    assert_eq!(state.tab, ChatTab::General);
    assert!(state.flashing.is_empty());
    assert_eq!(state.scroll, 0);
}

#[test]
fn scrolling_stays_between_the_newest_and_the_oldest_message() {
    let mut state = ChatFrameState::default();
    state.scroll_by(3, 10);
    assert_eq!(state.scroll, 3);
    state.scroll_by(1000, 10);
    assert_eq!(state.scroll, 9);
    state.scroll_by(-20, 10);
    assert_eq!(state.scroll, 0);
    state.scroll_by(1, 0);
    assert_eq!(state.scroll, 0);
}

#[test]
fn messages_fill_the_area_from_the_newest_and_drop_partial_ones() {
    // Heights newest first; 5 between messages.
    assert_eq!(messages_that_fit(&[14.0, 28.0, 14.0], 60.0, 5.0), 2);
    assert_eq!(messages_that_fit(&[14.0, 28.0, 14.0], 71.0, 5.0), 3);
    assert_eq!(messages_that_fit(&[80.0], 60.0, 5.0), 0);
}

#[test]
fn copied_chat_lists_timestamped_lines_oldest_first() {
    let entries = vec![
        ChatEntry {
            timestamp: 60.0,
            line: ChatLine::plain(COMBAT_LOG_COLOR, "first"),
        },
        ChatEntry {
            timestamp: 3661.0,
            line: ChatLine {
                color: COMBAT_LOG_COLOR,
                spans: vec![text("Al casts "), ChatSpan::SpellLink(133), text(".")],
            },
        },
    ];
    assert_eq!(
        copy_chat_text(&entries, |_| "Fireball".to_string(), &chrono::Utc),
        "[00:01:00] first\n[01:01:01] Al casts [Fireball]."
    );
}

#[test]
fn tab_flash_bounces_between_hidden_and_full_every_half_second() {
    let samples = [0.0, 0.25, 0.5, 0.75, 1.0].map(flash_alpha);
    assert_eq!(samples, [0.0, 0.5, 1.0, 0.5, 0.0]);
}
