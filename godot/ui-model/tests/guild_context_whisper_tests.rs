//! Mainline COMMUNITIES_GUILD_MEMBER Whisper through the real screen/reducer boundary.
use super::guild_context_leadership_tests::session;
use super::*;

const WHISPER: &str = "GuildMemberWhisper";
const TRANSFER: &str = "GuildMemberTransferLeadership";

fn top(registry: &FrameRegistry, name: &str) -> f32 {
    let frame = registry
        .get(registry.get_by_name(name).expect(name))
        .unwrap();
    match frame.position.top {
        ui_toolkit::layout_values::Val::Px(top) => top,
        other => panic!("{name} top is not pixels: {other:?}"),
    }
}

#[test]
fn guild_context_whisper_shows_for_online_other_member_before_guildmaster_both_skins() {
    let _lock = SKIN.lock().unwrap_or_else(|poison| poison.into_inner());
    for skin in [ActiveSkin::Modern, ActiveSkin::Forever] {
        set_thread_skin(skin);
        let mut leader = session(0, false, true);
        leader.click("guild:member-menu:Cara", &Default::default());
        let registry = mounted(&leader);
        assert!(
            registry.get_by_name(WHISPER).is_some(),
            "source Whisper entry required"
        );
        assert_eq!(text(&registry, "GuildMemberWhisperText"), "Whisper");
        assert!(!action(&registry, WHISPER).is_empty());
        assert!(
            top(&registry, WHISPER) < top(&registry, TRANSFER),
            "Mainline Whisper precedes Promote to Guildmaster"
        );
        for (name, expected) in [
            ("GuildMemberPromote", "guild:promote"),
            ("GuildMemberDemote", "guild:demote"),
            ("GuildMemberRemove", "guild:remove_member"),
            (TRANSFER, "guild:transfer_leadership"),
        ] {
            assert_eq!(action(&registry, name), expected);
        }
        // Whisper CanShow does not depend on guild authority or pending guild replies.
        let mut member = session(2, false, true);
        member.awaiting_state = true;
        member.click("guild:member-menu:Cara", &Default::default());
        let registry = mounted(&member);
        assert!(registry.get_by_name(WHISPER).is_some());
        assert!(registry.get_by_name(TRANSFER).is_none());
    }
}

#[test]
fn guild_context_whisper_hidden_for_self_offline_and_unknown_profile_both_skins() {
    let _lock = SKIN.lock().unwrap_or_else(|poison| poison.into_inner());
    for skin in [ActiveSkin::Modern, ActiveSkin::Forever] {
        set_thread_skin(skin);
        for case in ["self", "offline", "unknown-profile"] {
            let mut session = match case {
                "self" => session(0, true, true),
                "offline" => session(0, false, false),
                _ => {
                    let mut session = session(0, false, true);
                    session.profile = None;
                    session
                }
            };
            session.click("guild:member-menu:Cara", &Default::default());
            let registry = mounted(&session);
            let responder = registry
                .get_by_name(WHISPER)
                .and_then(|id| registry.get(id))
                .and_then(|frame| frame.onclick.as_deref());
            assert!(
                responder.is_none_or(str::is_empty),
                "{case}: Whisper must be hidden"
            );
            assert_eq!(session.click("guild:whisper", &Default::default()), None);
            assert_eq!(session.chat_open, None, "{case}: no chat opened");
        }
    }
}

#[test]
fn guild_context_whisper_opens_existing_chat_prefill_without_guild_request_both_skins() {
    let _lock = SKIN.lock().unwrap_or_else(|poison| poison.into_inner());
    for skin in [ActiveSkin::Modern, ActiveSkin::Forever] {
        set_thread_skin(skin);
        let mut session = session(0, false, true);
        let before = (session.state().cloned(), session.profile.clone());
        session.click("guild:member-menu:Cara", &Default::default());
        let whisper = action(&mounted(&session), WHISPER);
        assert!(!whisper.is_empty());
        assert_eq!(session.click(&whisper, &Default::default()), None);
        assert_eq!(
            session.chat_open.as_deref(),
            Some("/w Cara "),
            "CHAT_WHISPER_HEADER_ABSENT: existing /w prefill, not Retail /cw header mode"
        );
        assert!(session.commands.is_empty(), "no guild or network command");
        assert!(!session.awaiting_state);
        assert_eq!((session.state().cloned(), session.profile.clone()), before);
        assert!(
            mounted(&session)
                .get_by_name("GuildMemberContextMenu")
                .is_none(),
            "menu closes after Whisper"
        );
        session.chat_open = None;
        assert_eq!(session.click("guild:whisper", &Default::default()), None);
        assert_eq!(
            session.chat_open, None,
            "no stale target without an open menu"
        );
    }
}
