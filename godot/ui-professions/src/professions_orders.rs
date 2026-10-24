//! Fixed-output Retail orders; escrow, visibility and action eligibility are server-owned.
use game_engine_ui_model::bag_data::InventoryState;
use shared::protocol::*;
use std::collections::BTreeMap;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum OrderMenu {
    Recipient,
    Duration,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CraftingOrders {
    pub npc: Option<u64>,
    pub menu: Option<OrderMenu>,
    pub snapshot: Option<CraftingOrdersSnapshot>,
    pub customer: bool,
    pub creating: bool,
    pub kind: CraftingOrderKind,
    pub selected: Option<u64>,
    pub recipe: Option<u32>,
    pub recipient: String,
    /// Copper, matching the protocol money unit.
    pub commission: String,
    pub duration_hours: u8,
    pub note: String,
    pub supplied: BTreeMap<u32, u32>,
    pub pending: bool,
    pub status: String,
}
impl Default for CraftingOrders {
    fn default() -> Self {
        Self {
            npc: None,
            menu: None,
            snapshot: None,
            customer: false,
            creating: false,
            kind: CraftingOrderKind::Public,
            selected: None,
            recipe: None,
            recipient: String::new(),
            commission: "0".into(),
            duration_hours: 24,
            note: String::new(),
            supplied: BTreeMap::new(),
            pending: false,
            status: String::new(),
        }
    }
}
impl CraftingOrders {
    pub fn open_customer(&mut self, npc: u64) -> CraftingOrderRequest {
        *self = Self {
            npc: Some(npc),
            customer: true,
            creating: true,
            pending: true,
            ..Default::default()
        };
        CraftingOrderRequest::Query {
            skill_line: 0,
            kind: CraftingOrderKind::Public,
            mine: true,
        }
    }
    pub fn close_for(&mut self, npc: u64) -> bool {
        if self.npc != Some(npc) {
            return false;
        }
        *self = Self::default();
        true
    }
    pub fn selected_order(&self) -> Option<&CraftingOrderSnapshot> {
        self.snapshot
            .as_ref()?
            .orders
            .iter()
            .find(|o| Some(o.id) == self.selected)
    }
    pub fn selected_recipe(&self) -> Option<&CraftingOrderRecipeSnapshot> {
        self.snapshot
            .as_ref()?
            .recipes
            .iter()
            .find(|recipe| Some(recipe.spell_id) == self.recipe)
    }
    pub fn place_request(
        &mut self,
        recipe: &CraftingOrderRecipeSnapshot,
        bags: &InventoryState,
    ) -> Result<CraftingOrderRequest, String> {
        let request = self.validate_place(recipe, bags)?;
        self.pending = true;
        Ok(request)
    }
    pub fn validate_place(
        &self,
        recipe: &CraftingOrderRecipeSnapshot,
        bags: &InventoryState,
    ) -> Result<CraftingOrderRequest, String> {
        if !self.customer || self.pending {
            return Err("An order request is already pending or this is the crafter view.".into());
        }
        if self.recipe != Some(recipe.spell_id) {
            return Err("Select an order recipe.".into());
        }
        let recipient = if self.kind == CraftingOrderKind::Personal {
            let name = self.recipient.trim();
            if name.is_empty() {
                return Err("Enter the crafter's character name.".into());
            }
            Some(name.to_owned())
        } else {
            None
        };
        let commission = self
            .commission
            .trim()
            .parse::<u64>()
            .map_err(|_| "Enter a whole, non-negative commission in copper.")?;
        if ![12, 24, 48].contains(&self.duration_hours) {
            return Err("Choose 12, 24 or 48 hours.".into());
        }
        let reagents = recipe
            .reagents
            .iter()
            .map(|requirement| {
                let item_id = requirement.item_id;
                let quantity = requirement.quantity;
                let supplied = self.supplied.get(&item_id).copied().unwrap_or(0);
                let owned = owned_count(bags, item_id);
                if supplied > quantity || supplied > owned {
                    return Err(format!(
                        "Not enough supplied reagent {item_id}: need {supplied}, own {owned}."
                    ));
                }
                Ok(CraftingOrderReagent {
                    item_id,
                    quantity,
                    supplied,
                })
            })
            .collect::<Result<Vec<_>, String>>()?;
        Ok(CraftingOrderRequest::Place {
            spell_id: recipe.spell_id,
            kind: self.kind,
            recipient,
            commission,
            reagents,
            duration_hours: self.duration_hours,
            note: self.note.clone(),
        })
    }
    pub fn action_request(&mut self, action: &str) -> Option<CraftingOrderRequest> {
        if self.pending {
            return None;
        }
        let order = self.selected_order()?;
        let request = match action {
            "claim" if order.can_claim => CraftingOrderRequest::Claim { order_id: order.id },
            "fulfill" if order.can_fulfill => CraftingOrderRequest::Fulfill { order_id: order.id },
            "release" if order.can_release => CraftingOrderRequest::Release { order_id: order.id },
            "cancel" if order.can_cancel => CraftingOrderRequest::Cancel { order_id: order.id },
            _ => return None,
        };
        self.pending = true;
        Some(request)
    }
    pub fn receive(&mut self, update: CraftingOrderStateUpdate) {
        self.pending = false;
        self.status = update.error.or(update.message).unwrap_or_default();
        if let Some(snapshot) = update.snapshot {
            self.snapshot = Some(snapshot);
            if self.selected_order().is_none() {
                self.selected = None;
            }
        }
    }
}
pub fn owned_count(bags: &InventoryState, item: u32) -> u32 {
    bags.slots
        .iter()
        .flatten()
        .filter(|slot| slot.item_id == item)
        .fold(0u32, |total, slot| total.saturating_add(slot.count))
}
#[cfg(test)]
#[path = "professions_orders_tests.rs"]
mod tests;
