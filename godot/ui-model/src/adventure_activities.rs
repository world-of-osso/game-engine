//! Source-backed Traveler's Log.
//! Retail: Blizzard_MonthlyActivities.{lua,xml} (filter list, activity tree/rows).
use std::collections::{BTreeMap, BTreeSet};

use shared::protocol::{AdventureGuideSnapshot, PerksActivityProgress, PerksActivitySnapshot};
use ui_toolkit::{atlas::ActiveSkin, rsx, screen::SharedContext, widget_def::Element};

use crate::{
    adventure_guide::{LIST, button, scroll, text},
    minimal_scroll_bar::{MinimalScrollBar, Unscrollable, pixel_geometry, scroll_list_attr},
    quest_art::DynName,
};

const TOP: f32 = 156.;
const HEIGHT: f32 = 325.;
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
    let mut out = text(
        "JournalMonthlyTitle",
        &format!(
            "Traveler's Log — {} {}",
            snapshot.month.name, snapshot.month.year
        ),
        (15., 65., 770., 24.),
    );
    let tender = ctx
        .get::<crate::adventure_guide::View>()
        .and_then(|view| view.trading_post.as_ref());
    let contributions = match tender {
        Some(tender) => {
            let cap: u32 = tender
                .thresholds
                .iter()
                .map(|threshold| threshold.tender)
                .sum();
            let earned: u32 = tender
                .thresholds
                .iter()
                .filter(|threshold| threshold.claimed)
                .map(|threshold| threshold.tender)
                .sum();
            format!(
                "{} activity points · Tender earned {earned}/{cap} · Balance {}",
                snapshot.points, tender.tender
            )
        }
        None => format!(
            "Completed activity contributions: {} points",
            snapshot.points
        ),
    };
    out.extend(text(
        "JournalMonthlyPoints",
        &contributions,
        (15., 93., 770., 20.),
    ));
    let notice = source_notice(snapshot);
    out.extend(text(
        "JournalMonthlyNotice",
        &notice,
        (240., 116., 545., 36.),
    ));
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
        (240., TOP, 545., HEIGHT),
        content_height,
    ));
    out
}

fn source_notice(snapshot: &AdventureGuideSnapshot) -> String {
    match &snapshot.source_error {
        Some(error) => format!("Missing source data: {error}"),
        None => snapshot.activity_scope_notice.clone().unwrap_or_default(),
    }
}

fn filter_controls(
    ctx: &SharedContext,
    snapshot: &AdventureGuideSnapshot,
    filters: &ActivityFilters,
) -> Element {
    let mut out = button(
        "JournalActivityFilterAll".into(),
        if filters.tracked_only {
            "All"
        } else {
            "All ✓"
        },
        "journal:activity-filter:all",
        (15., 116., 98., 26.),
    );
    out.extend(button(
        "JournalActivityFilterTracked".into(),
        if filters.tracked_only {
            "Tracked ✓"
        } else {
            "Tracked"
        },
        "journal:activity-filter:tracked",
        (117., 116., 108., 26.),
    ));
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
    tags.sort_by(|a, b| a.name.cmp(&b.name));
    let tags: Vec<_> = std::iter::once((0, "All tags"))
        .chain(tags.into_iter().map(|tag| (tag.id, tag.name.as_str())))
        .collect();
    let geometry = pixel_geometry(HEIGHT, tags.len() as f32 * 24., HEIGHT);
    let offset = geometry.clamp(ctx.scroll_first_row(TAG_LIST));
    let rows: Element = tags
        .iter()
        .enumerate()
        .filter(|(index, _)| {
            (*index + 1) as f32 * 24. > offset as f32
                && *index as f32 * 24. < offset as f32 + HEIGHT
        })
        .flat_map(|(index, (id, name))| {
            let title = if filters.tag == *id {
                format!("{name} ✓")
            } else {
                (*name).into()
            };
            button(
                format!("JournalActivityTag{id}"),
                &title,
                &format!("journal:activity-tag:{id}"),
                (0., index as f32 * 24. - offset as f32, 188., 23.),
            )
        })
        .collect();
    let bar = MinimalScrollBar {
        list: TAG_LIST,
        left: 196.,
        top: 0.,
        height: HEIGHT,
        geometry,
        offset,
        unscrollable: Unscrollable::HideThumb,
    };
    let config = scroll_list_attr(&geometry);
    out.extend(rsx! { r#frame { name:{DynName(TAG_LIST.into())}, left:15.0, top:TOP, width:210.0, height:HEIGHT, pos_type:"absolute", mouse_enabled:true, scroll_list:config, {rows} {bar.element()} } });
    out
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
