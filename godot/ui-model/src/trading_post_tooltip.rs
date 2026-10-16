//! PerksProgramElements.lua button ShowTooltip branches; GlobalStrings70391 labels.
use super::View;
use crate::game_tooltip::GameTooltip;
use crate::tooltip_presentation::{TOOLTIP_DESCRIPTION_COLOR, TOOLTIP_WHITE, TooltipPresentation};

impl View {
    /// Content only; the native hovered button supplies source ANCHOR_LEFT ownership.
    pub fn button_tooltip(&self, name: &str, enabled: bool) -> Option<GameTooltip> {
        let (title, color) = match (name, enabled) {
            ("PerksProgramPurchaseButton", _) => {
                self.purchase_hint(enabled, u64::from(self.selected_item()?.price), "Purchase")
            }
            ("PerksProgramCartPurchaseButton", _) => {
                self.purchase_hint(enabled, self.cart_price(), "Purchase Cart")
            }
            ("PerksProgramViewCartButton", _) => ("View Cart", TOOLTIP_WHITE),
            ("PerksProgramRefundButton", false) => ("Refund Unavailable", TOOLTIP_WHITE),
            ("PerksProgramCartClearButton", true) => ("Clear Cart", TOOLTIP_WHITE),
            _ => return None,
        };
        Some(GameTooltip::new(
            TooltipPresentation {
                title: title.into(),
                title_color: color,
                ..TooltipPresentation::hidden()
            },
            None,
        ))
    }

    fn purchase_hint<'a>(
        &self,
        enabled: bool,
        price: u64,
        available: &'a str,
    ) -> (&'a str, [f32; 4]) {
        if enabled {
            return (available, TOOLTIP_WHITE);
        }
        if self
            .snapshot
            .as_ref()
            .is_some_and(|snapshot| u64::from(snapshot.tender) < price)
        {
            return (
                "You don't have enough currency for this purchase",
                TOOLTIP_DESCRIPTION_COLOR,
            );
        }
        ("Purchasing Unavailable", TOOLTIP_WHITE)
    }
}
