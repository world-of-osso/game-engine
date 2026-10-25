//! Source confirmation: opening/cancelling/Escape do not send; Yes sends once.
use super::guild_context_leadership_tests::session;
use super::*;

#[test]
fn guild_disband_confirmation_is_master_only_and_yes_sends_once_both_skins() {
    let _lock = SKIN.lock().unwrap_or_else(|poison| poison.into_inner());
    for skin in [ActiveSkin::Modern, ActiveSkin::Forever] {
        set_thread_skin(skin);
        let mut leader = session(0, false, true);
        leader.communities_tab = game_engine_ui_model::guild_ranks::CommunitiesTab::Info;
        let registry = mounted(&leader);
        assert!(
            registry.get_by_name("GuildDisbandButton").is_some(),
            "master needs a native disband entry"
        );
        assert_eq!(leader.click("guild:disband", &Default::default()), None);
        assert!(mounted(&leader).get_by_name("GuildDisbandPopup").is_some());
        assert_eq!(
            text(&mounted(&leader), "GuildDisbandPopupText"),
            "Do you really want to disband your guild?"
        );
        assert!(!leader.awaiting_state);
        assert_eq!(
            leader.click("guild:disband_cancel", &Default::default()),
            None
        );
        assert!(mounted(&leader).get_by_name("GuildDisbandPopup").is_none());
        leader.click("guild:disband", &Default::default());
        assert!(leader.dismiss_subdialog());
        assert!(mounted(&leader).get_by_name("GuildDisbandPopup").is_none());
        leader.click("guild:disband", &Default::default());
        let request = leader
            .click("guild:disband_confirm", &Default::default())
            .expect("Yes sends the actual disband request");
        assert_eq!(
            serde_json::to_value(request).unwrap(),
            serde_json::json!("Disband")
        );
        assert!(leader.awaiting_state);
        assert_eq!(
            leader.click("guild:disband_confirm", &Default::default()),
            None
        );
        for own_rank in [1, 4] {
            let mut member = session(own_rank, false, true);
            member.communities_tab = game_engine_ui_model::guild_ranks::CommunitiesTab::Info;
            assert!(mounted(&member).get_by_name("GuildDisbandButton").is_none());
            assert_eq!(member.click("guild:disband", &Default::default()), None);
            assert!(mounted(&member).get_by_name("GuildDisbandPopup").is_none());
            assert_eq!(
                member.click("guild:disband_confirm", &Default::default()),
                None
            );
        }
    }
}

#[test]
fn guild_disband_slash_waits_for_actual_authority_without_opening_nonleader_panel() {
    let _lock = SKIN.lock().unwrap_or_else(|poison| poison.into_inner());
    for own_rank in [0, 1] {
        let mut session = GuildRanksSession::default();
        assert_eq!(
            session.click("guild:disband_request", &Default::default()),
            Some(GuildRankRequest::Query)
        );
        assert!(!session.visible);
        assert_eq!(
            session.click("guild:disband_request", &Default::default()),
            None
        );
        session.apply(state(own_rank));
        assert!(
            !session.visible,
            "The slash confirmation must not open Communities"
        );
        assert_eq!(
            mounted(&session).get_by_name("GuildDisbandPopup").is_some(),
            own_rank == 0
        );
    }
}

#[test]
fn guild_disband_cancelled_query_can_retry_without_late_reopen() {
    let _lock = SKIN.lock().unwrap_or_else(|poison| poison.into_inner());
    let mut session = GuildRanksSession::default();
    assert_eq!(
        session.click("guild:disband_request", &Default::default()),
        Some(GuildRankRequest::Query)
    );
    assert_eq!(
        session.click("guild:disband_cancel", &Default::default()),
        None
    );
    session.apply(state(0));
    assert!(mounted(&session).get_by_name("GuildDisbandPopup").is_none());
    assert_eq!(
        session.click("guild:disband_request", &Default::default()),
        Some(GuildRankRequest::Query)
    );
    session.apply(state(0));
    assert!(mounted(&session).get_by_name("GuildDisbandPopup").is_some());
}

#[test]
fn guild_disband_loses_confirmation_on_membership_or_leadership_change() {
    let _lock = SKIN.lock().unwrap_or_else(|poison| poison.into_inner());
    let mut leader = session(0, false, true);
    leader.click("guild:disband", &Default::default());
    let same = leader.profile.clone().unwrap();
    leader.apply_profile(GuildStateUpdate {
        guild: Some(same.clone()),
        message: None,
        error: None,
    });
    assert!(mounted(&leader).get_by_name("GuildDisbandPopup").is_some());
    let mut other = same;
    other.guild_id = 8;
    leader.apply_profile(GuildStateUpdate {
        guild: Some(other),
        message: None,
        error: None,
    });
    assert!(mounted(&leader).get_by_name("GuildDisbandPopup").is_none());
    leader.click("guild:disband", &Default::default());
    leader.apply(state(1));
    assert_eq!(
        leader.click("guild:disband_confirm", &Default::default()),
        None
    );
    assert!(mounted(&leader).get_by_name("GuildDisbandPopup").is_none());
}
