//! Own test binary: every test holds SKIN while switching process-wide atlases.
use game_engine_ui_model::guild_rank_frame::{GOLD_BOX, NAME_BOX, guild_screen};
use game_engine_ui_model::guild_ranks::GuildRanksSession;
use shared::protocol::*;
use std::sync::Mutex;
use ui_toolkit::atlas::{ActiveSkin, set_active_skin};
use ui_toolkit::frame::WidgetData;
use ui_toolkit::registry::FrameRegistry;
use ui_toolkit::screen::{Screen, SharedContext};
static SKIN: Mutex<()> = Mutex::new(());

fn state(own_rank: u8) -> GuildRanksState {
    GuildRanksState {
        own_rank,
        ranks: ["Guild Master", "Officer", "Member", "Initiate"]
            .iter()
            .map(|name| GuildRankSettings {
                name: name.to_string(),
                rights: GUILD_RIGHT_CHAT_LISTEN | GUILD_RIGHT_WITHDRAW_GOLD,
                gold_per_day: 30_000,
                tabs: vec![GuildRankTab {
                    view: true,
                    deposit: false,
                    withdrawals_per_day: 2,
                }],
            })
            .collect(),
        members: vec![GuildRankMember {
            character_name: "Cara".into(),
            rank: 2,
        }],
        tab_names: vec!["Materials".into()],
        error: None,
    }
}
fn mounted(session: &GuildRanksSession) -> FrameRegistry {
    game_engine_ui_model::paths::set_data_root(
        std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../data"),
    )
    .unwrap();
    let mut shared = SharedContext::new();
    shared.insert(session.clone());
    let mut registry = FrameRegistry::new(1920.0, 1080.0);
    Screen::new(guild_screen).sync(&shared, &mut registry);
    registry
}
fn text(reg: &FrameRegistry, name: &str) -> String {
    let f = reg.get(reg.get_by_name(name).expect(name)).unwrap();
    match f.widget_data.as_ref().unwrap() {
        WidgetData::FontString(s) => s.text.clone(),
        WidgetData::Button(s) => s.text.clone(),
        WidgetData::EditBox(s) => s.text.clone(),
        _ => panic!("not text"),
    }
}
fn action(reg: &FrameRegistry, name: &str) -> String {
    reg.get(reg.get_by_name(name).expect(name))
        .unwrap()
        .onclick
        .clone()
        .unwrap_or_default()
}

#[test]
fn guild_rank_widgets_wait_for_authority_before_next_permission_write() {
    let _lock = SKIN.lock().unwrap_or_else(|poison| poison.into_inner());
    for skin in [ActiveSkin::Modern, ActiveSkin::Forever] {
        set_active_skin(skin);
        let mut session = GuildRanksSession::default();
        session.open();
        session.apply(state(0));
        session.select_rank(2);
        session.click("guild:settings", &Default::default());
        let old = mounted(&session);
        let first = session
            .click(&action(&old, "GuildPermission4"), &Default::default())
            .unwrap();
        assert_eq!(
            first,
            GuildRankRequest::SetPermissions {
                rank: 2,
                rights: GUILD_RIGHT_CHAT_LISTEN | GUILD_RIGHT_WITHDRAW_GOLD | GUILD_RIGHT_INVITE,
                gold_per_day: 30_000
            }
        );
        // Second queued click precedes the first wire reply: never overwrite its bit.
        assert!(
            session
                .click(&action(&old, "GuildPermission5"), &Default::default())
                .is_none()
        );
        assert!(action(&mounted(&session), "GuildPermission5").is_empty());
        assert_eq!(
            session.selected().unwrap().rights,
            GUILD_RIGHT_CHAT_LISTEN | GUILD_RIGHT_WITHDRAW_GOLD
        );
        let mut reply = state(0);
        reply.ranks[2].rights |= GUILD_RIGHT_INVITE;
        session.apply(reply);
        let fresh = mounted(&session);
        assert_eq!(
            session.click(&action(&fresh, "GuildPermission5"), &Default::default()),
            Some(GuildRankRequest::SetPermissions {
                rank: 2,
                rights: GUILD_RIGHT_CHAT_LISTEN
                    | GUILD_RIGHT_WITHDRAW_GOLD
                    | GUILD_RIGHT_INVITE
                    | GUILD_RIGHT_REMOVE,
                gold_per_day: 30_000
            })
        );
    }
    set_active_skin(ActiveSkin::Modern);
}

#[test]
fn guild_rank_widgets_guild_master_limits_render_unlimited() {
    let _lock = SKIN.lock().unwrap_or_else(|poison| poison.into_inner());
    for skin in [ActiveSkin::Modern, ActiveSkin::Forever] {
        set_active_skin(skin);
        let mut session = GuildRanksSession::default();
        session.open();
        session.apply(state(0));
        session.click("guild:settings", &Default::default());
        let reg = mounted(&session);
        assert_eq!(text(&reg, GOLD_BOX), "Unlimited");
        assert_eq!(text(&reg, "GuildTabItems0"), "Unlimited");
        assert!(action(&reg, "GuildGoldSave").is_empty());
        assert!(action(&reg, "GuildTabSave0").is_empty());
    }
    set_active_skin(ActiveSkin::Modern);
}

#[test]
fn guild_rank_widgets_gold_is_readonly_without_withdraw_or_repair_permission() {
    let _lock = SKIN.lock().unwrap_or_else(|poison| poison.into_inner());
    for skin in [ActiveSkin::Modern, ActiveSkin::Forever] {
        set_active_skin(skin);
        let mut session = GuildRanksSession::default();
        session.open();
        let mut snapshot = state(0);
        snapshot.ranks[2].rights = GUILD_RIGHT_CHAT_LISTEN;
        session.apply(snapshot);
        session.click("guild:settings", &Default::default());
        session.select_rank(2);
        let reg = mounted(&session);
        assert!(action(&reg, "GuildGoldSave").is_empty());
        assert!(matches!(
            reg.get(reg.get_by_name(GOLD_BOX).unwrap())
                .unwrap()
                .widget_data,
            Some(WidgetData::FontString(_))
        ));
    }
    set_active_skin(ActiveSkin::Modern);
}

#[test]
fn guild_rank_widgets_entry_is_available_in_the_native_micro_menu() {
    let _lock = SKIN.lock().unwrap_or_else(|poison| poison.into_inner());
    assert!(
        game_engine_ui_model::micro_menu::unavailable_message("micro:GuildMicroButton").is_none()
    );
}
#[test]
fn guild_rank_widgets_render_authority_and_click_exact_requests_both_skins() {
    let _lock = SKIN.lock().unwrap();
    for skin in [ActiveSkin::Modern, ActiveSkin::Forever] {
        set_active_skin(skin);
        let mut session = GuildRanksSession::default();
        session.open();
        session.apply(state(0));
        session.click("guild:settings", &Default::default());
        session.select_rank(2);
        let reg = mounted(&session);
        assert_eq!(text(&reg, "GuildRankRow2"), "Member");
        assert_eq!(text(&reg, GOLD_BOX), "3");
        assert_eq!(text(&reg, "GuildTabItems0"), "2");
        let inputs = [
            (NAME_BOX.to_owned(), "Raider".into()),
            (GOLD_BOX.to_owned(), "7".into()),
            ("GuildTabItems0".into(), "5".into()),
        ]
        .into();
        assert_eq!(
            session.click(&action(&reg, "GuildRankRename"), &inputs),
            Some(GuildRankRequest::Rename {
                rank: 2,
                name: "Raider".into()
            })
        );
        assert_eq!(
            session.click(&action(&reg, "GuildRankAdd"), &inputs),
            Some(GuildRankRequest::Add {
                name: "Raider".into()
            })
        );
        assert_eq!(
            session.click(&action(&reg, "GuildRankUp"), &inputs),
            Some(GuildRankRequest::Move { rank: 2, up: true })
        );
        assert_eq!(
            session.click(&action(&reg, "GuildRankDown"), &inputs),
            Some(GuildRankRequest::Move { rank: 2, up: false })
        );
        assert_eq!(
            session.click(&action(&reg, "GuildGoldSave"), &inputs),
            Some(GuildRankRequest::SetPermissions {
                rank: 2,
                rights: GUILD_RIGHT_CHAT_LISTEN | GUILD_RIGHT_WITHDRAW_GOLD,
                gold_per_day: 70_000
            })
        );
        assert_eq!(
            session.click(&action(&reg, "GuildPermission4"), &inputs),
            Some(GuildRankRequest::SetPermissions {
                rank: 2,
                rights: GUILD_RIGHT_CHAT_LISTEN | GUILD_RIGHT_WITHDRAW_GOLD | GUILD_RIGHT_INVITE,
                gold_per_day: 30_000
            })
        );
        assert_eq!(
            session.click(&action(&reg, "GuildTabDeposit0"), &inputs),
            Some(GuildRankRequest::SetTab {
                rank: 2,
                tab: 0,
                view: true,
                deposit: true,
                withdrawals_per_day: 2
            })
        );
        assert_eq!(
            session.click(&action(&reg, "GuildTabSave0"), &inputs),
            Some(GuildRankRequest::SetTab {
                rank: 2,
                tab: 0,
                view: true,
                deposit: false,
                withdrawals_per_day: 5
            })
        );
        assert_eq!(session.selected().unwrap().name, "Member");
        let mut reply = state(0);
        reply.ranks[2].name = "Raider".into();
        session.apply(reply);
        assert_eq!(text(&mounted(&session), "GuildRankRow2"), "Raider");
    }
    set_active_skin(ActiveSkin::Modern);
}
#[test]
fn guild_rank_widgets_disable_unauthorized_and_occupied_controls_both_skins() {
    let _lock = SKIN.lock().unwrap();
    for skin in [ActiveSkin::Modern, ActiveSkin::Forever] {
        set_active_skin(skin);
        let mut session = GuildRanksSession::default();
        session.open();
        session.apply(state(1));
        session.click("guild:settings", &Default::default());
        session.select_rank(2);
        let reg = mounted(&session);
        for name in [
            "GuildRankAdd",
            "GuildRankRename",
            "GuildRankRemove",
            "GuildRankUp",
            "GuildRankDown",
            "GuildPermission4",
            "GuildTabView0",
            "GuildTabDeposit0",
            "GuildTabSave0",
            "GuildGoldSave",
        ] {
            assert!(action(&reg, name).is_empty(), "{name} {skin:?}");
        }
        assert!(
            session
                .click("guild:permission:4", &Default::default())
                .is_none()
        );
        session.apply(state(0));
        assert!(action(&mounted(&session), "GuildRankRemove").is_empty());
        session.select_rank(3);
        let reg = mounted(&session);
        assert_eq!(
            session.click(&action(&reg, "GuildRankRemove"), &Default::default()),
            Some(GuildRankRequest::Remove { rank: 3 })
        );
    }
    set_active_skin(ActiveSkin::Modern);
}
#[test]
fn guild_rank_roster_pages_keep_all_members_reachable() {
    let _lock = SKIN.lock().unwrap_or_else(|poison| poison.into_inner());
    for skin in [ActiveSkin::Modern, ActiveSkin::Forever] {
        set_active_skin(skin);
        let mut session = GuildRanksSession::default();
        session.open();
        let mut snapshot = state(0);
        snapshot.members = (0..13)
            .map(|index| GuildRankMember {
                character_name: format!("Member{index}"),
                rank: 2,
            })
            .collect();
        session.apply(snapshot);
        let reg = mounted(&session);
        assert!(reg.get_by_name("GuildMember9").is_none());
        session.click(&action(&reg, "GuildRosterNext"), &Default::default());
        let reg = mounted(&session);
        session.click(&action(&reg, "GuildMember12"), &Default::default());
        assert_eq!(session.member_menu.as_deref(), Some("Member12"));
        session.click(&action(&reg, "GuildRosterPrevious"), &Default::default());
        assert!(mounted(&session).get_by_name("GuildMember0").is_some());
    }
    set_active_skin(ActiveSkin::Modern);
}

#[test]
fn guild_rank_roster_context_uses_strict_hierarchy_both_skins() {
    let _lock = SKIN.lock().unwrap();
    for skin in [ActiveSkin::Modern, ActiveSkin::Forever] {
        set_active_skin(skin);
        let mut session = GuildRanksSession::default();
        session.open();
        session.apply(state(0));
        session.click("guild:member:Cara", &Default::default());
        let reg = mounted(&session);
        assert_eq!(
            session.click(&action(&reg, "GuildMemberPromote"), &Default::default()),
            Some(GuildRankRequest::Promote {
                character_name: "Cara".into()
            })
        );
        assert_eq!(
            session.click(&action(&reg, "GuildMemberDemote"), &Default::default()),
            Some(GuildRankRequest::Demote {
                character_name: "Cara".into()
            })
        );
        session.apply(state(1));
        let reg = mounted(&session);
        assert!(reg.get_by_name("GuildMemberPromote").is_none());
        assert!(reg.get_by_name("GuildMemberDemote").is_none());
    }
    set_active_skin(ActiveSkin::Modern);
}
