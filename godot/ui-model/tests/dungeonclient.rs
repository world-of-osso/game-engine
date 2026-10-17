use game_engine_ui_model::achievements::{AchievementWindow, achievement_screen};
use game_engine_ui_model::dungeon_progress::DungeonObjectives;
use game_engine_ui_model::objective_tracker_component::{
    ObjectiveTrackerState, objective_tracker_screen,
};
use shared::protocol::*;
use ui_toolkit::atlas::{ActiveSkin, set_thread_skin};
use ui_toolkit::frame::WidgetData;
use ui_toolkit::layout_values::Val;
use ui_toolkit::registry::FrameRegistry;
use ui_toolkit::screen::{Screen, SharedContext};
use ui_toolkit::widgets::font_string::{JustifyH, JustifyV};

fn retail_categories() -> Vec<AchievementCategoryEntry> {
    let records = game_engine_core::csv_util::parse_csv_records(include_str!(
        "../../../data/db2/active/Achievement_Category.csv"
    ));
    let header = &records[0];
    let column = |name| header.iter().position(|field| field == name).unwrap();
    let id = column("ID");
    let name = column("Name_lang");
    let parent = column("Parent");
    let order = column("Ui_order");
    records
        .iter()
        .skip(1)
        .map(|fields| AchievementCategoryEntry {
            category_id: fields[id].parse().unwrap(),
            name: fields[name].clone(),
            parent_id: fields[parent].parse().unwrap(),
            order_index: fields[order].parse().unwrap(),
        })
        .collect()
}

#[test]
fn achievements_retail_category_tabs_use_real_db2_hierarchy_both_skins() {
    for skin in [ActiveSkin::Modern, ActiveSkin::Forever] {
        assets(skin);
        let mut window = AchievementWindow::default();
        window.action("micro:AchievementMicroButton");
        window.apply(AchievementCatalogPage::Categories {
            categories: retail_categories(),
            next_id: None,
        });
        let ids = |window: &AchievementWindow| {
            window
                .category_tree()
                .iter()
                .map(|(row, _)| row.category_id)
                .collect::<Vec<_>>()
        };
        assert_eq!(
            ids(&window),
            vec![
                92, 96, 97, 15522, 95, 168, 169, 201, 155, 15117, 15246, 15301, 15234, 81
            ]
        );
        window.action("achievement:category:168");
        assert!(
            window
                .category_tree()
                .iter()
                .any(|(row, depth)| row.category_id == 14808 && *depth == 1)
        );
        window.action("achievement:category:14808");
        assert_eq!(window.selected_category, Some(14808));
        let registry = achievement_registry(&window, skin);
        assert!(registry.get_by_name("AchievementTabGuild").is_some());
        window.action("achievement:guild");
        assert_eq!(
            ids(&window),
            vec![15088, 15077, 15078, 15079, 15080, 15089, 15093]
        );
        window.action("achievement:statistics");
        assert_eq!(
            ids(&window),
            vec![
                130, 128, 122, 133, 132, 134, 131, 15533, 14807, 21, 15219, 15227, 15176, 15549
            ]
        );
    }
}

#[test]
fn achievements_retail_completed_filters_and_feats_are_not_scope_filters() {
    assets(ActiveSkin::Modern);
    let mut window = AchievementWindow::default();
    window.action("micro:AchievementMicroButton");
    window.apply(AchievementCatalogPage::Categories {
        categories: retail_categories(),
        next_id: None,
    });
    window.action("achievement:category:14808");
    let earned = achievement();
    let mut incomplete = earned.clone();
    incomplete.achievement_id = 646;
    incomplete.earned = false;
    incomplete.earned_at = None;
    window.apply(AchievementCatalogPage::Category {
        category_id: 14808,
        achievements: vec![earned.clone(), incomplete.clone()],
        next_id: None,
    });
    let registry = achievement_registry(&window, ActiveSkin::Modern);
    let dropdown = registry
        .get(registry.get_by_name("AchievementFilterDropdown").unwrap())
        .unwrap();
    window.action(
        dropdown
            .onclick
            .as_deref()
            .expect("clickable Retail filter"),
    );
    let menu = achievement_registry(&window, ActiveSkin::Modern);
    let choice = menu
        .get(menu.get_by_name("AchievementFiltercompleted").unwrap())
        .unwrap();
    assert_eq!(choice.height.value(), 20.0);
    assert!(menu.get_by_name("AchievementFiltercompletedBox").is_some());
    assert!(
        menu.get_by_name("AchievementFilterMenuBackground")
            .is_some()
    );
    window.action(
        choice
            .onclick
            .as_deref()
            .expect("clickable completed choice"),
    );
    assert_eq!(
        window
            .displayed_entries()
            .iter()
            .map(|row| row.achievement_id)
            .collect::<Vec<_>>(),
        vec![633]
    );
    window.action("achievement:filter:incomplete");
    assert_eq!(
        window
            .displayed_entries()
            .iter()
            .map(|row| row.achievement_id)
            .collect::<Vec<_>>(),
        vec![646]
    );
    window.action("achievement:filter:all");
    assert_eq!(window.displayed_entries().len(), 2);
    window.action("achievement:category:81");
    window.apply(AchievementCatalogPage::Category {
        category_id: 81,
        achievements: vec![earned, incomplete],
        next_id: None,
    });
    assert_eq!(
        window.displayed_entries().len(),
        1,
        "Retail Feats shows earned entries only"
    );
}

#[test]
fn achievements_comparison_correlates_target_and_preserves_own_progress_both_skins() {
    for skin in [ActiveSkin::Modern, ActiveSkin::Forever] {
        assets(skin);
        let mut window = AchievementWindow::default();
        let requests = window.action("achievement:compare:42");
        assert!(requests.iter().any(|request| matches!(
            request,
            QueryAchievementCatalog::Comparison {
                target_entity: 42,
                ..
            }
        )));
        window.apply(AchievementCatalogPage::Categories {
            categories: vec![category()],
            next_id: None,
        });
        window.action("achievement:category:14808");
        let own = achievement();
        window.apply(AchievementCatalogPage::Category {
            category_id: 14808,
            achievements: vec![own.clone()],
            next_id: None,
        });
        let mut other = own.clone();
        other.earned = true;
        other.earned_at = Some(1_791_244_800);
        let reply = |target| AchievementCatalogPage::Comparison {
            target_entity: target,
            target_name: "Otherplayer".into(),
            page: Some(Box::new(AchievementComparisonPage::Category {
                category_id: 14808,
                achievements: vec![other.clone()],
                next_id: None,
            })),
            error: None,
        };
        window.apply(reply(41));
        let waiting = achievement_registry(&window, skin);
        assert_eq!(
            text(&waiting, "Achievement633ComparisonStatus"),
            "Loading..."
        );
        window.apply(reply(42));
        assert_eq!(window.pages[&14808].entries[0].earned, own.earned);
        let registry = achievement_registry(&window, skin);
        assert_eq!(text(&registry, "AchievementComparisonName"), "Otherplayer");
        assert_eq!(
            text(&registry, "Achievement633ComparisonStatus"),
            "Completed"
        );
        window.action("achievement:compare:43");
        window.action("achievement:category:14808");
        window.apply(reply(42));
        let registry = achievement_registry(&window, skin);
        assert_eq!(
            text(&registry, "Achievement633ComparisonStatus"),
            "Loading..."
        );
        window.apply(AchievementCatalogPage::Comparison {
            target_entity: 43,
            target_name: String::new(),
            page: None,
            error: Some("comparison target is not an online player".into()),
        });
        let registry = achievement_registry(&window, skin);
        assert_eq!(
            text(&registry, "AchievementComparisonError"),
            "comparison target is not an online player"
        );
    }
}

#[test]
fn achievements_art_categories_collapse_and_rows_expand_in_place_both_skins() {
    for skin in [ActiveSkin::Modern, ActiveSkin::Forever] {
        assets(skin);
        let mut window = AchievementWindow::default();
        window.action("micro:AchievementMicroButton");
        let mut child = category();
        child.parent_id = 168;
        window.apply(AchievementCatalogPage::Categories {
            categories: vec![
                AchievementCategoryEntry {
                    category_id: 168,
                    parent_id: -1,
                    order_index: 1,
                    name: "Dungeons & Raids".into(),
                },
                child,
            ],
            next_id: None,
        });
        assert_eq!(
            window.category_tree().len(),
            1,
            "Retail starts with collapsed headers"
        );
        window.action("achievement:category:168");
        assert_eq!(window.category_tree().len(), 2);
        window.action("achievement:category:168");
        assert_eq!(window.category_tree().len(), 1);
        window.action("achievement:category:14808");
        let mut second = achievement();
        second.achievement_id = 646;
        window.apply(AchievementCatalogPage::Category {
            category_id: 14808,
            achievements: vec![achievement(), second],
            next_id: None,
        });
        window.action("achievement:row:633");
        let expanded = achievement_registry(&window, skin);
        let first = expanded
            .get(expanded.get_by_name("Achievement633Row").unwrap())
            .unwrap();
        let next = expanded
            .get(expanded.get_by_name("Achievement646Row").unwrap())
            .unwrap();
        let ui_toolkit::frame::Dimension::Fixed(height) = first.height else {
            panic!("row height");
        };
        let ui_toolkit::layout_values::Val::Px(top) = first.position.top else {
            panic!("row top");
        };
        let ui_toolkit::layout_values::Val::Px(next_top) = next.position.top else {
            panic!("next top");
        };
        assert!(height > 84.0);
        assert!(
            next_top >= top + height,
            "criteria push later rows down, not separate panel"
        );
        assert!(expanded.get_by_name("Achievement633Criterion1").is_some());
        window.action("achievement:row:633");
        let collapsed = achievement_registry(&window, skin);
        assert!(collapsed.get_by_name("Achievement633Criterion1").is_none());
        assert!(collapsed.get_by_name("Achievement633IconFrame").is_some());
    }
}

#[test]
fn achievements_art_category_scroll_reaches_last_child_both_skins() {
    for skin in [ActiveSkin::Modern, ActiveSkin::Forever] {
        assets(skin);
        let mut window = AchievementWindow::default();
        window.action("micro:AchievementMicroButton");
        let mut categories = vec![
            AchievementCategoryEntry {
                category_id: 168,
                parent_id: -1,
                order_index: 7,
                name: "Dungeons & Raids".into(),
            },
            category(),
        ];
        for id in 20000..20024 {
            categories.push(AchievementCategoryEntry {
                category_id: id,
                parent_id: 168,
                order_index: (id - 20000) as i32,
                name: format!("Dungeon category {id}"),
            });
        }
        window.apply(AchievementCatalogPage::Categories {
            categories,
            next_id: None,
        });
        window.action("achievement:category:168");
        let mut shared = SharedContext::new();
        shared.insert(window);
        let mut registry = FrameRegistry::new(1920.0, 1080.0);
        let mut screen = Screen::new(achievement_screen);
        screen.sync(&shared, &mut registry);
        assert!(
            registry
                .scroll_lists
                .scroll_by("AchievementCategoriesScrollBox", 10000),
            "wheel/thumb scrolling must reach categories beyond the visible tree"
        );
        screen.sync(&shared, &mut registry);
        assert_eq!(
            text(&registry, "AchievementCategory20023"),
            "Dungeon category 20023"
        );
        let state = registry
            .scroll_lists
            .get("AchievementCategoriesScrollBox")
            .unwrap();
        assert_eq!(state.first_row, state.geometry.max_first_row());
    }
}

#[test]
fn achievements_art_statistics_never_reuses_recent_achievement_rows() {
    let mut window = AchievementWindow::default();
    window.action("micro:AchievementMicroButton");
    window.apply(AchievementCatalogPage::Summary(AchievementSummary {
        scope: AchievementScope::All,
        points: 10,
        total: 8078,
        completed: 1,
        categories: vec![],
        recent: vec![achievement()],
    }));
    window.action("achievement:statistics");
    assert!(
        window.displayed_entries().is_empty(),
        "statistics must not display earned achievement rows"
    );
}

#[test]
fn achievements_art_rows_scroll_and_cursor_append_preserves_expansion_both_skins() {
    for skin in [ActiveSkin::Modern, ActiveSkin::Forever] {
        assets(skin);
        let mut window = AchievementWindow::default();
        window.action("micro:AchievementMicroButton");
        window.apply(AchievementCatalogPage::Categories {
            categories: vec![category()],
            next_id: None,
        });
        window.action("achievement:category:14808");
        let entries = (633..=650)
            .map(|id| AchievementCatalogEntry {
                achievement_id: id,
                name: format!("Dungeon achievement {id}"),
                ..achievement()
            })
            .collect();
        window.apply(AchievementCatalogPage::Category {
            category_id: 14808,
            achievements: entries,
            next_id: Some(650),
        });
        window.action("achievement:row:633");
        assert_eq!(
            window.action("achievement:more_rows"),
            vec![QueryAchievementCatalog::Category {
                category_id: 14808,
                after_id: 650
            }]
        );
        window.apply(AchievementCatalogPage::Category {
            category_id: 14808,
            achievements: vec![AchievementCatalogEntry {
                achievement_id: 651,
                ..achievement()
            }],
            next_id: None,
        });
        assert_eq!(
            window.expanded_achievement,
            Some(633),
            "appending a cursor page must not collapse the selected row"
        );
        let mut shared = SharedContext::new();
        shared.insert(window);
        let mut registry = FrameRegistry::new(1920.0, 1080.0);
        let mut screen = Screen::new(achievement_screen);
        screen.sync(&shared, &mut registry);
        assert!(
            registry
                .scroll_lists
                .scroll_by("AchievementRowsScrollBox", 10000)
        );
        screen.sync(&shared, &mut registry);
        assert_eq!(text(&registry, "Achievement651Name"), "Stormwind Stockade");
        let state = registry
            .scroll_lists
            .get("AchievementRowsScrollBox")
            .unwrap();
        assert_eq!(state.first_row, state.geometry.max_first_row());
    }
}

#[test]
fn achievements_art_postsetup_paints_only_rendered_category_highlights() {
    assets(ActiveSkin::Modern);
    let mut window = AchievementWindow::default();
    window.visible = true;
    window.statistics = true;
    window.selected_category = Some(14822);
    let mut outside_statistics_tree = category();
    outside_statistics_tree.category_id = 14822;
    window.categories.insert(14822, outside_statistics_tree);
    let mut registry = achievement_registry(&window, ActiveSkin::Modern);
    game_engine_ui_model::achievements::apply_achievement_postsetup(&mut registry);
    assert_eq!(
        text(&registry, "AchievementEmpty"),
        "Loading achievements..."
    );
}

fn assets(skin: ActiveSkin) {
    game_engine_ui_model::paths::set_data_root(
        std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../data"),
    )
    .unwrap();
    set_thread_skin(skin);
}
fn text(registry: &FrameRegistry, name: &str) -> String {
    let frame = registry
        .get(registry.get_by_name(name).expect(name))
        .unwrap();
    match frame.widget_data.as_ref().unwrap() {
        WidgetData::FontString(value) => value.text.clone(),
        other => panic!("{name}: {other:?}"),
    }
}
fn tracker(objectives: &DungeonObjectives, skin: ActiveSkin) -> FrameRegistry {
    let mut state = ObjectiveTrackerState::default();
    state.dungeon = objectives.block.clone();
    let mut shared = SharedContext::new();
    shared.insert(skin);
    shared.insert(state);
    let mut registry = FrameRegistry::new(1920.0, 1080.0);
    Screen::new(objective_tracker_screen).sync(&shared, &mut registry);
    registry
}
#[test]
fn dungeonclient_stockade_checks_three_bosses_in_order_then_clears_both_skins() {
    for skin in [ActiveSkin::Modern, ActiveSkin::Forever] {
        assets(skin);
        let mut objectives = DungeonObjectives::default();
        objectives.begin_map(34);
        let names = ["Randolph Moloch", "Lord Overheat", "Hogger"];
        for killed in 0..=3 {
            assert!(
                objectives.apply(
                    DungeonProgress {
                        map_id: 34,
                        instance_id: 7,
                        difficulty_id: 1,
                        encounters: names
                            .iter()
                            .enumerate()
                            .map(|(i, name)| DungeonEncounterProgress {
                                encounter_id: i as u32 + 1,
                                name: (*name).into(),
                                defeated: i < killed,
                                optional: None,
                                flags: 0,
                            })
                            .collect(),
                    },
                    "The Stockade".into()
                )
            );
            let registry = tracker(&objectives, skin);
            assert_eq!(
                text(&registry, "DungeonObjectiveTrackerHeaderText"),
                "The Stockade"
            );
            for (i, name) in names.iter().enumerate() {
                assert_eq!(text(&registry, &format!("DungeonBoss{i}Text")), *name);
                assert_eq!(
                    registry
                        .get_by_name(&format!("DungeonBoss{i}Check"))
                        .is_some(),
                    i < killed
                );
            }
        }
        assert!(!objectives.apply(
            DungeonProgress {
                map_id: 34,
                instance_id: 8,
                difficulty_id: 1,
                encounters: vec![]
            },
            "wrong copy".into()
        ));
        objectives.begin_map(0);
        objectives.apply(
            DungeonProgress {
                map_id: 0,
                instance_id: 0,
                difficulty_id: 1,
                encounters: vec![],
            },
            "Eastern Kingdoms".into(),
        );
        assert!(
            tracker(&objectives, skin)
                .get_by_name("DungeonObjectiveTrackerHeaderText")
                .is_none()
        );
    }
}
#[test]
fn achievements_ui_summary_opens_by_default_and_uses_authoritative_points_both_skins() {
    for skin in [ActiveSkin::Modern, ActiveSkin::Forever] {
        assets(skin);
        let mut window = AchievementWindow::default();
        let requests = window.action("micro:AchievementMicroButton");
        assert!(requests.contains(&QueryAchievementCatalog::Summary {
            scope: AchievementScope::All
        }));
        window.apply(AchievementCatalogPage::Summary(AchievementSummary {
            scope: AchievementScope::All,
            points: 120,
            total: 100,
            completed: 12,
            categories: vec![AchievementCategoryTotals {
                category_id: 14808,
                total: 10,
                completed: 1,
            }],
            recent: vec![achievement()],
        }));
        let mut shared = SharedContext::new();
        shared.insert(window);
        let mut registry = FrameRegistry::new(1920.0, 1080.0);
        Screen::new(achievement_screen).sync(&shared, &mut registry);
        assert_eq!(text(&registry, "AchievementFrameHeaderPoints"), "120");
        assert_eq!(text(&registry, "AchievementSummaryTotalText"), "12 / 100");
        assert_eq!(text(&registry, "Achievement633Name"), "Stormwind Stockade");
    }
}

#[test]
fn achievements_ui_scope_tabs_filter_authored_account_flag_both_skins() {
    for skin in [ActiveSkin::Modern, ActiveSkin::Forever] {
        assets(skin);
        let mut window = AchievementWindow::default();
        window.action("micro:AchievementMicroButton");
        window.apply(AchievementCatalogPage::Categories {
            categories: vec![category()],
            next_id: None,
        });
        window.action("achievement:category:14808");
        let mut account = achievement();
        account.achievement_id = 646;
        account.name = "Stratholme".into();
        account.description = "Defeat the evil masterminds inhabiting Stratholme.".into();
        account.icon_fdid = 237511;
        account.flags = 0x20000;
        account.criteria.clear();
        window.apply(AchievementCatalogPage::Category {
            category_id: 14808,
            achievements: vec![achievement(), account],
            next_id: None,
        });
        assert_eq!(
            window.action("achievement:scope:account"),
            vec![QueryAchievementCatalog::Summary {
                scope: AchievementScope::Account
            }]
        );
        let mut shared = SharedContext::new();
        shared.insert(window.clone());
        let mut registry = FrameRegistry::new(1920.0, 1080.0);
        let mut screen = Screen::new(achievement_screen);
        screen.sync(&shared, &mut registry);
        assert!(registry.get_by_name("Achievement633Name").is_none());
        assert_eq!(text(&registry, "Achievement646Name"), "Stratholme");
        assert_eq!(
            window.action("achievement:scope:character"),
            vec![QueryAchievementCatalog::Summary {
                scope: AchievementScope::Character
            }]
        );
        shared.insert(window);
        screen.sync(&shared, &mut registry);
        assert_eq!(text(&registry, "Achievement633Name"), "Stormwind Stockade");
        assert!(registry.get_by_name("Achievement646Name").is_none());
    }
}

#[test]
fn achievements_ui_summary_recent_criteria_continuation_merges_into_selected_row() {
    // Authentic pinned DB2: achievement1676, tree65059; server's first page is16 IDs.
    let rows = [
        (65060, "Arathi Highlands Quests", 0, 15204),
        (65061, "Badlands Quests", 0, 15205),
        (65062, "Blasted Lands Quests", 0, 15206),
        (65063, "Burning Steppes Quests", 0, 15207),
        (65064, "Cape of Stranglethorn Quests", 0, 15208),
        (65066, "Eastern Plaguelands Quests", 0, 15209),
        (65067, "Hinterlands Quests", 0, 15212),
        (65069, "Northern Stranglethorn Quests", 0, 15213),
        (65070, "Redridge Mountains Quests", 32, 15198),
        (65071, "Searing Gorge Quests", 0, 15214),
        (65072, "Swamp of Sorrows Quests", 0, 15216),
        (65073, "Western Plaguelands Quests", 0, 15217),
        (65074, "Westfall Quests & Duskwood Quests", 32, 39734),
        (65077, "", 2, 22229),
        (65081, "Silverpine Forest Quests", 32, 15215),
        (65083, "Hillsbrad Foothills Quests", 32, 15211),
        (65087, "Ghostlands Quests", 32, 15210),
        (65292, "Ghostlands Quests", 32, 39733),
    ]
    .map(
        |(tree_id, description, order_index, criteria_id)| AchievementCriterionLine {
            tree_id,
            description: description.into(),
            order_index,
            criteria_id,
            flags: 0,
            current: 0,
            required: 0,
            completed: false,
            progress_supported: true,
        },
    );
    let mut window = AchievementWindow::default();
    window.action("micro:AchievementMicroButton");
    let mut recent = achievement();
    recent.achievement_id = 1676;
    recent.name = "Loremaster of Eastern Kingdoms".into();
    recent.description = "Complete the Eastern Kingdoms quest achievements listed below.".into();
    recent.icon_fdid = 236759;
    recent.flags = 0x20000;
    recent.criteria = rows[..16].to_vec();
    recent.next_criteria_id = Some(65083);
    window.apply(AchievementCatalogPage::Summary(AchievementSummary {
        scope: AchievementScope::All,
        points: 10,
        total: 100,
        completed: 1,
        categories: vec![],
        recent: vec![recent],
    }));
    window.action("achievement:row:1676");
    window.action("achievement:more_criteria");
    window.action("achievement:more_criteria");
    assert_eq!(
        window.action("achievement:more_criteria"),
        vec![QueryAchievementCatalog::Criteria {
            achievement_id: 1676,
            after_id: 65083
        }]
    );
    window.apply(AchievementCatalogPage::Criteria {
        achievement_id: 1676,
        criteria: rows[16..].to_vec(),
        next_id: None,
    });
    assert_eq!(window.selected().unwrap().criteria.len(), 18);
    assert!(window.selected().unwrap().next_criteria_id.is_none());
    assert_eq!(
        window.selected().unwrap().criteria.last().unwrap().tree_id,
        65292
    );
}

#[test]
fn achievements_ui_uses_authored_sheet_regions_for_recent_icons_and_shields() {
    assets(ActiveSkin::Modern);
    let mut window = AchievementWindow::default();
    window.action("micro:AchievementMicroButton");
    window.apply(AchievementCatalogPage::Summary(AchievementSummary {
        scope: AchievementScope::All,
        points: 10,
        total: 8078,
        completed: 1,
        categories: vec![],
        recent: vec![achievement()],
    }));
    let registry = achievement_registry(&window, ActiveSkin::Modern);
    for (name, coords) in [
        ("Achievement633IconFrame", [0.0, 0.5625, 0.0, 0.5625]),
        ("Achievement633Shield", [0.0, 0.5, 0.0, 0.5]),
    ] {
        let frame = registry.get(registry.get_by_name(name).unwrap()).unwrap();
        let Some(WidgetData::Texture(texture)) = frame.widget_data.as_ref() else {
            panic!("rendered texture")
        };
        assert_eq!(texture.tex_coords, coords);
        if name == "Achievement633Shield" {
            assert_eq!(
                texture.source,
                ui_toolkit::widgets::texture::TextureSource::FileDataId(130666)
            );
        }
    }
}

#[test]
fn achievementlinks_toast_uses_mainline_icon_shield_and_plate_layout_both_skins() {
    for skin in [ActiveSkin::Modern, ActiveSkin::Forever] {
        assets(skin);
        let mut shared = SharedContext::new();
        shared.insert(skin);
        shared.insert(AchievementToastSnapshot {
            achievement_id: 9,
            name: "Level 40".into(),
            points: 10,
            icon_fdid: 236565,
        });
        let mut registry = FrameRegistry::new(1920.0, 1080.0);
        Screen::new(game_engine_ui_model::achievements::achievement_toast_screen)
            .sync(&shared, &mut registry);
        assert!(registry.get_by_name("AchievementToastIcon").is_some());
        assert!(registry.get_by_name("AchievementToastShield").is_some());
        assert_eq!(text(&registry, "AchievementToastPoints"), "10");
        let frame = registry
            .get(registry.get_by_name("AchievementToast").unwrap())
            .unwrap();
        assert_eq!((frame.width.value(), frame.height.value()), (300.0, 101.0));
    }
}

#[test]
fn achievementlinks_open_unloaded_achievement_and_keep_server_earned_state() {
    let mut window = AchievementWindow::default();
    window.apply(AchievementCatalogPage::Categories {
        categories: vec![
            AchievementCategoryEntry {
                category_id: 92,
                parent_id: -1,
                order_index: 1,
                name: "Characters".into(),
            },
            AchievementCategoryEntry {
                category_id: 130,
                parent_id: 1,
                order_index: 1,
                name: "Character".into(),
            },
        ],
        next_id: None,
    });
    window.action("achievement:statistics");
    assert!(window.action("achievement:open:633").contains(
        &QueryAchievementCatalog::Achievement {
            achievement_id: 633
        }
    ));
    window.apply(AchievementCatalogPage::Achievement {
        achievement_id: 633,
        entry: Some(achievement()),
    });
    assert!(window.visible);
    assert_eq!(window.selected().unwrap().achievement_id, 633);
    assert!(window.selected().unwrap().earned);
    assert_eq!(
        window
            .category_tree()
            .iter()
            .map(|(row, _)| row.name.as_str())
            .collect::<Vec<_>>(),
        vec!["Characters"]
    );
}

#[test]
fn achievementlinks_chat_displays_one_clickable_name_without_transport_markup() {
    use game_engine_ui_model::chat_frame::{chat_message_line, wrap_chat_line};
    let message = game_engine_ui_model::chat_data::ChatMessage {
        channel_type: game_engine_ui_model::chat_data::ChatChannelType::System,
        channel_name: String::new(),
        sender: String::new(),
        timestamp: 0.0,
        text: "You have earned the achievement |Hachievement:633|h[Stormwind Stockade]|h.".into(),
    };
    let line = chat_message_line(&message);
    assert_eq!(
        line.plain_text(|_| String::new()),
        "You have earned the achievement [Stormwind Stockade]."
    );
    let rows = wrap_chat_line(&line, |_| String::new(), 1000.0, |text| text.len() as f32);
    assert!(
        rows.iter()
            .flat_map(|row| &row.runs)
            .any(|run| run.text == "[Stormwind Stockade]")
    );
}

fn category() -> AchievementCategoryEntry {
    AchievementCategoryEntry {
        category_id: 14808,
        parent_id: 168,
        order_index: 1,
        name: "Classic".into(),
    }
}
fn criterion(tree_id: u32) -> AchievementCriterionLine {
    AchievementCriterionLine {
        tree_id,
        order_index: tree_id as i32,
        criteria_id: 3666,
        description: "Hogger defeated".into(),
        flags: 0,
        current: 1,
        required: 1,
        completed: true,
        progress_supported: true,
    }
}
fn achievement() -> AchievementCatalogEntry {
    AchievementCatalogEntry {
        achievement_id: 633,
        name: "Stormwind Stockade".into(),
        description: "Defeat Hogger in the Stormwind Stockade.".into(),
        points: 10,
        icon_fdid: 136363,
        flags: 0,
        statistic_value: None,
        earned: true,
        earned_at: Some(1791244800),
        progress_supported: true,
        criteria: vec![criterion(1)],
        next_criteria_id: Some(1),
    }
}
#[test]
fn dungeonclient_live_update_requests_fresh_page_even_with_query_in_flight() {
    let mut window = AchievementWindow::default();
    window.action("micro:AchievementMicroButton");
    window.apply(AchievementCatalogPage::Categories {
        categories: vec![category()],
        next_id: None,
    });
    window.action("achievement:category:14808");
    assert_eq!(
        window.refresh(&AchievementStateUpdate {
            snapshot: None,
            completed: None,
            message: None,
            error: None
        }),
        vec![
            QueryAchievementCatalog::Summary {
                scope: AchievementScope::All
            },
            QueryAchievementCatalog::Category {
                category_id: 14808,
                after_id: 0
            }
        ]
    );
    let mut stale = achievement();
    stale.earned = false;
    stale.earned_at = None;
    window.apply(AchievementCatalogPage::Category {
        category_id: 14808,
        achievements: vec![stale],
        next_id: None,
    });
    assert!(!window.selected().unwrap().earned);
    window.apply(AchievementCatalogPage::Category {
        category_id: 14808,
        achievements: vec![achievement()],
        next_id: None,
    });
    assert!(window.selected().unwrap().earned);
}

#[test]
fn achievements_ui_tracking_restores_authoritative_rows_after_new_session() {
    let mut window = AchievementWindow::default();
    assert_eq!(
        window.action("achievement:restore:633"),
        vec![QueryAchievementCatalog::Achievement {
            achievement_id: 633
        }]
    );
    let mut row = achievement();
    row.earned = false;
    window.apply(AchievementCatalogPage::Achievement {
        achievement_id: 633,
        entry: Some(row),
    });
    assets(ActiveSkin::Modern);
    let mut shared = SharedContext::new();
    shared.insert(ActiveSkin::Modern);
    shared.insert(ObjectiveTrackerState::default());
    shared.insert(window);
    let mut registry = FrameRegistry::new(1920.0, 1080.0);
    Screen::new(objective_tracker_screen).sync(&shared, &mut registry);
    assert_eq!(
        text(&registry, "TrackedAchievement633HeaderText"),
        "Stormwind Stockade"
    );
}

#[test]
fn achievements_ui_browse_statistics_and_correlated_full_search_both_skins() {
    for skin in [ActiveSkin::Modern, ActiveSkin::Forever] {
        assets(skin);
        let mut window = AchievementWindow::default();
        window.action("micro:AchievementMicroButton");
        window.apply(AchievementCatalogPage::Categories {
            categories: vec![AchievementCategoryEntry {
                category_id: 14822,
                parent_id: -1,
                order_index: 1,
                name: "The Burning Crusade".into(),
            }],
            next_id: None,
        });
        window.action("achievement:statistics");
        assert_eq!(
            window.action("achievement:category:14822"),
            vec![QueryAchievementCatalog::Statistics {
                category_id: 14822,
                after_id: 0
            }]
        );
        let row = AchievementCatalogEntry {
            achievement_id: 1068,
            name: "Keli'dan the Breaker kills (The Blood Furnace)".into(),
            flags: 1,
            statistic_value: Some(7),
            earned: false,
            criteria: vec![],
            ..achievement()
        };
        window.apply(AchievementCatalogPage::Statistics {
            category_id: 14822,
            achievements: vec![row],
            next_id: None,
        });
        let statistics = achievement_registry(&window, skin);
        assert_eq!(text(&statistics, "Achievement1068Statistic"), "7");
        assert!(statistics.get_by_name("AchievementCriteriaTitle").is_none());
        assert!(statistics.get_by_name("AchievementTrack").is_none());
        assert_eq!(
            window.action("achievement:search:stockade"),
            vec![QueryAchievementCatalog::Search {
                text: "stockade".into(),
                scope: AchievementScope::All,
                after_id: 0
            }]
        );
        // A prior term must not replace current search results.
        window.apply(AchievementCatalogPage::Search {
            text: "level".into(),
            scope: AchievementScope::All,
            achievements: vec![],
            next_id: None,
        });
        window.apply(AchievementCatalogPage::Search {
            text: "stockade".into(),
            scope: AchievementScope::All,
            achievements: vec![achievement()],
            next_id: None,
        });
        assert_eq!(
            text(&achievement_registry(&window, skin), "Achievement633Name"),
            "Stormwind Stockade"
        );
        assert!(window.action("achievement:row:633").contains(
            &QueryAchievementCatalog::Achievement {
                achievement_id: 633
            }
        ));
    }
}

fn achievement_registry(window: &AchievementWindow, skin: ActiveSkin) -> FrameRegistry {
    let mut shared = SharedContext::new();
    shared.insert(skin);
    shared.insert(window.clone());
    let mut registry = FrameRegistry::new(1920.0, 1080.0);
    Screen::new(achievement_screen).sync(&shared, &mut registry);
    registry
}

#[test]
fn achievements_ui_tracking_survives_close_refreshes_and_clears_on_earn_both_skins() {
    for skin in [ActiveSkin::Modern, ActiveSkin::Forever] {
        assets(skin);
        let mut window = AchievementWindow::default();
        window.action("achievement:open:633");
        let mut row = achievement();
        row.earned = false;
        row.earned_at = None;
        row.criteria[0].current = 0;
        row.criteria[0].completed = false;
        window.apply(AchievementCatalogPage::Achievement {
            achievement_id: 633,
            entry: Some(row.clone()),
        });
        window.action("achievement:track:633");
        window.action("achievement:close");
        let mut shared = SharedContext::new();
        shared.insert(skin);
        shared.insert(ObjectiveTrackerState::default());
        shared.insert(window.clone());
        let mut registry = FrameRegistry::new(1920.0, 1080.0);
        let mut screen = Screen::new(objective_tracker_screen);
        screen.sync(&shared, &mut registry);
        assert_eq!(
            text(&registry, "TrackedAchievement633HeaderText"),
            "Stormwind Stockade"
        );
        assert!(text(&registry, "TrackedAchievement633Criterion0Text").contains("0/1"));
        assert_eq!(
            window.refresh(&AchievementStateUpdate {
                snapshot: None,
                completed: None,
                message: None,
                error: None,
            }),
            vec![QueryAchievementCatalog::Achievement {
                achievement_id: 633
            }]
        );
        row.earned = true;
        row.criteria[0].current = 1;
        row.criteria[0].completed = true;
        window.apply(AchievementCatalogPage::Achievement {
            achievement_id: 633,
            entry: Some(row),
        });
        shared.insert(window);
        screen.sync(&shared, &mut registry);
        assert!(
            registry
                .get_by_name("TrackedAchievement633HeaderText")
                .is_none()
        );
    }
}

#[test]
fn dungeonclient_closed_live_update_refreshes_on_reopen() {
    let mut window = AchievementWindow::default();
    window.action("micro:AchievementMicroButton");
    window.apply(AchievementCatalogPage::Categories {
        categories: vec![category()],
        next_id: None,
    });
    window.action("achievement:category:14808");
    window.apply(AchievementCatalogPage::Category {
        category_id: 14808,
        achievements: vec![achievement()],
        next_id: None,
    });
    window.action("achievement:close");
    assert!(
        window
            .refresh(&AchievementStateUpdate {
                snapshot: None,
                completed: None,
                message: None,
                error: None
            })
            .is_empty()
    );
    assert_eq!(
        window.action("micro:AchievementMicroButton"),
        vec![QueryAchievementCatalog::Category {
            category_id: 14808,
            after_id: 0
        }]
    );
}

#[test]
fn dungeonclient_nonempty_cursor_pages_and_authored_tree_are_browsable() {
    let mut window = AchievementWindow::default();
    window.action("micro:AchievementMicroButton");
    let mut child = category();
    child.parent_id = 168;
    assert_eq!(
        window.apply(AchievementCatalogPage::Categories {
            categories: vec![child],
            next_id: Some(14808)
        }),
        vec![QueryAchievementCatalog::Categories { after_id: 14808 }]
    );
    window.apply(AchievementCatalogPage::Categories {
        categories: vec![AchievementCategoryEntry {
            category_id: 168,
            parent_id: -1,
            order_index: 7,
            name: "Dungeons & Raids".into(),
        }],
        next_id: None,
    });
    assert_eq!(
        window
            .category_tree()
            .iter()
            .map(|(row, depth)| (row.category_id, *depth))
            .collect::<Vec<_>>(),
        vec![(168, 0)]
    );
    window.action("achievement:category:14808");
    let entries = (625..=632)
        .map(|id| AchievementCatalogEntry {
            achievement_id: id,
            criteria: vec![],
            next_criteria_id: None,
            ..achievement()
        })
        .collect();
    window.apply(AchievementCatalogPage::Category {
        category_id: 14808,
        achievements: entries,
        next_id: Some(632),
    });
    assert_eq!(
        window.action("achievement:more_rows"),
        vec![QueryAchievementCatalog::Category {
            category_id: 14808,
            after_id: 632
        }]
    );
    assert!(
        window.action("achievement:more_rows").is_empty(),
        "a cursor already in flight is not sent twice"
    );
    let mut stockade = achievement();
    stockade.criteria = (1..=16).map(criterion).collect();
    stockade.next_criteria_id = Some(16);
    window.apply(AchievementCatalogPage::Category {
        category_id: 14808,
        achievements: vec![stockade],
        next_id: None,
    });
    window.action("achievement:more_rows");
    window.action("achievement:row:633");
    assert_eq!(window.selected().unwrap().achievement_id, 633);
    for _ in 0..2 {
        window.action("achievement:more_criteria");
    }
    assert_eq!(
        window.action("achievement:more_criteria"),
        vec![QueryAchievementCatalog::Criteria {
            achievement_id: 633,
            after_id: 16
        }]
    );
    window.apply(AchievementCatalogPage::Criteria {
        achievement_id: 633,
        criteria: vec![criterion(17), criterion(18)],
        next_id: None,
    });
    assert_eq!(window.selected().unwrap().criteria.len(), 18);
    assert!(window.selected().unwrap().next_criteria_id.is_none());
    window.action("achievement:criteria_prev");
    assert_eq!(window.criterion_offset, 6);
}

#[test]
fn dungeonclient_launcher_requests_categories_and_catalog_pages_render_both_skins() {
    for skin in [ActiveSkin::Modern, ActiveSkin::Forever] {
        assets(skin);
        let mut launcher = game_engine_ui_model::launcher::LauncherView::default();
        launcher.action(game_engine_ui_model::launcher::ACTION_OPEN);
        let destination = launcher.action("micro:AchievementMicroButton").unwrap();
        let mut window = AchievementWindow::default();
        assert_eq!(
            window.action(&destination),
            vec![
                QueryAchievementCatalog::Categories { after_id: 0 },
                QueryAchievementCatalog::Summary {
                    scope: AchievementScope::All
                }
            ]
        );
        assert_eq!(
            window.apply(AchievementCatalogPage::Categories {
                categories: retail_categories(),
                next_id: Some(14808)
            }),
            vec![QueryAchievementCatalog::Categories { after_id: 14808 }]
        );
        assert!(
            window
                .apply(AchievementCatalogPage::Categories {
                    categories: vec![],
                    next_id: None
                })
                .is_empty()
        );
        assert_eq!(
            window.action("achievement:category:14808"),
            vec![QueryAchievementCatalog::Category {
                category_id: 14808,
                after_id: 0
            }]
        );
        window.apply(AchievementCatalogPage::Category {
            category_id: 14808,
            achievements: vec![achievement()],
            next_id: Some(633),
        });
        assert_eq!(
            window.action("achievement:more_rows"),
            vec![QueryAchievementCatalog::Category {
                category_id: 14808,
                after_id: 633
            }]
        );
        window.apply(AchievementCatalogPage::Category {
            category_id: 14808,
            achievements: vec![],
            next_id: None,
        });
        assert_eq!(
            window.action("achievement:more_criteria"),
            vec![QueryAchievementCatalog::Criteria {
                achievement_id: 633,
                after_id: 1
            }]
        );
        window.apply(AchievementCatalogPage::Criteria {
            achievement_id: 633,
            criteria: vec![criterion(2)],
            next_id: None,
        });
        window.apply(AchievementCatalogPage::Summary(AchievementSummary {
            scope: AchievementScope::All,
            points: 10,
            total: 100,
            completed: 1,
            categories: vec![],
            recent: vec![achievement()],
        }));
        window.action("achievement:row:633");
        let mut shared = SharedContext::new();
        shared.insert(skin);
        shared.insert(window.clone());
        let mut registry = FrameRegistry::new(1920.0, 1080.0);
        Screen::new(achievement_screen).sync(&shared, &mut registry);
        assert_eq!(text(&registry, "AchievementCategory14808"), "Classic");
        assert_eq!(text(&registry, "Achievement633Name"), "Stormwind Stockade");
        assert_eq!(text(&registry, "Achievement633Points"), "10");
        assert_eq!(
            text(&registry, "Achievement633Date"),
            "Earned 2026-10-06 (UTC)"
        );
        for tree in [1, 2] {
            assert_eq!(
                text(&registry, &format!("Achievement633Criterion{tree}")),
                "Hogger defeated"
            );
            assert!(
                registry
                    .get_by_name(&format!("Achievement633Criterion{tree}Check"))
                    .is_some()
            );
        }
        assert_eq!(text(&registry, "AchievementFrameHeaderPoints"), "10");
        let points = registry
            .get(
                registry
                    .get_by_name("AchievementFrameHeaderPoints")
                    .unwrap(),
            )
            .unwrap();
        let shield = registry
            .get(
                registry
                    .get_by_name("AchievementFrameHeaderShield")
                    .unwrap(),
            )
            .unwrap();
        let ui_toolkit::frame::Dimension::Fixed(width) = points.width else {
            panic!("fixed points width")
        };
        let ui_toolkit::layout_values::Val::Px(left) = points.position.left else {
            panic!("absolute points position")
        };
        assert_eq!(left + width / 2.0, 409.0);
        assert_eq!(
            points.position.top,
            ui_toolkit::layout_values::Val::Px(-8.0)
        );
        assert_eq!(points.height, ui_toolkit::frame::Dimension::Fixed(20.0));
        assert_eq!(
            shield.position.left,
            ui_toolkit::layout_values::Val::Px(left + width + 3.0)
        );
        assert_eq!(
            shield.position.top,
            ui_toolkit::layout_values::Val::Px(-7.0)
        );
        assert_eq!(
            (shield.width, shield.height),
            (
                ui_toolkit::frame::Dimension::Fixed(20.0),
                ui_toolkit::frame::Dimension::Fixed(20.0)
            )
        );
        window.action("achievement:close");
        assert_eq!(window.action(&destination), vec![]);
        assert_eq!(window.action("achievement:category:14808"), vec![]);
    }
}

fn source_presentation_entry(achievement_id: u32, tree_id: u32) -> AchievementCatalogEntry {
    let root = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../data/db2/active");
    let read_row = |table: &str, id: u32| {
        let csv = std::fs::read_to_string(root.join(format!("{table}.csv"))).unwrap();
        let records = game_engine_core::csv_util::parse_csv_records(&csv);
        let header = &records[0];
        let id_column = header.iter().position(|name| name == "ID").unwrap();
        let row = records
            .iter()
            .skip(1)
            .find(|row| row[id_column] == id.to_string())
            .unwrap();
        header
            .iter()
            .cloned()
            .zip(row.iter().cloned())
            .collect::<std::collections::BTreeMap<_, _>>()
    };
    let tree = read_row("CriteriaTree", tree_id);
    let source = read_row("Achievement", achievement_id);
    let mut entry = achievement();
    entry.achievement_id = achievement_id;
    entry.name = source["Title_lang"].clone();
    entry.description = source["Description_lang"].clone();
    entry.flags = source["Flags"].parse().unwrap();
    entry.earned = false;
    entry.earned_at = None;
    entry.next_criteria_id = None;
    // This is a received UI payload fixture, not a persisted/seeded gameplay counter.
    // Decode the received criterion payload, including its authored presentation flags.
    entry.criteria = vec![
        serde_json::from_value(serde_json::json!({
            "tree_id": tree_id,
            "order_index": tree["OrderIndex"].parse::<i32>().unwrap(),
            "criteria_id": tree["CriteriaID"].parse::<u32>().unwrap(),
            "description": tree["Description_lang"],
            "flags": tree["Flags"].parse::<u32>().unwrap(),
            "current": 0, "required": tree["Amount"].parse::<u64>().unwrap(),
            "completed": false, "progress_supported": true
        }))
        .unwrap(),
    ];
    entry
}

fn source_presentation_registry(entry: AchievementCatalogEntry, skin: ActiveSkin) -> FrameRegistry {
    assets(skin);
    let mut window = AchievementWindow::default();
    window.action(&format!("achievement:open:{}", entry.achievement_id));
    window.apply(AchievementCatalogPage::Achievement {
        achievement_id: entry.achievement_id,
        entry: Some(entry),
    });
    achievement_registry(&window, skin)
}

#[test]
fn achievements_presentation_authored_single_quantity_flag_requires_bar_both_skins() {
    for skin in [ActiveSkin::Modern, ActiveSkin::Forever] {
        let entry = source_presentation_entry(1017, 22151);
        assert_eq!(entry.criteria[0].required, 1);
        let registry = source_presentation_registry(entry, skin);
        let bar = registry
            .get_by_name("Achievement1017Criterion22151Bar")
            .expect("authored ProgressBar1 must render a bar even when required quantity is1");
        let bar = registry.get(bar).unwrap();
        assert_eq!((bar.width.value(), bar.height.value()), (212.0, 14.0));
        assert_eq!(
            (bar.position.left, bar.position.top),
            (Val::Px(177.0), Val::Px(88.0))
        );
        let caption = registry
            .get(
                registry
                    .get_by_name("Achievement1017Criterion22151")
                    .unwrap(),
            )
            .unwrap();
        let Some(WidgetData::FontString(font)) = &caption.widget_data else {
            panic!("quantity caption");
        };
        // GameFontHighlightSmall -> GameFontNormalSmall -> SystemFont_Shadow_Small.
        assert_eq!(font.font_size, 10.0);
        assert_eq!(
            (font.justify_h, font.justify_v),
            (JustifyH::Center, JustifyV::Middle)
        );
        assert_eq!(font.shadow_color, Some([0.0, 0.0, 0.0, 1.0]));
        assert_eq!(font.shadow_offset, [1.0, -1.0]);
        assert_eq!(text(&registry, "Achievement1017Criterion22151"), "0/1");
    }
}

#[test]
fn achievements_presentation_unflagged_hundred_quantity_uses_checklist_both_skins() {
    for skin in [ActiveSkin::Modern, ActiveSkin::Forever] {
        let entry = source_presentation_entry(155, 4716);
        assert_eq!(entry.criteria[0].required, 100);
        let description = entry.criteria[0].description.clone();
        let registry = source_presentation_registry(entry, skin);
        assert!(
            registry
                .get_by_name("Achievement155Criterion4716Bar")
                .is_none(),
            "authored Flags0 must not become a bar merely because quantity is100"
        );
        assert_eq!(
            text(&registry, "Achievement155Criterion4716"),
            format!("- {description}")
        );
    }
}

#[test]
fn achievements_presentation_quantity_fill_and_unknown_progress_both_skins() {
    for skin in [ActiveSkin::Modern, ActiveSkin::Forever] {
        for quantity in [0, 3, 10] {
            let mut entry = source_presentation_entry(2141, 10645);
            assert_eq!(entry.criteria[0].required, 10);
            entry.criteria[0].current = quantity;
            entry.criteria[0].completed = quantity == 10;
            let registry = source_presentation_registry(entry, skin);
            assert_eq!(
                text(&registry, "Achievement2141Criterion10645"),
                format!("{quantity}/10")
            );
            let fill = registry
                .get(
                    registry
                        .get_by_name("Achievement2141Criterion10645Fill")
                        .unwrap(),
                )
                .unwrap();
            assert!((fill.width.value() - 212.0 * quantity as f32 / 10.0).abs() < 0.001);
            assert!(
                registry
                    .get_by_name("Achievement2141Criterion10645Check")
                    .is_none()
            );
        }
        let mut entry = source_presentation_entry(2141, 10645);
        entry.criteria[0].progress_supported = false;
        let description = entry.criteria[0].description.clone();
        let registry = source_presentation_registry(entry, skin);
        assert!(
            registry
                .get_by_name("Achievement2141Criterion10645Bar")
                .is_none()
        );
        assert_eq!(
            text(&registry, "Achievement2141Criterion10645"),
            format!("[Unevaluated] {description}")
        );
        assert!(
            registry
                .get_by_name("Achievement2141Criterion10645Check")
                .is_none()
        );
    }
}

#[test]
fn achievements_presentation_check_completion_and_cursor_flags_both_skins() {
    for skin in [ActiveSkin::Modern, ActiveSkin::Forever] {
        let mut entry = source_presentation_entry(155, 4716);
        entry.criteria[0].current = 100;
        entry.criteria[0].completed = true;
        let description = entry.criteria[0].description.clone();
        let registry = source_presentation_registry(entry, skin);
        assert_eq!(text(&registry, "Achievement155Criterion4716"), description);
        let caption = registry
            .get(registry.get_by_name("Achievement155Criterion4716").unwrap())
            .unwrap();
        let Some(WidgetData::FontString(font)) = &caption.widget_data else {
            panic!("checklist caption");
        };
        // AchievementCriteriaFont -> AchievementDescriptionFont -> SystemFont_Small.
        assert_eq!(font.font_size, 10.0);
        assert_eq!(
            (font.justify_h, font.justify_v),
            (JustifyH::Left, JustifyV::Middle)
        );
        assert_eq!(font.shadow_color, Some([0.0, 0.0, 0.0, 1.0]));
        assert_eq!(font.shadow_offset, [1.0, -1.0]);
        let check = registry
            .get(
                registry
                    .get_by_name("Achievement155Criterion4716Check")
                    .unwrap(),
            )
            .unwrap();
        assert_eq!((check.width.value(), check.height.value()), (20.0, 16.0));
        assert_eq!(
            (check.position.left, check.position.top),
            (Val::Px(18.0), Val::Px(2.5))
        );
        assert!(
            registry
                .get_by_name("Achievement155Criterion4716Bar")
                .is_none()
        );
        let mut entry = source_presentation_entry(155, 4716);
        entry.next_criteria_id = Some(4716);
        let mut window = AchievementWindow::default();
        window.action("achievement:open:155");
        window.apply(AchievementCatalogPage::Achievement {
            achievement_id: 155,
            entry: Some(entry),
        });
        assert_eq!(
            window.action("achievement:more_criteria"),
            vec![QueryAchievementCatalog::Criteria {
                achievement_id: 155,
                after_id: 4716
            }]
        );
        let continued = source_presentation_entry(155, 79890).criteria;
        window.apply(AchievementCatalogPage::Criteria {
            achievement_id: 155,
            criteria: continued,
            next_id: None,
        });
        let registry = achievement_registry(&window, skin);
        for tree in [4716, 79890] {
            assert!(
                registry
                    .get_by_name(&format!("Achievement155Criterion{tree}Bar"))
                    .is_none()
            );
            assert!(text(&registry, &format!("Achievement155Criterion{tree}")).starts_with("- "));
        }
        assert!(window.action("achievement:more_criteria").is_empty());
    }
}
