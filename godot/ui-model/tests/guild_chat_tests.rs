use super::*;

#[test]
fn guild_chat_open_requests_existing_history_once_while_waiting() {
    let mut session = GuildRanksSession::default();
    session.open();
    session.apply(state(0));
    session.click("guild:chat_open", &Default::default());
    assert_eq!(
        session.commands.len(),
        1,
        "opening must request actual history"
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
        session.commands.len(),
        1,
        "Enter must emit a real chat request"
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
