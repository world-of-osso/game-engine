//! Retail12.1 Sets, Custom Sets, outfits and illusions over the authoritative TransmogState.
//! Blizzard_Transmog.lua: SaveOutfitButton calls CommitAndApplyAllPending, so saving pending
//! appearances goes through the unchanged paid ApplyTransmog and commits the outfit afterwards.
use super::*;
use shared::protocol::{
    SaveTransmogCustomSet, SaveTransmogOutfit, TransmogCollectionState, TransmogCustomSet,
    TransmogIllusion, TransmogOutfit, TransmogOutfitChoice, TransmogOutfitError,
    TransmogOutfitFailed, TransmogOutfitIllusion, TransmogSet,
};

/// TransmogSetBaseModelTemplate178×218 with xPadding27/yPadding19 in the592×741 PagedContent.
pub const SETS_PER_PAGE: usize = 9;
/// Server outfits.rs: TransmogOutfitEntry.Source AutomaticallyAwarded, Flags AwardOnLogin.
const AUTO_AWARDED: u8 = 1;
const AWARD_ON_LOGIN: u32 = 1;
/// TransmogOutfitSlotInfo.Flags bit for slots that carry an illusion location.
const ALLOWS_ILLUSION: u32 = 2;

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum WardrobeTab {
    #[default]
    Items,
    Sets,
    CustomSets,
    Situations,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SaveKind {
    Outfit,
    OutfitInfo,
    CustomSet,
}

/// Messages the host sends for wardrobe actions.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum TransmogSend {
    Apply(ApplyTransmog),
    SaveOutfit(SaveTransmogOutfit),
    SaveCustomSet(SaveTransmogCustomSet),
}

/// One OutfitList row: an authored outfit entry and its saved player name, if any.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct OutfitRow {
    pub id: u32,
    pub name: String,
    pub icon_fdid: u32,
}

/// C_TransmogSets set source counts: collected primary appearances of all of them.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct SetProgress {
    pub collected: usize,
    pub total: usize,
}

/// Items-tab grid entry: the hidden visual is listed first (SetCollectionEntries isHideVisual).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ItemEntry<'a> {
    Hidden,
    Source(&'a TransmogSource),
    Illusion(&'a TransmogIllusion),
}

impl TransmogBook {
    pub fn set_tab(&mut self, tab: WardrobeTab) {
        if tab != self.tab {
            self.tab = tab;
            self.filters_open = false;
            self.page = 0;
            self.search.clear();
        }
    }

    pub fn outfit_rows(&self) -> Vec<OutfitRow> {
        let Some(state) = &self.state else {
            return Vec::new();
        };
        let mut entries: Vec<_> = state
            .outfit_entries
            .iter()
            .filter(|entry| {
                entry.source == AUTO_AWARDED && entry.flags & AWARD_ON_LOGIN != 0 && entry.cost == 0
            })
            .collect();
        entries.sort_by_key(|entry| entry.order_index);
        entries
            .into_iter()
            .map(|entry| OutfitRow {
                id: entry.id,
                icon_fdid: state
                    .outfits
                    .iter()
                    .find(|outfit| outfit.id == entry.id)
                    .map_or(0, |outfit| outfit.icon_fdid),
                name: state
                    .outfits
                    .iter()
                    .find(|outfit| outfit.id == entry.id)
                    .map_or_else(|| entry.name.clone(), |outfit| outfit.name.clone()),
            })
            .collect()
    }

    pub(super) fn default_viewed_outfit(&self) -> Option<u32> {
        let rows = self.outfit_rows();
        self.viewed_outfit
            .filter(|id| rows.iter().any(|row| row.id == *id))
            .or_else(|| rows.first().map(|row| row.id))
    }

    fn viewed_record(&self) -> Option<&TransmogOutfit> {
        let id = self.viewed_outfit?;
        self.state
            .as_ref()?
            .outfits
            .iter()
            .find(|outfit| outfit.id == id)
    }

    /// ChangeViewedOutfit: the outfit's saved appearances become pending against the
    /// equipped items; its saved illusions are its current illusion state.
    pub fn view_outfit(&mut self, id: u32) -> bool {
        if self.busy() || !self.outfit_rows().iter().any(|row| row.id == id) {
            return false;
        }
        self.viewed_outfit = Some(id);
        self.pending.clear();
        self.pending_illusions.clear();
        self.error.clear();
        let choices = self
            .viewed_record()
            .map(|outfit| outfit.choices.clone())
            .unwrap_or_default();
        self.stage_choices(&choices);
        true
    }

    fn stage_choices(&mut self, choices: &[TransmogOutfitChoice]) {
        let Some(state) = &self.state else {
            return;
        };
        for choice in choices {
            let Some(slot) = state.slots.iter().find(|slot| slot.slot == choice.slot) else {
                continue;
            };
            if !offered(slot, choice.choice) {
                continue;
            }
            if choice.choice == slot.applied {
                self.pending.remove(&slot.slot);
            } else {
                self.pending.insert(slot.slot, choice.choice);
            }
        }
    }

    pub fn busy(&self) -> bool {
        self.applying || self.saving.is_some() || self.queued_save.is_some()
    }

    pub fn effective_choice(&self, slot: &TransmogSlot) -> TransmogChoice {
        self.pending
            .get(&slot.slot)
            .copied()
            .unwrap_or(slot.applied)
    }

    pub fn saved_illusion(&self, slot: EquipmentSlot) -> Option<u32> {
        self.viewed_record()?
            .illusions
            .iter()
            .find(|entry| entry.slot == slot)
            .map(|entry| entry.illusion_id)
    }

    pub fn effective_illusion(&self, slot: EquipmentSlot) -> Option<u32> {
        self.pending_illusions
            .get(&slot)
            .copied()
            .unwrap_or_else(|| self.saved_illusion(slot))
    }

    pub fn illusion(&self, id: u32) -> Option<&TransmogIllusion> {
        self.state
            .as_ref()?
            .illusions
            .iter()
            .find(|entry| entry.id == id)
    }

    pub fn illusion_allowed(&self, slot: EquipmentSlot) -> bool {
        self.state.as_ref().is_some_and(|state| {
            state.slots.iter().any(|entry| entry.slot == slot)
                && state
                    .outfit_slots
                    .iter()
                    .any(|entry| entry.slot == slot && entry.flags & ALLOWS_ILLUSION != 0)
        })
    }

    pub fn select_illusion(&mut self, slot: EquipmentSlot) {
        if self.illusion_allowed(slot) {
            self.selected = Some(slot);
            self.illusion_selected = true;
            self.set_tab(WardrobeTab::Items);
            self.page = 0;
        }
    }

    /// Stage an illusion (or none, the "Show Equipped Gear" display type) on the selected
    /// weapon. Only collected illusions are listed (SetCollectionEntries isCollected).
    pub fn stage_illusion(&mut self, illusion: Option<u32>) -> bool {
        let Some(slot) = self.selected.filter(|_| self.illusion_selected) else {
            return false;
        };
        if self.busy()
            || !self.illusion_allowed(slot)
            || illusion.is_some_and(|id| !self.illusion(id).is_some_and(|entry| entry.collected))
        {
            return false;
        }
        if illusion == self.saved_illusion(slot) {
            self.pending_illusions.remove(&slot);
        } else {
            self.pending_illusions.insert(slot, illusion);
        }
        self.error.clear();
        true
    }

    pub(super) fn matching_illusions(&self) -> impl Iterator<Item = &TransmogIllusion> {
        let query = self.search.trim().to_lowercase();
        self.state
            .iter()
            .flat_map(|state| &state.illusions)
            .filter(move |entry| {
                entry.collected && (query.is_empty() || entry.name.to_lowercase().contains(&query))
            })
    }

    pub fn page_items(&self) -> Vec<ItemEntry<'_>> {
        let entries: Box<dyn Iterator<Item = ItemEntry<'_>>> = if self.illusion_selected {
            Box::new(self.matching_illusions().map(ItemEntry::Illusion))
        } else {
            let hidden = self.selected_slot().is_some_and(|slot| slot.can_hide)
                && self.search.trim().is_empty();
            Box::new(
                hidden
                    .then_some(ItemEntry::Hidden)
                    .into_iter()
                    .chain(self.matching_sources().map(ItemEntry::Source)),
            )
        };
        entries
            .skip(self.page * SOURCES_PER_PAGE)
            .take(SOURCES_PER_PAGE)
            .collect()
    }

    pub(super) fn item_entry_count(&self) -> usize {
        if self.illusion_selected {
            return self.matching_illusions().count();
        }
        let hidden =
            self.selected_slot().is_some_and(|slot| slot.can_hide) && self.search.trim().is_empty();
        usize::from(hidden) + self.matching_sources().count()
    }

    pub fn pending_count(&self) -> usize {
        self.pending.len() + self.pending_illusions.len()
    }

    /// SaveOutfitButton:SetEnabled(canApply): pending changes whose quote is affordable.
    pub fn can_save_outfit(&self) -> bool {
        !self.busy()
            && self.viewed_outfit.is_some()
            && self.pending_count() > 0
            && self.pending_cost().is_some_and(|cost| cost <= self.money)
    }

    pub(super) fn outfit_snapshot(&self) -> Option<TransmogOutfit> {
        let state = self.state.as_ref()?;
        let id = self.viewed_outfit?;
        let name = self
            .outfit_rows()
            .into_iter()
            .find(|row| row.id == id)?
            .name;
        let choices = state
            .slots
            .iter()
            .filter_map(|slot| {
                let choice = self.effective_choice(slot);
                offered(slot, choice).then_some(TransmogOutfitChoice {
                    slot: slot.slot,
                    choice,
                })
            })
            .collect();
        let illusions = state
            .slots
            .iter()
            .filter_map(|slot| {
                Some(TransmogOutfitIllusion {
                    slot: slot.slot,
                    illusion_id: self.effective_illusion(slot.slot)?,
                })
            })
            .collect();
        Some(TransmogOutfit {
            id,
            name,
            icon_fdid: self.viewed_record().map_or(0, |outfit| outfit.icon_fdid),
            choices,
            illusions,
        })
    }

    /// CommitAndApplyAllPending: pending appearances use the existing paid ApplyTransmog;
    /// the outfit record is committed only after that application succeeds.
    pub fn save_outfit(&mut self) -> Option<TransmogSend> {
        if !self.can_save_outfit() {
            return None;
        }
        let npc = self.state.as_ref()?.npc;
        let outfit = self.outfit_snapshot()?;
        let save = SaveTransmogOutfit { npc, outfit };
        if self.pending.is_empty() {
            self.saving = Some(SaveKind::Outfit);
            return Some(TransmogSend::SaveOutfit(save));
        }
        let apply = self.apply()?;
        self.queued_save = Some(save);
        Some(TransmogSend::Apply(apply))
    }

    /// The outfit commit queued behind a paid Apply, once the server confirmed it.
    pub fn take_queued_save(&mut self) -> Option<SaveTransmogOutfit> {
        if self.applying {
            return None;
        }
        let save = self.queued_save.take()?;
        self.saving = Some(SaveKind::Outfit);
        Some(save)
    }

    pub fn receive_outfit_failure(&mut self, failure: TransmogOutfitFailed) {
        if !self
            .state
            .as_ref()
            .is_some_and(|state| state.npc == failure.npc)
        {
            return;
        }
        self.saving = None;
        self.queued_save = None;
        self.error = match failure.error {
            TransmogOutfitError::NotInteracting => "You are too far away.",
            // TRANSMOG_OUTFIT_INVALID_NAME
            TransmogOutfitError::InvalidName => {
                "That name is unavailable. Please choose a different name and try again."
            }
            TransmogOutfitError::EntryUnavailable => "That outfit slot is unavailable.",
            TransmogOutfitError::InvalidChoice => "You cannot use that appearance.",
        }
        .into();
    }

    pub fn custom_set_choices(&self) -> Vec<TransmogOutfitChoice> {
        self.state
            .iter()
            .flat_map(|state| &state.slots)
            .filter_map(|slot| {
                let choice = self.effective_choice(slot);
                (choice != TransmogChoice::Original && offered(slot, choice)).then_some(
                    TransmogOutfitChoice {
                        slot: slot.slot,
                        choice,
                    },
                )
            })
            .collect()
    }

    /// RefreshNewCustomSetButton: at least one valid appearance in the current view.
    pub fn can_create_custom_set(&self) -> bool {
        !self.busy() && !self.custom_set_choices().is_empty()
    }

    pub fn open_custom_set_name(&mut self) {
        if self.can_create_custom_set() {
            self.naming_custom_set = true;
        }
    }

    /// TRANSMOG_CUSTOM_SET_NAME accept: create a server-allocated custom set.
    pub fn create_custom_set(&mut self, name: &str) -> Option<TransmogSend> {
        let name = name.trim();
        if !self.naming_custom_set || name.is_empty() || !self.can_create_custom_set() {
            return None;
        }
        let save = SaveTransmogCustomSet {
            npc: self.state.as_ref()?.npc,
            id: None,
            name: name.to_owned(),
            choices: self.custom_set_choices(),
        };
        self.saving = Some(SaveKind::CustomSet);
        Some(TransmogSend::SaveCustomSet(save))
    }

    /// TransmogCustomSetModelMixin OnMouseDown: SetOutfitToCustomSet.
    pub fn use_custom_set(&mut self, id: u64) -> bool {
        let Some(set) = self
            .custom_sets()
            .into_iter()
            .find(|set| set.id == id)
            .cloned()
        else {
            return false;
        };
        if self.busy() {
            return false;
        }
        self.stage_choices(&set.choices);
        self.error.clear();
        true
    }

    /// Custom sets sorted by name (all choices are collected: the server validates them).
    pub fn custom_sets(&self) -> Vec<&TransmogCustomSet> {
        let mut sets: Vec<_> = self
            .state
            .iter()
            .flat_map(|state| &state.custom_sets)
            .collect();
        sets.sort_by(|left, right| left.name.cmp(&right.name).then(left.id.cmp(&right.id)));
        sets
    }

    pub fn page_custom_sets(&self) -> Vec<&TransmogCustomSet> {
        self.custom_sets()
            .into_iter()
            .skip(self.page * SETS_PER_PAGE)
            .take(SETS_PER_PAGE)
            .collect()
    }

    /// C_TransmogSets primary appearances: one piece per transmog slot (server-computed).
    pub fn set_progress(&self, set: &TransmogSet) -> SetProgress {
        SetProgress {
            collected: set
                .primary_appearances
                .iter()
                .filter(|piece| piece.collection == TransmogCollectionState::Collected)
                .count(),
            total: set.primary_appearances.len(),
        }
    }

    /// GetAvailableSets (the server lists only started sets), collected first (SortSets);
    /// expansion/patch/uiOrder keys are not exported, so setID descending decides ties.
    pub fn available_sets(&self) -> Vec<(&TransmogSet, SetProgress)> {
        let query = self.search.trim().to_lowercase();
        let mut sets: Vec<_> = self
            .state
            .iter()
            .flat_map(|state| &state.sets)
            .filter(|set| query.is_empty() || set.name.to_lowercase().contains(&query))
            .map(|set| (set, self.set_progress(set)))
            .filter(|(_, progress)| self.matches_set_completion(*progress))
            .collect();
        sets.sort_by(|(left, lp), (right, rp)| {
            let left_done = lp.collected == lp.total;
            let right_done = rp.collected == rp.total;
            right_done
                .cmp(&left_done)
                .then(right.set_id.cmp(&left.set_id))
        });
        sets
    }

    pub fn page_sets(&self) -> Vec<(&TransmogSet, SetProgress)> {
        self.available_sets()
            .into_iter()
            .skip(self.page * SETS_PER_PAGE)
            .take(SETS_PER_PAGE)
            .collect()
    }

    /// TransmogSetModelMixin OnMouseDown: SetOutfitToSet stages each usable set appearance.
    pub fn use_set(&mut self, set_id: u32) -> bool {
        let Some(set) = self
            .state
            .as_ref()
            .and_then(|state| state.sets.iter().find(|set| set.set_id == set_id))
            .cloned()
        else {
            return false;
        };
        if self.busy() {
            return false;
        }
        let choices: Vec<_> = self
            .state
            .iter()
            .flat_map(|state| &state.slots)
            .filter_map(|slot| {
                let source = slot
                    .sources
                    .iter()
                    .find(|source| set.source_ids.contains(&source.source_id))?;
                Some(TransmogOutfitChoice {
                    slot: slot.slot,
                    choice: TransmogChoice::Source(source.source_id),
                })
            })
            .collect();
        if choices.is_empty() {
            return false;
        }
        self.stage_choices(&choices);
        self.error.clear();
        true
    }

    /// GetFirstMatchingSetID: Some(pending) when every slot the set covers shows one of
    /// its members; a slot with nothing equipped shows none.
    pub fn set_match(&self, set: &TransmogSet) -> Option<bool> {
        let state = self.state.as_ref()?;
        let mut pending = false;
        for piece in &set.primary_appearances {
            let slot = state
                .slots
                .iter()
                .find(|slot| piece.slots.contains(&slot.slot))?;
            let TransmogChoice::Source(id) = self.effective_choice(slot) else {
                return None;
            };
            if !set.source_ids.contains(&id) {
                return None;
            }
            pending |= self.pending.contains_key(&slot.slot);
        }
        (!set.primary_appearances.is_empty()).then_some(pending)
    }

    /// GetFirstMatchingCustomSetID.
    pub fn custom_set_match(&self, set: &TransmogCustomSet) -> Option<bool> {
        let state = self.state.as_ref()?;
        let mut pending = false;
        for choice in &set.choices {
            let slot = state.slots.iter().find(|slot| slot.slot == choice.slot)?;
            if self.effective_choice(slot) != choice.choice {
                return None;
            }
            pending |= self.pending.contains_key(&slot.slot);
        }
        (!set.choices.is_empty()).then_some(pending)
    }

    /// TransmogSetModelMixin:UpdateSet: an undressed player trying on every primary appearance.
    pub fn set_equipment(&self, set_id: u32) -> Option<shared::components::EquipmentAppearance> {
        let set = self
            .state
            .as_ref()?
            .sets
            .iter()
            .find(|set| set.set_id == set_id)?;
        let entries = set
            .primary_appearances
            .iter()
            .filter_map(|piece| {
                let source = &piece.source;
                Some(shared::components::EquippedAppearanceEntry {
                    slot: piece.slots.first()?.visual_slot()?,
                    item_id: Some(source.item_id),
                    definition_source: Some(shared::item_data::ItemDefinitionSource::Retail),
                    display_info_id: Some(source.display_id),
                    inventory_type: source.inventory_type,
                    hidden: false,
                })
            })
            .collect();
        Some(shared::components::EquipmentAppearance { entries })
    }

    /// Illusion entries show the selected weapon's current appearance (UpdateItem IsIllusion).
    pub fn weapon_equipment(
        &self,
        slot: EquipmentSlot,
    ) -> Option<shared::components::EquipmentAppearance> {
        let slot = self.state.as_ref()?.slots.iter().find(|s| s.slot == slot)?;
        let entry = match self.effective_choice(slot) {
            TransmogChoice::Source(id) => {
                appearance_entry(slot, slot.sources.iter().find(|s| s.source_id == id)?)?
            }
            _ => shared::components::EquippedAppearanceEntry {
                slot: slot.slot.visual_slot()?,
                item_id: Some(slot.item_id),
                definition_source: Some(slot.definition_source),
                display_info_id: None,
                inventory_type: slot.inventory_type,
                hidden: false,
            },
        };
        Some(shared::components::EquipmentAppearance {
            entries: vec![entry],
        })
    }

    pub fn custom_set_equipment(&self, id: u64) -> Option<shared::components::EquipmentAppearance> {
        let state = self.state.as_ref()?;
        let set = state.custom_sets.iter().find(|set| set.id == id)?;
        let entries = set
            .choices
            .iter()
            .filter_map(|choice| {
                let slot = state.slots.iter().find(|slot| slot.slot == choice.slot)?;
                match choice.choice {
                    TransmogChoice::Source(id) => {
                        appearance_entry(slot, slot.sources.iter().find(|s| s.source_id == id)?)
                    }
                    TransmogChoice::Hidden => Some(shared::components::EquippedAppearanceEntry {
                        slot: slot.slot.visual_slot()?,
                        item_id: Some(slot.item_id),
                        definition_source: Some(slot.definition_source),
                        display_info_id: None,
                        inventory_type: slot.inventory_type,
                        hidden: true,
                    }),
                    TransmogChoice::Original => None,
                }
            })
            .collect();
        Some(shared::components::EquipmentAppearance { entries })
    }
}

#[cfg(test)]
#[path = "wardrobe_tests.rs"]
mod tests;

pub(super) fn appearance_entry(
    slot: &TransmogSlot,
    source: &TransmogSource,
) -> Option<shared::components::EquippedAppearanceEntry> {
    Some(shared::components::EquippedAppearanceEntry {
        slot: slot.slot.visual_slot()?,
        item_id: Some(slot.item_id),
        definition_source: Some(slot.definition_source),
        display_info_id: Some(source.display_id),
        inventory_type: source.inventory_type,
        hidden: false,
    })
}
