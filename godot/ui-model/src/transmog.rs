//! Server-authoritative Transmogrify pending state; staging never mutates equipment.
use shared::protocol::{
    ApplyTransmog, EquipmentSlot, TransmogChange, TransmogChoice, TransmogError, TransmogFailed,
    TransmogSlot, TransmogSource, TransmogState,
};
use std::collections::BTreeMap;

/// Blizzard_Wardrobe.lua:566–568: three rows of six DressUpModel cells.
pub const SOURCES_PER_PAGE: usize = 18;

#[derive(Default, Debug, Clone, PartialEq, Eq)]
pub struct TransmogBook {
    pub state: Option<TransmogState>,
    pub money: u64,
    pub pending: BTreeMap<EquipmentSlot, TransmogChoice>,
    pub selected: Option<EquipmentSlot>,
    pub applying: bool,
    pub error: String,
    pub page: usize,
}

impl TransmogBook {
    pub fn receive(&mut self, state: TransmogState) {
        let new_service = self
            .state
            .as_ref()
            .is_none_or(|previous| previous.npc != state.npc);
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
            self.page = 0;
        }
        self.applying = false;
        self.state = Some(state);
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
            self.page = 0;
        }
    }

    pub fn stage(&mut self, choice: TransmogChoice) -> bool {
        if self.applying {
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

    pub fn undo(&mut self) {
        if !self.applying {
            self.pending.clear();
            self.error.clear();
        }
    }

    pub fn close(&mut self) {
        *self = Self::default();
    }

    pub fn page_sources(&self) -> &[TransmogSource] {
        let Some(slot) = self.selected_slot() else {
            return &[];
        };
        let start = (self.page * SOURCES_PER_PAGE).min(slot.sources.len());
        let end = (start + SOURCES_PER_PAGE).min(slot.sources.len());
        &slot.sources[start..end]
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
        self.selected_slot().map_or(1, |slot| {
            slot.sources.len().div_ceil(SOURCES_PER_PAGE).max(1)
        })
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
