use super::*;
use shared::protocol::MissKind;

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

const ME: CombatLogActor = CombatLogActor {
    name: "Shot",
    unit: CombatLogUnit::Mine,
};
const KOBOLD: CombatLogActor = CombatLogActor {
    name: "Kobold Vermin",
    unit: CombatLogUnit::Hostile,
};
const BOB: CombatLogActor = CombatLogActor {
    name: "Bob",
    unit: CombatLogUnit::Friendly,
};
/// A unit the client does not replicate, or none (environmental damage).
const UNKNOWN: CombatLogActor = CombatLogActor {
    name: "Unknown",
    unit: CombatLogUnit::Unknown,
};
const WHITE: [f32; 4] = [1.0; 4];
const MINE_GREY: [f32; 4] = [0.7, 0.7, 0.7, 1.0];
const ACTION_GREY: [f32; 4] = [0.5, 0.5, 0.5, 1.0];
const HOSTILE_RED: [f32; 4] = [0.75, 0.05, 0.05, 1.0];
const HOSTILE_BRIGHT: [f32; 4] = [1.0, 0.05 * 1.5, 0.05 * 1.5, 1.0];

fn spell_name(id: u32) -> String {
    match id {
        35395 => "Crusader Strike",
        19750 => "Flash of Light",
        26573 => "Consecration",
        _ => "?",
    }
    .to_string()
}

fn event(
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
        extra_spell_id: None,
        timestamp_unix_ms: 0,
        kind,
    }
}

fn combat_text(
    event: &CombatLogEvent,
    source: CombatLogActor,
    target: CombatLogActor,
) -> Option<String> {
    combat_log_line(event, source, target).map(|line| line.plain_text(spell_name))
}

/// `(text, colour)` of each differently coloured piece of the line.
fn colored_runs(line: &ChatLine) -> Vec<(String, [f32; 4])> {
    wrap_chat_line(line, spell_name, 10_000.0, fixed_width)[0]
        .runs
        .iter()
        .map(|run| (run.text.clone(), run.color))
        .collect()
}

fn piece(text: &str, color: [f32; 4]) -> (String, [f32; 4]) {
    (text.to_string(), color)
}

#[test]
fn the_players_swing_reads_your_melee_hit_in_the_players_grey() {
    let swing = event(CombatLogKind::Damage, None, 1, 12);
    let line = combat_log_line(&swing, ME, KOBOLD).expect("the player's swing is listed");
    assert_eq!(
        line.plain_text(spell_name),
        "Your Melee hit Kobold Vermin 12 Physical."
    );
    assert_eq!(
        colored_runs(&line),
        [
            piece("Your ", MINE_GREY),
            piece("Melee", WHITE),
            piece(" hit Kobold Vermin ", MINE_GREY),
            piece("12", WHITE),
            piece(" ", MINE_GREY),
            piece("Physical", WHITE),
            piece(".", MINE_GREY),
        ]
    );
}

#[test]
fn the_players_spell_damage_names_the_spell_and_its_results() {
    let mut strike = event(CombatLogKind::Damage, Some(35395), 1, 1_234);
    strike.overflow = 300;
    strike.crit = true;
    assert_eq!(
        combat_text(&strike, ME, KOBOLD).unwrap(),
        "Your Crusader Strike hit Kobold Vermin 1,234 Physical. (300 Overkill) (Critical)"
    );
    let mut tick = event(CombatLogKind::Damage, Some(26573), 2, 5);
    tick.periodic = true;
    tick.resisted = 2;
    tick.absorbed = 1;
    assert_eq!(
        combat_text(&tick, ME, KOBOLD).unwrap(),
        "Your Consecration damaged Kobold Vermin 5 Holy. (2 Resisted) (1 Absorbed)"
    );
    let link = combat_log_line(&strike, ME, KOBOLD)
        .unwrap()
        .spans
        .into_iter()
        .find_map(|span| match span {
            ChatSpan::SpellLink { id, .. } => Some(id),
            _ => None,
        });
    assert_eq!(link, Some(35395), "the spell name is a spell link");
}

#[test]
fn damage_on_the_player_reads_you_in_the_attackers_red() {
    let mut swing = event(CombatLogKind::Damage, None, 1, 3);
    swing.blocked = 2;
    swing.glancing = true;
    let line = combat_log_line(&swing, KOBOLD, ME).expect("damage on the player is listed");
    assert_eq!(
        line.plain_text(spell_name),
        "Kobold Vermin Melee hit You 3 Physical. (2 Blocked) (Glancing)"
    );
    assert_eq!(
        colored_runs(&line),
        [
            piece("Kobold Vermin ", HOSTILE_RED),
            piece("Melee", HOSTILE_BRIGHT),
            piece(" hit You ", HOSTILE_RED),
            piece("3", HOSTILE_BRIGHT),
            piece(" ", HOSTILE_RED),
            piece("Physical", HOSTILE_BRIGHT),
            piece(". (2 Blocked) (Glancing)", HOSTILE_RED),
        ]
    );
}

#[test]
fn heals_read_healed_without_the_overhealing() {
    let mut heal = event(CombatLogKind::Heal, Some(19750), 2, 35);
    heal.overflow = 5;
    let line = combat_log_line(&heal, ME, ME).expect("the player's heal is listed");
    assert_eq!(
        line.plain_text(spell_name),
        "Your Flash of Light healed You 30 Holy. (5 Overhealed)"
    );
    assert!(colored_runs(&line).contains(&piece("healed", ACTION_GREY)));
    let mut hot = event(CombatLogKind::Heal, Some(19750), 2, 8);
    hot.periodic = true;
    hot.crit = true;
    let line = combat_log_line(&hot, BOB, ME).expect("a heal on the player is listed");
    assert_eq!(
        line.plain_text(spell_name),
        "Bob Flash of Light healed You 8 Holy. (Critical)"
    );
    assert_eq!(line.color, [0.34, 0.64, 1.0, 1.0]);
}

#[test]
fn deaths_read_you_killed_and_the_death_recap_link() {
    let death = event(CombatLogKind::Death, None, 0, 0);
    let kill = combat_log_line(&death, ME, KOBOLD).expect("the player's kill is listed");
    assert_eq!(
        colored_runs(&kill),
        [
            piece("You ", WHITE),
            piece("killed", ACTION_GREY),
            piece(" Kobold Vermin.", WHITE),
        ]
    );
    let died = combat_log_line(&death, KOBOLD, ME).expect("the player's death is listed");
    assert_eq!(colored_runs(&died), [piece("[You died.]", LINK_COLOR)]);
}

/// Neither default quick filter lists these, and `hideBuffs`/`hideDebuffs` drop auras.
#[test]
fn events_outside_the_default_filters_make_no_line() {
    let unlisted = [
        CombatLogKind::Miss(MissKind::Dodge),
        CombatLogKind::Miss(MissKind::Miss),
        CombatLogKind::AuraApplied,
        CombatLogKind::AuraRemoved,
        CombatLogKind::AuraRefreshed,
        CombatLogKind::Energize,
        CombatLogKind::Interrupt,
        CombatLogKind::Dispel,
        CombatLogKind::CastStart,
        CombatLogKind::CastSuccess,
    ];
    for kind in unlisted {
        let event = event(kind, Some(35395), 1, 7);
        assert_eq!(
            combat_text(&event, ME, KOBOLD),
            None,
            "{kind:?} by the player"
        );
        assert_eq!(
            combat_text(&event, KOBOLD, ME),
            None,
            "{kind:?} on the player"
        );
    }
    // The player's pet and its target: the event reaches the client, the player is in neither role.
    for kind in [
        CombatLogKind::Damage,
        CombatLogKind::Heal,
        CombatLogKind::Death,
    ] {
        let event = event(kind, None, 1, 7);
        assert_eq!(
            combat_text(&event, BOB, KOBOLD),
            None,
            "{kind:?} between others"
        );
    }
}

#[test]
fn combat_log_keeps_newest_lines() {
    let mut log = CombatLogChat::default();
    for index in 0..MAX_COMBAT_LINES + 5 {
        log.push(0.0, ChatLine::plain(WHITE, index.to_string()));
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
        color: WHITE,
        spans: vec![
            text("Al hits "),
            ChatSpan::SpellLink {
                id: 133,
                color: MINE_GREY,
            },
            text(" ok"),
        ],
    };
    let rows = wrap_chat_line(&line, |_| "Fire Ball".to_string(), 140.0, fixed_width);
    let texts: Vec<Vec<&str>> = rows
        .iter()
        .map(|row| row.runs.iter().map(|run| run.text.as_str()).collect())
        .collect();
    assert_eq!(texts, vec![vec!["Al hits "], vec!["Fire Ball", " ok"]]);
    let link = &rows[1].runs[0];
    assert_eq!(link.spell_id, Some(133));
    assert_eq!(link.color, MINE_GREY);
    assert_eq!((link.x, link.width), (0.0, 90.0));
    assert_eq!(rows[1].runs[1].x, 90.0);
}

/// A measure with 4 units of padding per measured string, as a shaped text has: a run
/// starts where the text before it ends, not at the sum of its words' own measures.
#[test]
fn a_coloured_run_starts_where_the_text_before_it_ends() {
    let padded = |value: &str| match value.len() {
        0 => 0.0,
        count => count as f32 * 10.0 + 4.0,
    };
    let line = ChatLine {
        color: WHITE,
        spans: vec![text("Your Melee hit You "), colored(MINE_GREY, "8")],
    };
    let rows = wrap_chat_line(&line, |_| String::new(), 1_000.0, padded);
    let runs = &rows[0].runs;
    assert_eq!(runs[0].text, "Your Melee hit You ");
    assert_eq!(runs[0].width, 194.0);
    assert_eq!((runs[1].text.as_str(), runs[1].x), ("8", 194.0));
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
fn selecting_a_tab_clears_every_flash_and_each_tab_keeps_its_scroll_position() {
    let mut state = ChatFrameState {
        tab: ChatTab::Whispers,
        flashing: vec![ChatTab::General, ChatTab::CombatLog],
        ..Default::default()
    };
    state.scroll_by(4, 10);
    state.select_tab(ChatTab::General);
    assert_eq!(state.tab, ChatTab::General);
    assert!(state.flashing.is_empty());
    assert_eq!(state.scroll(), 0, "General was not scrolled");
    state.scroll_by(2, 10);
    state.select_tab(ChatTab::Whispers);
    assert_eq!(state.scroll(), 4);
    state.scroll_to_bottom();
    state.select_tab(ChatTab::General);
    assert_eq!(state.scroll(), 2);
}

#[test]
fn scrolling_stays_between_the_newest_and_the_oldest_message() {
    let mut state = ChatFrameState::default();
    state.scroll_by(3, 10);
    assert_eq!(state.scroll(), 3);
    state.scroll_by(1000, 10);
    assert_eq!(state.scroll(), 9);
    state.scroll_by(-20, 10);
    assert_eq!(state.scroll(), 0);
    state.scroll_by(1, 0);
    assert_eq!(state.scroll(), 0);
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
            line: ChatLine::plain(WHITE, "first"),
        },
        ChatEntry {
            timestamp: 3661.0,
            line: ChatLine {
                color: WHITE,
                spans: vec![
                    text("Your "),
                    ChatSpan::SpellLink {
                        id: 133,
                        color: WHITE,
                    },
                    colored(WHITE, " hit"),
                    text(" Kobold."),
                ],
            },
        },
    ];
    assert_eq!(
        copy_chat_text(&entries, |_| "Fireball".to_string(), &chrono::Utc),
        "[00:01:00] first\n[01:01:01] Your Fireball hit Kobold."
    );
}

#[test]
fn tab_flash_bounces_between_hidden_and_full_every_half_second() {
    let samples = [0.0, 0.25, 0.5, 0.75, 1.0].map(flash_alpha);
    assert_eq!(samples, [0.0, 0.5, 1.0, 0.5, 0.0]);
}

/// `C_Spell.GetSchoolString` names a multi-school mask: Fire + Frost is `STRING_SCHOOL_FROSTFIRE`
/// "Frostfire", Physical + Holy `STRING_SCHOOL_HOLYSTRIKE` "Holystrike".
#[test]
fn multi_school_damage_names_the_combined_school() {
    let frostfire = event(CombatLogKind::Damage, Some(35395), 0x14, 640);
    assert_eq!(
        combat_text(&frostfire, ME, KOBOLD).unwrap(),
        "Your Crusader Strike hit Kobold Vermin 640 Frostfire."
    );
    let holystrike = event(CombatLogKind::Damage, Some(35395), 0x03, 75);
    assert_eq!(
        combat_text(&holystrike, KOBOLD, ME).unwrap(),
        "Kobold Vermin Crusader Strike hit You 75 Holystrike."
    );
}

/// ENVIRONMENTAL_DAMAGE on the player: no source ("Unknown", unknown-unit grey), the type as
/// the spell, "damaged" in the action colour, the amount without its overkill.
#[test]
fn falling_damage_on_the_player_reads_unknown_falling_damaged_you() {
    let mut fall = event(
        CombatLogKind::Environmental(EnvironmentalKind::Falling),
        None,
        1,
        1_500,
    );
    fall.source = None;
    fall.overflow = 300;
    let line = combat_log_line(&fall, UNKNOWN, ME).expect("damage on the player is listed");
    assert_eq!(
        line.plain_text(spell_name),
        "Unknown Falling damaged You 1,200 Physical. (300 Overkill)"
    );
    let grey = [0.75, 0.75, 0.75, 1.0];
    let bright = [1.0, 1.0, 1.0, 1.0];
    assert_eq!(
        colored_runs(&line),
        [
            piece("Unknown ", grey),
            piece("Falling", bright),
            piece(" ", grey),
            piece("damaged", ACTION_GREY),
            piece(" You ", grey),
            piece("1,200", bright),
            piece(" ", grey),
            piece("Physical", bright),
            piece(". (300 Overkill)", grey),
        ]
    );
    let drowning = event(
        CombatLogKind::Environmental(EnvironmentalKind::Drowning),
        None,
        1,
        210,
    );
    assert_eq!(
        combat_text(&drowning, UNKNOWN, ME).unwrap(),
        "Unknown Drowning damaged You 210 Physical."
    );
}
