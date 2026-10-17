//! PerksProgramElements.lua button ShowTooltip branches; GlobalStrings70391 labels.
use super::View;
use crate::game_tooltip::GameTooltip;
use crate::tooltip_presentation::{TOOLTIP_DESCRIPTION_COLOR, TOOLTIP_WHITE, TooltipPresentation};

impl View {
    /// Elements.lua123–137: refund time belongs to the hovered row, not the preview.
    pub fn refund_icon_tooltip(&self, vendor_id: u32) -> Option<GameTooltip> {
        if !self.visible {
            return None;
        }
        let row = self
            .snapshot
            .as_ref()?
            .items
            .iter()
            .find(|row| row.id == vendor_id)?;
        let seconds = row.refund_seconds.saturating_sub(self.refund_elapsed);
        if seconds == 0 {
            return None;
        }
        let time_color = if seconds > 3600 {
            TOOLTIP_WHITE
        } else {
            [1.0, 0.1, 0.1, 1.0]
        };
        Some(GameTooltip::new(
            TooltipPresentation {
                title: "Time remaining to refund this item: ".into(),
                title_color: TOOLTIP_WHITE,
                title_suffix: Some((super::history::format_time_left(seconds), time_color)),
                ..TooltipPresentation::hidden()
            },
            None,
        ))
    }

    /// Actual expanded cart owner, not the product selected in the model scene.
    pub fn cart_item_tooltip(
        &self,
        owner: &str,
        player_level: Option<u16>,
    ) -> Result<Option<GameTooltip>, String> {
        use crate::game_tooltip::item::{item_game_tooltip, named_item_for, without_sell_price};
        if !self.cart_visible {
            return Ok(None);
        }
        let Some(row) = super::cart_rows::expand(self)?
            .into_iter()
            .find(|row| !row.header && row.name == owner)
        else {
            return Ok(None);
        };
        let item = named_item_for(
            shared::item_data::ItemDefinitionSource::Retail,
            row.item.item_id,
            &row.item.name,
            row.item.quality,
            1,
            0,
        );
        Ok(Some(without_sell_price(item_game_tooltip(
            &item,
            player_level,
        ))))
    }

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
            _ => {
                let id = name.strip_prefix("PerksProgramCartToggle")?.parse().ok()?;
                if !self.can_cart_product(id) {
                    return None;
                }
                let title = if self.cart.contains(&id) {
                    "Remove from Cart"
                } else {
                    "Add to Cart"
                };
                (title, TOOLTIP_WHITE)
            }
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
