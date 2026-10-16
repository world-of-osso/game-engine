//! Retail quality dialog allocation. Acceptance is a draft until server validation;
//! it never consumes reagents or assigns crafted quality.
use shared::crafting_reagents::{ReagentAllocation, ReagentSlot};
use shared::protocol::{CraftingOperationRequest, CraftingOperationStateUpdate};
use std::collections::BTreeMap;

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct ReagentSelection {
    pub spell_id: Option<u32>,
    pub allocations: Vec<ReagentAllocation>,
    pub dialog: Option<QualityDialog>,
    pub pending: bool,
    pub response: Option<CraftingOperationStateUpdate>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct QualityDialog {
    pub slot_index: u32,
    pub allocations: Vec<ReagentAllocation>,
}

impl ReagentSelection {
    pub fn select_recipe(&mut self, spell_id: Option<u32>) {
        if self.spell_id != spell_id {
            *self = Self {
                spell_id,
                ..Default::default()
            };
        }
    }

    pub fn open(&mut self, slot: &ReagentSlot) {
        if self.pending || !selectable(slot) {
            return;
        }
        self.dialog = Some(QualityDialog {
            slot_index: slot.slot_index,
            allocations: self
                .allocations
                .iter()
                .filter(|a| a.slot_index == slot.slot_index)
                .cloned()
                .collect(),
        });
    }

    pub fn accept(&mut self, slot: &ReagentSlot) -> Option<CraftingOperationRequest> {
        let dialog = self.dialog.as_ref()?;
        if self.pending || dialog.slot_index != slot.slot_index || dialog.total() != slot.quantity {
            return None;
        }
        self.allocations.retain(|a| a.slot_index != slot.slot_index);
        self.allocations.extend(dialog.allocations.clone());
        self.dialog = None;
        self.pending = true;
        self.response = None;
        Some(CraftingOperationRequest {
            spell_id: self.spell_id?,
            allocations: self.allocations.clone(),
        })
    }

    pub fn receive(&mut self, response: CraftingOperationStateUpdate) {
        if self.spell_id == Some(response.spell_id) {
            self.pending = false;
            self.response = Some(response);
        }
    }
}

pub fn selectable(slot: &ReagentSlot) -> bool {
    slot.reagent_type == 1
        && slot.player_condition == 0
        && (1..=3).contains(&slot.choices.len())
        && slot
            .choices
            .iter()
            .all(|choice| choice.item_id != 0 && choice.currency_id == 0)
}

impl QualityDialog {
    pub fn quantity(&self, item: u32) -> u32 {
        self.allocations
            .iter()
            .find(|a| a.item_id == item)
            .map_or(0, |a| a.quantity)
    }

    pub fn total(&self) -> u32 {
        self.allocations.iter().map(|a| a.quantity).sum()
    }

    /// QualityDialog.lua18-51: clamp to owned/required; remove overflow from the
    /// other tiers in authored order, preserving the tier the user just changed.
    pub fn allocate(
        &mut self,
        slot: &ReagentSlot,
        item: u32,
        requested: u32,
        owned: &BTreeMap<u32, u32>,
    ) {
        if self.slot_index != slot.slot_index
            || !slot.choices.iter().any(|choice| choice.item_id == item)
        {
            return;
        }
        let quantity = requested
            .min(slot.quantity)
            .min(owned.get(&item).copied().unwrap_or(0));
        self.set_quantity(item, quantity);
        let mut overflow = self.total().saturating_sub(slot.quantity);
        for choice in &slot.choices {
            if choice.item_id != item {
                let old = self.quantity(choice.item_id);
                let removed = overflow.min(old);
                self.set_quantity(choice.item_id, old - removed);
                overflow -= removed;
            }
        }
    }

    fn set_quantity(&mut self, item: u32, quantity: u32) {
        self.allocations.retain(|a| a.item_id != item);
        if quantity > 0 {
            self.allocations.push(ReagentAllocation {
                slot_index: self.slot_index,
                item_id: item,
                quantity,
            });
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn quality_dialog_preserves_new_tier_and_cancel_does_not_commit() {
        let catalog = shared::crafting_reagents::CraftingReagentCatalog::load(
            &std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../data/db2/active"),
        )
        .unwrap();
        let slot = &catalog.recipes[&375188][1];
        let owned = [(193208, 10), (193210, 6), (193211, 3)].into();
        let mut state = ReagentSelection {
            spell_id: Some(375188),
            ..Default::default()
        };
        state.open(slot);
        let dialog = state.dialog.as_mut().unwrap();
        dialog.allocate(slot, 193208, 10, &owned);
        dialog.allocate(slot, 193210, 6, &owned);
        assert_eq!((dialog.quantity(193208), dialog.quantity(193210)), (4, 6));
        dialog.allocate(slot, 193211, 20, &owned);
        assert_eq!(
            (
                dialog.quantity(193208),
                dialog.quantity(193210),
                dialog.quantity(193211)
            ),
            (1, 6, 3)
        );
        assert!(state.allocations.is_empty());
        state.dialog = None;
        assert!(state.allocations.is_empty());
        state.open(slot);
        state
            .dialog
            .as_mut()
            .unwrap()
            .allocate(slot, 193208, 10, &owned);
        let request = state.accept(slot).unwrap();
        assert_eq!(
            request.allocations,
            [ReagentAllocation {
                slot_index: 1,
                item_id: 193208,
                quantity: 10
            }]
        );
        assert!(state.pending);
        state.select_recipe(Some(375110));
        assert!(state.allocations.is_empty());
        assert!(!state.pending);
    }
}
