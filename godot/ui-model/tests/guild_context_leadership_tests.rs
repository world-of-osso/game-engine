//! Source roster entry and captured confirmation through the real screen/reducer boundary.
use super::*;

const ENTRY: &str = "GuildMemberTransferLeadership";
const POPUP: &str = "GuildLeadershipTransferPopup";
const ACCEPT: &str = "GuildLeadershipTransferAccept";
const CANCEL: &str = "GuildLeadershipTransferCancel";

fn session(own_rank: u8, target_self: bool, online: bool) -> GuildRanksSession {
    let mut session = GuildRanksSession::default();
    session.open();
    let mut roster = state(own_rank);
    roster.members = vec![
        GuildRankMember {
            character_name: "Cara".into(),
            rank: if target_self { own_rank } else { 2 },
        },
        GuildRankMember {
            character_name: "Dora".into(),
            rank: 2,
        },
    ];
    session.apply(roster);
    session.profile = Some(GuildSnapshot {
        guild_id: 7,
        guild_name: "Leadership Guild".into(),
        tabard: None,
        created_at: None,
        motd: String::new(),
        info_text: String::new(),
        members: ["Cara", "Dora"]
            .into_iter()
            .map(|name| GuildMemberSnapshot {
                character_name: name.into(),
                level: 32,
                class_name: "Paladin".into(),
                rank_name: "Member".into(),
                is_online: online,
                officer_note: "private".into(),
                public_note: "public".into(),
                is_self: target_self && name == "Cara",
                last_logout_unix_secs: None,
                online_zone: online.then(|| "Elwynn Forest".into()),
            })
            .collect(),
    });
    session
}

fn open_direct(session: &mut GuildRanksSession) {
    session.click("guild:member-menu:Cara", &Default::default());
    assert_eq!(
        session.click("guild:transfer_leadership", &Default::default()),
        None,
        "opening a confirmation must not send a transfer"
    );
}

#[test]
#[ignore = "RED first: sourced current-master/nonself context entry is absent"]
fn guild_context_leadership_menu_eligibility_and_ordinary_rank_controls_both_skins() {
    let _lock = SKIN.lock().unwrap_or_else(|poison| poison.into_inner());
    for skin in [ActiveSkin::Modern, ActiveSkin::Forever] {
        set_thread_skin(skin);
        for online in [true, false] {
            let mut session = session(0, false, online);
            session.click("guild:member-menu:Cara", &Default::default());
            let registry = mounted(&session);
            for (name, expected) in [
                ("GuildMemberPromote", "guild:promote"),
                ("GuildMemberDemote", "guild:demote"),
                ("GuildMemberRemove", "guild:remove_member"),
            ] {
                assert_eq!(action(&registry, name), expected);
            }
            assert!(
                registry.get_by_name(ENTRY).is_some(),
                "source GUILD/GUILD_OFFLINE entry required"
            );
            assert_eq!(
                text(&registry, "GuildMemberTransferLeadershipText"),
                "Promote to Guildmaster"
            );
            let click = action(&registry, ENTRY);
            assert!(!click.is_empty());
            assert_eq!(session.click(&click, &Default::default()), None);
            assert!(mounted(&session).get_by_name(POPUP).is_some());
            assert!(session.commands.is_empty());
            assert!(!session.awaiting_state);
        }
        for (own_rank, target_self) in [(0, true), (1, false)] {
            let mut session = session(own_rank, target_self, true);
            session.click("guild:member-menu:Cara", &Default::default());
            assert!(mounted(&session).get_by_name(ENTRY).is_none());
            assert_eq!(
                session.click("guild:transfer_leadership", &Default::default()),
                None
            );
            assert!(mounted(&session).get_by_name(POPUP).is_none());
        }
        for waiting in ["rank", "profile", "missing-profile"] {
            let mut session = session(0, false, true);
            match waiting {
                "rank" => session.awaiting_state = true,
                "profile" => session.profile_waiting = true,
                _ => session.profile = None,
            }
            session.click("guild:member-menu:Cara", &Default::default());
            assert!(action(&mounted(&session), ENTRY).is_empty());
            assert_eq!(
                session.click("guild:transfer_leadership", &Default::default()),
                None
            );
            assert!(mounted(&session).get_by_name(POPUP).is_none());
        }
    }
}

#[test]
#[ignore = "RED first: captured-target confirmation and typed Accept are absent"]
fn guild_context_leadership_accept_keeps_captured_member_after_sort_selection_and_refresh() {
    let _lock = SKIN.lock().unwrap_or_else(|poison| poison.into_inner());
    for skin in [ActiveSkin::Modern, ActiveSkin::Forever] {
        set_thread_skin(skin);
        let mut session = session(0, false, true);
        open_direct(&mut session);
        assert_eq!(
            text(&mounted(&session), "GuildLeadershipTransferPopupText"),
            "Really promote Cara to Guildmaster?"
        );
        session.click("guild:member:Dora", &Default::default());
        session.click("guild:roster-sort:name", &Default::default());
        session.click("guild:roster-sort:name", &Default::default());
        let mut refresh = session.state().unwrap().clone();
        refresh.members.reverse();
        session.apply(refresh);
        let update = shared::protocol::GuildStateUpdate {
            guild: session.profile.clone(),
            message: None,
            error: None,
        };
        session.apply_profile(update);
        let before = session.state().cloned();
        let profile = session.profile.clone();
        let registry = mounted(&session);
        assert_eq!(
            text(&registry, "GuildLeadershipTransferPopupText"),
            "Really promote Cara to Guildmaster?"
        );
        assert_eq!(
            text(&registry, "GuildLeadershipTransferAcceptText"),
            "Accept"
        );
        assert_eq!(
            text(&registry, "GuildLeadershipTransferCancelText"),
            "Cancel"
        );
        let accept = action(&registry, ACCEPT);
        assert!(!accept.is_empty());
        assert_eq!(
            session.click(&accept, &Default::default()),
            Some(GuildRankRequest::TransferLeadership {
                character_name: "Cara".into()
            })
        );
        assert_eq!(
            session.click(&accept, &Default::default()),
            None,
            "Accept sends at most once"
        );
        assert!(session.awaiting_state);
        assert!(session.commands.is_empty());
        assert_eq!(
            session.state(),
            before.as_ref(),
            "no optimistic leader/rank changes"
        );
        assert_eq!(session.profile, profile);
        assert!(mounted(&session).get_by_name(POPUP).is_none());
    }
}

#[test]
#[ignore = "RED first: new captured popup must dismiss on Cancel/Escape and close"]
fn guild_context_leadership_cancel_escape_and_reopen_never_send_transfer_both_skins() {
    let _lock = SKIN.lock().unwrap_or_else(|poison| poison.into_inner());
    for skin in [ActiveSkin::Modern, ActiveSkin::Forever] {
        set_thread_skin(skin);
        let mut session = session(0, false, true);
        let before = session.state().cloned();
        open_direct(&mut session);
        assert!(
            session.dismiss_subdialog(),
            "native Escape uses this existing dismissal boundary"
        );
        assert!(session.visible, "Escape closes only confirmation");
        assert!(mounted(&session).get_by_name(POPUP).is_none());
        assert_eq!(
            session.click("guild:transfer_leadership_confirm", &Default::default()),
            None
        );
        open_direct(&mut session);
        let registry = mounted(&session);
        let cancel = action(&registry, CANCEL);
        assert!(!cancel.is_empty());
        assert_eq!(session.click(&cancel, &Default::default()), None);
        assert!(mounted(&session).get_by_name(POPUP).is_none());
        assert_eq!(
            session.click("guild:transfer_leadership_confirm", &Default::default()),
            None
        );
        open_direct(&mut session);
        session.close();
        session.open();
        session.apply(before.clone().unwrap());
        assert!(
            mounted(&session).get_by_name(POPUP).is_none(),
            "closed popup cannot reopen stale"
        );
        assert_eq!(
            session.click("guild:transfer_leadership_confirm", &Default::default()),
            None
        );
        assert_eq!(session.state(), before.as_ref());
        assert!(!session.awaiting_state);
        assert!(session.commands.is_empty());
    }
}

#[test]
#[ignore = "RED first: source leadership popup labels and shared dialog geometry are absent"]
fn guild_context_leadership_popup_has_source_labels_and_bounded_dialog_geometry_both_skins() {
    let _lock = SKIN.lock().unwrap_or_else(|poison| poison.into_inner());
    for skin in [ActiveSkin::Modern, ActiveSkin::Forever] {
        set_thread_skin(skin);
        let mut session = session(0, false, false);
        open_direct(&mut session);
        let registry = mounted(&session);
        let popup = registry
            .get_by_name(POPUP)
            .expect("source confirmation must render");
        let popup = registry.get(popup).unwrap();
        assert_eq!(popup.width.value(), 320.0);
        assert_eq!(
            popup.position.top,
            ui_toolkit::layout_values::Val::Px(135.0)
        );
        let message = registry
            .get(
                registry
                    .get_by_name("GuildLeadershipTransferPopupText")
                    .unwrap(),
            )
            .unwrap();
        assert_eq!(message.width.value(), 290.0);
        for (name, caption) in [(ACCEPT, "Accept"), (CANCEL, "Cancel")] {
            let button = registry.get(registry.get_by_name(name).unwrap()).unwrap();
            assert_eq!((button.width.value(), button.height.value()), (128.0, 21.0));
            assert_eq!(text(&registry, &format!("{name}Text")), caption);
            assert!(!action(&registry, name).is_empty());
        }
        assert_eq!(session.state().unwrap().members[0].rank, 2);
        assert!(session.commands.is_empty());
    }
}
