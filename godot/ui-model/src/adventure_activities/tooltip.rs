//! MonthlyActivitiesButtonMixin:ShowTooltip / OnClick_Internal (Lua302–407).
use shared::protocol::{
    AchievementCriterionLine, AdventureGuideSnapshot, PerksActivityProgress, PerksActivitySnapshot,
};

use super::{presentation, rows::completed};
use crate::{
    game_tooltip::{GameTooltip, TooltipAnchor},
    tooltip_presentation::{TooltipLineState, TooltipPresentation},
};

const WHITE: [f32; 4] = [1., 1., 1., 1.];
const GRAY: [f32; 4] = [0.5, 0.5, 0.5, 1.];
const GREEN: [f32; 4] = [0., 1., 0., 1.];

pub fn activity_tooltip(snapshot: &AdventureGuideSnapshot, frame: &str) -> Option<GameTooltip> {
    let id: u32 = frame.strip_prefix("JournalActivity")?.parse().ok()?;
    let activity = snapshot.activities.iter().find(|row| row.id == id)?;
    let mut lines = vec![TooltipLineState::new("")];
    if !activity.description.is_empty() {
        lines.push(TooltipLineState::colored(&activity.description, WHITE));
        lines.push(TooltipLineState::new(""));
    }
    lines.extend(progress_lines(activity));
    let help = if activity.tracked {
        Some("MONTHLY_ACTIVITIES_UNTRACK")
    } else if !completed(activity) {
        Some("MONTHLY_ACTIVITIES_TRACK")
    } else {
        None
    };
    if let Some(key) = help {
        lines.push(TooltipLineState::new(""));
        lines.push(TooltipLineState::colored(presentation::global(key), GREEN));
    }
    let mut tooltip = GameTooltip::new(
        TooltipPresentation {
            title: activity.name.clone(),
            title_color: [1., 0.82, 0., 1.],
            lines,
            ..Default::default()
        },
        None,
    );
    tooltip.anchor = TooltipAnchor::CursorRight;
    Some(tooltip)
}

fn progress_lines(activity: &PerksActivitySnapshot) -> Vec<TooltipLineState> {
    match &activity.progress {
        PerksActivityProgress::Unavailable { .. } => Vec::new(),
        PerksActivityProgress::Available {
            current,
            required,
            completed,
            criteria,
        } => {
            let mut criteria: Vec<_> = criteria.iter().collect();
            criteria.sort_by_key(|row| (row.order_index, row.tree_id));
            // The snapshot may know aggregate progress while individual criteria are unevaluated.
            let aggregate_known_only =
                criteria.is_empty() || criteria.iter().any(|row| !row.progress_supported);
            let aggregate = aggregate_known_only.then(|| {
                TooltipLineState::colored(
                    format!("{current}/{required}"),
                    if *completed { GRAY } else { WHITE },
                )
            });
            aggregate
                .into_iter()
                .chain(criteria.into_iter().filter_map(criterion_line))
                .collect()
        }
    }
}

fn criterion_line(criterion: &AchievementCriterionLine) -> Option<TooltipLineState> {
    if !criterion.progress_supported {
        return None;
    }
    let progress = format!("{}/{}", criterion.current, criterion.required);
    let text = if criterion.description.is_empty() {
        progress
    } else {
        format!("{} — {progress}", criterion.description)
    };
    let color = if criterion.completed { GRAY } else { WHITE };
    Some(TooltipLineState::colored(text, color))
}
