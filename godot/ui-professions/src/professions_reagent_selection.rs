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
        let spell_id = self.spell_id?;
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
            spell_id,
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

/// Passive native profession status, independent of Godot scene ownership.
pub fn craft_status(error: &str, selection: &ReagentSelection) -> String {
    if !error.is_empty() {
        return error.into();
    }
    if selection.pending {
        return "Validating reagents...".into();
    }
    if let Some(response) = &selection.response {
        if let Some(error) = &response.validation_error {
            return error.clone();
        }
    }
    // Missing operation composition is a diagnostic, not a passive player status.
    String::new()
}

pub fn selectable(slot: &ReagentSlot) -> bool {
    let basic_unconditional = slot.reagent_type == 1 && slot.player_condition == 0;
    // Professions.lua745-752: one basic reagent is Fixed, not Quality input mode.
    let fits_quality_dialog = (2..=3).contains(&slot.choices.len());
    let item_choices = slot
        .choices
        .iter()
        .all(|choice| choice.item_id != 0 && choice.currency_id == 0);
    basic_unconditional && fits_quality_dialog && item_choices
}

/// ReagentSlot.lua267-306: allocations own the glyph; inventory describes it only
/// while no quality allocation exists. This never assigns a crafted-item quality.
pub fn reagent_quality_atlas<'a>(
    slot: &ReagentSlot,
    allocations: &[ReagentAllocation],
    owned: &BTreeMap<u32, u32>,
    qualities: &'a BTreeMap<u32, game_engine_ui_model::professions_quality::CraftingQuality>,
) -> Option<&'a str> {
    let allocated = allocations
        .iter()
        .filter(|allocation| allocation.slot_index == slot.slot_index && allocation.quantity > 0)
        .filter_map(|allocation| qualities.get(&allocation.item_id));
    if let Some(icon) = choose_quality_atlas(allocated) {
        return Some(icon);
    }
    let inventory = slot
        .choices
        .iter()
        .filter(|choice| owned.get(&choice.item_id).copied().unwrap_or(0) > 0)
        .filter_map(|choice| qualities.get(&choice.item_id));
    choose_quality_atlas(inventory)
}

fn choose_quality_atlas<'a>(
    mut qualities: impl Iterator<Item = &'a game_engine_ui_model::professions_quality::CraftingQuality>,
) -> Option<&'a str> {
    let first = qualities.next()?;
    Some(match qualities.next() {
        Some(second) => second.mixed_inventory_atlas.as_str(),
        None => first.inventory_atlas.as_str(),
    })
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
    fn professions_reagent_glyph_uses_allocated_tiers_or_actual_stock_never_a_default_tier() {
        let dir =
            std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../data/db2/active");
        let catalog = game_engine_ui_model::professions_catalog::RecipeCatalog::load(&dir).unwrap();
        let slot = &catalog.reagent_slots[&375188][1];
        let owned = [(193208, 6), (193210, 4)].into();
        let qualities = &catalog.crafting_quality;
        assert_eq!(
            reagent_quality_atlas(slot, &[], &owned, qualities),
            Some("Professions-Icon-Quality-Mixed-Inv")
        );
        let selected = [ReagentAllocation {
            slot_index: 1,
            item_id: 193210,
            quantity: 4,
        }];
        assert_eq!(
            reagent_quality_atlas(slot, &selected, &owned, qualities),
            Some("Professions-Icon-Quality-Tier2-Inv")
        );
        let mixed = [
            selected[0].clone(),
            ReagentAllocation {
                slot_index: 1,
                item_id: 193208,
                quantity: 6,
            },
        ];
        assert_eq!(
            reagent_quality_atlas(slot, &mixed, &owned, qualities),
            Some("Professions-Icon-Quality-Mixed-Inv")
        );
        assert_eq!(
            reagent_quality_atlas(slot, &[], &BTreeMap::new(), qualities),
            None
        );
        assert_eq!(
            qualities[&244634].mixed_inventory_atlas,
            "Professions-Icon-Quality-12-Mixed-Inv"
        );
    }

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
