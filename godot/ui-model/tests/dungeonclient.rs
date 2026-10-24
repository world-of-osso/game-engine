//! Existing dungeonclient cases, partitioned without changing their names/assertions.
use game_engine_ui_model::achievements::AchievementWindow;
use game_engine_ui_model::dungeon_progress::DungeonObjectives;
use game_engine_ui_model::objective_tracker_component::{
    ObjectiveTrackerState, objective_tracker_screen,
};
use shared::protocol::*;
use ui_toolkit::atlas::{ActiveSkin, set_thread_skin};
use ui_toolkit::frame::WidgetData;
use ui_toolkit::registry::FrameRegistry;
use ui_toolkit::screen::{Screen, SharedContext};

// AchievementUI.xml246–342/2555 and SharedUIPanelTemplates.lua403–435/475:
// custom Header strips, centered GameFontNormalSmall, text width +20+30.

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
