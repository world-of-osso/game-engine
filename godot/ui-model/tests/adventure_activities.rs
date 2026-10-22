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
) -> FrameRegistry {
    let ctx = context(skin);
    let mut registry = FrameRegistry::new(800., 496.);
    Screen::new(move |ctx| monthly(ctx, data.as_ref(), &filters)).sync(&ctx, &mut registry);
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
fn monthly_uses_retail_inset_header_options_filters_and_list_rects() {
    for skin in [ActiveSkin::Modern, ActiveSkin::Forever] {
        let mut data = snapshot();
        data.activities[0].tags = vec![1, 2];
        data.tags = vec![
            PerksActivityTag {
                id: 2,
                name: "Other".into(),
            },
            PerksActivityTag {
                id: 1,
                name: "Quests".into(),
            },
        ];
        let registry = render(skin, Some(data), ActivityFilters::default());
        for (name, [left, top, width, height]) in [
            (
                "EncounterJournalMonthlyActivitiesFrame",
                [4., 60., 792., 431.],
            ),
            ("JournalMonthlyThresholdContainer", [46., 74., 663., 33.]),
            ("EncounterJournalFilterList", [5., 137., 220., 290.]),
            ("EncounterJournalContent", [241., 138., 521., 291.]),
            ("JournalActivityTag0", [10., 10., 175., 20.]),
            ("JournalActivityTag1", [10., 32., 175., 20.]),
            ("JournalActivityTag2", [10., 54., 175., 20.]),
        ] {
            let actual = frame(&registry, name);
            assert_eq!(
                actual.position.left,
                ui_toolkit::layout_values::Val::Px(left),
                "{skin:?} {name} left"
            );
            assert_eq!(
                actual.position.top,
                ui_toolkit::layout_values::Val::Px(top),
                "{skin:?} {name} top"
            );
            assert_eq!(
                actual.width,
                ui_toolkit::frame::Dimension::Fixed(width),
                "{skin:?} {name} width"
            );
            assert_eq!(
                actual.height,
                ui_toolkit::frame::Dimension::Fixed(height),
                "{skin:?} {name} height"
            );
        }
        assert_eq!(text(&registry, "JournalMonthlyTitle"), "Traveler's Log");
        assert_eq!(text(&registry, "JournalMonthlyMonth"), "October");
        assert_eq!(text(&registry, "JournalActivityTag0Name"), "All");
        assert_eq!(text(&registry, "JournalActivityTag1Name"), "Quests");
        assert!(
            registry
                .get_by_name("JournalActivityTag0RowSelected")
                .is_some()
        );
        assert!(registry.get_by_name("JournalActivityFilterAll").is_none());
        assert!(
            registry
                .get_by_name("JournalActivityFilterTracked")
                .is_none()
        );
    }
}

#[test]
fn monthly_thresholds_use_received_points_not_claimed_currency_state() {
    use shared::protocol::{TradingPostSnapshot, TradingPostThreshold};
    use std::sync::Arc;
    for skin in [ActiveSkin::Modern, ActiveSkin::Forever] {
        let mut ctx = context(skin);
        let mut data = snapshot();
        data.points = 400;
        let tender = TradingPostSnapshot {
            npc_net_id: None,
            tender: 0,
            points: 400,
            now_unix: data.month.now_unix,
            month_end_unix: data.month.end_unix,
            frozen_item_id: None,
            categories: vec![],
            items: vec![],
            thresholds: (1..=5)
                .map(|id| TradingPostThreshold {
                    id,
                    points: id * 200,
                    tender: 100,
                    claimed: false,
                })
                .collect(),
            source_gaps: vec![],
            error: None,
            catalog_complete: false,
            reply_kind: shared::protocol::TradingPostReplyKind::Balance,
        };
        ctx.insert(game_engine_ui_model::adventure_guide::View {
            trading_post: Some(Arc::new(tender)),
            ..Default::default()
        });
        let mut registry = FrameRegistry::new(800., 496.);
        Screen::new(move |ctx| monthly(ctx, Some(&data), &ActivityFilters::default()))
            .sync(&ctx, &mut registry);
        assert_eq!(text(&registry, "JournalMonthlyPoints"), "Travel Points");
        assert_eq!(text(&registry, "JournalMonthlyProgress"), "400 / 1000");
        assert_eq!(
            frame(&registry, "JournalMonthlyBarFill").width,
            ui_toolkit::frame::Dimension::Fixed(265.2)
        );
        assert_eq!(
            frame(&registry, "JournalMonthlyThreshold1Diamond").width,
            ui_toolkit::frame::Dimension::Fixed(34.)
        );
        assert_eq!(
            frame(&registry, "JournalMonthlyThreshold3Diamond").width,
            ui_toolkit::frame::Dimension::Fixed(40.)
        );
        assert!(
            registry
                .get_by_name("JournalMonthlyThreshold1Points")
                .is_none()
        );
        assert_eq!(text(&registry, "JournalMonthlyThreshold3Points"), "100");
    }
}

#[test]
fn monthly_source_rows_collapse_children_and_leaf_clicks_do_not_expand_details() {
    use game_engine_ui_model::adventure_guide::View;
    use std::sync::Arc;
    for skin in [ActiveSkin::Modern, ActiveSkin::Forever] {
        let mut data = snapshot();
        data.activities = vec![activity(6), activity(81), activity(4)];
        let mut view = View {
            activities: Some(Arc::new(data.clone())),
            ..Default::default()
        };
        let registry = render(skin, Some(data.clone()), view.activity_filters.clone());
        assert!(
            registry.get_by_name("JournalActivity81").is_none(),
            "Source trees start collapsed"
        );
        assert_eq!(
            frame(&registry, "JournalActivity6").height,
            ui_toolkit::frame::Dimension::Fixed(78.)
        );
        assert_eq!(
            frame(&registry, "JournalActivity6").width,
            ui_toolkit::frame::Dimension::Fixed(521.)
        );
        assert!(registry.get_by_name("JournalActivity6Track").is_none());
        view.apply("journal:activity:6").unwrap();
        let registry = render(skin, Some(data.clone()), view.activity_filters.clone());
        assert_eq!(
            frame(&registry, "JournalActivity81").height,
            ui_toolkit::frame::Dimension::Fixed(48.)
        );
        assert_eq!(
            frame(&registry, "JournalActivity81").width,
            ui_toolkit::frame::Dimension::Fixed(521.)
        );
        assert_eq!(
            frame(&registry, "JournalActivity81").position.left,
            ui_toolkit::layout_values::Val::Px(32.)
        );
        let previous = view.activity_filters.clone();
        view.apply("journal:activity:4").unwrap();
        assert_eq!(
            view.activity_filters, previous,
            "Source leaf click has no details-pane action"
        );
        view.apply("journal:activity-tag:1").unwrap();
        view.apply("journal:activity-tag:0").unwrap();
        let registry = render(skin, Some(data), view.activity_filters);
        assert!(
            registry.get_by_name("JournalActivity81").is_none(),
            "Changing source filter collapses the tree"
        );
    }
}

fn tooltip_lines(data: &AdventureGuideSnapshot, id: u32) -> String {
    game_engine_ui_model::adventure_activities::activity_tooltip(
        data,
        &format!("JournalActivity{id}"),
    )
    .unwrap()
    .content
    .lines
    .iter()
    .map(|line| line.left_text.as_str())
    .collect::<Vec<_>>()
    .join("\n")
}

#[test]
fn ui_template_behavior_remaining_journal_monthly_owner() {
    use game_engine_ui_model::adventure_activities::{activity_tooltip, tracking_request};
    use shared::protocol::AdventureGuideRequest;
    for skin in [ActiveSkin::Modern, ActiveSkin::Forever] {
        let data = snapshot();
        let registry = render(skin, Some(data.clone()), ActivityFilters::default());
        assert!(
            registry.get_by_name("JournalMonthlyProgress").is_none(),
            "Do not invent a missing threshold maximum"
        );
        for old in [
            "JournalActivity3Progress",
            "JournalActivity3Track",
            "JournalActivityDetails",
        ] {
            assert!(registry.get_by_name(old).is_none());
        }
        let hit = registry.get_by_name("JournalActivity3").unwrap();
        let (owner, binding) =
            ui_toolkit::tooltip::tooltip_owner(&registry, hit).expect("live monthly owner");
        assert_eq!(owner.id, hit);
        assert_eq!(binding.provider.as_deref(), Some("journal_monthly"));
        assert_eq!(
            binding.anchor,
            ui_toolkit::tooltip::TooltipAnchor::CursorRight
        );
        let tooltip = activity_tooltip(&data, "JournalActivity3").unwrap();
        assert_eq!(tooltip.content.title, "Complete 30 Quests");
        assert!(matches!(
            tooltip.anchor,
            game_engine_ui_model::game_tooltip::TooltipAnchor::CursorRight
        ));
        let lines = tooltip_lines(&data, 3);
        assert!(lines.contains("Complete quests that grant experience."));
        assert!(!lines.contains("Unavailable") && !lines.contains("135693"));
        assert!(lines.contains("Shift click") && !lines.contains("0/"));
        let action = action(&registry, "JournalActivity3");
        assert_eq!(
            tracking_request(&data, &action, false, true),
            Some(AdventureGuideRequest::TrackActivity {
                activity_id: 3,
                tracked: true
            })
        );
        assert!(tracking_request(&data, &action, false, false).is_none());
        assert!(tracking_request(&data, &action, true, true).is_none());
        assert!(
            !data.activities[0].tracked,
            "Request construction must not change authoritative state"
        );
    }
}

#[test]
fn available_progress_and_unevaluated_criteria_remain_distinct() {
    use game_engine_ui_model::adventure_activities::tracking_request;
    for skin in [ActiveSkin::Modern, ActiveSkin::Forever] {
        let mut data = snapshot();
        data.activities[0].progress = PerksActivityProgress::Available {
            current: 7,
            required: 30,
            completed: false,
            criteria: vec![AchievementCriterionLine {
                tree_id: 135693,
                order_index: 0,
                criteria_id: 0,
                description: "Complete quests that grant experience.".into(),
                flags: 0,
                current: 0,
                required: 30,
                completed: false,
                progress_supported: false,
            }],
        };
        render(skin, Some(data.clone()), ActivityFilters::default());
        let lines = tooltip_lines(&data, 3);
        assert!(lines.contains("7/30") && !lines.contains("Unavailable"));
        assert!(!lines.contains("0/30"));
        data.activities[0].progress = PerksActivityProgress::Available {
            current: 30,
            required: 30,
            completed: true,
            criteria: vec![AchievementCriterionLine {
                tree_id: 135693,
                order_index: 0,
                criteria_id: 0,
                description: "Complete quests that grant experience.".into(),
                flags: 0,
                current: 30,
                required: 30,
                completed: true,
                progress_supported: true,
            }],
        };
        let registry = render(skin, Some(data.clone()), ActivityFilters::default());
        assert!(registry.get_by_name("JournalActivity3Check").is_some());
        assert!(tooltip_lines(&data, 3).contains("30/30"));
        assert!(!tooltip_lines(&data, 3).contains("Shift click"));
        assert!(tracking_request(&data, "journal:activity:3", false, true).is_none());
        data.activities[0].tracked = true;
        assert_eq!(
            tracking_request(&data, "journal:activity:3", false, true),
            Some(shared::protocol::AdventureGuideRequest::TrackActivity {
                activity_id: 3,
                tracked: false
            })
        );
    }
}

#[test]
fn source_tag_filters_preserve_tracking_marks_without_an_invented_toolbar() {
    use game_engine_ui_model::adventure_activities::tracking_request;
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
            tag: 1,
            ..Default::default()
        };
        let registry = render(skin, Some(data.clone()), filters.clone());
        assert!(registry.get_by_name("JournalActivity3Tracked").is_some());
        assert!(registry.get_by_name("JournalActivity4").is_none());
        assert_eq!(
            action(&registry, "JournalActivityTag0"),
            "journal:activity-tag:0"
        );
        assert_eq!(
            action(&registry, "JournalActivityTag1"),
            "journal:activity-tag:1"
        );
        assert_eq!(
            tracking_request(&data, "journal:activity:3", false, true),
            Some(shared::protocol::AdventureGuideRequest::TrackActivity {
                activity_id: 3,
                tracked: false
            })
        );
        assert!(tooltip_lines(&data, 3).contains("stop tracking"));
        let registry = render(skin, Some(data), ActivityFilters::default());
        assert!(registry.get_by_name("JournalActivity3").is_some());
        assert!(registry.get_by_name("JournalActivity4").is_some());
    }
}

#[test]
fn monthly_loading_source_error_and_empty_catalog_are_distinct() {
    for skin in [ActiveSkin::Modern, ActiveSkin::Forever] {
        let registry = render(skin, None, ActivityFilters::default());
        assert_eq!(
            text(&registry, "JournalMonthlyLoading"),
            "Loading Traveler's Log…"
        );
        assert!(registry.get_by_name("JournalMonthlyTitle").is_none());
        let mut data = snapshot();
        data.source_error = Some("PerksActivity catalog missing".into());
        let registry = render(skin, Some(data.clone()), ActivityFilters::default());
        assert!(
            text(&registry, "JournalMonthlyNotice")
                .contains("Missing source data: PerksActivity catalog missing")
        );
        data.source_error = None;
        data.activities.clear();
        let registry = render(skin, Some(data), ActivityFilters::default());
        assert!(registry.get_by_name("JournalMonthlyEmpty").is_none());
        assert!(registry.get_by_name("JournalActivity3").is_none());
        assert!(
            registry
                .get_by_name("EncounterJournalMonthlyActivitiesFrame")
                .is_some()
        );
    }
}

#[test]
fn completed_parent_promotes_unfinished_superseding_activity_to_source_parent_row() {
    for skin in [ActiveSkin::Modern, ActiveSkin::Forever] {
        let mut data = snapshot();
        data.activities = vec![activity(81), activity(4), activity(6)];
        data.activities[2].progress = PerksActivityProgress::Available {
            current: 5,
            required: 5,
            completed: true,
            criteria: vec![],
        };
        let registry = render(skin, Some(data.clone()), ActivityFilters::default());
        let promoted = frame(&registry, "JournalActivity81");
        assert_eq!(promoted.height, ui_toolkit::frame::Dimension::Fixed(78.));
        assert_eq!(
            promoted.position.left,
            ui_toolkit::layout_values::Val::Px(0.)
        );
        assert!(registry.get_by_name("JournalActivity6Check").is_some());
        assert!(!tooltip_lines(&data, 81).contains("Follows:"));
    }
}

#[test]
fn monthly_virtualization_reaches_later_real_rows_and_clamps_after_filtering() {
    for skin in [ActiveSkin::Modern, ActiveSkin::Forever] {
        let mut ctx = context(skin);
        let mut data = snapshot();
        data.activities = [3, 4, 6, 7, 8, 9, 81].into_iter().map(activity).collect();
        ctx.insert(data);
        ctx.insert(ActivityFilters::default());
        let mut registry = FrameRegistry::new(800., 496.);
        let mut screen = Screen::new(|ctx| monthly(ctx, ctx.get(), ctx.get().unwrap()));
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
        screen.sync(&ctx, &mut registry);
        assert_eq!(registry.scroll_lists.get(list).unwrap().first_row, 0);
        assert!(registry.get_by_name("JournalActivity3Tracked").is_some());
    }
}

fn assert_suggestion_truncated_text_hover(skin: ActiveSkin) {
    let _ctx = context(skin);
    let mut data = snapshot();
    data.suggestions = [642, 195, 227]
        .into_iter()
        .map(|id| AdventureSuggestion {
            id,
            name: "Trading Post, Open for Business".into(),
            description: "Visit Stormwind or Orgrimmar to see a multitude of mystical, magical items that are available to purchase this month. Be sure to review your Traveler's Log to see this month's activities.".into(),
            reward_description: String::new(),
            texture_fdid: 4696085,
            button_text: String::new(),
        })
        .collect();
    let ctx = context(skin);
    let mut registry = FrameRegistry::new(800., 496.);
    Screen::new(|ctx| suggested(ctx, Some(&data))).sync(&ctx, &mut registry);
    for id in [642, 195] {
        for (suffix, full_text) in [
            ("TitleFrame", data.suggestions[0].name.as_str()),
            ("DescriptionFrame", data.suggestions[0].description.as_str()),
        ] {
            let owner = format!("JournalSuggestion{id}{suffix}");
            let hit = registry.get_by_name(&owner).expect(&owner);
            let (resolved, binding) = ui_toolkit::tooltip::tooltip_owner(&registry, hit)
                .expect("live suggestion text owner");
            assert_eq!(resolved.id, hit);
            assert_eq!(binding.provider.as_deref(), Some("journal_suggestion_text"));
            assert_eq!(binding.anchor, ui_toolkit::tooltip::TooltipAnchor::Right);
            let tooltip =
                game_engine_ui_model::adventure_suggestions::text_tooltip(&data, &owner, true)
                    .expect("Retail truncated secondary text must show its full tooltip");
            assert_eq!(tooltip.content.title_color, [1.; 4]);
            assert!(
                tooltip
                    .content
                    .lines
                    .iter()
                    .all(|line| line.left_color == [1.; 4])
            );
            let actual = std::iter::once(tooltip.content.title.as_str())
                .chain(
                    tooltip
                        .content
                        .lines
                        .iter()
                        .map(|line| line.left_text.as_str()),
                )
                .collect::<Vec<_>>()
                .join(" ");
            assert_eq!(actual, full_text);
            assert_eq!(tooltip.record, None);
            assert!(
                game_engine_ui_model::adventure_suggestions::text_tooltip(&data, &owner, false,)
                    .is_none(),
                "untruncated text must not install hover"
            );
        }
    }
    // The scoped Lua3311 loop addresses physical Suggestion2/3, never primary1.
    for suffix in ["TitleFrame", "DescriptionFrame"] {
        assert!(
            game_engine_ui_model::adventure_suggestions::text_tooltip(
                &data,
                &format!("JournalSuggestion227{suffix}"),
                true,
            )
            .is_none()
        );
    }
    data.suggestions.pop();
    assert!(
        game_engine_ui_model::adventure_suggestions::text_tooltip(
            &data,
            "JournalSuggestion642TitleFrame",
            true,
        )
        .is_none(),
        "two records never visit physical Suggestion3 in the second pass"
    );
    assert!(
        game_engine_ui_model::adventure_suggestions::text_tooltip(
            &data,
            "JournalSuggestion195TitleFrame",
            true,
        )
        .is_some()
    );
    data.suggestions.pop();
    assert!(
        game_engine_ui_model::adventure_suggestions::text_tooltip(
            &data,
            "JournalSuggestion642DescriptionFrame",
            true,
        )
        .is_none(),
        "one record never enters the second pass"
    );
}

#[test]
fn ui_template_behavior_remaining_journal_suggestion_hover_modern() {
    assert_suggestion_truncated_text_hover(ActiveSkin::Modern);
}

#[test]
fn ui_template_behavior_remaining_journal_suggestion_hover_forever() {
    assert_suggestion_truncated_text_hover(ActiveSkin::Forever);
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

#[test]
fn suggestions_partial_records_preserve_literal_initial_lower_geometry() {
    use ui_toolkit::{frame::Dimension, layout_values::Val};
    for skin in [ActiveSkin::Modern, ActiveSkin::Forever] {
        let mut ctx = context(skin);
        let mut data = snapshot();
        // Authored records are rendering inputs, not fabricated live realm eligibility.
        data.suggestions = vec![AdventureSuggestion {
            id: 227,
            name: "Brewfest".into(),
            description: String::new(),
            reward_description: String::new(),
            texture_fdid: 133201,
            button_text: String::new(),
        }];
        ctx.insert(data.clone());
        let mut registry = FrameRegistry::new(800., 496.);
        let mut screen = Screen::new(|ctx| suggested(ctx, ctx.get()));
        screen.sync(&ctx, &mut registry);
        let assert_initial = |registry: &FrameRegistry| {
            let block = frame(registry, "JournalSuggestion227Text");
            assert_eq!(block.position.top, Val::Px(3.5));
            assert_eq!(block.height, Dimension::Fixed(152.));
            assert_eq!(
                frame(registry, "JournalSuggestion227TitleFrame").height,
                Dimension::Fixed(1.)
            );
            let description = frame(registry, "JournalSuggestion227DescriptionFrame");
            assert_eq!(description.position.top, Val::Px(5.));
            assert_eq!(description.height, Dimension::Fixed(1.));
        };
        let size = |registry: &FrameRegistry, name: &str| match &frame(registry, name).widget_data {
            Some(WidgetData::FontString(font)) => font.font_size,
            _ => panic!("Expected title font"),
        };
        assert_initial(&registry);
        assert_eq!(size(&registry, "JournalSuggestion227Name"), 18.);

        // Lua's first pass shrinks the upper title, but the skipped lower frame keeps18.
        data.suggestions.push(AdventureSuggestion {
            id: 823,
            name: "The Ghosts of K'aresh: A Shadowy Invitation".into(),
            description: String::new(),
            reward_description: String::new(),
            texture_fdid: 6937965,
            button_text: String::new(),
        });
        ctx.insert(data.clone());
        screen.sync(&ctx, &mut registry);
        assert_initial(&registry);
        assert_eq!(size(&registry, "JournalSuggestion227Name"), 18.);
        assert_eq!(size(&registry, "JournalSuggestion823Name"), 16.);

        // Three records finally run the second pass on both small frames.
        data.suggestions.push(AdventureSuggestion {
            id: 642,
            name: "Trading Post, Open for Business".into(),
            description: String::new(),
            reward_description: String::new(),
            texture_fdid: 4696085,
            button_text: String::new(),
        });
        ctx.insert(data);
        screen.sync(&ctx, &mut registry);
        assert_eq!(size(&registry, "JournalSuggestion227Name"), 16.);
        assert_eq!(size(&registry, "JournalSuggestion823Name"), 16.);
        assert_eq!(
            frame(&registry, "JournalSuggestion227Text").position.top,
            Val::Px(63.5)
        );
        assert_eq!(
            frame(&registry, "JournalSuggestion227TitleFrame").height,
            Dimension::Fixed(16.)
        );
    }
}

// EncounterJournal.lua2860–2861: only Dungeons/Raids/LootJournal show search.
#[test]
fn monthly_source_hides_encounter_search() {
    use game_engine_ui_model::adventure_guide::{Tab, View, adventure_guide_screen};
    for skin in [ActiveSkin::Modern, ActiveSkin::Forever] {
        let mut ctx = context(skin);
        ctx.insert(View {
            visible: true,
            loading: false,
            tab: Tab::Monthly,
            search: "no matching authored name".into(),
            activities: Some(std::sync::Arc::new(snapshot())),
            ..Default::default()
        });
        let mut registry = FrameRegistry::new(800., 496.);
        Screen::new(adventure_guide_screen).sync(&ctx, &mut registry);
        assert!(registry.get_by_name("EncounterJournalSearchBox").is_none());
        assert!(registry.get_by_name("EncounterJournalNavBar").is_none());
        assert!(registry.get_by_name("JournalActivity3").is_some());
    }
}

#[test]
fn monthly_tooltip_does_not_expose_unsupported_criteria_diagnostics() {
    let _ = context(ActiveSkin::Modern);
    let data = snapshot();
    let lines = tooltip_lines(&data, 3);
    assert!(lines.contains("Complete quests that grant experience."));
    assert!(lines.contains("Shift click"));
    assert!(
        !lines.contains("Criteria tree")
            && !lines.contains("135693")
            && !lines.contains("Unavailable")
    );
    assert!(
        !lines.contains("0/"),
        "Unknown progress must not become zero"
    );
}
