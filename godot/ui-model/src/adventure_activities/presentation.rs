//! Blizzard_MonthlyActivities.xml:166–596 and Lua:1399–1457,1505–1572.
use shared::protocol::{AdventureGuideSnapshot, TradingPostSnapshot, TradingPostThreshold};
use ui_toolkit::{
    atlas::{resolve_region, thread_skin},
    rsx,
    widget_def::Element,
    widgets::font_string::GameFont,
};

use crate::{
    bank_art::label,
    quest_art::{DynName, HIGHLIGHT_FONT_COLOR, NORMAL_FONT_COLOR, named_atlas_texture},
};

pub(super) fn global(key: &str) -> String {
    crate::auction_mail::text(key).unwrap_or_else(|| panic!("Missing authored GlobalString {key}"))
}

pub(super) fn sized_atlas(name: String, atlas: &str, center: (f32, f32)) -> Element {
    let region = resolve_region(atlas, thread_skin())
        .unwrap_or_else(|| panic!("Missing MonthlyActivities atlas {atlas}"));
    named_atlas_texture(
        name,
        atlas,
        (
            center.0 - region.width / 2.,
            center.1 - region.height / 2.,
            region.width,
            region.height,
        ),
    )
}

pub(super) fn background() -> Element {
    let mut out = named_atlas_texture(
        "JournalMonthlyBackground".into(),
        "activities-background",
        (0., 0., 792., 431.),
    );
    out.extend(named_atlas_texture(
        "JournalMonthlyFilterBackground".into(),
        "activities-container",
        (5., 137., 220., 290.),
    ));
    out.extend(sized_atlas(
        "JournalMonthlyDivider".into(),
        "activities-divider",
        (396., 134.),
    ));
    out.extend(sized_atlas(
        "JournalMonthlyDividerVertical".into(),
        "activities-divider-vertical",
        (228., 282.5),
    ));
    out.extend(named_atlas_texture(
        "JournalMonthlyLeftShadow".into(),
        "activities-container-shadow-left",
        (5., 137., 220., 290.),
    ));
    out.extend(named_atlas_texture(
        "JournalMonthlyRightShadow".into(),
        "activities-container-shadow-right",
        (229., 135., 560., 294.),
    ));
    out
}

fn text_size(text: &str, size: f32) -> (f32, f32) {
    ui_toolkit::text_measure::measure_text(text, GameFont::FrizQuadrata, size)
        .expect("MonthlyActivities authored Friz metrics")
}

pub(super) fn header(snapshot: &AdventureGuideSnapshot) -> Element {
    let title = global("MONTHLY_ACTIVITIES_TAB");
    let (width, height) = text_size(&title, 18.);
    let mut out = label(
        "JournalMonthlyTitle".into(),
        &title,
        (20., 16., width, height),
        (18., NORMAL_FONT_COLOR, "LEFT"),
    );
    let month_size = text_size(&snapshot.month.name, 14.);
    out.extend(label(
        "JournalMonthlyMonth".into(),
        &snapshot.month.name,
        (
            20. + width + 16.,
            16. + (height - month_size.1) / 2.,
            month_size.0,
            month_size.1,
        ),
        (14., HIGHLIGHT_FONT_COLOR, "LEFT"),
    ));
    let left = snapshot
        .month
        .end_unix
        .saturating_sub(snapshot.month.now_unix)
        .max(0) as u64;
    let text = global("MONTHLY_ACTIVITIES_DAYS");
    let text = text
        .strip_prefix("|A:activities-clock:0:0:0:0|a ")
        .expect("Source MonthlyActivities clock prefix")
        .replace("%s", &time_left(left));
    let (width, height) = text_size(&text, 12.);
    out.extend(label(
        "JournalMonthlyTimeLeft".into(),
        &text,
        (772. - width, 18., width, height),
        (12., HIGHLIGHT_FONT_COLOR, "LEFT"),
    ));
    out.extend(sized_atlas(
        "JournalMonthlyClock".into(),
        "activities-clock",
        (762. - width, 18. + height / 2.),
    ));
    out
}

// SecondsFormatterMixin: days→minutes, two nonzero units, floor, one-letter/strip whitespace.
fn time_left(mut seconds: u64) -> String {
    let mut units = Vec::new();
    for (interval, key) in [
        (86400, "DAY_ONELETTER_ABBR"),
        (3600, "HOUR_ONELETTER_ABBR"),
        (60, "MINUTE_ONELETTER_ABBR"),
    ] {
        let count = seconds / interval;
        seconds %= interval;
        if count > 0 {
            units.push(
                global(key)
                    .replace("%d", &count.to_string())
                    .replace(' ', "")
                    .to_lowercase(),
            );
        }
        if units.len() == 2 {
            break;
        }
    }
    if units.is_empty() {
        return global("MINUTE_ONELETTER_ABBR")
            .replace("%d", "0")
            .replace(' ', "")
            .to_lowercase();
    }
    units.join(&global("TIME_UNIT_DELIMITER"))
}

pub(super) fn thresholds(
    snapshot: &AdventureGuideSnapshot,
    tender: Option<&TradingPostSnapshot>,
) -> Element {
    let mut content = sized_atlas(
        "JournalMonthlyBarBackground".into(),
        "activities-bar-background",
        (331.5, 16.5),
    );
    if let Some(tender) = tender {
        content.extend(threshold_progress(snapshot.points, &tender.thresholds));
    }
    content.extend(sized_atlas(
        "JournalMonthlyBarBorder".into(),
        "activities-bar-border",
        (331.5, 18.5),
    ));
    rsx! { r#frame { name: "JournalMonthlyThresholdContainer", left: 46., top: 74., width: 663., height: 33., pos_type: "absolute", {content} } }
}

fn threshold_progress(points: u32, thresholds: &[TradingPostThreshold]) -> Element {
    let Some(maximum) = thresholds
        .iter()
        .map(|row| row.points)
        .max()
        .filter(|maximum| *maximum > 0)
    else {
        return Element::default();
    };
    let points = points.min(maximum);
    let width = points as f32 / maximum as f32 * 663.;
    let mut out = Element::default();
    if points > 0 {
        out.extend(bar_fill(width));
        out.extend(sized_atlas(
            "JournalMonthlyBarEnd".into(),
            "activities-bar-end",
            (width, 16.5),
        ));
    }
    out.extend(progress_labels(points, maximum));
    for (index, threshold) in thresholds.iter().enumerate() {
        let left = threshold.points as f32 / maximum as f32 * 663. - 20.;
        let content = threshold_reward(
            threshold,
            points >= threshold.points,
            index + 1 < thresholds.len(),
        );
        out.extend(rsx! { r#frame { name: {DynName(format!("JournalMonthlyThreshold{}", threshold.id))}, left, top: 13., width: 40., height: 40., pos_type: "absolute", {content} } });
    }
    out
}

fn bar_fill(width: f32) -> Element {
    let region = resolve_region("activities-bar-fill", thread_skin())
        .expect("MonthlyActivities bar fill atlas");
    let ui_toolkit::atlas::AtlasSource::FileDataId(fdid) = region.source else {
        panic!("MonthlyActivities bar fill must resolve to published FDID");
    };
    let right = region.left + (region.right - region.left) * width / 663.;
    let coordinates = format!("{},{right},{},{}", region.left, region.top, region.bottom);
    rsx! { texture { name: "JournalMonthlyBarFill", left: 0., top: 0., width, height: 33., pos_type: "absolute", texture_fdid: fdid, tex_coords: coordinates, } }
}

fn progress_labels(points: u32, maximum: u32) -> Element {
    let title = global("MONTHLY_ACTIVITIES_POINTS");
    let progress = global("MONTHLY_ACTIVITIES_PROGRESS_TEXT")
        .replacen("%d", &points.to_string(), 1)
        .replacen("%d", &maximum.to_string(), 1);
    let progress_width = text_size(&progress, 12.).0;
    let mut out = label(
        "JournalMonthlyPoints".into(),
        &title,
        (15., 0., text_size(&title, 12.).0, 33.),
        (12., HIGHLIGHT_FONT_COLOR, "LEFT"),
    );
    out.extend(label(
        "JournalMonthlyProgress".into(),
        &progress,
        (633. - progress_width, 0., progress_width, 33.),
        (12., HIGHLIGHT_FONT_COLOR, "RIGHT"),
    ));
    out
}

fn threshold_reward(threshold: &TradingPostThreshold, earned: bool, show_line: bool) -> Element {
    let id = threshold.id;
    let mut out = Element::default();
    if show_line {
        let atlas = if earned {
            "activities-complete-line"
        } else {
            "activities-incomplete-line"
        };
        out.extend(sized_atlas(
            format!("JournalMonthlyThreshold{id}Line"),
            atlas,
            (20., 0.),
        ));
    }
    let (atlas, center) = if earned {
        ("activities-complete-diamond", (21., 20.))
    } else {
        ("activities-incomplete-diamond", (22., 21.))
    };
    out.extend(sized_atlas(
        format!("JournalMonthlyThreshold{id}Diamond"),
        atlas,
        center,
    ));
    if earned {
        out.extend(sized_atlas(
            format!("JournalMonthlyThreshold{id}Check"),
            "activities-icon-checkmark-small",
            (27.5, 28.5),
        ));
    } else {
        let amount = threshold.tender.to_string();
        out.extend(rsx! { fontstring { name: {DynName(format!("JournalMonthlyThreshold{id}Points"))}, text: amount, left: 20., top: 22., width: 40., height: 12., pos_type: "absolute", font: GameFont::ArialNarrow, font_size: 12., font_color: HIGHLIGHT_FONT_COLOR, justify_h: "LEFT", } });
    }
    out
}
