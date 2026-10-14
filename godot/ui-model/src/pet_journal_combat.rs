//! Owned combat snapshot and Retail CompanionLoadOutSpell/PetSpellSelect behavior.
use super::{PetJournalView, PetRowKey};
use shared::protocol::{PetBattleQueueState, PetJournalDetailsUpdate, SetBattlePetAbility};
#[path = "pet_journal_combat_art.rs"]
mod art;
pub(super) use art::{card_stats, slot_stats, spell_select};

impl PetJournalView {
    pub fn receive_details(&mut self, update: PetJournalDetailsUpdate) {
        self.combat = update
            .pets
            .into_iter()
            .map(|pet| (pet.pet_id, pet))
            .collect();
        self.battle_locked = update.locked;
        self.ability_pending = false;
        self.ability_selection = None;
        self.error = update.error.unwrap_or_default();
    }
    pub fn combat_editable(&self) -> bool {
        !self.battle_locked && !self.ability_pending && self.queue == PetBattleQueueState::Idle
    }
    pub fn open_ability_choices(&mut self, loadout: usize, slot: u8) -> bool {
        if !self.combat_editable() || !(1..=3).contains(&slot) {
            return false;
        }
        let Some(pet_id) = self.journal.battle_slots.get(loadout).copied().flatten() else {
            return false;
        };
        if !self.combat.contains_key(&pet_id) {
            return false;
        }
        let selection = (loadout, slot);
        self.ability_selection = (self.ability_selection != Some(selection)).then_some(selection);
        true
    }
    pub fn choose_ability(&mut self, ability_id: u32) -> Option<SetBattlePetAbility> {
        if !self.combat_editable() {
            return None;
        }
        let (loadout, slot) = self.ability_selection?;
        let pet_id = self.journal.battle_slots[loadout]?;
        let pet = self.journal.get(pet_id)?;
        let details = self.combat.get(&pet_id)?;
        details.choices.iter().find(|choice| {
            choice.id == ability_id && choice.slot == slot && choice.required_level <= pet.level
        })?;
        self.ability_pending = true;
        Some(SetBattlePetAbility {
            pet_id,
            slot,
            ability_id,
        })
    }
    pub(super) fn selected_owned_id(&self) -> Option<u64> {
        match self.selected? {
            PetRowKey::Owned(id) => Some(id),
            PetRowKey::Species(_) => None,
        }
    }
}
