//! Retail Traveler's Log: Blizzard_MonthlyActivities.{lua,xml}.
mod presentation;
mod rows;
mod tooltip;

use std::collections::BTreeSet;

use shared::protocol::{AdventureGuideRequest, AdventureGuideSnapshot};
use ui_toolkit::{atlas::ActiveSkin, rsx, screen::SharedContext, widget_def::Element};

use crate::adventure_guide::{LIST, scroll, text};
use game_engine_ui_model::minimal_scroll_bar::{
    MinimalScrollBar, Unscrollable, pixel_geometry, scroll_list_attr,
};
use game_engine_ui_model::quest_art::DynName;

pub use tooltip::activity_tooltip;

const HEIGHT: f32 = 291.;
pub const TAG_LIST: &str = "EncounterJournalFilterList";

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct ActivityFilters {
    pub tag: u32,
    pub expanded: BTreeSet<u32>,
}

impl ActivityFilters {
    pub fn toggle_group(&mut self, snapshot: Option<&AdventureGuideSnapshot>, id: u32) {
        if !snapshot.is_some_and(|snapshot| rows::has_children(snapshot, id)) {
            return;
        }
        if !self.expanded.remove(&id) {
            self.expanded.insert(id);
        }
    }
}

/// A request only: the owning realm reply changes tracking, never the click itself.
pub fn tracking_request(
    snapshot: &AdventureGuideSnapshot,
    action: &str,
    right: bool,
    shift: bool,
) -> Option<AdventureGuideRequest> {
    if right || !shift {
        return None;
    }
    let id: u32 = action.strip_prefix("journal:activity:")?.parse().ok()?;
    let row = snapshot.activities.iter().find(|row| row.id == id)?;
    if rows::completed(row) && !row.tracked {
        return None;
    }
    Some(AdventureGuideRequest::TrackActivity {
        activity_id: id,
        tracked: !row.tracked,
    })
}

pub fn monthly(
    ctx: &SharedContext,
    snapshot: Option<&AdventureGuideSnapshot>,
    filters: &ActivityFilters,
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
    out.extend(activity_viewport(ctx, snapshot, filters));
    rsx! { r#frame { name: "EncounterJournalMonthlyActivitiesFrame", left: 4., top: 60., width: 792., height: 431., pos_type: "absolute", {out} } }
}

fn activity_viewport(
    ctx: &SharedContext,
    snapshot: &AdventureGuideSnapshot,
    filters: &ActivityFilters,
) -> Element {
    let rows = rows::layout(snapshot, filters);
    let content_height = rows
        .last()
        .map_or(HEIGHT, |row| row.top + row.height)
        .max(HEIGHT);
    let offset =
        pixel_geometry(HEIGHT, content_height, HEIGHT).clamp(ctx.scroll_first_row(LIST)) as f32;
    let reward_available = ctx
        .get::<crate::adventure_guide::View>()
        .and_then(|view| view.trading_post.as_ref())
        .and_then(|snapshot| snapshot.thresholds.iter().map(|row| row.points).max())
        .filter(|maximum| *maximum > 0)
        .map(|maximum| snapshot.points < maximum);
    let content = rows
        .iter()
        .filter(|row| row.top + row.height > offset && row.top < offset + HEIGHT)
        .flat_map(|row| rows::render(row, reward_available))
        .collect();
    scroll(ctx, content, (241., 138., 521., HEIGHT), content_height)
}

fn filter_tags(snapshot: &AdventureGuideSnapshot) -> Vec<(u32, &str)> {
    let used: BTreeSet<_> = snapshot
        .activities
        .iter()
        .flat_map(|row| row.tags.iter().copied())
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
    std::iter::once((0, presentation::global("ALL")))
        .chain(tags.into_iter().map(|tag| (tag.id, tag.name.as_str())))
        .collect()
}

fn filter_controls(
    ctx: &SharedContext,
    snapshot: &AdventureGuideSnapshot,
    filters: &ActivityFilters,
) -> Element {
    let tags = filter_tags(snapshot);
    let geometry = pixel_geometry(290., 20. + tags.len() as f32 * 22. - 2., 279.);
    let offset = geometry.clamp(ctx.scroll_first_row(TAG_LIST));
    let rows: Element = tags
        .iter()
        .enumerate()
        .filter(|(index, _)| {
            let top = 10. + *index as f32 * 22.;
            top + 20. > offset as f32 && top < offset as f32 + 290.
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
    rsx! { r#frame { name: {DynName(TAG_LIST.into())}, left: 5., top: 137., width: 220., height: 290., pos_type: "absolute", mouse_enabled: true, scroll_list: config, {rows} {bar.element()} } }
}

fn filter_row(id: u32, name: &str, selected: bool, top: f32) -> Element {
    use game_engine_ui_model::selection_list::{Art, RowArt, RowTemplate};
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
        game_engine_ui_model::quest_art::HIGHLIGHT_FONT_COLOR
    } else {
        game_engine_ui_model::quest_art::NORMAL_FONT_COLOR
    };
    let label = game_engine_ui_model::bank_art::label(
        format!("JournalActivityTag{id}Name"),
        name,
        (10., -1., 165., 20.),
        (12., color, "LEFT"),
    );
    game_engine_ui_model::selection_list::row(
        format!("JournalActivityTag{id}"),
        &format!("journal:activity-tag:{id}"),
        true,
        (10., top, 175., 20.),
        (&template, selected),
        label,
    )
}
