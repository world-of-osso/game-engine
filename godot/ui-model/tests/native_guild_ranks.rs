use game_engine_ui_model::guild_ranks::GuildRanksSession;
use shared::protocol::*;

fn snapshot(own_rank: u8) -> GuildRanksState {
    let rank = |name: &str, rights| GuildRankSettings {
        name: name.into(),
        rights,
        gold_per_day: 30_000,
        tabs: vec![GuildRankTab {
            view: true,
            deposit: false,
            withdrawals_per_day: 2,
        }],
    };
    GuildRanksState {
        ranks: vec![
            rank("Guild Master", GUILD_RIGHT_ALL),
            rank("Officer", GUILD_RIGHT_PROMOTE | GUILD_RIGHT_DEMOTE),
            rank("Reader", GUILD_RIGHT_CHAT_LISTEN),
        ],
        members: vec![
            GuildRankMember {
                character_name: "Alice".into(),
                rank: 0,
            },
            GuildRankMember {
                character_name: "Bob".into(),
                rank: 1,
            },
            GuildRankMember {
                character_name: "Cara".into(),
                rank: 2,
            },
        ],
        own_rank,
        tab_names: vec!["Materials".into()],
        error: None,
    }
}

#[test]
fn guild_ranks_model_sends_exact_requests_without_optimistic_changes() {
    let mut session = GuildRanksSession::default();
    assert_eq!(session.open(), GuildRankRequest::Query);
    session.apply(snapshot(0));
    session.select_rank(2);
    assert_eq!(
        session.set_permission(GUILD_RIGHT_WITHDRAW_GOLD, true),
        Some(GuildRankRequest::SetPermissions {
            rank: 2,
            rights: GUILD_RIGHT_CHAT_LISTEN | GUILD_RIGHT_WITHDRAW_GOLD,
            gold_per_day: 30_000
        })
    );
    assert_eq!(
        session.set_gold_limit("7"),
        Some(GuildRankRequest::SetPermissions {
            rank: 2,
            rights: GUILD_RIGHT_CHAT_LISTEN,
            gold_per_day: 70_000
        })
    );
    assert_eq!(
        session.set_tab(0, true, false, "0"),
        Some(GuildRankRequest::SetTab {
            rank: 2,
            tab: 0,
            view: true,
            deposit: false,
            withdrawals_per_day: 0
        })
    );
    assert_eq!(session.selected().unwrap().gold_per_day, 30_000);
    assert_eq!(session.selected().unwrap().tabs[0].withdrawals_per_day, 2);
    let mut updated = snapshot(0);
    updated.ranks[2].name = "Bank Reader".into();
    updated.ranks[2].tabs[0].withdrawals_per_day = 0;
    session.apply(updated);
    assert_eq!(session.selected().unwrap().name, "Bank Reader");
    assert_eq!(session.selected().unwrap().tabs[0].withdrawals_per_day, 0);
    assert!(session.set_gold_limit("-1").is_none());
    assert!(session.set_tab(0, true, false, "bad").is_none());
    session.select_rank(0);
    assert!(
        session
            .set_permission(GUILD_RIGHT_CHAT_LISTEN, false)
            .is_none()
    );
}

#[test]
fn guild_ranks_model_respects_hierarchy_and_displays_server_refusal() {
    let mut session = GuildRanksSession::default();
    session.apply(snapshot(1));
    assert!(session.promote("Cara").is_none());
    assert!(session.demote("Alice").is_none());
    let mut state = snapshot(1);
    state.ranks.push(state.ranks[2].clone());
    state.members[2].rank = 3;
    session.apply(state);
    assert_eq!(
        session.promote("Cara"),
        Some(GuildRankRequest::Promote {
            character_name: "Cara".into()
        })
    );
    assert!(session.demote("Cara").is_none());
    session.select_rank(2);
    assert!(session.set_permission(GUILD_RIGHT_INVITE, true).is_none());
    let mut state = snapshot(1);
    state.error = Some(GuildRankError::Permissions);
    session.apply(state);
    assert_eq!(
        session.error.as_deref(),
        Some("You don't have permission to do that.")
    );
}

#[test]
fn guild_ranks_officer_chat_sends_and_maps_the_officer_channel() {
    use game_engine_ui_model::chat_data::{ChatChannelType, runtime_chat_channel};
    use game_engine_ui_model::chat_frame::{ChatCommand, parse_chat_input};
    assert_eq!(
        parse_chat_input("/o hello officers", None),
        ChatCommand::Send {
            channel: ChatType::Officer,
            text: "hello officers".into()
        }
    );
    assert_eq!(
        runtime_chat_channel(&ChatType::Officer, "Alice", None).0,
        ChatChannelType::Officer
    );
}

#[test]
fn guild_ranks_model_rank_list_controls_send_and_protect_occupied_ranks() {
    let mut session = GuildRanksSession::default();
    session.apply(snapshot(0));
    assert_eq!(
        session.add_rank("Initiate"),
        Some(GuildRankRequest::Add {
            name: "Initiate".into()
        })
    );
    session.select_rank(2);
    assert_eq!(
        session.rename_rank("Bank Reader"),
        Some(GuildRankRequest::Rename {
            rank: 2,
            name: "Bank Reader".into()
        })
    );
    assert_eq!(
        session.move_rank(true),
        Some(GuildRankRequest::Move { rank: 2, up: true })
    );
    assert!(session.remove_rank().is_none());
    let mut state = snapshot(0);
    state.members.pop();
    session.apply(state);
    assert_eq!(
        session.remove_rank(),
        Some(GuildRankRequest::Remove { rank: 2 })
    );
    session.select_rank(0);
    assert!(session.rename_rank("Boss").is_none());
    assert!(session.move_rank(false).is_none());
    assert!(session.remove_rank().is_none());
    assert!(session.add_rank("Name too long for Retail").is_none());
}
