//! MonthlyActivities Lua1057–1090,1281–1344; XML29–165.
use std::{cmp::Ordering, collections::BTreeMap};

use shared::protocol::{AdventureGuideSnapshot, PerksActivityProgress, PerksActivitySnapshot};
use ui_toolkit::widget_def::Element;

use super::{ActivityFilters, presentation};
use crate::{
    bank_art::label,
    quest_art::{HIGHLIGHT_FONT_COLOR, line_height, wrapped_line_count},
    selection_list::{Art, RowArt, RowTemplate},
};

struct ActivityGroup<'a> {
    source_root: &'a PerksActivitySnapshot,
    activity: &'a PerksActivitySnapshot,
    children: Vec<&'a PerksActivitySnapshot>,
}

pub(super) struct ActivityRow<'a> {
    pub activity: &'a PerksActivitySnapshot,
    pub child: bool,
    pub has_children: bool,
    pub expanded: bool,
    pub top: f32,
    pub height: f32,
}

pub(super) fn completed(activity: &PerksActivitySnapshot) -> bool {
    matches!(
        activity.progress,
        PerksActivityProgress::Available {
            completed: true,
            ..
        }
    )
}

fn compare_activities(a: &PerksActivitySnapshot, b: &PerksActivitySnapshot) -> Ordering {
    completed(a)
        .cmp(&completed(b))
        .then_with(|| b.points.cmp(&a.points))
        .then_with(|| a.name.cmp(&b.name))
        .then_with(|| a.id.cmp(&b.id))
}

fn activity_groups(snapshot: &AdventureGuideSnapshot) -> Vec<ActivityGroup<'_>> {
    let by_id: BTreeMap<_, _> = snapshot
        .activities
        .iter()
        .map(|row| (row.id, row))
        .collect();
    // BuildActivityTree has one `child` per authored supersedes ID, not nested disclosure levels.
    // The current source has no forks and a maximum chain depth of3.
    let children: BTreeMap<_, _> = snapshot
        .activities
        .iter()
        .filter(|row| row.supersedes != 0)
        .map(|row| (row.supersedes, row))
        .collect();
    snapshot
        .activities
        .iter()
        .filter(|row| !by_id.contains_key(&row.supersedes))
        .flat_map(|root| group_chain(root, &children))
        .collect()
}

fn group_chain<'a>(
    root: &'a PerksActivitySnapshot,
    children: &BTreeMap<u32, &'a PerksActivitySnapshot>,
) -> Vec<ActivityGroup<'a>> {
    let mut groups = vec![ActivityGroup {
        source_root: root,
        activity: root,
        children: Vec::new(),
    }];
    let mut parent = 0;
    let mut current = root;
    while let Some(child) = children.get(&current.id).copied() {
        let promote_successor = completed(groups[parent].activity);
        if promote_successor || completed(child) {
            groups.push(ActivityGroup {
                source_root: root,
                activity: child,
                children: Vec::new(),
            });
            if promote_successor {
                parent = groups.len() - 1;
            }
        } else {
            groups[parent].children.push(child);
        }
        current = child;
    }
    groups
}

pub(super) fn has_children(snapshot: &AdventureGuideSnapshot, id: u32) -> bool {
    activity_groups(snapshot)
        .iter()
        .any(|group| group.activity.id == id && !group.children.is_empty())
}

pub(super) fn layout<'a>(
    snapshot: &'a AdventureGuideSnapshot,
    filters: &ActivityFilters,
) -> Vec<ActivityRow<'a>> {
    let mut groups = activity_groups(snapshot);
    groups.retain(|group| filters.tag == 0 || group.source_root.tags.contains(&filters.tag));
    groups.sort_by(|a, b| compare_activities(a.activity, b.activity));
    groups
        .into_iter()
        .flat_map(|group| group_rows(group, filters))
        .scan(0., |top, mut row| {
            row.top = *top;
            *top += row.height;
            Some(row)
        })
        .collect()
}

fn group_rows<'a>(
    mut group: ActivityGroup<'a>,
    filters: &ActivityFilters,
) -> impl Iterator<Item = ActivityRow<'a>> {
    let expanded = filters.expanded.contains(&group.activity.id);
    let parent = ActivityRow {
        activity: group.activity,
        child: false,
        has_children: !group.children.is_empty(),
        expanded,
        top: 0.,
        height: 78.,
    };
    group.children.sort_by(|a, b| compare_activities(a, b));
    let children = group
        .children
        .into_iter()
        .filter(move |_| expanded)
        .map(|activity| ActivityRow {
            activity,
            child: true,
            has_children: false,
            expanded: false,
            top: 0.,
            height: 48.,
        });
    std::iter::once(parent).chain(children)
}

pub(super) fn render(row: &ActivityRow<'_>, rewards_available: Option<bool>) -> Element {
    let normal = if completed(row.activity) {
        "activities-complete"
    } else {
        "activities-incomplete"
    };
    let template = RowTemplate {
        normal: Some(RowArt {
            art: Art::Atlas(normal),
            insets: [0.; 4],
            alpha: 1.,
            additive: false,
        }),
        hover: Some(RowArt {
            art: Art::Atlas("activities-incomplete"),
            insets: [0.; 4],
            alpha: 0.3,
            additive: true,
        }),
        selected: None,
    };
    let mut content = row_name(row);
    content.extend(row_marks(row));
    content.extend(row_rewards(row, rewards_available));
    let left = if row.child { 32. } else { 0. };
    crate::selection_list::row(
        format!("JournalActivity{}", row.activity.id),
        &format!("journal:activity:{}", row.activity.id),
        true,
        (left, row.top, 521., row.height),
        (&template, false),
        content,
    )
}

fn row_name(row: &ActivityRow<'_>) -> Element {
    let (left, width, maximum) = if row.child {
        (40., 350., 30.)
    } else {
        (36., 370., 55.)
    };
    let lines = wrapped_line_count(&row.activity.name, width, 14.).max(1);
    let height = (lines as f32 * line_height(14.)).min(maximum);
    let color = if completed(row.activity) {
        "0,0,0,1"
    } else {
        HIGHLIGHT_FONT_COLOR
    };
    label(
        format!("JournalActivity{}Name", row.activity.id),
        &row.activity.name,
        (left, (row.height - height) / 2., width, height),
        (14., color, "LEFT"),
    )
}

fn row_marks(row: &ActivityRow<'_>) -> Element {
    let id = row.activity.id;
    let mut out = Element::default();
    if row.activity.tracked {
        out.extend(presentation::sized_atlas(
            format!("JournalActivity{id}Tracked"),
            "activities-icon-checkmark-small-yellow",
            (23.5, row.height / 2.),
        ));
    }
    if row.has_children {
        let atlas = if row.expanded {
            "campaign_headericon_open"
        } else {
            "campaign_headericon_closed"
        };
        out.extend(presentation::sized_atlas(
            format!("JournalActivity{id}Collapse"),
            atlas,
            (421., row.height / 2.),
        ));
    }
    out
}

fn row_rewards(row: &ActivityRow<'_>, rewards_available: Option<bool>) -> Element {
    let earned = completed(row.activity);
    if !earned && rewards_available != Some(true) {
        return Element::default();
    }
    let id = row.activity.id;
    let mut out = Element::default();
    if !row.child {
        let (atlas, center) = if row.has_children {
            ("activities-ribbon-stacked", (474., 33.))
        } else {
            ("activities-ribbon", (465.5, 32.))
        };
        out.extend(presentation::sized_atlas(
            format!("JournalActivity{id}Ribbon"),
            atlas,
            center,
        ));
    }
    if earned {
        out.extend(presentation::sized_atlas(
            format!("JournalActivity{id}Check"),
            "activities-icon-checkmark",
            (465.5, 25.),
        ));
    } else {
        let (left, top, width, height) = if row.child {
            (437., 0., 44., 48.)
        } else {
            (432., 18., 67., 14.)
        };
        out.extend(label(
            format!("JournalActivity{id}Points"),
            &row.activity.points.to_string(),
            (left, top, width, height),
            (
                14.,
                HIGHLIGHT_FONT_COLOR,
                if row.child { "RIGHT" } else { "CENTER" },
            ),
        ));
    }
    out
}
