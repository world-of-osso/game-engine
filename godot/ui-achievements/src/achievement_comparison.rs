//! Retail Mainline AchievementComparisonTemplate: 496px player + 120px friend.
use super::*;

pub(super) fn content(ctx: &SharedContext, window: &AchievementWindow, summary: bool) -> Element {
    let comparison = window.comparison.as_ref().expect("comparison view");
    let mut content = header(comparison);
    if let Some(error) = &comparison.error {
        content.extend(label(
            "AchievementComparisonError".into(),
            error,
            (230.0, 59.0, 610.0, 24.0),
            WHITE,
            "",
        ));
    }
    if summary {
        content.extend(summary_content(ctx, window, comparison));
    } else {
        content.extend(rows(ctx, window, comparison));
    }
    content
}

fn header(comparison: &AchievementComparison) -> Element {
    let mut content = artwork::texture(
        "AchievementComparisonHeaderArt",
        235401,
        "0,0.821575,0,0.910625",
        (694.0, -43.0, 190.0, 57.0),
    );
    let name = if comparison.name.is_empty() {
        "Loading..."
    } else {
        &comparison.name
    };
    content.extend(label(
        "AchievementComparisonName".into(),
        name,
        (756.0, -18.0, 104.0, 16.0),
        GOLD,
        "",
    ));
    let points = comparison.window.summary.as_ref().map_or_else(
        || "—".into(),
        |summary| {
            game_engine_ui_model::damage_meter_data::break_up_large_number(summary.points.into())
        },
    );
    content.extend(label(
        "AchievementComparisonPoints".into(),
        &points,
        (760.0, 0.0, 72.0, 16.0),
        WHITE,
        "",
    ));
    content.extend(art(
        "AchievementComparisonShield".into(),
        HEADER_SHIELD_FDID,
        (834.0, -1.0, 20.0, 20.0),
    ));
    content
}

fn summary_content(
    ctx: &SharedContext,
    window: &AchievementWindow,
    comparison: &AchievementComparison,
) -> Element {
    let mut content = Vec::new();
    let own = window.summary.as_ref();
    let other = comparison.window.summary.as_ref();
    let own_text = own.map_or_else(
        || "Loading...".into(),
        |s| format!("Achievements completed: {} / {}", s.completed, s.total),
    );
    let other_text = other.map_or_else(
        || "Loading...".into(),
        |s| format!("{} / {}", s.completed, s.total),
    );
    content.extend(label(
        "AchievementComparisonOwnTotal".into(),
        &own_text,
        (10.0, 4.0, 480.0, 22.0),
        GOLD,
        "",
    ));
    content.extend(label(
        "AchievementComparisonOtherTotal".into(),
        &other_text,
        (502.0, 4.0, 112.0, 22.0),
        WHITE,
        "",
    ));
    let mut y = 35.0;
    if let Some(summary) = own {
        for entry in &summary.recent {
            content.extend(row(window, comparison, entry, y));
            y += 51.0;
        }
        for category in &summary.categories {
            let Some(name) = window
                .categories
                .get(&category.category_id)
                .map(|c| &c.name)
            else {
                continue;
            };
            let text = format!("{}: {} / {}", name, category.completed, category.total);
            let other = other.and_then(|s| {
                s.categories
                    .iter()
                    .find(|c| c.category_id == category.category_id)
            });
            let other_text =
                other.map_or_else(|| "—".into(), |c| format!("{} / {}", c.completed, c.total));
            let action = format!("achievement:category:{}", category.category_id);
            content.extend(label(
                format!("AchievementComparisonCategory{}", category.category_id),
                &text,
                (10.0, y, 480.0, 24.0),
                WHITE,
                &action,
            ));
            content.extend(label(
                format!("AchievementComparisonCategory{}Other", category.category_id),
                &other_text,
                (502.0, y, 112.0, 24.0),
                WHITE,
                &action,
            ));
            y += 25.0;
        }
    }
    pane(ctx, content, y)
}

fn rows(
    ctx: &SharedContext,
    window: &AchievementWindow,
    comparison: &AchievementComparison,
) -> Element {
    let mut content = Vec::new();
    let mut y = 0.0;
    for entry in window.displayed_entries() {
        content.extend(row(window, comparison, entry, y));
        let expanded = Some(entry.achievement_id) == window.expanded_achievement;
        y += if entry.flags & 1 != 0 {
            25.0
        } else {
            row_height(entry, expanded, window.criterion_offset) + 1.0
        };
    }
    if window.displayed_entries().is_empty() {
        content.extend(label(
            "AchievementEmpty".into(),
            empty_message(window),
            (12.0, 30.0, 470.0, 30.0),
            WHITE,
            "",
        ));
    }
    pane(ctx, content, y)
}

fn pane(ctx: &SharedContext, content: Element, height: f32) -> Element {
    artwork::scroll_pane(
        ctx,
        ROWS_SCROLLBOX,
        content,
        (222.0, 58.0, 620.0, 417.0),
        height.max(417.0),
    )
}

fn row_height(entry: &AchievementCatalogEntry, expanded: bool, offset: usize) -> f32 {
    if !expanded {
        return 50.0;
    }
    50.0 + entry.criteria.len().saturating_sub(offset).min(CRITERIA) as f32 * 18.0 + 24.0
}

fn row(
    window: &AchievementWindow,
    comparison: &AchievementComparison,
    entry: &AchievementCatalogEntry,
    y: f32,
) -> Element {
    let other = comparison.entries.get(&entry.achievement_id);
    if entry.flags & 1 != 0 {
        return statistic(entry, other, y);
    }
    let expanded = Some(entry.achievement_id) == window.expanded_achievement;
    let height = row_height(entry, expanded, window.criterion_offset);
    let prefix = format!("Achievement{}", entry.achievement_id);
    let mut content = player_card(entry, &prefix);
    content.extend(friend_card(&prefix, other, height));
    if expanded {
        content.extend(criteria(window, entry, other));
    }
    rsx! { button { name:{DynName(format!("{prefix}Row"))},width:616.0,height,left:0.0,top:y,pos_type:"absolute",
        onclick:{format!("achievement:row:{}",entry.achievement_id)},button_default_skin:false,{content}
    } }
}

fn player_card(entry: &AchievementCatalogEntry, prefix: &str) -> Element {
    let background = if entry.earned { 130661 } else { 130660 };
    let ink = if thread_skin() == ActiveSkin::Modern {
        "0.1,0.08,0.05,1"
    } else {
        WHITE
    };
    let mut content = artwork::texture(
        &format!("{prefix}Background"),
        background,
        "0,1,0,0.25",
        (3.0, 3.0, 490.0, 44.0),
    );
    content.extend(artwork::texture(
        &format!("{prefix}TitleBar"),
        452109,
        "0,0.9765625,0.66015625,0.73828125",
        (5.0, 4.0, 486.0, 20.0),
    ));
    content.extend(art(
        format!("{prefix}Icon"),
        entry.icon_fdid,
        (9.0, 6.0, 36.0, 36.0),
    ));
    content.extend(art(
        format!("{prefix}IconFrame"),
        130656,
        (3.0, 2.0, 46.0, 46.0),
    ));
    content.extend(label(
        format!("{prefix}Name"),
        &entry.name,
        (53.0, 4.0, 380.0, 20.0),
        WHITE,
        "",
    ));
    content.extend(label(
        format!("{prefix}Description"),
        &entry.description,
        (53.0, 27.0, 290.0, 18.0),
        ink,
        "",
    ));
    let status = if entry.earned {
        "Completed"
    } else {
        "Incomplete"
    };
    content.extend(label(
        format!("{prefix}OwnStatus"),
        status,
        (350.0, 27.0, 90.0, 18.0),
        ink,
        "",
    ));
    content.extend(art(
        format!("{prefix}Shield"),
        130666,
        (444.0, 4.0, 40.0, 40.0),
    ));
    content.extend(label(
        format!("{prefix}Points"),
        &entry.points.to_string(),
        (450.0, 10.0, 34.0, 22.0),
        GOLD,
        "",
    ));
    content
}

fn friend_card(
    prefix: &str,
    other: Option<&Option<AchievementCatalogEntry>>,
    height: f32,
) -> Element {
    let mut content = artwork::texture(
        &format!("{prefix}ComparisonBackground"),
        130661,
        "0.2,0.75,0,1",
        (499.0, 3.0, 114.0, height - 6.0),
    );
    content.extend(artwork::texture(
        &format!("{prefix}ComparisonTitleBar"),
        452109,
        "0.3,0.575,0.66015625,0.73828125",
        (501.0, 4.0, 110.0, 20.0),
    ));
    let status = match other {
        None => "Loading...",
        Some(None) => "Unavailable",
        Some(Some(e)) if e.earned => "Completed",
        Some(Some(_)) => "Incomplete",
    };
    let ink = if thread_skin() == ActiveSkin::Modern {
        "0.1,0.08,0.05,1"
    } else {
        WHITE
    };
    content.extend(label(
        format!("{prefix}ComparisonStatus"),
        status,
        (540.0, 29.0, 73.0, 18.0),
        ink,
        "",
    ));
    if let Some(Some(entry)) = other {
        content.extend(art(
            format!("{prefix}ComparisonIcon"),
            entry.icon_fdid,
            (505.0, 6.0, 36.0, 36.0),
        ));
        content.extend(label(
            format!("{prefix}ComparisonPoints"),
            &entry.points.to_string(),
            (574.0, 5.0, 32.0, 20.0),
            GOLD,
            "",
        ));
        if let Some(date) = entry
            .earned_at
            .and_then(|seconds| chrono::DateTime::from_timestamp(seconds, 0))
        {
            content.extend(label(
                format!("{prefix}ComparisonDate"),
                &date.format("%m/%d/%y").to_string(),
                (544.0, 9.0, 69.0, 18.0),
                WHITE,
                "",
            ));
        }
    }
    content
}

fn criteria(
    window: &AchievementWindow,
    entry: &AchievementCatalogEntry,
    other: Option<&Option<AchievementCatalogEntry>>,
) -> Element {
    let mut content = Vec::new();
    for (index, line) in entry
        .criteria
        .iter()
        .skip(window.criterion_offset)
        .take(CRITERIA)
        .enumerate()
    {
        let y = 50.0 + index as f32 * 18.0;
        let own = criterion_text(line);
        content.extend(label(
            format!(
                "Achievement{}Criterion{}",
                entry.achievement_id, line.tree_id
            ),
            &format!("{}: {own}", line.description),
            (12.0, y, 478.0, 18.0),
            WHITE,
            "",
        ));
        let other_line = other.and_then(|entry| entry.as_ref()).and_then(|entry| {
            entry
                .criteria
                .iter()
                .find(|other| other.tree_id == line.tree_id)
        });
        let text = other_line.map_or_else(|| "—".into(), criterion_text);
        content.extend(label(
            format!(
                "Achievement{}ComparisonCriterion{}",
                entry.achievement_id, line.tree_id
            ),
            &text,
            (504.0, y, 106.0, 18.0),
            WHITE,
            "",
        ));
    }
    let y = row_height(entry, true, window.criterion_offset) - 23.0;
    content.extend(panel_button(
        "AchievementCriteriaPrevious".into(),
        "Previous",
        "achievement:criteria_prev",
        window.criterion_offset > 0,
        (12.0, y, 90.0, 22.0),
    ));
    content.extend(panel_button(
        "AchievementCriteriaMore".into(),
        "More",
        "achievement:more_criteria",
        entry.criteria.len() > window.criterion_offset + CRITERIA
            || entry.next_criteria_id.is_some(),
        (104.0, y, 90.0, 22.0),
    ));
    content
}

fn criterion_text(line: &AchievementCriterionLine) -> String {
    if !line.progress_supported {
        return "Unevaluated".into();
    }
    format!("{}/{}", line.current, line.required)
}

fn statistic(
    entry: &AchievementCatalogEntry,
    other: Option<&Option<AchievementCatalogEntry>>,
    y: f32,
) -> Element {
    let mut content = statistic_row(entry, y, 0);
    let value = other
        .and_then(|entry| entry.as_ref())
        .and_then(|entry| entry.statistic_value)
        .map_or_else(
            || "Unevaluated".into(),
            game_engine_ui_model::damage_meter_data::break_up_large_number,
        );
    content.extend(label(
        format!("Achievement{}ComparisonStatistic", entry.achievement_id),
        &value,
        (504.0, y, 112.0, 24.0),
        WHITE,
        "",
    ));
    content
}
