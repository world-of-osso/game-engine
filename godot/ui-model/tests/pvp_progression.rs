use game_engine_ui_model::{
    spellbook_frame_component::{PlayerSpellsTab, spellbook_frame_screen},
    spellbook_preview::load_preview_state,
};
use ui_toolkit::{
    atlas::{ActiveSkin, set_thread_skin},
    frame::WidgetData,
    registry::FrameRegistry,
    screen::{Screen, SharedContext},
};

#[test]
fn retail_war_mode_control_exists_but_requires_authoritative_context_in_both_skins() {
    let data = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../data");
    game_engine_ui_model::paths::set_data_root(data.clone()).unwrap();
    for skin in [ActiveSkin::Modern, ActiveSkin::Forever] {
        set_thread_skin(skin);
        let mut state = load_preview_state(&data, PlayerSpellsTab::Talents).unwrap();
        let mut shared = SharedContext::new();
        shared.insert(state.clone());
        let mut registry = FrameRegistry::new(1920.0, 1080.0);
        let mut screen = Screen::new(spellbook_frame_screen);
        screen.sync(&shared, &mut registry);
        let button = registry
            .get_by_name("WarmodeButton")
            .expect("Retail talent footer War Mode control");
        let frame = registry.get(button).unwrap();
        let Some(WidgetData::Button(button)) = frame.widget_data.as_ref() else {
            panic!("War Mode must be a button")
        };
        assert_eq!(
            button.state,
            ui_toolkit::widgets::button::ButtonState::Disabled,
            "No optimistic permission without a server snapshot"
        );
        assert_eq!(frame.onclick.as_deref(), Some(""));
        let mode = shared::protocol::WarModeSnapshot {
            desired: false,
            active: false,
            can_enable: true,
            can_disable: true,
            enlisted_spell: None,
        };
        state.talents.as_mut().unwrap().war_mode = Some(mode.clone());
        shared.insert(state.clone());
        screen.sync(&shared, &mut registry);
        let frame = registry
            .get(registry.get_by_name("WarmodeButton").unwrap())
            .unwrap();
        let Some(WidgetData::Button(button)) = frame.widget_data.as_ref() else {
            panic!("War Mode button")
        };
        assert_eq!(
            button.state,
            ui_toolkit::widgets::button::ButtonState::Normal
        );
        assert_eq!(frame.onclick.as_deref(), Some("pvp:warmode"));
        let mut pvp = game_engine_ui_model::battleground::PvpView::default();
        pvp.snapshot = Some(shared::protocol::PvpSnapshot {
            lifetime_honor_xp: 0,
            honor_level: None,
            honor: 750,
            honor_max: 15000,
            conquest: 120,
            conquest_max: None,
            brackets: vec![],
            queue: None,
            battleground: None,
            war_mode: Some(mode),
            talents: None,
        });
        assert_eq!(
            pvp.action("pvp:warmode"),
            Some(game_engine_ui_model::battleground::PvpAction::WarMode(true))
        );
        assert_eq!(
            pvp.action("pvp:warmode"),
            None,
            "pending click cannot send another mutation"
        );
    }
}

#[test]
fn three_source_pvp_slots_and_authoritative_choice_actions_render_in_both_skins() {
    use shared::protocol::*;
    let data = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../data");
    game_engine_ui_model::paths::set_data_root(data.clone()).unwrap();
    let talents = PvpTalentsSnapshot {
        spec_id: 64,
        slots: vec![
            PvpTalentSlotSnapshot {
                index: 1,
                required_level: 20,
                selected: None,
                can_change: true,
                unlocked: true,
            },
            PvpTalentSlotSnapshot {
                index: 2,
                required_level: 30,
                selected: None,
                can_change: true,
                unlocked: true,
            },
            PvpTalentSlotSnapshot {
                index: 3,
                required_level: 40,
                selected: None,
                can_change: true,
                unlocked: true,
            },
        ],
        choices: vec![
            PvpTalentSnapshot {
                id: 632,
                spell_id: 198148,
                name: "Concentrated Coolness".into(),
                icon: 629077,
                required_level: 40,
                slot_mask: 15,
                unavailable_reason: None,
            },
            PvpTalentSnapshot {
                id: 5622,
                spell_id: 415945,
                name: "Improved Mass Invisibility".into(),
                icon: 1387356,
                required_level: 0,
                slot_mask: 15,
                unavailable_reason: Some("PlayerCondition112521 is not yet evaluated".into()),
            },
        ],
    };
    for skin in [ActiveSkin::Modern, ActiveSkin::Forever] {
        set_thread_skin(skin);
        let mut state = load_preview_state(&data, PlayerSpellsTab::Talents).unwrap();
        state.talents.as_mut().unwrap().pvp = Some(talents.clone());
        state.talents.as_mut().unwrap().pvp_slot = Some(1);
        let mut shared = SharedContext::new();
        shared.insert(state);
        let mut registry = FrameRegistry::new(1920.0, 1080.0);
        let mut screen = Screen::new(spellbook_frame_screen);
        screen.sync(&shared, &mut registry);
        for slot in 1..=3 {
            let frame = registry
                .get(
                    registry
                        .get_by_name(&format!("PvPTalentSlot{slot}"))
                        .expect("Mainline three-slot tray"),
                )
                .unwrap();
            assert_eq!(
                frame.onclick.as_deref(),
                Some(format!("pvp:talent-slot:{slot}").as_str())
            );
        }
        assert!(registry.get_by_name("PvPTalentSlot4").is_none());
        let available = registry
            .get(registry.get_by_name("PvPTalentChoice632").unwrap())
            .unwrap();
        assert_eq!(available.onclick.as_deref(), Some("pvp:talent-choice:632"));
        let locked = registry
            .get(registry.get_by_name("PvPTalentChoice5622").unwrap())
            .unwrap();
        assert_eq!(locked.onclick.as_deref(), Some(""));
        let mut view = game_engine_ui_model::battleground::PvpView::default();
        view.snapshot = Some(PvpSnapshot {
            lifetime_honor_xp: 0,
            honor_level: None,
            honor: 0,
            honor_max: 15000,
            conquest: 0,
            conquest_max: None,
            brackets: vec![],
            queue: None,
            battleground: None,
            war_mode: None,
            talents: Some(talents.clone()),
        });
        view.action("pvp:talent-slot:1");
        assert_eq!(view.action("pvp:talent-choice:5622"), None);
        assert_eq!(
            view.action("pvp:talent-choice:632"),
            Some(
                game_engine_ui_model::battleground::PvpAction::SelectTalent {
                    slot: 1,
                    talent_id: 632
                }
            )
        );
        assert!(view.pending);
        assert_eq!(view.talent_slot, None);
    }
}

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
