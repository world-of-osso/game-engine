//! Retail PerksProgramShoppingCartMixin: local ordered vendor IDs, never local debits.
use super::View;
use shared::protocol::TradingPostItem;

/// ConvertInvTypeToSelectionKey: hand variants share one selection; other slots stay distinct.
fn selection_key(inventory_type: u8) -> u8 {
    match inventory_type {
        14 | 22 | 23 => 22,
        15 | 17 | 25 | 26 => 17,
        13 | 21 => 21,
        value => value,
    }
}

impl View {
    fn cart_inventory_type(&self, item_id: u32) -> Result<u8, String> {
        // Mounts, pets and toys have no wearable selection even when their item teaches one.
        if self
            .cart_items()
            .iter()
            .any(|row| row.item_id == item_id && !matches!(row.category_id, 1 | 8))
        {
            return Ok(0);
        }
        crate::item_catalog::item_catalog()
            .ok_or("Trading Post cart item catalog still loading")?
            .get(item_id)
            .map(|item| item.inventory_type)
            .ok_or_else(|| format!("Trading Post cart item{item_id} metadata unavailable"))
    }

    fn select_cart_item(&mut self, item_id: u32) -> Result<(), String> {
        let key = selection_key(self.cart_inventory_type(item_id)?);
        // Disabled wearables clear OTHER before inspecting their retained selection.
        if key != 0 && self.cart_selected.remove(&0).is_some() {
            return Ok(());
        }
        if self.cart_selected.get(&key) == Some(&item_id) {
            self.cart_selected.remove(&key);
            return Ok(());
        }
        match key {
            17 => {
                self.cart_selected.remove(&21);
                self.cart_selected.remove(&22);
            }
            21 | 22 => {
                self.cart_selected.remove(&17);
            }
            _ => {}
        }
        self.cart_selected.insert(key, item_id);
        Ok(())
    }

    fn initialize_cart_selection(&mut self) -> Result<(), String> {
        self.cart_selected.clear();
        let item_ids: Vec<_> = self
            .cart_items()
            .iter()
            .flat_map(|row| {
                if row.category_id == 8 {
                    row.preview_item_ids.clone()
                } else {
                    vec![row.item_id]
                }
            })
            .collect();
        self.select_default_cart_items(&item_ids)?;
        if self.cart_selected.is_empty()
            && let Some(id) = item_ids.first()
        {
            self.select_cart_item(*id)?;
        }
        Ok(())
    }

    /// AddItemToDataProvider defaults only unoccupied wearable slots, retaining OTHER.
    fn select_default_cart_items(&mut self, item_ids: &[u32]) -> Result<(), String> {
        for id in item_ids {
            let key = selection_key(self.cart_inventory_type(*id)?);
            let occupied = self.cart_selected.contains_key(&key)
                || key == 17
                    && (self.cart_selected.contains_key(&21)
                        || self.cart_selected.contains_key(&22));
            if key == 0 || occupied {
                continue;
            }
            if matches!(key, 21 | 22) {
                self.cart_selected.remove(&17);
            }
            self.cart_selected.insert(key, *id);
        }
        Ok(())
    }

    pub fn preview_product(&self) -> Option<&TradingPostItem> {
        if !self.cart_visible {
            return self.selected_item();
        }
        if let Some(id) = self.cart_selected.get(&0) {
            return self
                .snapshot
                .as_ref()?
                .items
                .iter()
                .find(|row| self.cart.contains(&row.id) && row.item_id == *id);
        }
        self.snapshot
            .as_ref()?
            .items
            .iter()
            .find(|row| self.cart.contains(&row.id) && matches!(row.category_id, 1 | 8))
    }

    pub fn cart_preview_item_ids(&self) -> Vec<u32> {
        self.cart_selected
            .iter()
            .filter(|(key, _)| **key != 0)
            .map(|(_, id)| *id)
            .collect()
    }

    pub fn cart_items(&self) -> Vec<&TradingPostItem> {
        let Some(snapshot) = &self.snapshot else {
            return Vec::new();
        };
        self.cart
            .iter()
            .filter_map(|id| snapshot.items.iter().find(|row| row.id == *id))
            .collect()
    }

    pub fn cart_price(&self) -> u64 {
        self.cart_items()
            .iter()
            .map(|row| u64::from(row.price))
            .sum()
    }

    pub fn can_checkout(&self) -> bool {
        !self.pending
            && !self.cart.is_empty()
            && self.cart_items().len() == self.cart.len()
            && self
                .snapshot
                .as_ref()
                .is_some_and(|snapshot| self.cart_price() <= u64::from(snapshot.tender))
    }

    pub fn can_add_to_cart(&self) -> bool {
        self.selected.is_some_and(|id| self.can_cart_product(id))
    }

    /// Shared footer/row eligibility; hover must never substitute selected product identity.
    pub fn can_cart_product(&self, id: u32) -> bool {
        let Some(snapshot) = &self.snapshot else {
            return false;
        };
        snapshot
            .items
            .iter()
            .find(|row| row.id == id)
            .is_some_and(|row| {
                !self.pending
                    && !row.purchased
                    && !row.collected
                    && row.refund_seconds == 0
                    && snapshot.frozen_item_id != Some(id)
                    && row.grant_unavailable.is_none()
            })
    }

    fn toggle_product_cart(&mut self, id: u32) -> Result<(), String> {
        if !self.can_cart_product(id) {
            return Ok(());
        }
        if self.cart.contains(&id) {
            self.remove_from_cart(id);
        } else {
            self.cart.push(id);
            if self.cart_visible {
                let row = self
                    .cart_items()
                    .into_iter()
                    .find(|row| row.id == id)
                    .expect("cartable product");
                let items = if row.category_id == 8 {
                    row.preview_item_ids.clone()
                } else {
                    vec![row.item_id]
                };
                self.select_default_cart_items(&items)?;
            }
        }
        Ok(())
    }

    pub(super) fn click_cart(&mut self, action: &str) -> Result<(), String> {
        if let Some(id) = action.strip_prefix("perks:cart:toggle:") {
            return self.toggle_product_cart(
                id.parse()
                    .map_err(|_| "Invalid Trading Post cart product")?,
            );
        }
        if let Some(id) = action.strip_prefix("perks:cart:item:") {
            let id = id.parse().map_err(|_| "Invalid Trading Post cart item")?;
            let present = self
                .cart_items()
                .iter()
                .any(|row| row.item_id == id || row.preview_item_ids.contains(&id));
            if present {
                self.select_cart_item(id)?;
            }
            return Ok(());
        }
        if let Some(id) = action.strip_prefix("perks:cart:select:") {
            let id: u32 = id.parse().map_err(|_| "Invalid Trading Post cart item")?;
            if let Some(item_id) = self
                .cart_items()
                .iter()
                .find(|row| row.id == id)
                .map(|row| row.item_id)
            {
                self.select_cart_item(item_id)?;
            }
            return Ok(());
        }
        if let Some(id) = action.strip_prefix("perks:cart:remove:") {
            self.remove_from_cart(id.parse().map_err(|_| "Invalid Trading Post cart item")?);
            return Ok(());
        }
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
            "perks:cart:show" => {
                self.initialize_cart_selection()?;
                self.cart_visible = !self.cart.is_empty();
            }
            "perks:cart:close" => self.cart_visible = false,
            "perks:cart:clear" if !self.cart.is_empty() => self.confirm = Some("clear-cart".into()),
            "perks:cart:clear" => {}
            "perks:cart:checkout" if self.can_checkout() => {
                self.cart_confirmation = Some((self.cart_price(), self.cart.len()));
                self.confirm = Some("cart-checkout".into())
            }
            "perks:cart:checkout" => {}
            _ => return Err(format!("Unknown Trading Post cart action: {action}")),
        }
        Ok(())
    }

    pub fn remove_from_cart(&mut self, id: u32) {
        let removed: Vec<_> = self
            .cart_items()
            .iter()
            .filter(|row| row.id == id)
            .flat_map(|row| {
                std::iter::once(row.item_id).chain(row.preview_item_ids.iter().copied())
            })
            .collect();
        self.cart_selected.retain(|_, item| !removed.contains(item));
        self.cart.retain(|item| *item != id);
        if self.cart.is_empty() {
            self.cart_visible = false;
        }
    }

    pub fn clear_cart(&mut self) {
        self.cart.clear();
        self.cart_selected.clear();
        self.cart_visible = false;
        self.cart_hover = None;
        self.cart_item_hover = None;
    }
}
