//! Retail list countdown, driven only by authoritative server time plus monotonic elapsed.
use super::*;

impl View {
    pub fn month_remaining_seconds(&self) -> Option<u64> {
        self.snapshot.as_ref().map(|snapshot| {
            snapshot
                .month_end_unix
                .saturating_sub(snapshot.now_unix)
                .saturating_sub(i64::from(self.refund_elapsed))
                .max(0) as u64
        })
    }

    /// The native adapter sends the returned request once per expired month boundary.
    pub fn advance_server_clock(&mut self, elapsed_seconds: u32) -> Option<TradingPostRequest> {
        self.refund_elapsed = elapsed_seconds;
        if !self.visible || self.month_remaining_seconds()? > 0 {
            return None;
        }
        let boundary = self.snapshot.as_ref()?.month_end_unix;
        if self.clock_rollover_requested_for == Some(boundary) {
            return None;
        }
        self.clock_rollover_requested_for = Some(boundary);
        Some(TradingPostRequest::Query)
    }
}

pub(super) fn row_time(view: &View, row: &TradingPostItem) -> Element {
    // Elements.lua291–328: the cart toggle replaces time only while cartable-hovered.
    if view.product_hover == Some(row.id) && view.can_cart_product(row.id) {
        return vec![];
    }
    let Some(seconds) = view.month_remaining_seconds() else {
        return vec![];
    };
    let (text, color) = if row.purchased || seconds == 0 {
        // GlobalStrings51514/51986: purchased/zero-duration gray "--".
        ("--".into(), "0.5019608,0.5019608,0.5019608,1")
    } else {
        let color = if seconds > 3600 { WHITE } else { "1,0.1,0.1,1" };
        (format_list_time(seconds), color)
    };
    // Elements.xml340–345: LEFT of PriceIcon.RIGHT+30,42x38; row center y24.
    label(
        format!("PerksProgramTimeRemaining{}", row.id),
        &text,
        (440., 5., 42., 38.),
        (16., color, "CENTER"),
    )
}

/// TimeLeftListFormatter: one abbreviated unit, floor, minimum minutes.
fn format_list_time(seconds: u64) -> String {
    for (duration, suffix) in [(86400, "d"), (3600, "h"), (60, "m")] {
        if seconds >= duration {
            return format!("{}{suffix}", seconds / duration);
        }
    }
    "0m".into()
}
