use game_engine_ui_model::adventure_activities::{ActivityFilters, monthly};
use game_engine_ui_model::adventure_suggestions::suggested;
use shared::protocol::{
    AchievementCriterionLine, AdventureGuideSnapshot, AdventureSuggestion, JournalMonth,
    PerksActivityProgress, PerksActivitySnapshot, PerksActivityTag, SuggestionCensus,
};
use ui_toolkit::{
    atlas::{ActiveSkin, set_thread_skin},
    frame::{Frame, WidgetData},
    registry::FrameRegistry,
    screen::{Screen, SharedContext},
};

// Authored identities from the pinned PerksActivity/AdventureJournal diagnostics.
// Progress, tracking, tags and month are test inputs, not live realm evidence.
fn activity(id: u32) -> PerksActivitySnapshot {
    let (name, description, points, supersedes, priority) = match id {
        3 => (
            "Complete 30 Quests",
            "Complete quests that grant experience.",
            200,
            0,
            0,
        ),
        4 => (
            "Defeat 50 Dungeon Bosses",
            "Defeat dungeon bosses that grant experience.",
            200,
            0,
            0,
        ),
        6 => ("Complete 5 Mythic+ Dungeons", "", 200, 0, 7),
        7 => ("Complete 15 World Quests", "", 150, 0, 0),
        8 => ("Defeat 10 Dragonflight Raid Bosses", "", 200, 0, 0),
        9 => (
            "Kill Any Raid Boss in Dragonflight at Mythic Difficulty",
            "",
            200,
            0,
            0,
        ),
        81 => ("Complete 15 Mythic+ Dungeons", "", 200, 6, 5),
        _ => panic!("No authored fixture for {id}"),
    };
    PerksActivitySnapshot {
        id,
        name: name.into(),
        description: description.into(),
        points,
        supersedes,
        priority,
        tags: vec![],
        tracked: false,
        progress: PerksActivityProgress::Unavailable {
            reason: if id == 3 {
                "Criteria tree 135693 contains untracked criteria"
            } else {
                "Criteria are not tracked"
            }
            .into(),
        },
    }
}
fn snapshot() -> AdventureGuideSnapshot {
    AdventureGuideSnapshot {
        month: JournalMonth {
            year: 2026,
            month: 10,
            name: "October".into(),
            start_unix: 1790838000,
            end_unix: 1793516400,
            now_unix: 1791763200,
            tracking_since: 1791763200,
        },
        activities: vec![activity(3)],
        tags: vec![],
        suggestions: vec![],
        suggestion_census: SuggestionCensus::default(),
        points: 0,
        activity_scope_notice: Some(
            "Month assignments unverified; showing all source activities.".into(),
        ),
        source_error: None,
    }
}
fn context(skin: ActiveSkin) -> SharedContext {
    game_engine_ui_model::paths::set_data_root(
        std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../data"),
    )
    .unwrap();
    set_thread_skin(skin);
    let mut ctx = SharedContext::new();
    ctx.insert(skin);
    ctx
}
fn render(
    skin: ActiveSkin,
    data: Option<AdventureGuideSnapshot>,
    filters: ActivityFilters,
    search: &str,
) -> FrameRegistry {
    let ctx = context(skin);
    let search = search.to_owned();
    let mut registry = FrameRegistry::new(800., 496.);
    Screen::new(move |ctx| monthly(ctx, data.as_ref(), &filters, &search))
        .sync(&ctx, &mut registry);
    registry
}
fn frame<'a>(registry: &'a FrameRegistry, name: &str) -> &'a Frame {
    registry
        .get(
            registry
                .get_by_name(name)
                .unwrap_or_else(|| panic!("Missing {name}")),
        )
        .unwrap()
}
fn text(registry: &FrameRegistry, name: &str) -> String {
    match &frame(registry, name).widget_data {
        Some(WidgetData::FontString(font)) => font.text.clone(),
        other => panic!("Expected text for {name}, got {other:?}"),
    }
}
fn action(registry: &FrameRegistry, name: &str) -> String {
    frame(registry, name).onclick.clone().expect("action")
}

#[test]
fn monthly_unavailable_is_not_zero_progress_in_both_skins() {
    for skin in [ActiveSkin::Modern, ActiveSkin::Forever] {
        let registry = render(
            skin,
            Some(snapshot()),
            ActivityFilters {
                selected: Some(3),
                ..Default::default()
            },
            "",
        );
        assert_eq!(
            text(&registry, "JournalMonthlyTitle"),
            "Traveler's Log — October 2026"
        );
        assert_eq!(
            text(&registry, "JournalMonthlyPoints"),
            "Completed activity contributions: 0 points"
        );
        let progress = text(&registry, "JournalActivity3Progress");
        assert!(progress.contains("Unavailable"));
        assert!(progress.contains("135693"));
        assert!(!progress.contains("0 /"));
        assert!(
            text(&registry, "JournalActivityDetails")
                .contains("Complete quests that grant experience.")
        );
        assert_eq!(text(&registry, "JournalActivity3Points"), "200 points");
        assert_eq!(action(&registry, "JournalActivity3"), "journal:activity:3");
        assert_eq!(
            action(&registry, "JournalActivity3Track"),
            "journal:activity-track:3:1"
        );
        assert!(text(&registry, "JournalMonthlyNotice").contains("all source activities"));
    }
}

#[test]
fn available_progress_and_unevaluated_criteria_remain_distinct() {
    for skin in [ActiveSkin::Modern, ActiveSkin::Forever] {
        let mut data = snapshot();
        // Deliberately supplied Available wire state: not a claim tree135693 is supported.
        data.activities[0].progress = PerksActivityProgress::Available {
            current: 7,
            required: 30,
            completed: false,
            criteria: vec![AchievementCriterionLine {
                tree_id: 135693,
                order_index: 0,
                criteria_id: 0,
                description: "Complete quests that grant experience.".into(),
                current: 0,
                required: 30,
                completed: false,
                progress_supported: false,
            }],
        };
        let filters = ActivityFilters {
            selected: Some(3),
            ..Default::default()
        };
        let registry = render(skin, Some(data.clone()), filters.clone(), "");
        assert_eq!(
            text(&registry, "JournalActivity3Progress"),
            "7 / 30 — In progress"
        );
        let details = text(&registry, "JournalActivityDetails");
        assert!(details.contains("Unavailable"));
        assert!(!details.contains("0 / 30"));
        data.activities[0].progress = PerksActivityProgress::Available {
            current: 30,
            required: 30,
            completed: true,
            criteria: vec![AchievementCriterionLine {
                tree_id: 135693,
                order_index: 0,
                criteria_id: 0,
                description: "Complete quests that grant experience.".into(),
                current: 30,
                required: 30,
                completed: true,
                progress_supported: true,
            }],
        };
        data.points = 200;
        let registry = render(skin, Some(data), filters, "");
        assert_eq!(
            text(&registry, "JournalActivity3Progress"),
            "30 / 30 — Complete"
        );
        assert!(text(&registry, "JournalActivityDetails").contains("30 / 30 — Complete"));
        assert!(text(&registry, "JournalMonthlyPoints").contains("200 points"));
    }
}

#[test]
fn filtering_selection_and_tracking_use_source_rows_and_parent_actions() {
    for skin in [ActiveSkin::Modern, ActiveSkin::Forever] {
        let mut data = snapshot();
        data.activities[0].tags = vec![1];
        data.activities[0].tracked = true;
        data.activities.push(activity(4));
        data.tags = vec![PerksActivityTag {
            id: 1,
            name: "Quests".into(),
        }];
        let filters = ActivityFilters {
            tracked_only: true,
            tag: 1,
            selected: Some(3),
        };
        let registry = render(skin, Some(data.clone()), filters.clone(), "EXPERIENCE");
        assert!(registry.get_by_name("JournalActivity3").is_some());
        assert!(registry.get_by_name("JournalActivity4").is_none());
        assert_eq!(
            action(&registry, "JournalActivity3Track"),
            "journal:activity-track:3:0"
        );
        assert_eq!(
            action(&registry, "JournalActivityFilterAll"),
            "journal:activity-filter:all"
        );
        assert_eq!(
            action(&registry, "JournalActivityFilterTracked"),
            "journal:activity-filter:tracked"
        );
        assert_eq!(
            action(&registry, "JournalActivityTag0"),
            "journal:activity-tag:0"
        );
        assert_eq!(
            action(&registry, "JournalActivityTag1"),
            "journal:activity-tag:1"
        );
        let registry = render(
            skin,
            Some(data.clone()),
            ActivityFilters {
                tag: 1,
                ..Default::default()
            },
            "",
        );
        assert!(registry.get_by_name("JournalActivity4").is_none());
        let registry = render(
            skin,
            Some(data.clone()),
            ActivityFilters {
                selected: Some(3),
                ..Default::default()
            },
            "dungeon",
        );
        assert!(registry.get_by_name("JournalActivity3").is_none());
        assert!(registry.get_by_name("JournalActivityDetails").is_none());
        assert!(registry.get_by_name("JournalActivity4").is_some());
        let registry = render(skin, Some(data), filters, "no matching authored name");
        assert_eq!(
            text(&registry, "JournalMonthlyEmpty"),
            "No activities match these filters."
        );
    }
}

#[test]
fn monthly_loading_source_error_and_empty_catalog_are_explicit() {
    for skin in [ActiveSkin::Modern, ActiveSkin::Forever] {
        let registry = render(skin, None, ActivityFilters::default(), "");
        assert_eq!(
            text(&registry, "JournalMonthlyLoading"),
            "Loading Traveler's Log…"
        );
        assert!(registry.get_by_name("JournalMonthlyTitle").is_none());
        let mut data = snapshot();
        data.source_error = Some("PerksActivity catalog missing".into());
        let registry = render(skin, Some(data.clone()), ActivityFilters::default(), "");
        assert!(
            text(&registry, "JournalMonthlyNotice")
                .contains("Missing source data: PerksActivity catalog missing")
        );
        data.source_error = None;
        data.activities.clear();
        let registry = render(skin, Some(data), ActivityFilters::default(), "");
        assert_eq!(
            text(&registry, "JournalMonthlyEmpty"),
            "No source activities available."
        );
    }
}

#[test]
fn superseded_activities_are_grouped_without_hiding_source_progress() {
    for skin in [ActiveSkin::Modern, ActiveSkin::Forever] {
        let mut data = snapshot();
        data.activities = vec![activity(81), activity(4), activity(6)];
        let registry = render(
            skin,
            Some(data),
            ActivityFilters {
                selected: Some(81),
                ..Default::default()
            },
            "",
        );
        assert!(registry.get_by_name("JournalActivity6").is_some());
        assert!(registry.get_by_name("JournalActivity81").is_some());
        assert!(
            text(&registry, "JournalActivityDetails")
                .contains("Follows: Complete 5 Mythic+ Dungeons")
        );
    }
}

#[test]
fn monthly_virtualization_reaches_later_real_rows_and_clamps_after_filtering() {
    for skin in [ActiveSkin::Modern, ActiveSkin::Forever] {
        let ctx = context(skin);
        let mut data = snapshot();
        data.activities = [3, 4, 6, 7, 8, 9, 81].into_iter().map(activity).collect();
        ctx_fixture_virtualization(ctx, data);
    }
}
fn ctx_fixture_virtualization(mut ctx: SharedContext, data: AdventureGuideSnapshot) {
    // Seven distinct authored rows exceed one viewport even without expanded details.
    ctx.insert(data);
    ctx.insert(ActivityFilters {
        selected: Some(3),
        ..Default::default()
    });
    let mut registry = FrameRegistry::new(800., 496.);
    let mut screen = Screen::new(|ctx| monthly(ctx, ctx.get(), ctx.get().unwrap(), ""));
    screen.sync(&ctx, &mut registry);
    let list = "EncounterJournalContent";
    assert!(
        registry
            .scroll_lists
            .get(list)
            .unwrap()
            .geometry
            .max_first_row()
            > 0
    );
    assert!(registry.scroll_lists.scroll_to(list, usize::MAX));
    screen.sync(&ctx, &mut registry);
    assert!(registry.get_by_name("JournalActivity7").is_some());
    assert!(registry.get_by_name("JournalActivity3").is_none());
    let mut data = snapshot();
    data.activities[0].tracked = true;
    ctx.insert(data);
    ctx.insert(ActivityFilters {
        tracked_only: true,
        ..Default::default()
    });
    screen.sync(&ctx, &mut registry);
    assert_eq!(registry.scroll_lists.get(list).unwrap().first_row, 0);
    assert!(registry.get_by_name("JournalActivity3").is_some());
}

#[test]
fn suggestions_display_only_authored_eligible_records_without_actions() {
    for skin in [ActiveSkin::Modern, ActiveSkin::Forever] {
        let mut ctx = context(skin);
        let mut data = snapshot();
        data.suggestion_census.source_rows = 698;
        data.suggestions = vec![AdventureSuggestion {
            id: 642, name: "Trading Post, Open for Business".into(),
            description: "Visit Stormwind or Orgrimmar to see a multitude of mystical, magical items that are available to purchase this month. Be sure to review your Traveler's Log to see this month's activities.".into(),
            reward_description: String::new(), texture_fdid: 4696085, button_text: String::new(),
        }];
        ctx.insert(data);
        let mut registry = FrameRegistry::new(800., 496.);
        let mut screen = Screen::new(|ctx| suggested(ctx, ctx.get()));
        screen.sync(&ctx, &mut registry);
        assert_eq!(
            text(&registry, "JournalSuggestion642Name"),
            "Trading Post, Open for Business"
        );
        assert!(
            text(&registry, "JournalSuggestion642Description")
                .contains("Visit Stormwind or Orgrimmar")
        );
        assert!(registry.get_by_name("JournalSuggestion642Reward").is_none());
        assert!(registry.get_by_name("JournalSuggestion642Button").is_none());
        assert!(frame(&registry, "JournalSuggestion642").onclick.is_none());
        for (name, [left, top, width, height]) in [
            ("EncounterJournalSuggestedFrame", [9., 111., 784., 380.]),
            ("JournalSuggestionSlot1", [393., 194., 344., 161.]),
            ("JournalSuggestionSlot2", [393., 22., 344., 161.]),
            ("JournalSuggestionSlot3", [28., 18., 335., 337.]),
        ] {
            let pane = frame(&registry, name);
            assert_eq!(pane.position.left, ui_toolkit::layout_values::Val::Px(left));
            assert_eq!(pane.position.top, ui_toolkit::layout_values::Val::Px(top));
            assert_eq!(pane.width, ui_toolkit::frame::Dimension::Fixed(width));
            assert_eq!(pane.height, ui_toolkit::frame::Dimension::Fixed(height));
        }
        assert!(registry.get_by_name("JournalSuggestedTitle").is_none());
        for (name, left) in [
            ("JournalSuggestedPrev", 16.),
            ("JournalSuggestedNext", 287.),
        ] {
            let button = frame(&registry, name);
            assert_eq!(
                button.position.left,
                ui_toolkit::layout_values::Val::Px(left)
            );
            assert_eq!(
                button.position.top,
                ui_toolkit::layout_values::Val::Px(293.)
            );
            assert_eq!(button.width, ui_toolkit::frame::Dimension::Fixed(32.));
            assert_eq!(button.height, ui_toolkit::frame::Dimension::Fixed(32.));
            assert!(
                matches!(&button.widget_data, Some(WidgetData::Button(data)) if data.state == ui_toolkit::widgets::button::ButtonState::Disabled)
            );
            assert!(button.onclick.as_deref().unwrap_or_default().is_empty());
        }
        assert_eq!(
            frame(&registry, "JournalSuggestion642").width,
            ui_toolkit::frame::Dimension::Fixed(344.)
        );
        assert_eq!(
            frame(&registry, "JournalSuggestion642").height,
            ui_toolkit::frame::Dimension::Fixed(161.)
        );
        match &frame(&registry, "JournalSuggestion642Texture").widget_data {
            Some(WidgetData::Texture(texture)) => assert_eq!(
                texture.source,
                ui_toolkit::widgets::texture::TextureSource::FileDataId(4696085)
            ),
            other => panic!("Expected source texture, got {other:?}"),
        }
        let mut rewarded = snapshot();
        rewarded.suggestions = vec![AdventureSuggestion {
            id: 195, name: "Bonus Event: World Quests".into(),
            description: "Complete World Quests in Midnight for increased rewards for the duration of this Bonus Event!".into(),
            reward_description: "Completing this quest will grant you renown with the Midnight faction of your choice!".into(),
            texture_fdid: 136349, button_text: "Start Quest".into(),
        }];
        rewarded.suggestions.insert(
            0,
            ctx.get::<AdventureGuideSnapshot>().unwrap().suggestions[0].clone(),
        );
        rewarded.suggestions.push(AdventureSuggestion {
            id: 227, name: "Brewfest".into(),
            description: "The yearly celebration of the fermented fruits of the harvest is happening right now!  Find the beer garden outside Ironforge.".into(),
            reward_description: "You can win a variety of fun prizes like pets and seasonal garb!".into(),
            texture_fdid: 133201, button_text: String::new(),
        });
        ctx.insert(rewarded);
        screen.sync(&ctx, &mut registry);
        for id in [642, 195] {
            assert_eq!(
                frame(&registry, &format!("JournalSuggestion{id}")).width,
                ui_toolkit::frame::Dimension::Fixed(344.)
            );
            assert_eq!(
                frame(&registry, &format!("JournalSuggestion{id}")).height,
                ui_toolkit::frame::Dimension::Fixed(161.)
            );
        }
        assert_eq!(
            frame(&registry, "JournalSuggestion227").width,
            ui_toolkit::frame::Dimension::Fixed(335.)
        );
        assert_eq!(
            frame(&registry, "JournalSuggestion227").height,
            ui_toolkit::frame::Dimension::Fixed(337.)
        );
        let title = &frame(&registry, "JournalSuggestion227Name").widget_data;
        assert!(
            matches!(title, Some(WidgetData::FontString(font)) if font.font == ui_toolkit::widgets::font_string::GameFont::Morpheus && font.font_size == 32.)
        );
        assert!(
            registry.get_by_name("JournalSuggestion195Reward").is_none(),
            "Reward description belongs to reward hover, not pane body"
        );
        assert!(
            registry
                .scroll_lists
                .get("EncounterJournalContent")
                .is_none(),
            "Retail Suggested panes are fixed, not groups on a generic ScrollBox"
        );
        assert!(
            registry.get_by_name("JournalSuggestion195Button").is_none(),
            "Authored button text alone does not grant activation"
        );
        let mut missing = snapshot();
        missing.source_error = Some("AdventureJournal catalog missing".into());
        ctx.insert(missing);
        screen.sync(&ctx, &mut registry);
        assert_eq!(
            text(&registry, "JournalSuggestedNotice"),
            "Missing source data: AdventureJournal catalog missing"
        );
        ctx.insert(snapshot());
        screen.sync(&ctx, &mut registry);
        assert!(registry.get_by_name("JournalSuggestion642").is_none());
        assert!(
            registry.get_by_name("JournalSuggestedEmpty").is_none(),
            "Retail leaves its three empty source panes, without an invented notice"
        );
        for index in 1..=3 {
            assert!(
                registry
                    .get_by_name(&format!("JournalSuggestionSlot{index}"))
                    .is_some()
            );
        }
        let mut registry = FrameRegistry::new(800., 496.);
        Screen::new(|ctx| suggested(ctx, None)).sync(&ctx, &mut registry);
        assert_eq!(
            text(&registry, "JournalSuggestedLoading"),
            "Loading suggested content…"
        );
    }
}
