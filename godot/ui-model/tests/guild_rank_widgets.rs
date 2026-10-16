//! Own test binary: every test holds SKIN while switching process-wide atlases.
use game_engine_ui_model::guild_rank_frame::{GOLD_BOX, guild_screen};
use game_engine_ui_model::guild_ranks::GuildRanksSession;
use shared::protocol::*;
use std::sync::Mutex;
use ui_toolkit::atlas::{ActiveSkin, set_thread_skin};
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
    game_engine_ui_model::guild_rank_frame::apply_guild_postsetup(session, &mut registry);
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
fn guild_control_retail_shell_and_officer_flag_use_source_geometry_both_skins() {
    use ui_toolkit::layout_values::Val;
    let _lock = SKIN.lock().unwrap_or_else(|p| p.into_inner());
    for skin in [ActiveSkin::Modern, ActiveSkin::Forever] {
        set_thread_skin(skin);
        let mut session = GuildRanksSession::default();
        session.open();
        let mut snapshot = state(0);
        snapshot.ranks[1].rights = GUILD_RIGHT_OFFICER_LISTEN | GUILD_RIGHT_EDIT_MOTD;
        session.apply(snapshot);
        session.select_rank(1);
        session.click("guild:settings", &Default::default());
        session.click("guild:control-page:1", &Default::default());
        let reg = mounted(&session);
        let shell = reg.get(reg.get_by_name("GuildControlUI").unwrap()).unwrap();
        assert_eq!((shell.width.value(), shell.height.value()), (338.0, 444.0));
        let officer = reg
            .get(
                reg.get_by_name("GuildControlUIRankSettingsFrameOfficerCheckbox")
                    .unwrap(),
            )
            .unwrap();
        assert_eq!(
            (officer.width.value(), officer.height.value()),
            (20.0, 20.0)
        );
        assert_eq!(
            (officer.position.left, officer.position.top),
            (Val::Px(11.0), Val::Px(61.0))
        );
        assert!(
            reg.get_by_name("GuildControlUIRankSettingsFrameOfficerCheckboxCheck")
                .is_some()
        );
        let toggle = action(&reg, "GuildControlUIRankSettingsFrameOfficerCheckbox");
        let request = session.click(&toggle, &Default::default()).unwrap();
        assert_eq!(
            request,
            GuildRankRequest::SetPermissions {
                rank: 1,
                rights: GUILD_RIGHT_EDIT_MOTD,
                gold_per_day: 30_000
            }
        );
        assert_eq!(
            session.selected().unwrap().rights,
            GUILD_RIGHT_OFFICER_LISTEN | GUILD_RIGHT_EDIT_MOTD
        );
        assert!(session.click(&toggle, &Default::default()).is_none());
        let mut reply = state(0);
        reply.ranks[1].rights = GUILD_RIGHT_EDIT_MOTD;
        session.apply(reply);
        assert_eq!(
            session.click(&toggle, &Default::default()),
            Some(GuildRankRequest::SetPermissions {
                rank: 1,
                rights: GUILD_RIGHT_EDIT_MOTD
                    | GUILD_RIGHT_OFFICER_LISTEN
                    | GUILD_RIGHT_OFFICER_SPEAK,
                gold_per_day: 30_000
            })
        );
    }
}

#[test]
fn guild_communities_invite_and_profile_controls_send_authoritative_actions_both_skins() {
    use game_engine_ui_model::guild_ranks::GuildCommand;
    let _lock = SKIN.lock().unwrap_or_else(|poison| poison.into_inner());
    for skin in [ActiveSkin::Modern, ActiveSkin::Forever] {
        set_thread_skin(skin);
        let mut session = GuildRanksSession::default();
        session.open();
        session.apply(state(0));
        session.profile = Some(GuildSnapshot {
            guild_id: 7,
            guild_name: "Raid Team".into(),
            created_at: None,
            tabard: None,
            motd: "Meeting tonight".into(),
            info_text: "Raid Tuesdays".into(),
            members: vec![],
        });
        assert!(
            mounted(&session)
                .get_by_name("GuildRosterInviteButton")
                .is_none()
        );
        session.click("guild:chat_open", &Default::default());
        let registry = mounted(&session);
        assert_eq!(
            action(&registry, "GuildRosterInviteButton"),
            "guild:invite_open"
        );
        session.click("guild:invite_open", &Default::default());
        let registry = mounted(&session);
        assert_eq!(
            action(&registry, "GuildInviteMemberAccept"),
            "guild:invite_send"
        );
        session.click(
            "guild:invite_send",
            &[("GuildInviteMemberName".into(), "Cara".into())]
                .into_iter()
                .collect(),
        );
        assert_eq!(session.commands, vec![GuildCommand::Invite("Cara".into())]);
        session.commands.clear();
        session.click("guild:info_open", &Default::default());
        let registry = mounted(&session);
        assert_eq!(text(&registry, "GuildMOTDText"), "Meeting tonight");
        assert_eq!(text(&registry, "GuildInfoText"), "Raid Tuesdays");
        session.click("guild:edit_motd", &Default::default());
        session.click(
            "guild:profile_accept",
            &[("GuildProfileEditText".into(), "Meeting tomorrow".into())]
                .into_iter()
                .collect(),
        );
        assert_eq!(
            session.commands,
            vec![GuildCommand::SetMotd("Meeting tomorrow".into())]
        );
        assert_eq!(session.profile.as_ref().unwrap().motd, "Meeting tonight");
    }
}

#[test]
fn guild_info_editor_preserves_multiline_text_limits_and_authoritative_pacing() {
    use game_engine_ui_model::guild_ranks::{GuildCommand, GuildProfileField};
    let _lock = SKIN.lock().unwrap_or_else(|poison| poison.into_inner());
    for skin in [ActiveSkin::Modern, ActiveSkin::Forever] {
        set_thread_skin(skin);
        let mut session = GuildRanksSession::default();
        session.open();
        session.apply(state(0));
        session.profile = Some(GuildSnapshot {
            guild_id: 7,
            guild_name: "Raid Team".into(),
            created_at: None,
            tabard: None,
            motd: "Meeting".into(),
            info_text: "Tuesday raid\nThursday dungeon".into(),
            members: vec![],
        });
        session.click("guild:edit_info", &Default::default());
        assert_eq!(session.profile_editor, Some(GuildProfileField::Info));
        let registry = mounted(&session);
        let frame = registry
            .get(registry.get_by_name("GuildProfileEditText").unwrap())
            .unwrap();
        let Some(WidgetData::EditBox(editor)) = &frame.widget_data else {
            panic!("missing editor");
        };
        assert!(editor.multi_line);
        assert_eq!(editor.max_letters, Some(499));
        assert_eq!(editor.text, "Tuesday raid\nThursday dungeon");
        let texts = [(
            "GuildProfileEditText".into(),
            "Friday raid\nSunday dungeon".into(),
        )]
        .into_iter()
        .collect();
        session.click("guild:profile_accept", &texts);
        session.click("guild:edit_info", &Default::default());
        session.click("guild:profile_accept", &texts);
        assert_eq!(
            session.commands,
            vec![GuildCommand::SetInfo("Friday raid\nSunday dungeon".into())]
        );
        assert_eq!(
            session.profile.as_ref().unwrap().info_text,
            "Tuesday raid\nThursday dungeon"
        );
        session.apply_profile(GuildStateUpdate {
            guild: None,
            message: None,
            error: Some("You don't have permission to do that.".into()),
        });
        assert!(!session.profile_waiting);
        assert_eq!(
            session.profile.as_ref().unwrap().info_text,
            "Tuesday raid\nThursday dungeon"
        );
        session.apply(state(2));
        session.click("guild:edit_info", &Default::default());
        assert!(session.profile_editor.is_none());
    }
}

#[test]
fn guild_invitation_decline_keeps_source_size_right_and_bottom_anchors() {
    use ui_toolkit::layout_values::Val;
    let _lock = SKIN.lock().unwrap_or_else(|poison| poison.into_inner());
    for skin in [ActiveSkin::Modern, ActiveSkin::Forever] {
        set_thread_skin(skin);
        let mut session = GuildRanksSession::default();
        session.invitation = Some(GuildInvitation {
            guild_id: 7,
            guild_name: "Raid Team".into(),
            inviter_name: "Alice".into(),
        });
        let registry = mounted(&session);
        let decline = registry
            .get(
                registry
                    .get_by_name("GuildInviteFrameDeclineButton")
                    .unwrap(),
            )
            .unwrap();
        // GuildInviteFrame.xml:140×22, BOTTOMRIGHT(-24,+24) in310×188.
        assert_eq!(
            (decline.width.value(), decline.height.value()),
            (140.0, 22.0)
        );
        assert_eq!(
            (decline.position.left, decline.position.top),
            (Val::Px(146.0), Val::Px(142.0))
        );
        let join = registry
            .get(registry.get_by_name("GuildInviteFrameJoinButton").unwrap())
            .unwrap();
        assert_eq!((join.width.value(), join.height.value()), (110.0, 22.0));
        assert_eq!(
            (join.position.left, join.position.top),
            (Val::Px(24.0), Val::Px(142.0))
        );
    }
    set_thread_skin(ActiveSkin::Modern);
}

#[test]
fn guild_invitation_uses_source_translucent_dialog_chrome_in_both_skins() {
    use ui_toolkit::layout_values::Val;
    use ui_toolkit::widgets::texture::TextureSource;
    let _lock = SKIN.lock().unwrap_or_else(|poison| poison.into_inner());
    for skin in [ActiveSkin::Modern, ActiveSkin::Forever] {
        set_thread_skin(skin);
        let mut session = GuildRanksSession::default();
        session.invitation = Some(GuildInvitation {
            guild_id: 7,
            guild_name: "Raid Team".into(),
            inviter_name: "Alice".into(),
        });
        let registry = mounted(&session);
        // TranslucentFrameTemplate:10px black0.9 inset and32px quadrant corners.
        let background = registry
            .get(
                registry
                    .get_by_name("GuildInviteFrameBg")
                    .expect("source inset"),
            )
            .unwrap();
        assert_eq!(background.background_color, Some([0.0, 0.0, 0.0, 0.9]));
        assert_eq!(
            (background.position.left, background.position.top),
            (Val::Px(10.0), Val::Px(10.0))
        );
        assert_eq!(
            (background.width.value(), background.height.value()),
            (290.0, 168.0)
        );
        for (suffix, x, y, uv) in [
            ("TopLeftCorner", 0.0, 0.0, [0.0, 0.5, 0.0, 0.5]),
            ("TopRightCorner", 278.0, 0.0, [0.5, 1.0, 0.0, 0.5]),
            ("BottomLeftCorner", 0.0, 156.0, [0.0, 0.5, 0.5, 1.0]),
            ("BottomRightCorner", 278.0, 156.0, [0.5, 1.0, 0.5, 1.0]),
        ] {
            let name = format!("GuildInviteFrame{suffix}");
            let frame = registry
                .get(registry.get_by_name(&name).expect(&name))
                .unwrap();
            assert_eq!(
                (frame.position.left, frame.position.top),
                (Val::Px(x), Val::Px(y))
            );
            assert_eq!((frame.width.value(), frame.height.value()), (32.0, 32.0));
            let Some(WidgetData::Texture(texture)) = &frame.widget_data else {
                panic!("source corner texture")
            };
            assert_eq!(texture.source, TextureSource::FileDataId(407329));
            assert_eq!(texture.tex_coords, uv);
        }
        for (suffix, fdid, x, y, width, height, horizontal) in [
            ("TopBorder", 407332, 32.0, 1.0, 246.0, 16.0, true),
            ("BottomBorder", 407328, 32.0, 172.0, 246.0, 16.0, true),
            ("LeftBorder", 407330, 1.0, 32.0, 16.0, 124.0, false),
            ("RightBorder", 407331, 294.0, 32.0, 16.0, 124.0, false),
        ] {
            let name = format!("GuildInviteFrame{suffix}");
            let frame = registry
                .get(registry.get_by_name(&name).expect(&name))
                .unwrap();
            assert_eq!(
                (frame.position.left, frame.position.top),
                (Val::Px(x), Val::Px(y))
            );
            assert_eq!((frame.width.value(), frame.height.value()), (width, height));
            let Some(WidgetData::Texture(texture)) = &frame.widget_data else {
                panic!("source edge texture")
            };
            assert_eq!(texture.source, TextureSource::FileDataId(fdid));
            assert_eq!(
                (texture.horiz_tile, texture.vert_tile),
                (horizontal, !horizontal)
            );
        }
        assert_eq!(text(&registry, "GuildInviteFrameInviterName"), "Alice");
        assert_eq!(text(&registry, "GuildInviteFrameGuildName"), "Raid Team");
        assert!(session.commands.is_empty());
    }
    set_thread_skin(ActiveSkin::Modern);
}

#[test]
fn guild_invitation_popup_works_without_open_roster_in_both_skins() {
    let _lock = SKIN.lock().unwrap_or_else(|poison| poison.into_inner());
    for skin in [ActiveSkin::Modern, ActiveSkin::Forever] {
        set_thread_skin(skin);
        let mut session = GuildRanksSession::default();
        session.invitation = Some(GuildInvitation {
            guild_id: 7,
            guild_name: "Raid Team".into(),
            inviter_name: "Alice".into(),
        });
        let registry = mounted(&session);
        assert_eq!(text(&registry, "GuildInviteFrameInviterName"), "Alice");
        assert_eq!(text(&registry, "GuildInviteFrameGuildName"), "Raid Team");
        assert_eq!(
            action(&registry, "GuildInviteFrameJoinButton"),
            "guild:accept_invite"
        );
        assert_eq!(
            action(&registry, "GuildInviteFrameDeclineButton"),
            "guild:decline_invite"
        );
        session.click("guild:accept_invite", &Default::default());
        session.click("guild:accept_invite", &Default::default());
        assert_eq!(
            session.commands,
            vec![game_engine_ui_model::guild_ranks::GuildCommand::AcceptInvite]
        );
        let mut declined = GuildRanksSession::default();
        declined.invitation = session.invitation.clone();
        declined.click("guild:decline_invite", &Default::default());
        assert_eq!(
            declined.commands,
            vec![game_engine_ui_model::guild_ranks::GuildCommand::DeclineInvite]
        );
    }
}

#[test]
fn guild_rank_widgets_render_only_purchased_bank_tabs() {
    let _lock = SKIN.lock().unwrap_or_else(|p| p.into_inner());
    for skin in [ActiveSkin::Modern, ActiveSkin::Forever] {
        set_thread_skin(skin);
        let mut session = GuildRanksSession::default();
        session.open();
        let mut snapshot = state(0);
        let settings = snapshot.ranks[2].tabs[0].clone();
        snapshot.ranks[2].tabs.resize(GUILD_BANK_MAX_TABS, settings);
        session.apply(snapshot);
        session.select_rank(2);
        session.click("guild:settings", &Default::default());
        session.click("guild:control-page:2", &Default::default());
        let reg = mounted(&session);
        assert!(reg.get_by_name("GuildTabName0").is_some());
        assert!(reg.get_by_name("GuildTabName1").is_none());
        assert!(session.click("guild:view:1", &Default::default()).is_none());
        let mut empty = state(0);
        empty.tab_names.clear();
        session.apply(empty);
        let reg = mounted(&session);
        assert!(reg.get_by_name("GuildTabName0").is_none());
        assert_eq!(
            text(&reg, "GuildBankNoTabs"),
            "Your guild has not purchased any guild bank space."
        );
    }
}
#[test]
fn guild_rank_widgets_wait_for_authority_before_next_permission_write() {
    let _lock = SKIN.lock().unwrap_or_else(|poison| poison.into_inner());
    for skin in [ActiveSkin::Modern, ActiveSkin::Forever] {
        set_thread_skin(skin);
        let mut session = GuildRanksSession::default();
        session.open();
        session.apply(state(0));
        session.select_rank(2);
        session.click("guild:settings", &Default::default());
        session.click("guild:control-page:1", &Default::default());
        let old = mounted(&session);
        let first = session
            .click(
                &action(&old, "GuildControlUIRankSettingsFrameCheckbox7"),
                &Default::default(),
            )
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
                .click(
                    &action(&old, "GuildControlUIRankSettingsFrameCheckbox8"),
                    &Default::default()
                )
                .is_none()
        );
        assert!(
            action(
                &mounted(&session),
                "GuildControlUIRankSettingsFrameCheckbox8"
            )
            .is_empty()
        );
        assert_eq!(
            session.selected().unwrap().rights,
            GUILD_RIGHT_CHAT_LISTEN | GUILD_RIGHT_WITHDRAW_GOLD
        );
        let mut reply = state(0);
        reply.ranks[2].rights |= GUILD_RIGHT_INVITE;
        session.apply(reply);
        let fresh = mounted(&session);
        assert_eq!(
            session.click(
                &action(&fresh, "GuildControlUIRankSettingsFrameCheckbox8"),
                &Default::default()
            ),
            Some(GuildRankRequest::SetPermissions {
                rank: 2,
                rights: GUILD_RIGHT_CHAT_LISTEN
                    | GUILD_RIGHT_WITHDRAW_GOLD
                    | GUILD_RIGHT_INVITE
                    | GUILD_RIGHT_REMOVE,
                gold_per_day: 30_000
            })
        );
        let mut refused = state(0);
        refused.ranks[2].rights |= GUILD_RIGHT_INVITE;
        refused.error = Some(GuildRankError::Permissions);
        session.apply(refused);
        assert_eq!(
            session.error.as_deref(),
            Some("You don't have permission to do that.")
        );
        assert!(
            !action(
                &mounted(&session),
                "GuildControlUIRankSettingsFrameCheckbox8"
            )
            .is_empty()
        );
        assert_eq!(session.selected().unwrap().rights & GUILD_RIGHT_REMOVE, 0);
    }
    set_thread_skin(ActiveSkin::Modern);
}

#[test]
fn guild_rank_widgets_source_rank_menu_excludes_immutable_guild_master() {
    let _lock = SKIN.lock().unwrap_or_else(|p| p.into_inner());
    for skin in [ActiveSkin::Modern, ActiveSkin::Forever] {
        set_thread_skin(skin);
        let mut session = GuildRanksSession::default();
        session.open();
        session.apply(state(0));
        session.click("guild:settings", &Default::default());
        let reg = mounted(&session);
        assert!(matches!(
            reg.get(reg.get_by_name("GuildRankOrderName0").unwrap())
                .unwrap()
                .widget_data,
            Some(WidgetData::FontString(_))
        ));
        assert!(action(&reg, "GuildControlUIRankOrderFrameRank1ShiftUpButton").is_empty());
        session.click("guild:control-page:1", &Default::default());
        session.click("guild:rank-menu", &Default::default());
        let reg = mounted(&session);
        assert!(reg.get_by_name("GuildControlRankChoice0").is_none());
        assert_eq!(action(&reg, "GuildControlRankChoice1"), "guild:rank:1");
        assert_eq!(session.selected_index(), 1);
    }
}
#[test]
fn guild_rank_widgets_gold_is_readonly_without_withdraw_or_repair_permission() {
    let _lock = SKIN.lock().unwrap_or_else(|p| p.into_inner());
    for skin in [ActiveSkin::Modern, ActiveSkin::Forever] {
        set_thread_skin(skin);
        let mut session = GuildRanksSession::default();
        session.open();
        let mut snapshot = state(0);
        snapshot.ranks[2].rights = GUILD_RIGHT_CHAT_LISTEN;
        session.apply(snapshot);
        session.select_rank(2);
        session.click("guild:settings", &Default::default());
        session.click("guild:control-page:1", &Default::default());
        let reg = mounted(&session);
        assert!(matches!(
            reg.get(reg.get_by_name(GOLD_BOX).unwrap())
                .unwrap()
                .widget_data,
            Some(WidgetData::FontString(_))
        ));
        assert!(
            reg.get_by_name("GuildGoldSave").is_none(),
            "No invented Save control"
        );
    }
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
    let _lock = SKIN.lock().unwrap_or_else(|p| p.into_inner());
    for skin in [ActiveSkin::Modern, ActiveSkin::Forever] {
        set_thread_skin(skin);
        let mut session = GuildRanksSession::default();
        session.open();
        session.apply(state(0));
        session.click("guild:settings", &Default::default());
        session.select_rank(2);
        let reg = mounted(&session);
        assert_eq!(text(&reg, "GuildRankOrderName2"), "Member");
        let inputs = [
            ("GuildRankOrderName2".into(), "Raider".into()),
            (GOLD_BOX.into(), "7".into()),
            ("GuildTabItems0".into(), "5".into()),
        ]
        .into();
        let rename =
            game_engine_ui_model::guild_control::input_action("GuildRankOrderName2").unwrap();
        assert_eq!(
            session.clone().click(&rename, &inputs),
            Some(GuildRankRequest::Rename {
                rank: 2,
                name: "Raider".into()
            })
        );
        assert_eq!(
            session.clone().click(
                &action(&reg, "GuildControlUIRankOrderFrameNewButton"),
                &inputs
            ),
            Some(GuildRankRequest::Add {
                name: "New Rank".into()
            })
        );
        assert_eq!(
            session.clone().click(
                &action(&reg, "GuildControlUIRankOrderFrameRank3ShiftUpButton"),
                &inputs
            ),
            Some(GuildRankRequest::Move { rank: 2, up: true })
        );
        assert_eq!(
            session.clone().click(
                &action(&reg, "GuildControlUIRankOrderFrameRank3ShiftDownButton"),
                &inputs
            ),
            Some(GuildRankRequest::Move { rank: 2, up: false })
        );
        session.click("guild:control-page:1", &Default::default());
        let reg = mounted(&session);
        assert_eq!(text(&reg, GOLD_BOX), "3");
        assert_eq!(
            session.clone().click(
                &game_engine_ui_model::guild_control::input_action(GOLD_BOX).unwrap(),
                &inputs
            ),
            Some(GuildRankRequest::SetPermissions {
                rank: 2,
                rights: GUILD_RIGHT_CHAT_LISTEN | GUILD_RIGHT_WITHDRAW_GOLD,
                gold_per_day: 70_000
            })
        );
        assert_eq!(
            session.clone().click(
                &action(&reg, "GuildControlUIRankSettingsFrameCheckbox7"),
                &inputs
            ),
            Some(GuildRankRequest::SetPermissions {
                rank: 2,
                rights: GUILD_RIGHT_CHAT_LISTEN | GUILD_RIGHT_WITHDRAW_GOLD | GUILD_RIGHT_INVITE,
                gold_per_day: 30_000
            })
        );
        session.click("guild:control-page:2", &Default::default());
        let reg = mounted(&session);
        assert_eq!(text(&reg, "GuildTabItems0"), "2");
        assert_eq!(
            session
                .clone()
                .click(&action(&reg, "GuildTabDeposit0"), &inputs),
            Some(GuildRankRequest::SetTab {
                rank: 2,
                tab: 0,
                view: true,
                deposit: true,
                withdrawals_per_day: 2
            })
        );
        assert_eq!(
            session.clone().click(
                &game_engine_ui_model::guild_control::input_action("GuildTabItems0").unwrap(),
                &inputs
            ),
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
        session.click("guild:control-page:0", &Default::default());
        assert_eq!(session.selected_index(), 2);
        assert_eq!(text(&mounted(&session), "GuildRankOrderName2"), "Raider");
    }
}
#[test]
fn guild_rank_widgets_disable_unauthorized_and_occupied_controls_both_skins() {
    let _lock = SKIN.lock().unwrap_or_else(|p| p.into_inner());
    for skin in [ActiveSkin::Modern, ActiveSkin::Forever] {
        set_thread_skin(skin);
        let mut session = GuildRanksSession::default();
        session.open();
        session.apply(state(1));
        session.click("guild:settings", &Default::default());
        session.select_rank(2);
        let reg = mounted(&session);
        for name in [
            "GuildControlUIRankOrderFrameNewButton",
            "GuildControlUIRankOrderFrameRank3DeleteButton",
            "GuildControlUIRankOrderFrameRank3ShiftUpButton",
            "GuildControlUIRankOrderFrameRank3ShiftDownButton",
        ] {
            assert!(action(&reg, name).is_empty());
        }
        session.click("guild:control-page:1", &Default::default());
        assert!(
            action(
                &mounted(&session),
                "GuildControlUIRankSettingsFrameCheckbox7"
            )
            .is_empty()
        );
        assert!(
            session
                .click("guild:retail-permission:7", &Default::default())
                .is_none()
        );
        session.click("guild:control-page:2", &Default::default());
        let reg = mounted(&session);
        for name in ["GuildTabView0", "GuildTabDeposit0"] {
            assert!(action(&reg, name).is_empty());
        }
        session.apply(state(0));
        session.click("guild:control-page:0", &Default::default());
        let reg = mounted(&session);
        assert!(action(&reg, "GuildControlUIRankOrderFrameRank3DeleteButton").is_empty());
        assert_eq!(
            session.click(
                &action(&reg, "GuildControlUIRankOrderFrameRank4DeleteButton"),
                &Default::default()
            ),
            Some(GuildRankRequest::Remove { rank: 3 })
        );
    }
}
#[test]
fn guild_communities_roster_uses_profile_metadata_without_exposing_officer_notes_as_public_notes() {
    let _lock = SKIN.lock().unwrap_or_else(|p| p.into_inner());
    let mut session = GuildRanksSession::default();
    session.open();
    session.apply(state(0));
    session.profile = Some(GuildSnapshot {
        guild_id: 7,
        guild_name: "Raid Team".into(),
        created_at: None,
        tabard: None,
        motd: String::new(),
        info_text: String::new(),
        members: vec![GuildMemberSnapshot {
            character_name: "Cara".into(),
            level: 80,
            class_name: "Paladin".into(),
            rank_name: "Member".into(),
            is_online: false,
            officer_note: "Private strategy".into(),
            public_note: String::new(),
            is_self: false,
            last_online: "Yesterday".into(),
        }],
    });
    let reg = mounted(&session);
    assert_eq!(text(&reg, "GuildMember0Level"), "80");
    assert_eq!(text(&reg, "GuildMember0Zone"), "Yesterday");
    assert_eq!(text(&reg, "GuildMember0Note"), "");
    assert_eq!(text(&reg, "GuildMember0Name"), "Cara");
    assert!(reg.get_by_name("GuildMember0Class").is_some());
}

#[test]
fn guild_communities_no_design_portrait_matches_retail_both_skins() {
    use ui_toolkit::layout_values::Val;
    use ui_toolkit::widgets::texture::TextureSource;
    let _lock = SKIN.lock().unwrap_or_else(|p| p.into_inner());
    for skin in [ActiveSkin::Modern, ActiveSkin::Forever] {
        set_thread_skin(skin);
        let mut session = GuildRanksSession::default();
        session.open();
        session.apply(state(0));
        let reg = mounted(&session);
        // CommunitiesFrame.xml328–353; GuildUtil.lua115–127,180–202.
        for (name, fdid, rect, uv, color) in [
            (
                "CommunitiesTabardBackground",
                410251,
                (-5.0, -8.0, 60.0, 60.0),
                [0.63183594, 0.69238281, 0.61914063, 0.74023438],
                [0.2245, 0.2088, 0.1794, 1.0],
            ),
            (
                "CommunitiesTabardEmblem",
                460904,
                (-7.0, -9.0, 64.0, 64.0),
                [0.0, 1.0, 0.0, 1.0],
                [1.0; 4],
            ),
            (
                "CommunitiesTabardBorder",
                410251,
                (-5.0, -7.0, 60.0, 59.0),
                [0.63183594, 0.69238281, 0.74414063, 0.86523438],
                [0.2, 0.2, 0.2, 1.0],
            ),
        ] {
            let frame = reg.get(reg.get_by_name(name).expect(name)).unwrap();
            assert_eq!(
                (frame.position.left, frame.position.top),
                (Val::Px(rect.0), Val::Px(rect.1))
            );
            assert_eq!(
                (frame.width.value(), frame.height.value()),
                (rect.2, rect.3)
            );
            let Some(WidgetData::Texture(texture)) = &frame.widget_data else {
                panic!("{name} must render texture");
            };
            assert_eq!(texture.source, TextureSource::FileDataId(fdid));
            assert_eq!(texture.tex_coords, uv);
            assert_eq!(texture.vertex_color, color);
        }
    }
}

#[test]
fn guild_communities_registered_design_uses_authoritative_sheet_palette_and_narrow_width() {
    use shared::guild_creation::GuildTabardDesign;
    use ui_toolkit::widgets::texture::TextureSource;
    let _lock = SKIN.lock().unwrap_or_else(|p| p.into_inner());
    for skin in [ActiveSkin::Modern, ActiveSkin::Forever] {
        set_thread_skin(skin);
        let mut session = GuildRanksSession::default();
        session.open();
        session.apply(state(0));
        session.profile = Some(GuildSnapshot {
            guild_id: 7,
            guild_name: "Raid Team".into(),
            created_at: None,
            motd: String::new(),
            info_text: String::new(),
            members: vec![],
            tabard: Some(GuildTabardDesign {
                icon: 1,
                icon_color: 2,
                border: 3,
                border_color: 4,
                background_color: 5,
            }),
        });
        let registry = mounted(&session);
        let emblem = registry
            .get(registry.get_by_name("CommunitiesTabardEmblem").unwrap())
            .unwrap();
        assert_eq!((emblem.width.value(), emblem.height.value()), (56.0, 64.0));
        let Some(WidgetData::Texture(texture)) = &emblem.widget_data else {
            panic!("emblem texture");
        };
        assert_eq!(texture.source, TextureSource::FileDataId(459023));
        assert_eq!(texture.tex_coords, [0.0625, 0.125, 0.0, 0.0625]);
        assert_eq!(
            texture.vertex_color,
            [103.0 / 255.0, 69.0 / 255.0, 0.0, 1.0]
        );
        for (name, color) in [
            (
                "CommunitiesTabardBackground",
                [177.0 / 255.0, 0.0, 46.0 / 255.0, 1.0],
            ),
            (
                "CommunitiesTabardBorder",
                [99.0 / 255.0, 148.0 / 255.0, 0.0, 1.0],
            ),
        ] {
            let layer = registry.get(registry.get_by_name(name).unwrap()).unwrap();
            let Some(WidgetData::Texture(texture)) = &layer.widget_data else {
                panic!("tabard layer");
            };
            assert_eq!(texture.vertex_color, color);
        }
    }
}

#[test]
fn guild_communities_column_sort_keeps_authoritative_rank_identity() {
    let _lock = SKIN.lock().unwrap_or_else(|p| p.into_inner());
    let mut session = GuildRanksSession::default();
    session.open();
    let mut snapshot = state(0);
    snapshot.members = vec![
        GuildRankMember {
            character_name: "Zara".into(),
            rank: 1,
        },
        GuildRankMember {
            character_name: "Alessio".into(),
            rank: 2,
        },
    ];
    session.apply(snapshot.clone());
    let reg = mounted(&session);
    session.click(
        &action(&reg, "CommunitiesRosterColumn2"),
        &Default::default(),
    );
    assert_eq!(text(&mounted(&session), "GuildMember0Name"), "Alessio");
    session.click("guild:member:Alessio", &Default::default());
    assert_eq!(
        session.promote("Alessio"),
        Some(GuildRankRequest::Promote {
            character_name: "Alessio".into()
        })
    );
    session.click("guild:roster-sort:name", &Default::default());
    assert_eq!(text(&mounted(&session), "GuildMember0Name"), "Zara");
    session.click("guild:roster-sort:rank", &Default::default());
    assert_eq!(text(&mounted(&session), "GuildMember0Name"), "Zara");
    assert_eq!(
        session.state().unwrap(),
        &snapshot,
        "Sorting never mutates authoritative ranks or member positions"
    );
}

#[test]
fn guild_communities_metadata_columns_sort_numeric_values_and_preserve_member_identity() {
    let _lock = SKIN.lock().unwrap_or_else(|poison| poison.into_inner());
    for skin in [ActiveSkin::Modern, ActiveSkin::Forever] {
        set_thread_skin(skin);
        let mut session = GuildRanksSession::default();
        session.open();
        let mut snapshot = state(0);
        snapshot.members = ["Zara", "Cara", "Alessio", "Unknown"]
            .into_iter()
            .map(|name| GuildRankMember {
                character_name: name.into(),
                rank: 2,
            })
            .collect();
        session.apply(snapshot.clone());
        session.profile = Some(GuildSnapshot {
            guild_id: 7,
            guild_name: "Raid Team".into(),
            created_at: None,
            tabard: None,
            motd: String::new(),
            info_text: String::new(),
            // Deliberately not in the rank snapshot's member order.
            members: [
                ("Alessio", 80, "Paladin"),
                ("Zara", 20, "Druid"),
                ("Cara", 8, "Mage"),
            ]
            .into_iter()
            .map(|(name, level, class_name)| GuildMemberSnapshot {
                character_name: name.into(),
                level,
                class_name: class_name.into(),
                rank_name: "Member".into(),
                is_online: true,
                officer_note: "Private".into(),
                public_note: String::new(),
                is_self: false,
                last_online: String::new(),
            })
            .collect(),
        });
        let profile = session.profile.clone();
        // Retail numeric columns begin ascending; missing attributes begin first.
        // Class uses IDs2/8/11, not alphabetic Druid/Mage/Paladin or level ordering.
        for (column, ascending, descending) in [
            (
                0,
                ["Unknown", "Cara", "Zara", "Alessio"],
                ["Alessio", "Zara", "Cara", "Unknown"],
            ),
            (
                1,
                ["Unknown", "Alessio", "Cara", "Zara"],
                ["Zara", "Cara", "Alessio", "Unknown"],
            ),
        ] {
            for expected in [ascending, descending] {
                let reg = mounted(&session);
                let header = format!("CommunitiesRosterColumn{column}");
                let click = action(&reg, &header);
                assert!(
                    !click.is_empty(),
                    "{header} must accept a physical header click"
                );
                assert!(session.click(&click, &Default::default()).is_none());
                let reg = mounted(&session);
                for (index, name) in expected.into_iter().enumerate() {
                    assert_eq!(text(&reg, &format!("GuildMember{index}Name")), name);
                    assert_eq!(
                        action(&reg, &format!("GuildMember{index}")),
                        format!("guild:member:{name}")
                    );
                }
                assert_eq!(session.state().unwrap(), &snapshot);
                assert_eq!(session.profile, profile);
                assert!(session.commands.is_empty());
            }
        }
        assert_eq!(
            session.promote("Cara"),
            Some(GuildRankRequest::Promote {
                character_name: "Cara".into()
            })
        );
    }
    set_thread_skin(ActiveSkin::Modern);
}

#[test]
fn guild_communities_retail_scroll_reaches_last_member_both_skins() {
    let _lock = SKIN.lock().unwrap_or_else(|poison| poison.into_inner());
    for skin in [ActiveSkin::Modern, ActiveSkin::Forever] {
        set_thread_skin(skin);
        let mut session = GuildRanksSession::default();
        session.open();
        let mut snapshot = state(0);
        snapshot.members = (0..32)
            .map(|index| GuildRankMember {
                character_name: format!("Member{index}"),
                rank: 2,
            })
            .collect();
        session.apply(snapshot);
        let mut reg = mounted(&session);
        let root = reg
            .get(reg.get_by_name("CommunitiesFrame").unwrap())
            .unwrap();
        assert_eq!((root.width.value(), root.height.value()), (814.0, 426.0));
        assert!(reg.get_by_name("GuildRosterNext").is_none());
        assert!(reg.get_by_name("GuildRosterPrevious").is_none());
        assert!(reg.get_by_name("GuildRosterPage").is_none());
        let list = reg
            .get(reg.get_by_name("CommunitiesMemberListScrollBox").unwrap())
            .unwrap();
        assert_eq!((list.width.value(), list.height.value()), (592.0, 315.0));
        assert!(
            reg.scroll_lists
                .scroll_to("CommunitiesMemberListScrollBox", 10000)
        );
        let mut shared = SharedContext::new();
        shared.insert(session.clone());
        Screen::new(guild_screen).sync(&shared, &mut reg);
        assert_eq!(
            reg.scroll_lists
                .get("CommunitiesMemberListScrollBox")
                .unwrap()
                .first_row,
            325
        );
        let row = reg.get(reg.get_by_name("GuildMember31").unwrap()).unwrap();
        assert_eq!((row.width.value(), row.height.value()), (592.0, 20.0));
        session.click(&action(&reg, "GuildMember31"), &Default::default());
        assert_eq!(session.member_detail.as_deref(), Some("Member31"));
        assert_eq!(session.member_menu, None);
        assert_eq!(text(&reg, "GuildMember31Name"), "Member31");
        assert_eq!(text(&reg, "GuildMember31Rank"), "Member");
        for (name, top, action_name) in [
            ("GuildRosterTab", 88.0, "guild:roster_open"),
            ("GuildInfoTab", 192.0, "guild:info_open"),
        ] {
            let tab = reg.get(reg.get_by_name(name).unwrap()).unwrap();
            assert_eq!((tab.width.value(), tab.height.value()), (32.0, 32.0));
            assert_eq!(tab.position.top, ui_toolkit::layout_values::Val::Px(top));
            assert_eq!(action(&reg, name), action_name);
        }
        for (name, tab_action) in [
            ("CommunitiesChatTab", "guild:chat_open"),
            ("CommunitiesGuildBenefitsTab", "guild:benefits_open"),
        ] {
            assert_eq!(action(&reg, name), tab_action);
            session.click(tab_action, &Default::default());
            assert!(
                session.commands.is_empty(),
                "Tab navigation does not fabricate service requests"
            );
        }
        for (index, title) in ["Lvl", "Class", "Name", "Zone", "Rank", "Note"]
            .iter()
            .enumerate()
        {
            assert_eq!(
                text(&reg, &format!("CommunitiesRosterColumn{index}Label")),
                *title
            );
        }
    }
    set_thread_skin(ActiveSkin::Modern);
}

#[test]
fn guild_rank_roster_context_uses_strict_hierarchy_both_skins() {
    let _lock = SKIN.lock().unwrap();
    for skin in [ActiveSkin::Modern, ActiveSkin::Forever] {
        set_thread_skin(skin);
        let mut session = GuildRanksSession::default();
        session.open();
        session.apply(state(0));
        session.click("guild:member-menu:Cara", &Default::default());
        let reg = mounted(&session);
        assert_eq!(
            session
                .clone()
                .click(&action(&reg, "GuildMemberPromote"), &Default::default()),
            Some(GuildRankRequest::Promote {
                character_name: "Cara".into()
            })
        );
        assert_eq!(
            session
                .clone()
                .click(&action(&reg, "GuildMemberDemote"), &Default::default()),
            Some(GuildRankRequest::Demote {
                character_name: "Cara".into()
            })
        );
        session.apply(state(1));
        let reg = mounted(&session);
        assert!(reg.get_by_name("GuildMemberPromote").is_none());
        assert!(reg.get_by_name("GuildMemberDemote").is_none());
    }
    set_thread_skin(ActiveSkin::Modern);
}

#[test]
fn guild_info_reports_actual_creation_date_without_backdating_legacy_records() {
    use game_engine_ui_model::guild_ranks::guild_creation_notice;
    assert_eq!(
        guild_creation_notice(None),
        "Guild creation date is unavailable."
    );
    assert_eq!(
        guild_creation_notice(Some(1728600000)),
        "Guild created on October 10, 2024 (UTC)."
    );
}

#[test]
fn guild_member_notes_gate_self_public_officer_visibility_and_wait_for_authority_both_skins() {
    let _lock = SKIN.lock().unwrap_or_else(|p| p.into_inner());
    for skin in [ActiveSkin::Modern, ActiveSkin::Forever] {
        set_thread_skin(skin);
        let mut session = GuildRanksSession::default();
        session.open();
        session.apply(state(2));
        session.profile = Some(GuildSnapshot {
            guild_id: 7,
            guild_name: "Guild Testers".into(),
            tabard: None,
            created_at: None,
            motd: String::new(),
            info_text: String::new(),
            members: vec![GuildMemberSnapshot {
                character_name: "Cara".into(),
                level: 80,
                class_name: "Priest".into(),
                rank_name: "Member".into(),
                is_online: true,
                officer_note: String::new(),
                public_note: "Alchemy".into(),
                is_self: true,
                last_online: "Online".into(),
            }],
        });
        session.click("guild:member:Cara", &Default::default());
        let reg = mounted(&session);
        assert_eq!(text(&reg, "GuildMemberDetailNoteText"), "Alchemy");
        assert!(
            reg.get_by_name("GuildMemberDetailOfficerNoteText")
                .is_none()
        );
        assert_eq!(text(&reg, "GuildMember0Note"), "Alchemy");
        session.click("guild:detail_group_invite", &Default::default());
        assert_eq!(
            session.commands.last(),
            Some(&game_engine_ui_model::guild_ranks::GuildCommand::GroupInvite("Cara".into()))
        );
        session.click("guild:note_officer", &Default::default());
        assert_eq!(session.note_editor, None);
        session.click("guild:note_public", &Default::default());
        assert_eq!(session.note_editor, Some(("Cara".into(), false)));
        let mut texts = std::collections::HashMap::new();
        texts.insert(
            "GuildMemberNoteEditBox".to_owned(),
            "Alchemy 100".to_owned(),
        );
        assert_eq!(
            session.click("guild:note_accept", &texts),
            Some(GuildRankRequest::SetMemberNote {
                character_name: "Cara".into(),
                note: "Alchemy 100".into(),
                officer: false,
            })
        );
        assert_eq!(
            session.profile.as_ref().unwrap().members[0].public_note,
            "Alchemy"
        );
        assert_eq!(session.click("guild:note_accept", &texts), None);
        session.apply(state(2));
        session.click("guild:note_public", &Default::default());
        assert_eq!(
            session.note_editor, None,
            "Rank ACK alone must not allow a stale note write"
        );
        let mut profile = session.profile.clone().unwrap();
        profile.members[0].public_note = "Alchemy 100".into();
        session.apply_profile(GuildStateUpdate {
            guild: Some(profile),
            message: None,
            error: None,
        });
        assert_eq!(session.detail_member().unwrap().public_note, "Alchemy 100");
        session.profile.as_mut().unwrap().members[0].is_self = false;
        session.click("guild:note_public", &Default::default());
        assert_eq!(session.note_editor, None);
        session.apply(state(0));
        session.profile.as_mut().unwrap().members[0].officer_note = "Raid healer".into();
        let reg = mounted(&session);
        assert_eq!(
            text(&reg, "GuildMemberDetailOfficerNoteText"),
            "Raid healer"
        );
        session.click("guild:note_officer", &Default::default());
        assert_eq!(session.note_editor, Some(("Cara".into(), true)));
        session.click("guild:note_close", &Default::default());
        assert_eq!(session.click("guild:note_accept", &texts), None);
    }
}

#[test]
fn guild_member_note_popup_keeps_original_target_when_roster_selection_changes() {
    let mut session = GuildRanksSession::default();
    session.open();
    let mut roster = state(0);
    roster.members.push(GuildRankMember {
        character_name: "Dora".into(),
        rank: 2,
    });
    session.apply(roster);
    let member = |name: &str| GuildMemberSnapshot {
        character_name: name.into(),
        level: 80,
        class_name: "Priest".into(),
        rank_name: "Member".into(),
        is_online: true,
        public_note: String::new(),
        officer_note: String::new(),
        is_self: false,
        last_online: "Online".into(),
    };
    session.profile = Some(GuildSnapshot {
        guild_id: 7,
        guild_name: "Guild Testers".into(),
        tabard: None,
        created_at: None,
        motd: String::new(),
        info_text: String::new(),
        members: vec![member("Cara"), member("Dora")],
    });
    session.click("guild:member:Cara", &Default::default());
    session.click("guild:note_public", &Default::default());
    session.click("guild:member:Dora", &Default::default());
    let texts = std::collections::HashMap::from([(
        "GuildMemberNoteEditBox".to_owned(),
        "Alchemy".to_owned(),
    )]);
    assert_eq!(
        session.click("guild:note_accept", &texts),
        Some(GuildRankRequest::SetMemberNote {
            character_name: "Cara".into(),
            note: "Alchemy".into(),
            officer: false,
        })
    );
}

#[test]
fn guild_news_info_renders_source_controls_and_only_real_motd_both_skins() {
    let _lock = SKIN.lock().unwrap_or_else(|p| p.into_inner());
    for skin in [ActiveSkin::Modern, ActiveSkin::Forever] {
        set_thread_skin(skin);
        let mut session = GuildRanksSession::default();
        session.open();
        session.apply(state(2));
        session.profile = Some(GuildSnapshot {
            guild_id: 7,
            guild_name: "Guild Testers".into(),
            tabard: None,
            created_at: None,
            motd: "Raid at seven".into(),
            info_text: String::new(),
            members: vec![],
        });
        assert!(
            mounted(&session)
                .get_by_name("CommunitiesGuildNewsHeaderText")
                .is_none()
        );
        session.click("guild:info_open", &Default::default());
        let registry = mounted(&session);
        assert_eq!(
            text(&registry, "CommunitiesGuildNewsHeaderText"),
            "Guild News"
        );
        assert_eq!(
            text(&registry, "CommunitiesGuildNewsSetFiltersText"),
            "[Click to set news filters]"
        );
        assert_eq!(
            text(&registry, "CommunitiesGuildNewsMOTDText"),
            "MotD: Raid at seven"
        );
        assert!(
            registry.get_by_name("CommunitiesGuildNewsEntry0").is_none(),
            "No invented news from present roster"
        );
        assert!(
            registry.get_by_name("CommunitiesGuildNewsNoNews").is_none(),
            "Actual MOTD is a source news row"
        );
        session.profile.as_mut().unwrap().motd.clear();
        assert_eq!(
            text(&mounted(&session), "CommunitiesGuildNewsNoNews"),
            "No Guild News"
        );
    }
}

#[test]
fn guild_news_info_queries_once_and_opens_source_filter_dialog_both_skins() {
    let _lock = SKIN.lock().unwrap_or_else(|p| p.into_inner());
    for skin in [ActiveSkin::Modern, ActiveSkin::Forever] {
        set_thread_skin(skin);
        let mut session = GuildRanksSession::default();
        session.open();
        session.apply(state(0));
        session.profile = Some(GuildSnapshot {
            guild_id: 7,
            guild_name: "Guild Testers".into(),
            tabard: None,
            created_at: None,
            motd: String::new(),
            info_text: String::new(),
            members: vec![],
        });
        session.click("guild:info_open", &Default::default());
        assert_eq!(
            session.commands.len(),
            1,
            "opening Info must query real news"
        );
        session.click("guild:info_open", &Default::default());
        assert_eq!(
            session.commands.len(),
            1,
            "pending query is independent and deduplicated"
        );
        session.click("guild:news_filters", &Default::default());
        let registry = mounted(&session);
        assert_eq!(
            text(&registry, "GuildNewsFiltersFrameTitle"),
            "Guild News Filters"
        );
        let dialog = registry
            .get(registry.get_by_name("GuildNewsFiltersFrame").unwrap())
            .unwrap();
        assert_eq!(
            (dialog.width.value(), dialog.height.value()),
            (264.0, 230.0)
        );
        session.click("guild:news_filters_close", &Default::default());
        assert!(
            mounted(&session)
                .get_by_name("GuildNewsFiltersFrame")
                .is_none()
        );
    }
}

#[test]
fn guild_news_real_entries_filter_sticky_and_late_push_keep_independent_authority_both_skins() {
    let _lock = SKIN.lock().unwrap_or_else(|p| p.into_inner());
    for skin in [ActiveSkin::Modern, ActiveSkin::Forever] {
        set_thread_skin(skin);
        let mut session = GuildRanksSession::default();
        session.open();
        session.apply(state(0));
        session.profile = Some(GuildSnapshot {
            guild_id: 7,
            guild_name: "Guild Testers".into(),
            tabard: None,
            created_at: None,
            motd: String::new(),
            info_text: String::new(),
            members: vec![],
        });
        session.click("guild:info_open", &Default::default());
        let news = GuildNewsState {
            server_time: 1792100000,
            error: None,
            entries: vec![
                GuildNewsEntry {
                    id: 1,
                    timestamp: 1792090000,
                    sticky: false,
                    kind: GuildNewsKind::PlayerAchievement {
                        member: "Cara".into(),
                        achievement_id: 14782,
                        name: "Level 50".into(),
                        description: "Reach level 50.".into(),
                    },
                },
                GuildNewsEntry {
                    id: 2,
                    timestamp: 1792100000,
                    sticky: false,
                    kind: GuildNewsKind::PlayerAchievement {
                        member: "Lena".into(),
                        achievement_id: 14783,
                        name: "Level 60".into(),
                        description: "Reach level 60.".into(),
                    },
                },
            ],
        };
        session.news.apply(news.clone());
        assert_eq!(
            text(&mounted(&session), "CommunitiesGuildNewsEntry2Text"),
            "Lena earned the achievement [Level 60]"
        );
        session.click("guild:news_filter:2", &Default::default());
        assert!(
            mounted(&session)
                .get_by_name("CommunitiesGuildNewsEntry2Text")
                .is_none()
        );
        session.click("guild:news_filter:2", &Default::default());
        session.click("guild:news_menu:1", &Default::default());
        assert_eq!(
            text(&mounted(&session), "GuildNewsMakeStickyText"),
            "Make Sticky"
        );
        session.click("guild:news_sticky", &Default::default());
        assert_eq!(session.news.pending_sticky, Some((1, true)));
        session.news.apply(news.clone());
        assert_eq!(
            session.news.pending_sticky,
            Some((1, true)),
            "old queued push is not sticky ACK"
        );
        let mut confirmed = news;
        confirmed.entries[0].sticky = true;
        session.news.apply(confirmed);
        assert_eq!(session.news.pending_sticky, None);
        assert_eq!(session.news.entries()[0].id, 1);
        session.click("guild:news_filter:2", &Default::default());
        assert!(
            mounted(&session)
                .get_by_name("CommunitiesGuildNewsEntry1Text")
                .is_some(),
            "source sticky ignores category filters"
        );
        let mut departed = state(0);
        departed.ranks.clear();
        session.apply(departed);
        assert!(session.news.state.is_none());
        assert!(session.news.pending_sticky.is_none());
    }
}

#[test]
fn guild_event_log_opens_from_info_and_closes_independently_both_skins() {
    let _lock = SKIN.lock().unwrap_or_else(|p| p.into_inner());
    for skin in [ActiveSkin::Modern, ActiveSkin::Forever] {
        set_thread_skin(skin);
        let mut session = GuildRanksSession::default();
        session.open();
        session.apply(state(2));
        assert!(
            mounted(&session)
                .get_by_name("CommunitiesGuildLogButton")
                .is_none()
        );
        session.click("guild:info_open", &Default::default());
        let registry = mounted(&session);
        assert_eq!(text(&registry, "CommunitiesGuildLogButton"), "View Log");
        session.click(
            &action(&registry, "CommunitiesGuildLogButton"),
            &Default::default(),
        );
        assert_eq!(
            text(&mounted(&session), "CommunitiesGuildLogFrameTitle"),
            "Log"
        );
        session.click("guild:log_close", &Default::default());
        assert!(
            mounted(&session)
                .get_by_name("CommunitiesGuildLogFrame")
                .is_none()
        );
        assert!(session.visible, "Closing the log keeps Communities open");
    }
}

#[test]
fn guild_event_log_renders_real_newest_entries_and_keeps_rank_authority_independent_both_skins() {
    let _lock = SKIN.lock().unwrap_or_else(|p| p.into_inner());
    for skin in [ActiveSkin::Modern, ActiveSkin::Forever] {
        set_thread_skin(skin);
        let mut session = GuildRanksSession::default();
        session.open();
        session.apply(state(2));
        session.click("guild:log_open", &Default::default());
        assert_eq!(
            session.commands,
            vec![game_engine_ui_model::guild_ranks::GuildCommand::QueryEventLog]
        );
        assert!(session.log_waiting);
        assert!(!session.awaiting_state);
        assert!(
            mounted(&session)
                .get_by_name("CommunitiesGuildLogEntry0Row0Run0")
                .is_none()
        );
        session.awaiting_state = true; // Independent rank request already in flight.
        let reply = GuildEventLog {
            events: vec![
                GuildEvent {
                    timestamp: 100,
                    kind: GuildEventKind::Join {
                        member: "Ada".into(),
                    },
                },
                GuildEvent {
                    timestamp: 200,
                    kind: GuildEventKind::Quit {
                        member: "Ben".into(),
                    },
                },
            ],
            server_time: 260,
            error: None,
        };
        session.apply_event_log(reply.clone());
        assert!(!session.log_waiting);
        assert!(
            session.awaiting_state,
            "A log reply cannot permit another rank write"
        );
        let reg = mounted(&session);
        assert_eq!(
            text(&reg, "CommunitiesGuildLogEntry0Row0Run0"),
            "Ben leaves the guild"
        );
        assert_eq!(
            text(&reg, "CommunitiesGuildLogEntry0Row0Run1"),
            "   ( 1 min ago )"
        );
        assert_eq!(
            text(&reg, "CommunitiesGuildLogEntry1Row0Run0"),
            "Ada joins the guild"
        );
        assert!(session.dismiss_subdialog());
        session.apply_event_log(reply);
        assert!(
            !session.log_open,
            "Late query reply cannot reopen a closed log"
        );
        let mut removed = state(u8::MAX);
        removed.ranks.clear();
        removed.members.clear();
        session.apply(removed);
        assert!(
            session.event_log.is_none(),
            "Leaving clears member-only history"
        );
    }
}

#[test]
fn guild_event_log_formats_only_the_six_source_lifecycle_kinds() {
    use game_engine_ui_model::guild_event_log::event_text;
    for (event, expected) in [
        (
            GuildEventKind::Invite {
                actor: "Ada".into(),
                member: "Ben".into(),
            },
            "Ada invites Ben",
        ),
        (
            GuildEventKind::Join {
                member: "Ben".into(),
            },
            "Ben joins the guild",
        ),
        (
            GuildEventKind::Promote {
                actor: "Ada".into(),
                member: "Ben".into(),
                rank: "Veteran".into(),
            },
            "Ada promotes Ben to Veteran",
        ),
        (
            GuildEventKind::Demote {
                actor: "Ada".into(),
                member: "Ben".into(),
                rank: "Member".into(),
            },
            "Ada demotes Ben to Member",
        ),
        (
            GuildEventKind::Remove {
                actor: "Ada".into(),
                member: "Ben".into(),
            },
            "Ada removes Ben from the guild",
        ),
        (
            GuildEventKind::Quit {
                member: "Ben".into(),
            },
            "Ben leaves the guild",
        ),
    ] {
        assert_eq!(event_text(&event), expected);
    }
}

#[test]
fn guild_member_remove_requires_yes_and_keeps_original_target_both_skins() {
    let _lock = SKIN.lock().unwrap_or_else(|p| p.into_inner());
    for skin in [ActiveSkin::Modern, ActiveSkin::Forever] {
        set_thread_skin(skin);
        for (entry, remove) in [
            ("guild:member:Cara", "guild:detail_remove"),
            ("guild:member-menu:Cara", "guild:remove_member"),
        ] {
            let mut session = GuildRanksSession::default();
            session.open();
            let mut roster = state(0);
            roster.members.push(GuildRankMember {
                character_name: "Dora".into(),
                rank: 2,
            });
            session.apply(roster);
            session.click(entry, &Default::default());
            assert_eq!(
                session.click(remove, &Default::default()),
                None,
                "Opening a destructive action must not send it"
            );
            assert_eq!(
                text(&mounted(&session), "GuildMemberRemovePopupText"),
                "Are you sure you want to remove Cara from the guild?"
            );
            session.click("guild:remove_cancel", &Default::default());
            assert!(
                mounted(&session)
                    .get_by_name("GuildMemberRemovePopup")
                    .is_none()
            );
            session.click(entry, &Default::default());
            session.click(remove, &Default::default());
            assert!(session.dismiss_subdialog());
            assert!(session.visible, "Escape closes popup, not the guild window");
            assert!(session.removal_target.is_none());
            assert!(!session.awaiting_state, "Cancel sends no removal request");
            session.click(entry, &Default::default());
            session.click(remove, &Default::default());
            let reopened = session.state().unwrap().clone();
            session.close();
            session.open();
            assert!(
                session.removal_target.is_none(),
                "Closed popup cannot reopen stale"
            );
            session.apply(reopened);
            session.click(entry, &Default::default());
            session.click(remove, &Default::default());
            session.click("guild:member:Dora", &Default::default());
            let before = session.state().cloned();
            assert_eq!(
                session.click("guild:remove_confirm", &Default::default()),
                Some(GuildRankRequest::RemoveMember {
                    character_name: "Cara".into()
                })
            );
            assert_eq!(
                session.state(),
                before.as_ref(),
                "Removal waits for authoritative roster"
            );
        }
    }
}

#[test]
fn guild_member_detail_opens_from_roster_both_skins() {
    let _lock = SKIN.lock().unwrap_or_else(|p| p.into_inner());
    for skin in [ActiveSkin::Modern, ActiveSkin::Forever] {
        set_thread_skin(skin);
        let mut session = GuildRanksSession::default();
        session.open();
        session.apply(state(0));
        session.click("guild:member:Cara", &Default::default());
        let reg = mounted(&session);
        assert_eq!(text(&reg, "GuildMemberDetailName"), "Cara");
        assert_eq!(text(&reg, "GuildMemberDetailRank"), "Member");
    }
}
