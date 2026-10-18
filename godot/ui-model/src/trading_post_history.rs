//! Retail PurchasedHistoryFrame is current-item ownership/refund status, not a ledger list.
use super::*;
const HISTORY_WIDTH: f32 = 158.;
const HISTORY_HEIGHT: f32 = 39.;
const FONT_SIZE: f32 = 14.;
const GOLD: &str = "1,0.82,0,1";
const REFUND_PREFIX: &str = "Time remaining to refund this item:";

pub(super) fn render(view: &View) -> Element {
    let Some(_row) = view
        .selected_item()
        .filter(|row| row.purchased || row.collected)
    else {
        return vec![];
    };
    let seconds = view.remaining_refund_seconds();
    let children = if seconds > 0 {
        refund_status(seconds)
    } else {
        owned_status()
    };
    // RIGHT-50; BOTTOM of LeaveButton+16 in WoW coordinates (Leave's local bottom80).
    rsx! {r#frame{name:"PerksProgramPurchasedHistoryFrame",left:{WIDTH-50.0-HISTORY_WIDTH},top:{80.0-16.0-HISTORY_HEIGHT},width:HISTORY_WIDTH,height:HISTORY_HEIGHT,pos_type:"absolute",{children}}}
}

fn owned_status() -> Element {
    let mut out = label(
        "PerksProgramHistoryPurchasedText".into(),
        "You own this item",
        (-42., 0., 200., 39.),
        (FONT_SIZE, GOLD, "LEFT"),
    );
    out.extend(named_atlas_texture(
        "PerksProgramHistoryPurchasedIcon".into(),
        "perks-owned-large",
        (-42. - 12. - 38., 0., 38., 39.),
    ));
    out
}

fn refund_status(seconds: u32) -> Element {
    let (lines, final_width, line_height) = prefix_layout(REFUND_PREFIX);
    let time_top =
        -70. + (39. - lines as f32 * line_height) / 2. + (lines - 1) as f32 * line_height;
    let space = ui_toolkit::text_measure::measure_text(
        " ",
        ui_toolkit::widgets::font_string::GameFont::FrizQuadrata,
        FONT_SIZE,
    )
    .expect("history font")
    .0;
    let time_left = -62. + final_width + space;
    let time_color = if seconds > 3600 { WHITE } else { "1,0.1,0.1,1" };
    let mut out = label(
        "PerksProgramHistoryRefundPrefix".into(),
        REFUND_PREFIX,
        (-62., -70., 200., 39.),
        (FONT_SIZE, GOLD, "LEFT"),
    );
    out.extend(label(
        "PerksProgramHistoryRefundTime".into(),
        &format_time_left(seconds),
        (time_left, time_top, 200. - final_width - space, line_height),
        (FONT_SIZE, time_color, "LEFT"),
    ));
    out.extend(named_atlas_texture(
        "PerksProgramHistoryRefundIcon".into(),
        "common-icon-undo",
        (-62. - 10. - 32., -70. + (39. - 32.) / 2., 32., 32.),
    ));
    out
}

fn prefix_layout(text: &str) -> (usize, f32, f32) {
    use ui_toolkit::{text_measure::measure_text, widgets::font_string::GameFont};
    let space = measure_text(" ", GameFont::FrizQuadrata, FONT_SIZE)
        .expect("history font")
        .0;
    let line_height = measure_text(text, GameFont::FrizQuadrata, FONT_SIZE)
        .expect("history font")
        .1;
    let mut lines = 1;
    let mut used = 0.;
    for word in text.split_whitespace() {
        let width = measure_text(word, GameFont::FrizQuadrata, FONT_SIZE)
            .expect("history font")
            .0;
        if used > 0. && used + space + width > 200. {
            lines += 1;
            used = 0.;
        }
        if used > 0. {
            used += space;
        }
        used += width;
    }
    (lines, used, line_height)
}

/// TimeLeftFooterFormatter: two nonzero units, minimum minutes, floor, lowercase.
pub(super) fn format_time_left(mut seconds: u32) -> String {
    let mut units = Vec::new();
    for (duration, suffix) in [(86400, "d"), (3600, "h"), (60, "m")] {
        let quantity = seconds / duration;
        if quantity > 0 {
            units.push(format!("{quantity}{suffix}"));
            if units.len() == 2 {
                break;
            }
        }
        seconds %= duration;
    }
    if units.is_empty() {
        "0m".into()
    } else {
        units.join(" ")
    }
}
