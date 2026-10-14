//! Retail PerksProgramShoppingCartMixin: local ordered vendor IDs, never local debits.
use super::View;
use shared::protocol::TradingPostItem;

impl View {
    pub fn cart_items(&self) -> Vec<&TradingPostItem> {
        let Some(snapshot) = &self.snapshot else {
            return Vec::new();
        };
        self.cart
            .iter()
            .filter_map(|id| snapshot.items.iter().find(|row| row.id == *id))
            .collect()
    }

    pub fn can_add_to_cart(&self) -> bool {
        let frozen = self
            .snapshot
            .as_ref()
            .and_then(|snapshot| snapshot.frozen_item_id);
        self.selected_item().is_some_and(|row| {
            !self.pending
                && !row.purchased
                && !row.collected
                && row.refund_seconds == 0
                && frozen != Some(row.id)
                && row.grant_unavailable.is_none()
        })
    }

    pub(super) fn click_cart(&mut self, action: &str) -> Result<(), String> {
        match action {
            "perks:cart:add" if self.can_add_to_cart() => {
                let id = self.selected.expect("cartable selection");
                if !self.cart.contains(&id) {
                    self.cart.push(id);
                }
            }
            "perks:cart:add" => {}
            "perks:cart:remove" => {
                if let Some(id) = self.selected {
                    self.remove_from_cart(id);
                }
            }
            "perks:cart:show" => self.cart_visible = !self.cart.is_empty(),
            "perks:cart:close" => self.cart_visible = false,
            "perks:cart:clear" if !self.cart.is_empty() => self.confirm = Some("clear-cart".into()),
            "perks:cart:clear" => {}
            _ => return Err(format!("Unknown Trading Post cart action: {action}")),
        }
        Ok(())
    }

    pub fn remove_from_cart(&mut self, id: u32) {
        self.cart.retain(|item| *item != id);
        if self.cart.is_empty() {
            self.cart_visible = false;
        }
    }

    pub fn clear_cart(&mut self) {
        self.cart.clear();
        self.cart_visible = false;
    }
}
