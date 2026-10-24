use ui_toolkit::{
    atlas::{ActiveSkin, set_thread_skin},
    frame::WidgetData,
    registry::FrameRegistry,
    screen::{Screen, SharedContext},
};

#[test]
fn quick_match_and_rated_tabs_show_wallet_balances_without_fake_season_policy() {
    use game_engine_ui_model::battleground::{PvpAction, PvpView, pvp_screen};
    use shared::protocol::{PvpBracketSnapshot, PvpSnapshot};
    let data = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../data");
    game_engine_ui_model::paths::set_data_root(data).unwrap();
    for skin in [ActiveSkin::Modern, ActiveSkin::Forever] {
        set_thread_skin(skin);
        let mut view = PvpView::default();
        view.visible = true;
        view.snapshot = Some(PvpSnapshot {
            lifetime_honor_xp: 0,
            honor_level: None,
            honor: 750,
            honor_max: 15000,
            conquest: 120,
            conquest_max: None,
            brackets: vec![],
            queue: None,
            battleground: None,
            war_mode: None,
            talents: None,
        });
        let mut shared = SharedContext::new();
        shared.insert(view.clone());
        let mut registry = FrameRegistry::new(1280.0, 720.0);
        let mut screen = Screen::new(pvp_screen);
        screen.sync(&shared, &mut registry);
        let honor = registry
            .get(
                registry
                    .get_by_name("PVPUIHonorCurrency")
                    .expect("Honor wallet balance"),
            )
            .unwrap();
        let Some(WidgetData::FontString(text)) = honor.widget_data.as_ref() else {
            panic!("Honor balance text")
        };
        assert!(text.text.contains("750"));
        assert!(text.text.contains("15000"));
        let rated_category = registry
            .get(
                registry
                    .get_by_name("PVPQueueFrameCategoryButton2")
                    .unwrap(),
            )
            .unwrap();
        assert_eq!(
            rated_category.onclick.as_deref(),
            Some("pvp:category:rated")
        );
        assert_eq!(
            view.action(rated_category.onclick.as_deref().unwrap()),
            None
        );
        shared.insert(view.clone());
        screen.sync(&shared, &mut registry);
        let conquest = registry
            .get(
                registry
                    .get_by_name("ConquestFrameBalance")
                    .expect("Rated Conquest balance"),
            )
            .unwrap();
        let Some(WidgetData::FontString(text)) = conquest.widget_data.as_ref() else {
            panic!("Conquest balance text")
        };
        assert_eq!(
            text.text, "Conquest: 120",
            "wallet, not invented season progress/cap"
        );
        // The Retail activity rows replaced the standalone policy annotation.
        for (name, expected) in [
            ("ConquestFrameArena2v2Text", "2v2"),
            ("ConquestFrameArena2v2TypeText", "Rated Arena"),
        ] {
            let frame = registry.get(registry.get_by_name(name).unwrap()).unwrap();
            let Some(WidgetData::FontString(text)) = frame.widget_data.as_ref() else {
                panic!("Rated activity text: {name}")
            };
            assert_eq!(text.text, expected);
        }
        for activity in [
            "ConquestFrameRatedSoloShuffle",
            "ConquestFrameRatedBGBlitz",
            "ConquestFrameArena2v2",
            "ConquestFrameArena3v3",
            "ConquestFrameRatedBG",
        ] {
            assert!(
                registry
                    .get_by_name(&format!("{activity}CurrentRating"))
                    .is_none(),
                "no invented rating without authoritative bracket stats"
            );
        }
        let arena = registry
            .get(registry.get_by_name("ConquestFrameArena2v2").unwrap())
            .unwrap();
        assert_eq!(arena.onclick.as_deref(), Some("pvp:rated:2v2"));
        assert_eq!(view.action(arena.onclick.as_deref().unwrap()), None);
        let join = registry
            .get(registry.get_by_name("ConquestFrameJoinButton").unwrap())
            .unwrap();
        assert_eq!(join.onclick.as_deref(), Some("pvp:rated:queue"));
        assert_eq!(
            view.action(join.onclick.as_deref().unwrap()),
            Some(PvpAction::Rated(PvpBracketSnapshot::Arena2v2))
        );
        assert!(
            registry.get_by_name("HonorFrameQueueButton").is_none(),
            "rated tab uses its own authoritative queue action"
        );
    }
}

#[test]
fn honor_lifetime_display_never_substitutes_spendable_cap_for_unknown_level_curve() {
    use game_engine_ui_model::battleground::{PvpView, pvp_screen};
    let data = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../data");
    game_engine_ui_model::paths::set_data_root(data).unwrap();
    for skin in [ActiveSkin::Modern, ActiveSkin::Forever] {
        set_thread_skin(skin);
        let mut view = PvpView::default();
        view.visible = true;
        view.viewport = [1920.0, 1080.0];
        view.honor_faction = Some(game_engine_ui_model::char_create_data::Faction::Alliance);
        view.snapshot = Some(shared::protocol::PvpSnapshot {
            lifetime_honor_xp: 47,
            honor_level: None,
            honor: 750,
            honor_max: 15000,
            conquest: 0,
            conquest_max: None,
            brackets: vec![],
            queue: None,
            battleground: None,
            war_mode: None,
            talents: None,
        });
        let tooltip = view.honor_tooltip().unwrap();
        assert_eq!(tooltip.content.title, "Lifetime Honor");
        assert_eq!(tooltip.content.lines[0].left_text, "47");
        let mut shared = SharedContext::new();
        shared.insert(view);
        let mut registry = FrameRegistry::new(1920.0, 1080.0);
        Screen::new(pvp_screen).sync(&shared, &mut registry);
        let level = registry
            .get(
                registry
                    .get_by_name("PVPUIHonorLevelLabel")
                    .expect("Retail HonorLevelDisplay label"),
            )
            .unwrap();
        let Some(WidgetData::FontString(text)) = level.widget_data.as_ref() else {
            panic!("honor level text")
        };
        assert_eq!(
            text.text, "Honor Level —",
            "unknown level, not fabricated zero or wallet amount"
        );
        assert!(registry.get_by_name("PVPUIHonorLevelDisplay").is_some());
        assert!(
            registry.get_by_name("PVPUIHonorFactionBadge").is_some(),
            "source faction identity, not an invented honor-level badge"
        );
        assert!(
            registry.get_by_name("PVPUIHonorLevelFill").is_none(),
            "no certified lifetime XP maximum"
        );
    }
}
