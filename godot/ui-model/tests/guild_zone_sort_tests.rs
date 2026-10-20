//! Real source header dispatch and rendered row identity, not comparator implementation shape.
use super::*;

fn member(
    name: &str,
    online: bool,
    zone: Option<&str>,
    logout: Option<u64>,
) -> GuildMemberSnapshot {
    GuildMemberSnapshot {
        character_name: name.into(),
        level: 32,
        class_name: "Warrior".into(),
        rank_name: "Member".into(),
        is_online: online,
        officer_note: "private".into(),
        public_note: "note".into(),
        is_self: false,
        last_logout_unix_secs: logout,
        online_zone: zone.map(str::to_owned),
    }
}

fn session(members: Vec<GuildMemberSnapshot>, missing: &[&str]) -> GuildRanksSession {
    let mut session = GuildRanksSession::default();
    session.open();
    let mut ranks = state(0);
    ranks.members = members
        .iter()
        .map(|member| GuildRankMember {
            character_name: member.character_name.clone(),
            rank: 2,
        })
        .chain(missing.iter().map(|name| GuildRankMember {
            character_name: (*name).into(),
            rank: 2,
        }))
        .collect();
    session.apply(ranks);
    session.profile = Some(GuildSnapshot {
        guild_id: 7,
        guild_name: "Zone Sort Guild".into(),
        tabard: None,
        created_at: None,
        motd: String::new(),
        info_text: String::new(),
        members,
    });
    session
}

#[test]
#[ignore = "RED first: existing Zone header must order represented authority by approved policy"]
fn guild_zone_sort_offline_seconds_direction_reset_and_name_ties_both_skins() {
    let _lock = SKIN.lock().unwrap_or_else(|p| p.into_inner());
    // GUILD_ZONE_SORT_OFFLINE_BY_LOGOUT_SECONDS: a one-second difference is deliberate.
    const OLDER: u64 = 1_792_400_100;
    const RECENT: u64 = OLDER + 1;
    for skin in [ActiveSkin::Modern, ActiveSkin::Forever] {
        set_thread_skin(skin);
        let mut session = session(
            vec![
                member("BetaWest", true, Some("Westfall"), None),
                member("ZRecent", false, None, Some(RECENT)),
                member("NoHistory", false, None, None),
                member("Older", false, None, Some(OLDER)),
                // Online authority masks a stale timestamp, as the display already does.
                member("AlphaEast", true, Some("Elwynn Forest"), Some(RECENT)),
                member("ARecent", false, None, Some(RECENT)),
                member("Unlocated", true, None, None),
            ],
            &["NoProfile"],
        );
        let ranks = session.state().cloned();
        let profile = session.profile.clone();
        // Old input is a no-op: no enum reference or compile failure substitutes for RED.
        for expected in [
            vec![
                "ARecent",
                "ZRecent",
                "Older",
                "AlphaEast",
                "BetaWest",
                "NoHistory",
                "NoProfile",
                "Unlocated",
            ],
            vec![
                "NoHistory",
                "NoProfile",
                "Unlocated",
                "BetaWest",
                "AlphaEast",
                "Older",
                "ARecent",
                "ZRecent",
            ],
        ] {
            assert_eq!(
                session.click("guild:roster-sort:zone", &Default::default()),
                None
            );
            assert_eq!(rendered_guild_names(&mounted(&session)), expected);
            assert_eq!(
                session.state(),
                ranks.as_ref(),
                "presentation cannot rewrite authority"
            );
            assert_eq!(session.profile, profile);
        }
        session.click("guild:roster-sort:name", &Default::default());
        session.click("guild:roster-sort:zone", &Default::default());
        assert_eq!(
            rendered_guild_names(&mounted(&session))[..3],
            ["ARecent", "ZRecent", "Older"]
        );
    }
}

#[test]
#[ignore = "RED first: Zone string ordering must use the explicitly allowed historical prose cases"]
fn guild_zone_sort_allowed_prose_collation_and_fixed_name_ties_both_skins() {
    let _lock = SKIN.lock().unwrap_or_else(|p| p.into_inner());
    // Allowed aso prose only, not a native Lua binding or Retail70448 parity claim.
    for skin in [ActiveSkin::Modern, ActiveSkin::Forever] {
        set_thread_skin(skin);
        for (zones, ascending, descending) in [
            (
                ["raid", "RAID", "zulu"],
                ["Ada", "Zed", "Cid"],
                ["Cid", "Ada", "Zed"],
            ),
            (
                ["é", "É", "ÿ"],
                ["Ada", "Zed", "Cid"],
                ["Cid", "Ada", "Zed"],
            ),
            (
                ["ÿ", "Ā", "Ÿ"],
                ["Zed", "Ada", "Cid"],
                ["Cid", "Ada", "Zed"],
            ),
            (
                ["Е", "Ё", "Ж"],
                ["Zed", "Ada", "Cid"],
                ["Cid", "Ada", "Zed"],
            ),
            (
                ["ё", "Е", "е"],
                ["Ada", "Cid", "Zed"],
                ["Zed", "Ada", "Cid"],
            ),
            (
                ["÷", "×", "z"],
                ["Cid", "Ada", "Zed"],
                ["Ada", "Zed", "Cid"],
            ),
            (
                ["😀", "�", "😁"],
                ["Ada", "Cid", "Zed"],
                ["Ada", "Cid", "Zed"],
            ),
            (
                ["e\u{301}", "é", "E"],
                ["Cid", "Zed", "Ada"],
                ["Ada", "Zed", "Cid"],
            ),
            (
                ["Ω", "ω", "z"],
                ["Cid", "Zed", "Ada"],
                ["Ada", "Zed", "Cid"],
            ),
            (
                ["SS", "ß", "zz"],
                ["Zed", "Cid", "Ada"],
                ["Ada", "Cid", "Zed"],
            ),
            (
                ["a", "AB", "aB"],
                ["Zed", "Ada", "Cid"],
                ["Ada", "Cid", "Zed"],
            ),
        ] {
            let members = ["Zed", "Ada", "Cid"]
                .into_iter()
                .zip(zones)
                .map(|(name, zone)| member(name, true, Some(zone), None))
                .collect();
            let mut session = session(members, &[]);
            session.click("guild:roster-sort:zone", &Default::default());
            assert_eq!(
                rendered_guild_names(&mounted(&session)),
                ascending,
                "source packet input {zones:?}"
            );
            session.click("guild:roster-sort:zone", &Default::default());
            assert_eq!(
                rendered_guild_names(&mounted(&session)),
                descending,
                "equal mapped strings keep name order in both directions"
            );
        }
    }
}

#[test]
#[ignore = "RED first: physical Zone action must preserve overflow/filtering/geometry/member targets"]
fn guild_zone_sort_real_header_dispatch_preserves_scroll_filter_and_member_identity() {
    let _lock = SKIN.lock().unwrap_or_else(|p| p.into_inner());
    game_engine_ui_model::paths::set_data_root(
        std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../data"),
    )
    .unwrap();
    for skin in [ActiveSkin::Modern, ActiveSkin::Forever] {
        set_thread_skin(skin);
        let members = (0..24)
            .map(|index| {
                member(
                    &format!("Member{index:02}"),
                    index % 2 == 0,
                    if index % 4 == 0 {
                        Some("Westfall")
                    } else if index % 2 == 0 {
                        Some("Elwynn Forest")
                    } else {
                        None
                    },
                    (index % 2 == 1).then_some(1_792_400_000 + index),
                )
            })
            .collect();
        let mut session = session(members, &[]);
        let authority = session.state().cloned();
        let mut shared = SharedContext::new();
        shared.insert(session.clone());
        let mut registry = FrameRegistry::new(1920.0, 1080.0);
        let mut screen = Screen::new(guild_screen);
        screen.sync(&shared, &mut registry);
        assert_eq!(
            rendered_guild_names(&registry).len(),
            24,
            "enough data for a real second viewport"
        );
        let list = game_engine_ui_model::guild_communities::SCROLL_BOX;
        assert!(registry.scroll_lists.scroll_to(list, usize::MAX));
        screen.sync(&shared, &mut registry);
        let before_offset = registry.scroll_lists.get(list).unwrap().first_row;
        assert!(before_offset > 0);
        let header = registry
            .get(registry.get_by_name("CommunitiesRosterColumn3").unwrap())
            .unwrap();
        let rect = (
            header.position.left,
            header.position.top,
            header.width.value(),
            header.height.value(),
        );
        let toggle = action(&registry, "CommunitiesRosterColumn3");
        assert!(
            !toggle.is_empty(),
            "the existing Zone header is a real action"
        );
        assert_eq!(session.click(&toggle, &Default::default()), None);
        shared.insert(session.clone());
        screen.sync(&shared, &mut registry);
        assert_eq!(
            registry.scroll_lists.get(list).unwrap().first_row,
            before_offset
        );
        let header = registry
            .get(registry.get_by_name("CommunitiesRosterColumn3").unwrap())
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
        let names = rendered_guild_names(&registry);
        assert_eq!(
            names[0], "Member23",
            "most recent offline event is first by explicit seconds policy"
        );
        for (index, name) in names.iter().enumerate() {
            assert_eq!(
                action(&registry, &format!("GuildMember{index}")),
                format!("guild:member:{name}")
            );
        }
        session.click(&action(&registry, "GuildMember0"), &Default::default());
        assert_eq!(session.member_detail.as_deref(), Some("Member23"));
        assert_eq!(
            session.promote("Member23"),
            Some(GuildRankRequest::Promote {
                character_name: "Member23".into()
            })
        );
        session.click("guild:show_offline", &Default::default());
        shared.insert(session.clone());
        screen.sync(&shared, &mut registry);
        assert_eq!(rendered_guild_names(&registry).len(), 12);
        assert!(
            rendered_guild_names(&registry)
                .iter()
                .all(|name| name[6..].parse::<u64>().unwrap() % 2 == 0)
        );
        assert_eq!(
            registry.scroll_lists.get(list).unwrap().first_row,
            0,
            "filtering clamps the now-short viewport"
        );
        assert_eq!(session.state(), authority.as_ref());
    }
}
