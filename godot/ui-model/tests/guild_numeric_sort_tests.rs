//! Observable numeric-header ties, with incoming rows reversed from the approved base order.
use super::*;

type Member = (GuildRankMember, Option<GuildMemberSnapshot>);

fn member(name: &str, online: bool, level: u16, rank: u8) -> Member {
    (
        GuildRankMember {
            character_name: name.into(),
            rank,
        },
        Some(GuildMemberSnapshot {
            character_name: name.into(),
            level,
            class_name: "Paladin".into(),
            rank_name: "Member".into(),
            is_online: online,
            officer_note: "private".into(),
            public_note: "same".into(),
            is_self: false,
            last_logout_unix_secs: None,
            online_zone: online.then(|| "Elwynn Forest".into()),
        }),
    )
}

fn missing(name: &str) -> Member {
    (
        GuildRankMember {
            character_name: name.into(),
            rank: 2,
        },
        None,
    )
}

fn session(members: Vec<Member>) -> GuildRanksSession {
    let mut session = GuildRanksSession::default();
    session.open();
    let mut ranks = state(0);
    ranks.members = members.iter().map(|member| member.0.clone()).collect();
    session.apply(ranks);
    session.profile = Some(GuildSnapshot {
        guild_id: 7,
        guild_name: "Numeric Ties Guild".into(),
        tabard: None,
        created_at: None,
        motd: String::new(),
        info_text: String::new(),
        // Different profile/rank input order proves lookup is by original member identity.
        members: members
            .into_iter()
            .rev()
            .filter_map(|member| member.1)
            .collect(),
    });
    session
}

fn assert_header_directions(
    mut session: GuildRanksSession,
    column: usize,
    ascending: &[&str],
    descending: &[&str],
) {
    let authority = session.state().cloned();
    let profile = session.profile.clone();
    let header = format!("CommunitiesRosterColumn{column}");
    for expected in [ascending, descending, ascending] {
        let registry = mounted(&session);
        let click = action(&registry, &header);
        assert!(!click.is_empty(), "existing numeric header must dispatch");
        assert_eq!(session.click(&click, &Default::default()), None);
        let registry = mounted(&session);
        assert_eq!(
            rendered_guild_names(&registry),
            expected,
            "equal-key rows must use fixed base order, not incoming snapshot order"
        );
        for (index, name) in expected.iter().enumerate() {
            assert_eq!(
                action(&registry, &format!("GuildMember{index}")),
                format!("guild:member:{name}")
            );
        }
        assert_eq!(session.state(), authority.as_ref());
        assert_eq!(session.profile, profile);
        assert!(session.commands.is_empty());
    }
    session.click("guild:roster-sort:name", &Default::default());
    let mut alphabetical: Vec<_> = authority
        .as_ref()
        .unwrap()
        .members
        .iter()
        .map(|member| member.character_name.clone())
        .collect();
    alphabetical.sort();
    assert_eq!(rendered_guild_names(&mounted(&session)), alphabetical);
    let click = action(&mounted(&session), &header);
    session.click(&click, &Default::default());
    assert_eq!(
        rendered_guild_names(&mounted(&session)),
        ascending,
        "switching back resets only the primary direction"
    );
}

#[test]
fn guild_numeric_sort_rank_equal_keys_keep_base_order_both_skins() {
    let _lock = SKIN.lock().unwrap_or_else(|poison| poison.into_inner());
    for skin in [ActiveSkin::Modern, ActiveSkin::Forever] {
        set_thread_skin(skin);
        let session = session(vec![
            missing("NoProfileZ"),
            missing("NoProfileA"),
            member("Ada", false, 80, 2),
            member("Low", true, 20, 2),
            member("Zed", true, 80, 2),
            member("Aaron", true, 80, 2),
            member("Control", false, 1, 1),
        ]);
        assert_header_directions(
            session,
            4,
            &[
                "Control",
                "Aaron",
                "Zed",
                "Low",
                "Ada",
                "NoProfileA",
                "NoProfileZ",
            ],
            &[
                "Aaron",
                "Zed",
                "Low",
                "Ada",
                "NoProfileA",
                "NoProfileZ",
                "Control",
            ],
        );
    }
}

#[test]
fn guild_numeric_sort_level_equal_keys_keep_base_order_both_skins() {
    let _lock = SKIN.lock().unwrap_or_else(|poison| poison.into_inner());
    for skin in [ActiveSkin::Modern, ActiveSkin::Forever] {
        set_thread_skin(skin);
        let session = session(vec![
            member("Ada", false, 80, 2),
            member("Zed", true, 80, 2),
            member("Aaron", true, 80, 2),
            member("Officer", true, 80, 1),
            missing("NoProfileZ"),
            missing("NoProfileA"),
            member("Control", false, 20, 2),
        ]);
        assert_header_directions(
            session,
            0,
            &[
                "NoProfileA",
                "NoProfileZ",
                "Control",
                "Officer",
                "Aaron",
                "Zed",
                "Ada",
            ],
            &[
                "Officer",
                "Aaron",
                "Zed",
                "Ada",
                "Control",
                "NoProfileA",
                "NoProfileZ",
            ],
        );
    }
}

#[test]
fn guild_numeric_sort_class_equal_keys_keep_base_order_both_skins() {
    let _lock = SKIN.lock().unwrap_or_else(|poison| poison.into_inner());
    for skin in [ActiveSkin::Modern, ActiveSkin::Forever] {
        set_thread_skin(skin);
        let mut control = member("Control", false, 1, 2);
        control.1.as_mut().unwrap().class_name = "Mage".into();
        let mut unclassed = member("Unclassed", true, 80, 2);
        unclassed.1.as_mut().unwrap().class_name.clear();
        let session = session(vec![
            missing("NoProfileZ"),
            missing("NoProfileA"),
            unclassed,
            member("Ada", false, 80, 2),
            member("Low", true, 20, 2),
            member("Zed", true, 80, 2),
            member("Aaron", true, 80, 2),
            member("Officer", true, 80, 1),
            control,
        ]);
        assert_header_directions(
            session,
            1,
            &[
                "Unclassed",
                "NoProfileA",
                "NoProfileZ",
                "Officer",
                "Aaron",
                "Zed",
                "Low",
                "Ada",
                "Control",
            ],
            &[
                "Control",
                "Officer",
                "Aaron",
                "Zed",
                "Low",
                "Ada",
                "Unclassed",
                "NoProfileA",
                "NoProfileZ",
            ],
        );
    }
}

#[test]
fn guild_numeric_sort_ties_preserve_registry_scroll_filter_and_original_targets() {
    let _lock = SKIN.lock().unwrap_or_else(|poison| poison.into_inner());
    game_engine_ui_model::paths::set_data_root(
        std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../data"),
    )
    .unwrap();
    for skin in [ActiveSkin::Modern, ActiveSkin::Forever] {
        set_thread_skin(skin);
        for column in [0, 1, 4] {
            let members = (0..24)
                .rev()
                .map(|index| member(&format!("Member{index:02}"), index % 2 == 0, 80, 2))
                .collect();
            let mut session = session(members);
            let authority = session.state().cloned();
            let profile = session.profile.clone();
            let mut shared = SharedContext::new();
            shared.insert(session.clone());
            let mut registry = FrameRegistry::new(1920.0, 1080.0);
            let mut screen = Screen::new(guild_screen);
            screen.sync(&shared, &mut registry);
            assert_eq!(rendered_guild_names(&registry).len(), 24);
            let list = game_engine_ui_model::guild_communities::SCROLL_BOX;
            assert!(registry.scroll_lists.scroll_to(list, usize::MAX));
            screen.sync(&shared, &mut registry);
            let before = registry.scroll_lists.get(list).unwrap().first_row;
            assert!(before > 0, "real second viewport required");
            let header_name = format!("CommunitiesRosterColumn{column}");
            let header = registry
                .get(registry.get_by_name(&header_name).unwrap())
                .unwrap();
            let rect = (
                header.position.left,
                header.position.top,
                header.width.value(),
                header.height.value(),
            );
            let expected: Vec<_> = (0..24)
                .filter(|index| index % 2 == 0)
                .chain((0..24).filter(|index| index % 2 == 1))
                .map(|index| format!("Member{index:02}"))
                .collect();
            for _ in 0..2 {
                let click = action(&registry, &header_name);
                session.click(&click, &Default::default());
                shared.insert(session.clone());
                screen.sync(&shared, &mut registry);
                assert_eq!(rendered_guild_names(&registry), expected);
                assert_eq!(registry.scroll_lists.get(list).unwrap().first_row, before);
                let header = registry
                    .get(registry.get_by_name(&header_name).unwrap())
                    .unwrap();
                assert_eq!(
                    (
                        header.position.left,
                        header.position.top,
                        header.width.value(),
                        header.height.value()
                    ),
                    rect
                );
            }
            let selected = action(&registry, "GuildMember0");
            session.click(&selected, &Default::default());
            assert_eq!(session.member_detail.as_deref(), Some("Member00"));
            assert_eq!(
                session.promote("Member00"),
                Some(GuildRankRequest::Promote {
                    character_name: "Member00".into()
                })
            );
            session.click(
                &action(&registry, "CommunitiesMemberListShowOfflineButton"),
                &Default::default(),
            );
            shared.insert(session.clone());
            screen.sync(&shared, &mut registry);
            let online: Vec<_> = (0..24)
                .filter(|index| index % 2 == 0)
                .map(|index| format!("Member{index:02}"))
                .collect();
            assert_eq!(rendered_guild_names(&registry), online);
            assert_eq!(registry.scroll_lists.get(list).unwrap().first_row, 0);
            assert_eq!(session.state(), authority.as_ref());
            assert_eq!(session.profile, profile);
        }
    }
}
