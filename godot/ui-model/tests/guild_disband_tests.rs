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
        assert_eq!(session.visible, own_rank == 0);
        assert_eq!(
            mounted(&session).get_by_name("GuildDisbandPopup").is_some(),
            own_rank == 0
        );
    }
}

#[test]
fn guild_disband_loses_confirmation_on_membership_or_leadership_change() {
    let _lock = SKIN.lock().unwrap_or_else(|poison| poison.into_inner());
    let mut leader = session(0, false, true);
    leader.click("guild:disband", &Default::default());
    leader.apply(state(1));
    assert_eq!(
        leader.click("guild:disband_confirm", &Default::default()),
        None
    );
    assert!(mounted(&leader).get_by_name("GuildDisbandPopup").is_none());
}
