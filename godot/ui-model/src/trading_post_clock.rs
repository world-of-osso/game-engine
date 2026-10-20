//! Retail list countdown, driven only by authoritative server time plus monotonic elapsed.
use super::*;

impl View {
    /// Retail Program.lua396–412: elapsed since request acceptance, not snapshot receipt.
    pub fn update_purchase_timeout(&mut self, elapsed_seconds: f64) {
        const SLOW_PURCHASE_SECONDS: f64 = 10.;
        let purchase_confirmation =
            matches!(self.confirm.as_deref(), Some("purchase" | "cart-checkout"));
        let waiting = self.visible && self.pending && purchase_confirmation;
        if waiting && elapsed_seconds >= SLOW_PURCHASE_SECONDS {
            self.slow_purchase = true;
        }
    }

    pub fn month_remaining_seconds(&self) -> Option<u64> {
        self.snapshot.as_ref().map(|snapshot| {
            snapshot
                .month_end_unix
                .saturating_sub(snapshot.now_unix)
                .saturating_sub(i64::from(self.refund_elapsed))
                .max(0) as u64
        })
    }

    pub(super) fn can_sort_time(&self) -> bool {
        let month_active = self
            .month_remaining_seconds()
            .is_some_and(|seconds| seconds > 0);
        month_active && !self.rows().is_empty()
    }

    pub(super) fn time_sort_minutes(&self, row: &TradingPostItem) -> u64 {
        let remaining = self
            .month_remaining_seconds()
            .expect("Time sort rows require an authoritative snapshot");
        // Products.lua437–465: all supplied ordinary rows share this monthly duration,
        // so the pairwise maximum is remaining; compare owned sentinels only after flooring.
        const OWNED_SORT_PADDING_SECONDS: u64 = 1000;
        let owned = remaining + OWNED_SORT_PADDING_SECONDS;
        let seconds = if row.refund_seconds.saturating_sub(self.refund_elapsed) > 0 {
            owned - 1
        } else if row.purchased {
            owned
        } else {
            remaining
        };
        seconds / 60
    }

    pub(super) fn reset_time_sort(&mut self) {
        // Products.lua572–573: provider replacement returns Time to default Price.
        if self.sort == Sort::Time {
            self.sort = Sort::Price;
            self.ascending = false;
        }
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
