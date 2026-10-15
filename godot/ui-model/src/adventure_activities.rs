//! Source-backed Traveler's Log.
//! Retail: Blizzard_MonthlyActivities.{lua,xml} (filter list, activity tree/rows).
mod presentation;

use std::collections::{BTreeMap, BTreeSet};

use shared::protocol::{AdventureGuideSnapshot, PerksActivityProgress, PerksActivitySnapshot};
use ui_toolkit::{atlas::ActiveSkin, rsx, screen::SharedContext, widget_def::Element};

use crate::{
    adventure_guide::{LIST, button, scroll, text},
    minimal_scroll_bar::{MinimalScrollBar, Unscrollable, pixel_geometry, scroll_list_attr},
    quest_art::DynName,
};

const TOP: f32 = 138.;
const HEIGHT: f32 = 291.;
const ROW_HEIGHT: f32 = 82.;
const TAG_LIST: &str = "EncounterJournalFilterList";

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct ActivityFilters {
    pub tracked_only: bool,
    pub tag: u32,
    pub selected: Option<u32>,
}

pub fn monthly(
    ctx: &SharedContext,
    snapshot: Option<&AdventureGuideSnapshot>,
    filters: &ActivityFilters,
    search: &str,
) -> Element {
    let _ = ctx.get::<ActiveSkin>();
    let Some(snapshot) = snapshot else {
        return text(
            "JournalMonthlyLoading",
            "Loading Traveler's Log…",
            (15., 65., 770., 32.),
        );
    };
    if let Some(error) = &snapshot.source_error {
        return text(
            "JournalMonthlyNotice",
            &format!("Missing source data: {error}"),
            (15., 93., 770., 40.),
        );
    }
    let tender = ctx
        .get::<crate::adventure_guide::View>()
        .and_then(|view| view.trading_post.as_deref());
    let mut out = presentation::background();
    out.extend(presentation::header(snapshot));
    out.extend(presentation::thresholds(snapshot, tender));
    out.extend(filter_controls(ctx, snapshot, filters));
    let rows = activity_rows(snapshot, filters, search);
    let content_height = rows
        .last()
        .map_or(HEIGHT, |row| row.top + row.height)
        .max(HEIGHT);
    let offset =
        pixel_geometry(HEIGHT, content_height, HEIGHT).clamp(ctx.scroll_first_row(LIST)) as f32;
    let mut content: Element = rows
        .iter()
        .filter(|row| row.top + row.height > offset && row.top < offset + HEIGHT)
        .flat_map(activity_row)
        .collect();
    if rows.is_empty() {
        let message = if snapshot.activities.is_empty() {
            "No source activities available."
        } else {
            "No activities match these filters."
        };
        content.extend(text("JournalMonthlyEmpty", message, (0., 0., 520., 40.)));
    }
    out.extend(scroll(
        ctx,
        content,
        (241., TOP, 521., HEIGHT),
        content_height,
    ));
    rsx! { r#frame { name: "EncounterJournalMonthlyActivitiesFrame", left: 4., top: 60., width: 792., height: 431., pos_type: "absolute", {out} } }
}

fn filter_controls(
    ctx: &SharedContext,
    snapshot: &AdventureGuideSnapshot,
    filters: &ActivityFilters,
) -> Element {
    // Retail only lists tags actually used by the source activities.
    let used: BTreeSet<_> = snapshot
        .activities
        .iter()
        .flat_map(|row| row.tags.iter())
        .collect();
    let mut tags: Vec<_> = snapshot
        .tags
        .iter()
        .filter(|tag| used.contains(&tag.id))
        .collect();
    let other = presentation::global("OTHER");
    tags.sort_by(|a, b| {
        (a.name == other)
            .cmp(&(b.name == other))
            .then_with(|| a.name.cmp(&b.name))
    });
    let all = presentation::global("ALL");
    let tags: Vec<_> = std::iter::once((0, all))
        .chain(tags.into_iter().map(|tag| (tag.id, tag.name.as_str())))
        .collect();
    let geometry = pixel_geometry(290., 20. + tags.len() as f32 * 22. - 2., 279.);
    let offset = geometry.clamp(ctx.scroll_first_row(TAG_LIST));
    let rows: Element = tags
        .iter()
        .enumerate()
        .filter(|(index, _)| {
            10. + *index as f32 * 22. + 20. > offset as f32
                && 10. + *index as f32 * 22. < offset as f32 + 290.
        })
        .flat_map(|(index, (id, name))| {
            filter_row(
                *id,
                name,
                filters.tag == *id,
                10. + index as f32 * 22. - offset as f32,
            )
        })
        .collect();
    let bar = MinimalScrollBar {
        list: TAG_LIST,
        left: 202.,
        top: 7.,
        height: 279.,
        geometry,
        offset,
        unscrollable: Unscrollable::HideThumb,
    };
    let config = scroll_list_attr(&geometry);
    rsx! { r#frame { name:{DynName(TAG_LIST.into())}, left:5.0, top:137.0, width:220.0, height:290.0, pos_type:"absolute", mouse_enabled:true, scroll_list:config, {rows} {bar.element()} } }
}

fn filter_row(id: u32, name: &str, selected: bool, top: f32) -> Element {
    use crate::selection_list::{Art, RowArt, RowTemplate};
    let art = |atlas| RowArt {
        art: Art::Atlas(atlas),
        insets: [-6., -0.5, -6., -0.5],
        alpha: 1.,
        additive: false,
    };
    let template = RowTemplate {
        normal: None,
        hover: Some(art("options_list_hover")),
        selected: Some(art("options_list_active")),
    };
    let color = if selected {
        crate::quest_art::HIGHLIGHT_FONT_COLOR
    } else {
        crate::quest_art::NORMAL_FONT_COLOR
    };
    let label = crate::bank_art::label(
        format!("JournalActivityTag{id}Name"),
        name,
        (10., -1., 165., 20.),
        (12., color, "LEFT"),
    );
    crate::selection_list::row(
        format!("JournalActivityTag{id}"),
        &format!("journal:activity-tag:{id}"),
        true,
        (10., top, 175., 20.),
        (&template, selected),
        label,
    )
}

struct ActivityRow<'a> {
    activity: &'a PerksActivitySnapshot,
    depth: usize,
    top: f32,
    height: f32,
    details: Option<String>,
}

fn activity_rows<'a>(
    snapshot: &'a AdventureGuideSnapshot,
    filters: &ActivityFilters,
    search: &str,
) -> Vec<ActivityRow<'a>> {
    let search = search.to_lowercase();
    let mut activities: Vec<_> = snapshot
        .activities
        .iter()
        .filter(|row| {
            (!filters.tracked_only || row.tracked)
                && (filters.tag == 0 || row.tags.contains(&filters.tag))
                && (row.name.to_lowercase().contains(&search)
                    || row.description.to_lowercase().contains(&search))
        })
        .collect();
    // Retail's non-timed ordering: unfinished first, then contributions and name.
    activities.sort_by(|a, b| {
        completed(a)
            .cmp(&completed(b))
            .then_with(|| b.points.cmp(&a.points))
            .then_with(|| a.name.cmp(&b.name))
            .then_with(|| a.id.cmp(&b.id))
    });
    let ids: BTreeSet<_> = activities.iter().map(|row| row.id).collect();
    let mut children: BTreeMap<u32, Vec<&PerksActivitySnapshot>> = BTreeMap::new();
    for activity in &activities {
        children
            .entry(activity.supersedes)
            .or_default()
            .push(activity);
    }
    let roots = activities
        .iter()
        .filter(|row| !ids.contains(&row.supersedes));
    let mut stack: Vec<_> = roots
        .chain(activities.iter())
        .map(|row| (*row, 0))
        .collect();
    stack.reverse();
    let mut visited = BTreeSet::new();
    let mut rows = Vec::new();
    let mut top = 0.;
    while let Some((activity, depth)) = stack.pop() {
        if !visited.insert(activity.id) {
            continue;
        }
        let details =
            (filters.selected == Some(activity.id)).then(|| activity_details(snapshot, activity));
        let height = ROW_HEIGHT
            + details
                .as_ref()
                .map_or(0., |value| text_height(value, 66) + 12.);
        rows.push(ActivityRow {
            activity,
            depth,
            top,
            height,
            details,
        });
        top += height;
        if let Some(children) = children.get(&activity.id) {
            stack.extend(children.iter().rev().map(|child| (*child, depth + 1)));
        }
    }
    rows
}

fn completed(activity: &PerksActivitySnapshot) -> bool {
    matches!(
        activity.progress,
        PerksActivityProgress::Available {
            completed: true,
            ..
        }
    )
}

fn count_progress(current: u64, required: u64, completed: bool) -> String {
    format!(
        "{current} / {required} — {}",
        if completed { "Complete" } else { "In progress" }
    )
}

fn activity_progress(activity: &PerksActivitySnapshot) -> String {
    match &activity.progress {
        PerksActivityProgress::Available {
            current,
            required,
            completed,
            ..
        } => count_progress(*current, *required, *completed),
        PerksActivityProgress::Unavailable { reason } => format!("Unavailable: {reason}"),
    }
}

fn activity_details(snapshot: &AdventureGuideSnapshot, activity: &PerksActivitySnapshot) -> String {
    let mut lines = vec![activity.description.clone()];
    if let Some(parent) = snapshot
        .activities
        .iter()
        .find(|row| row.id == activity.supersedes)
    {
        lines.push(format!("Follows: {}", parent.name));
    }
    match &activity.progress {
        PerksActivityProgress::Unavailable { reason } => {
            lines.push(format!("Unavailable: {reason}"))
        }
        PerksActivityProgress::Available { criteria, .. } => {
            let mut criteria: Vec<_> = criteria.iter().collect();
            criteria.sort_by_key(|criterion| (criterion.order_index, criterion.tree_id));
            lines.extend(criteria.into_iter().map(|criterion| {
                let progress = if criterion.progress_supported {
                    count_progress(criterion.current, criterion.required, criterion.completed)
                } else {
                    "Unavailable: criterion is not tracked".into()
                };
                format!("{} — {progress}", criterion.description)
            }));
        }
    }
    lines.retain(|line| !line.is_empty());
    lines.join("\n")
}

fn text_height(value: &str, columns: usize) -> f32 {
    value
        .lines()
        .map(|line| line.chars().count().div_ceil(columns).max(1))
        .sum::<usize>()
        .max(1) as f32
        * 18.
}

fn activity_row(row: &ActivityRow<'_>) -> Element {
    let activity = row.activity;
    let id = activity.id;
    let inset = (row.depth.min(3) * 12) as f32;
    let mut out = crate::selection_list::text_row(
        format!("JournalActivity{id}"),
        &activity.name,
        &format!("journal:activity:{id}"),
        (true, row.details.is_some()),
        (inset, row.top, 416. - inset, 32.),
        &crate::selection_list::SEARCH,
    );
    out.extend(text(
        &format!("JournalActivity{id}Progress"),
        &activity_progress(activity),
        (inset + 4., row.top + 34., 408. - inset, 44.),
    ));
    out.extend(text(
        &format!("JournalActivity{id}Points"),
        &format!("{} points", activity.points),
        (424., row.top + 2., 99., 24.),
    ));
    out.extend(button(
        format!("JournalActivity{id}Track"),
        if activity.tracked { "Untrack" } else { "Track" },
        &format!(
            "journal:activity-track:{id}:{}",
            u8::from(!activity.tracked)
        ),
        (424., row.top + 34., 99., 26.),
    ));
    if let Some(details) = &row.details {
        out.extend(text(
            "JournalActivityDetails",
            details,
            (
                inset + 4.,
                row.top + ROW_HEIGHT,
                519. - inset,
                row.height - ROW_HEIGHT,
            ),
        ));
    }
    out
}
