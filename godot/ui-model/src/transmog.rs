//! Server-authoritative Transmogrify pending state; staging never mutates equipment.
use shared::protocol::{
    ApplyTransmog, EquipmentSlot, SaveTransmogOutfit, TransmogChange, TransmogChoice,
    TransmogError, TransmogFailed, TransmogSlot, TransmogSource, TransmogState,
};
use std::collections::{BTreeMap, BTreeSet};

mod outfit_info;
pub use outfit_info::OutfitInfoDraft;
mod filters;
pub use filters::SOURCE_FILTERS;
mod wardrobe;
pub use wardrobe::{
    ItemEntry, OutfitRow, SETS_PER_PAGE, SaveKind, SetProgress, TransmogSend, WardrobeTab,
};

/// Retail12.1 TransmogItemModelTemplate100×132 in584×640 PagedContent, padding20.
/// Five columns and four rows; only visible cells own native subviewports.
pub const SOURCES_PER_PAGE: usize = 20;

#[derive(Default, Debug, Clone, PartialEq, Eq)]
pub struct TransmogBook {
    pub state: Option<TransmogState>,
    pub money: u64,
    pub pending: BTreeMap<EquipmentSlot, TransmogChoice>,
    pub selected: Option<EquipmentSlot>,
    pub applying: bool,
    pub error: String,
    pub page: usize,
    pub search: String,
    pub tab: WardrobeTab,
    pub filters_open: bool,
    pub excluded_source_types: BTreeSet<u8>,
    pub exclude_collected_sets: bool,
    pub exclude_uncollected_sets: bool,
    /// The selected weapon's illusion location is active (TransmogIllusionSlotTemplate).
    pub illusion_selected: bool,
    /// Staged TransmogIllusion IDs; None clears the slot's saved illusion.
    pub pending_illusions: BTreeMap<EquipmentSlot, Option<u32>>,
    /// Currently viewed authored outfit entry (C_TransmogOutfitInfo viewed outfit).
    pub viewed_outfit: Option<u32>,
    pub saving: Option<SaveKind>,
    pub queued_save: Option<SaveTransmogOutfit>,
    /// TRANSMOG_CUSTOM_SET_NAME popup is shown.
    pub naming_custom_set: bool,
    pub outfit_menu: Option<u32>,
    pub outfit_info: Option<OutfitInfoDraft>,
}

impl TransmogBook {
    pub fn receive(&mut self, state: TransmogState) {
        let new_service = self
            .state
            .as_ref()
            .is_none_or(|previous| previous.npc != state.npc);
        if new_service {
            self.search.clear();
            self.filters_open = false;
            self.excluded_source_types.clear();
            self.exclude_collected_sets = false;
            self.exclude_uncollected_sets = false;
            self.tab = WardrobeTab::Items;
            self.illusion_selected = false;
            self.viewed_outfit = None;
            self.pending_illusions.clear();
            self.queued_save = None;
            self.saving = None;
            self.naming_custom_set = false;
            self.outfit_menu = None;
            self.outfit_info = None;
        }
        match self.saving.take() {
            Some(SaveKind::Outfit) => {
                self.pending_illusions.clear();
                self.error.clear();
            }
            Some(SaveKind::CustomSet) => {
                self.naming_custom_set = false;
                self.error.clear();
            }
            Some(SaveKind::OutfitInfo) => self.error.clear(),
            None => {}
        }
        if new_service || self.applying {
            self.pending.clear();
            self.error.clear();
        } else if let Some(previous) = &self.state {
            self.pending.retain(|slot, choice| {
                let old = previous.slots.iter().find(|entry| entry.slot == *slot);
                let new = state.slots.iter().find(|entry| entry.slot == *slot);
                old.zip(new).is_some_and(|(old, new)| {
                    old.item_guid == new.item_guid
                        && *choice != new.applied
                        && offered(new, *choice)
                })
            });
        }
        if !state
            .slots
            .iter()
            .any(|slot| Some(slot.slot) == self.selected)
        {
            self.selected = state.slots.first().map(|slot| slot.slot);
            self.illusion_selected = false;
            self.page = 0;
        }
        self.applying = false;
        self.state = Some(state);
        if self.illusion_selected
            && !self
                .selected
                .is_some_and(|slot| self.illusion_allowed(slot))
        {
            self.illusion_selected = false;
        }
        let pending = std::mem::take(&mut self.pending_illusions);
        self.pending_illusions = pending
            .into_iter()
            .filter(|(slot, illusion)| {
                self.illusion_allowed(*slot)
                    && illusion.is_none_or(|id| self.illusion(id).is_some_and(|i| i.collected))
            })
            .collect();
        self.viewed_outfit = self.default_viewed_outfit();
        self.page = self.page.min(self.pages() - 1);
    }

    pub fn selected_slot(&self) -> Option<&TransmogSlot> {
        self.state
            .as_ref()?
            .slots
            .iter()
            .find(|slot| Some(slot.slot) == self.selected)
    }

    pub fn select(&mut self, slot: EquipmentSlot) {
        if self
            .state
            .as_ref()
            .is_some_and(|state| state.slots.iter().any(|entry| entry.slot == slot))
        {
            self.selected = Some(slot);
            self.illusion_selected = false;
            self.set_tab(WardrobeTab::Items);
            self.page = 0;
        }
    }

    pub fn stage(&mut self, choice: TransmogChoice) -> bool {
        if self.busy() || self.illusion_selected {
            return false;
        }
        let Some(slot) = self.selected_slot() else {
            return false;
        };
        if !offered(slot, choice) {
            return false;
        }
        let key = slot.slot;
        let changed = choice != slot.applied;
        if changed {
            self.pending.insert(key, choice);
        } else {
            self.pending.remove(&key);
        }
        self.error.clear();
        true
    }

    pub fn pending_cost(&self) -> Option<u64> {
        let mut total = 0u64;
        for (key, choice) in &self.pending {
            let slot = self
                .state
                .as_ref()?
                .slots
                .iter()
                .find(|slot| slot.slot == *key)?;
            let price = if *choice == TransmogChoice::Original {
                0
            } else {
                slot.cost?
            };
            total = total.checked_add(price)?;
        }
        Some(total)
    }

    pub fn can_apply(&self) -> bool {
        !self.applying
            && !self.pending.is_empty()
            && self.pending_cost().is_some_and(|cost| cost <= self.money)
    }

    pub fn apply(&mut self) -> Option<ApplyTransmog> {
        if !self.can_apply() {
            return None;
        }
        let state = self.state.as_ref()?;
        let changes = self
            .pending
            .iter()
            .map(|(slot, choice)| {
                let equipped = state
                    .slots
                    .iter()
                    .find(|entry| entry.slot == *slot)
                    .expect("pending slot exists");
                TransmogChange {
                    slot: *slot,
                    item_guid: equipped.item_guid,
                    choice: *choice,
                }
            })
            .collect();
        let request = ApplyTransmog {
            npc: state.npc,
            changes,
        };
        self.applying = true;
        Some(request)
    }

    pub fn receive_failure(&mut self, failure: TransmogFailed) {
        if !self
            .state
            .as_ref()
            .is_some_and(|state| state.npc == failure.npc)
        {
            return;
        }
        self.applying = false;
        self.queued_save = None;
        self.error = match failure.error {
            TransmogError::NotInteracting => "You are too far away.",
            TransmogError::InvalidSlot => "This item cannot be transmogrified.",
            TransmogError::ItemChanged => {
                "Your equipped item changed. Select its appearance again."
            }
            TransmogError::AppearanceNotUsable => "You cannot use that appearance.",
            TransmogError::CostUnavailable => {
                "Transmogrification cost is unavailable in local data."
            }
            TransmogError::NotEnoughMoney => "You don't have enough money.",
        }
        .into();
    }

    /// ClearAllPendingButton: ClearAllPendingTransmogs.
    pub fn undo(&mut self) {
        if !self.busy() {
            self.pending.clear();
            self.pending_illusions.clear();
            self.error.clear();
        }
    }

    pub fn close(&mut self) {
        *self = Self::default();
    }

    pub fn set_search(&mut self, search: String) {
        self.search = search;
        self.page = 0;
    }

    fn matching_sources(&self) -> impl Iterator<Item = &TransmogSource> {
        let query = self.search.trim().to_lowercase();
        self.selected_slot()
            .into_iter()
            .flat_map(|slot| &slot.sources)
            .filter(|source| !self.excluded_source_types.contains(&source.source_type))
            .filter(move |source| query.is_empty() || source.name.to_lowercase().contains(&query))
    }

    /// Appearance sources on the current Items page (without the hidden visual entry).
    pub fn page_sources(&self) -> Vec<&TransmogSource> {
        self.page_items()
            .into_iter()
            .filter_map(|entry| match entry {
                ItemEntry::Source(source) => Some(source),
                _ => None,
            })
            .collect()
    }

    /// A collection cell dresses only its offered source, not other pending choices.
    pub fn collection_equipment(
        &self,
        source_id: u32,
    ) -> Option<shared::components::EquipmentAppearance> {
        let slot = self.selected_slot()?;
        let source = slot
            .sources
            .iter()
            .find(|source| source.source_id == source_id)?;
        Some(shared::components::EquipmentAppearance {
            entries: vec![shared::components::EquippedAppearanceEntry {
                slot: slot.slot.visual_slot()?,
                item_id: Some(slot.item_id),
                definition_source: Some(slot.definition_source),
                display_info_id: Some(source.display_id),
                inventory_type: source.inventory_type,
                hidden: false,
            }],
        })
    }

    pub fn preview_equipment(
        &self,
        current: &shared::components::EquipmentAppearance,
    ) -> shared::components::EquipmentAppearance {
        let mut preview = current.clone();
        let Some(state) = &self.state else {
            return preview;
        };
        for (key, choice) in &self.pending {
            let Some(slot) = state.slots.iter().find(|slot| slot.slot == *key) else {
                continue;
            };
            let visual = slot.slot.visual_slot().expect("Transmog slot has a visual");
            let mut entry = shared::components::EquippedAppearanceEntry {
                slot: visual,
                item_id: Some(slot.item_id),
                definition_source: Some(slot.definition_source),
                display_info_id: None,
                inventory_type: slot.inventory_type,
                hidden: *choice == TransmogChoice::Hidden,
            };
            if let TransmogChoice::Source(id) = choice {
                let source = slot
                    .sources
                    .iter()
                    .find(|source| source.source_id == *id)
                    .expect("staged source was offered");
                entry.display_info_id = Some(source.display_id);
                entry.inventory_type = source.inventory_type;
            }
            preview.entries.retain(|entry| entry.slot != visual);
            preview.entries.push(entry);
        }
        preview
    }

    pub fn pages(&self) -> usize {
        let (count, per_page) = match self.tab {
            WardrobeTab::Items => (self.item_entry_count(), SOURCES_PER_PAGE),
            WardrobeTab::Sets => (self.available_sets().len(), SETS_PER_PAGE),
            WardrobeTab::CustomSets => (self.custom_sets().len(), SETS_PER_PAGE),
            WardrobeTab::Situations => (0, 1),
        };
        count.div_ceil(per_page).max(1)
    }
}

fn offered(slot: &TransmogSlot, choice: TransmogChoice) -> bool {
    match choice {
        TransmogChoice::Original => true,
        TransmogChoice::Hidden => slot.can_hide,
        TransmogChoice::Source(id) => slot.sources.iter().any(|source| source.source_id == id),
    }
}

#[cfg(test)]
#[path = "transmog_tests.rs"]
mod tests;
