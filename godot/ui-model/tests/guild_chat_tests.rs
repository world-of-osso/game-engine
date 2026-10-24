use super::*;

#[test]
fn guild_chat_open_requests_existing_history_once_while_waiting() {
    let mut session = GuildRanksSession::default();
    session.open();
    session.apply(state(0));
    session.click("guild:chat_open", &Default::default());
    assert_eq!(
        session.commands,
        vec![game_engine_ui_model::guild_ranks::GuildCommand::ChatHistory]
    );
    session.click("guild:chat_open", &Default::default());
    assert_eq!(
        session.commands.len(),
        1,
        "do not queue a duplicate request"
    );
}

#[test]
fn guild_chat_composer_sends_one_request_without_a_local_echo() {
    let mut session = GuildRanksSession::default();
    session.open();
    session.apply(state(0));
    session.click("guild:chat_open", &Default::default());
    session.commands.clear();
    let inputs = [(
        "CommunitiesChatEditBox".into(),
        "Ready for the guild run".into(),
    )]
    .into_iter()
    .collect();
    session.click("guild:chat_send", &inputs);
    assert_eq!(
        session.commands,
        vec![game_engine_ui_model::guild_ranks::GuildCommand::Chat {
            officer: false,
            text: "Ready for the guild run".into(),
        }]
    );
    assert!(
        session.chat.messages.is_empty(),
        "submission must not invent an echo"
    );
}

#[test]
fn guild_chat_mounts_source_transcript_composer_and_stream_picker_both_skins() {
    let _lock = SKIN.lock().unwrap_or_else(|p| p.into_inner());
    for skin in [ActiveSkin::Modern, ActiveSkin::Forever] {
        set_thread_skin(skin);
        let mut session = GuildRanksSession::default();
        session.open();
        session.apply(state(0));
        session.click("guild:chat_open", &Default::default());
        let registry = mounted(&session);
        for name in [
            "CommunitiesChatMessageFrame",
            "CommunitiesChatEditBox",
            "CommunitiesStreamDropdown",
        ] {
            assert!(
                registry.get_by_name(name).is_some(),
                "{skin:?}: missing {name}"
            );
        }
    }
}

fn chat_line(channel: ChatType, content: &str) -> ChatMessage {
    ChatMessage {
        sender: "Cara".into(),
        content: content.into(),
        channel,
    }
}

#[test]
fn guild_chat_history_and_live_rows_obey_independent_current_listen_rights() {
    let mut session = GuildRanksSession::default();
    session.open();
    session.apply(state(0));
    session.click("guild:chat_open", &Default::default());
    session.apply_chat_history(ChatHistoryResponse {
        messages: vec![
            chat_line(ChatType::Guild, "Guild run"),
            chat_line(ChatType::Officer, "Officer plan"),
            chat_line(ChatType::Say, "Not a guild stream"),
        ],
        error: None,
    });
    assert_eq!(session.chat.messages.len(), 2);
    session.click("guild:chat_officer", &Default::default());
    assert!(session.chat.officer);
    session.apply(state(1));
    assert!(!session.chat.officer);
    assert_eq!(session.chat.messages.len(), 1);
    assert_eq!(session.chat.messages[0].text, "Guild run");
    session.receive_chat(&chat_line(ChatType::Officer, "Private after demotion"));
    assert_eq!(session.chat.messages.len(), 1);
}

#[test]
fn guild_chat_history_replaces_rows_preserves_real_duplicates_and_ignores_late_close() {
    let mut session = GuildRanksSession::default();
    session.open();
    session.apply(state(0));
    let reply = || ChatHistoryResponse {
        messages: vec![
            chat_line(ChatType::Guild, "Ready"),
            chat_line(ChatType::Guild, "Ready"),
        ],
        error: None,
    };
    session.click("guild:chat_open", &Default::default());
    session.apply_chat_history(reply());
    assert_eq!(
        session.chat.messages.len(),
        2,
        "identical real sends remain distinct"
    );
    session.click("guild:chat_open", &Default::default());
    session.apply_chat_history(reply());
    assert_eq!(
        session.chat.messages.len(),
        2,
        "history snapshots replace rather than duplicate rows"
    );
    session.close();
    session.apply_chat_history(reply());
    assert!(session.chat.messages.is_empty());
}

#[test]
fn guild_chat_speak_only_officer_can_send_but_cannot_read_and_limits_are_bytes() {
    use game_engine_ui_model::guild_ranks::GuildCommand;
    let mut session = GuildRanksSession::default();
    let mut authority = state(1);
    authority.ranks[1].rights = GUILD_RIGHT_OFFICER_SPEAK;
    session.open();
    session.apply(authority);
    session.click("guild:chat_open", &Default::default());
    session.click("guild:chat_officer", &Default::default());
    assert!(session.chat.officer);
    session.commands.clear();
    let input = |text: &str| {
        [("CommunitiesChatEditBox".into(), text.into())]
            .into_iter()
            .collect()
    };
    session.click("guild:chat_send", &input("Officer report"));
    assert_eq!(
        session.commands,
        vec![GuildCommand::Chat {
            officer: true,
            text: "Officer report".into()
        }]
    );
    session.receive_chat(&chat_line(ChatType::Officer, "Not authorized to read"));
    assert!(session.chat.messages.is_empty());
    session.commands.clear();
    session.click("guild:chat_send", &input("   "));
    session.click("guild:chat_send", &input(&"é".repeat(257)));
    assert!(session.commands.is_empty());
    assert!(session.chat.error.as_deref().unwrap().contains("512 bytes"));
}

#[test]
fn guild_chat_paints_only_selected_actual_stream_and_source_geometry_both_skins() {
    let _lock = SKIN.lock().unwrap_or_else(|p| p.into_inner());
    for skin in [ActiveSkin::Modern, ActiveSkin::Forever] {
        set_thread_skin(skin);
        let mut session = GuildRanksSession::default();
        session.open();
        session.apply(state(0));
        session.click("guild:chat_open", &Default::default());
        session.apply_chat_history(ChatHistoryResponse {
            messages: vec![
                chat_line(ChatType::Guild, "Guild run"),
                chat_line(ChatType::Officer, "Officer plan"),
            ],
            error: None,
        });
        let registry = mounted(&session);
        let text = |registry: &FrameRegistry, name: &str| {
            let frame = registry.get(registry.get_by_name(name).unwrap()).unwrap();
            let Some(WidgetData::FontString(label)) = &frame.widget_data else {
                panic!("visible chat line")
            };
            label.text.clone()
        };
        assert_eq!(
            text(&registry, "CommunitiesChatMessage0"),
            "[Cara]: Guild run"
        );
        for (name, rect) in [
            ("CommunitiesChatMessageFrame", [754.0, 394.0, 390.0, 303.0]),
            ("CommunitiesChatEditBox", [750.0, 701.0, 397.0, 32.0]),
        ] {
            let actual = registry
                .get(registry.get_by_name(name).unwrap())
                .unwrap()
                .layout_rect
                .as_ref()
                .unwrap();
            for (value, expected) in [actual.x, actual.y, actual.width, actual.height]
                .into_iter()
                .zip(rect)
            {
                assert!(
                    (value - expected).abs() < 0.01,
                    "{skin:?} {name}: {actual:?}"
                );
            }
        }
        session.click("guild:chat_officer", &Default::default());
        let registry = mounted(&session);
        assert_eq!(
            text(&registry, "CommunitiesChatMessage0"),
            "[Cara]: Officer plan"
        );
    }
}
