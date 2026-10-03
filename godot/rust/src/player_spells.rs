//! The owning player's spell state from the server (Bevy `game/player_spells.rs`):
//! known spells in learn order, active specialization, the 120 action slots and
//! running cooldowns including the global cooldown.

use std::collections::HashMap;

use game_engine_core::spell_catalog::SpellCatalogData;
use shared::components::AuraView;
use shared::protocol::{ActionRef, SpellCooldownUpdate};

/// Server action bar slots 0..=119.
pub(crate) const ACTION_SLOT_COUNT: usize = 120;
/// Retail `NUM_ACTIONBAR_BUTTONS` / `NUM_ACTIONBAR_PAGES` (ActionButtonUtil.lua).
const NUM_ACTIONBAR_BUTTONS: usize = 12;
const NUM_ACTIONBAR_PAGES: usize = 6;

/// `C_ActionBar.GetBonusBarOffset`: the `BonusActionBar` of the player's shapeshift
/// form, which comes from its form aura (one form at a time).
pub(crate) fn bonus_bar_offset(auras: &[AuraView], catalog: &SpellCatalogData) -> u8 {
    auras
        .iter()
        .filter_map(|aura| catalog.get(aura.spell_id))
        .map(|spell| spell.bonus_bar)
        .find(|&offset| offset > 0)
        .unwrap_or(0)
}

/// Action slot of main bar button `index` (0-based). `ActionBarController_UpdateAll`
/// sets the main bar's `actionpage` to `GetBonusBarIndex()` (`NUM_ACTIONBAR_PAGES` +
/// offset) while a bonus bar is active, else page 1; `CalculateAction` adds
/// `(page - 1) * NUM_ACTIONBAR_BUTTONS`. Cat Form (offset 1) shows slots 72..=83.
pub(crate) fn main_bar_slot(index: usize, bonus_bar_offset: u8) -> usize {
    let page = match bonus_bar_offset {
        0 => 1,
        offset => NUM_ACTIONBAR_PAGES + usize::from(offset),
    };
    index + (page - 1) * NUM_ACTIONBAR_BUTTONS
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) struct CooldownTimer {
    pub duration: f32,
    pub remaining: f32,
}

impl CooldownTimer {
    fn from_ms(duration_ms: u32, remaining_ms: u32) -> Self {
        Self {
            duration: duration_ms as f32 / 1000.0,
            remaining: remaining_ms as f32 / 1000.0,
        }
    }

    fn tick(&mut self, dt: f32) -> bool {
        self.remaining = (self.remaining - dt).max(0.0);
        self.remaining > 0.0
    }
}

#[derive(Debug, Clone, PartialEq)]
pub(crate) struct PlayerSpells {
    known: Vec<u32>,
    spec: Option<u32>,
    slots: [Option<ActionRef>; ACTION_SLOT_COUNT],
    cooldowns: HashMap<u32, CooldownTimer>,
    gcd: Option<CooldownTimer>,
}

impl Default for PlayerSpells {
    fn default() -> Self {
        Self {
            known: Vec::new(),
            spec: None,
            slots: [None; ACTION_SLOT_COUNT],
            cooldowns: HashMap::new(),
            gcd: None,
        }
    }
}

impl PlayerSpells {
    pub fn known(&self) -> &[u32] {
        &self.known
    }

    pub fn spec(&self) -> Option<u32> {
        self.spec
    }

    /// `KnownSpellsSnapshot`: replaces the list.
    pub fn set_known(&mut self, spells: Vec<u32>) {
        self.known = spells;
    }

    /// `SpellsLearned`: appended in learn order, without duplicates.
    pub fn learn(&mut self, spells: &[u32]) {
        for &id in spells {
            if !self.known.contains(&id) {
                self.known.push(id);
            }
        }
    }

    /// `SpellsUnlearned`.
    pub fn unlearn(&mut self, spells: &[u32]) {
        self.known.retain(|id| !spells.contains(id));
    }

    pub fn set_spec(&mut self, spec_id: u32) {
        self.spec = Some(spec_id);
    }

    /// `ActionBarSnapshot`: the server's full bar; slots outside 0..120 are ignored.
    pub fn set_bar(&mut self, slots: &[(u8, ActionRef)]) {
        self.slots = [None; ACTION_SLOT_COUNT];
        for &(slot, action) in slots {
            if let Some(entry) = self.slots.get_mut(usize::from(slot)) {
                *entry = Some(action);
            }
        }
    }

    pub fn slot(&self, slot: usize) -> Option<ActionRef> {
        self.slots.get(slot).copied().flatten()
    }

    /// `SpellCooldownUpdate`: `is_gcd` sets the global cooldown; a zero remaining time
    /// ends the cooldown.
    pub fn apply_cooldown(&mut self, update: &SpellCooldownUpdate) {
        let timer = CooldownTimer::from_ms(update.duration_ms, update.remaining_ms);
        let running = timer.remaining > 0.0;
        if update.is_gcd {
            self.gcd = running.then_some(timer);
        } else if running {
            self.cooldowns.insert(update.spell_id, timer);
        } else {
            self.cooldowns.remove(&update.spell_id);
        }
    }

    pub fn tick(&mut self, dt: f32) {
        self.cooldowns.retain(|_, timer| timer.tick(dt));
        if let Some(gcd) = &mut self.gcd
            && !gcd.tick(dt)
        {
            self.gcd = None;
        }
    }

    /// What a spell button sweeps: the longer of the spell's own cooldown and, when the
    /// spell triggers the global cooldown, the GCD (Retail `ActionButton_UpdateCooldown`
    /// shows `GetSpellCooldown`, which includes the GCD).
    pub fn button_cooldown(&self, spell_id: u32, on_gcd: bool) -> Option<CooldownTimer> {
        let own = self.cooldowns.get(&spell_id).copied();
        let gcd = self.gcd.filter(|_| on_gcd);
        match (own, gcd) {
            (Some(own), Some(gcd)) if gcd.remaining > own.remaining => Some(gcd),
            (Some(own), _) => Some(own),
            (None, gcd) => gcd,
        }
    }

    pub fn gcd(&self) -> Option<CooldownTimer> {
        self.gcd
    }

    pub fn clear(&mut self) {
        *self = Self::default();
    }
}

#[cfg(test)]
mod tests {
    use game_engine_core::spell_catalog::CatalogSpell;

    use super::*;

    const SLAM: u32 = 1464;
    const CHARGE: u32 = 100;
    const ATTACK: u32 = 88163;
    const MOONFIRE: u32 = 8921;
    const WRATH: u32 = 5176;
    const SHRED: u32 = 5221;
    const FEROCIOUS_BITE: u32 = 22568;
    const MANGLE: u32 = 33917;
    const CAT_FORM: u32 = 768;
    const BEAR_FORM: u32 = 5487;
    const BATTLE_STANCE: u32 = 386164;

    fn cooldown(
        spell_id: u32,
        duration_ms: u32,
        remaining_ms: u32,
        is_gcd: bool,
    ) -> SpellCooldownUpdate {
        SpellCooldownUpdate {
            spell_id,
            category: if is_gcd { 133 } else { 0 },
            duration_ms,
            remaining_ms,
            is_gcd,
        }
    }

    #[test]
    fn learned_and_unlearned_spells_keep_learn_order_without_duplicates() {
        let mut spells = PlayerSpells::default();
        spells.set_known(vec![SLAM, 88163]);
        spells.learn(&[88163, CHARGE]);
        spells.unlearn(&[SLAM]);
        assert_eq!(spells.known(), &[88163, CHARGE]);
    }

    #[test]
    fn bar_snapshot_replaces_every_slot() {
        let mut spells = PlayerSpells::default();
        spells.set_bar(&[(0, ActionRef::Spell(SLAM)), (5, ActionRef::Spell(CHARGE))]);
        spells.set_bar(&[(1, ActionRef::Spell(CHARGE))]);
        assert_eq!(spells.slot(0), None);
        assert_eq!(spells.slot(1), Some(ActionRef::Spell(CHARGE)));
        assert_eq!(spells.slot(5), None);
    }

    #[test]
    fn button_shows_the_longer_of_spell_cooldown_and_gcd() {
        let mut spells = PlayerSpells::default();
        spells.apply_cooldown(&cooldown(CHARGE, 20_000, 20_000, false));
        spells.apply_cooldown(&cooldown(CHARGE, 1_500, 1_500, true));
        assert_eq!(spells.button_cooldown(CHARGE, true).unwrap().duration, 20.0);
        // Slam has no cooldown of its own: the GCD sweeps it, unless it is off the GCD.
        assert_eq!(spells.button_cooldown(SLAM, true).unwrap().duration, 1.5);
        assert_eq!(spells.button_cooldown(SLAM, false), None);

        spells.tick(1.5);
        assert_eq!(spells.gcd(), None);
        assert_eq!(spells.button_cooldown(SLAM, true), None);
        assert_eq!(
            spells.button_cooldown(CHARGE, true).unwrap().remaining,
            18.5
        );
        spells.apply_cooldown(&cooldown(CHARGE, 20_000, 0, false));
        assert_eq!(spells.button_cooldown(CHARGE, true), None);
    }

    /// `bonus_bar` values the real catalog carries (core `spell_catalog` test).
    fn form_catalog() -> SpellCatalogData {
        let spell = |id, bonus_bar| CatalogSpell {
            id,
            bonus_bar,
            ..Default::default()
        };
        let spells = vec![
            spell(CAT_FORM, 1),
            spell(BEAR_FORM, 3),
            spell(BATTLE_STANCE, 0),
        ];
        SpellCatalogData::from_parts(spells, Default::default())
    }

    fn aura(spell_id: u32) -> AuraView {
        AuraView {
            instance_id: 1,
            spell_id,
            caster: None,
            stacks: 1,
            charges: 0,
            duration_ms: 0,
            remaining_ms: 0,
            harmful: false,
            dispel_type: 0,
            flags: 0,
        }
    }

    fn main_bar(spells: &PlayerSpells, auras: &[AuraView]) -> Vec<Option<ActionRef>> {
        let offset = bonus_bar_offset(auras, &form_catalog());
        (0..NUM_ACTIONBAR_BUTTONS)
            .map(|index| spells.slot(main_bar_slot(index, offset)))
            .collect()
    }

    /// playercreateinfo_action (28, 11), Highmountain Tauren Druid.
    fn druid_bar() -> PlayerSpells {
        let mut spells = PlayerSpells::default();
        let slots = [
            (0, MOONFIRE),
            (1, WRATH),
            (72, SHRED),
            (73, FEROCIOUS_BITE),
            (96, MANGLE),
        ];
        let slots: Vec<_> = slots
            .iter()
            .map(|&(slot, spell)| (slot, ActionRef::Spell(spell)))
            .collect();
        spells.set_bar(&slots);
        spells
    }

    #[test]
    fn cat_and_bear_form_page_the_main_bar_to_their_bonus_bars() {
        let spells = druid_bar();
        let cat = main_bar(&spells, &[aura(CAT_FORM)]);
        assert_eq!(cat[0], Some(ActionRef::Spell(SHRED)));
        assert_eq!(cat[1], Some(ActionRef::Spell(FEROCIOUS_BITE)));
        assert_eq!(cat[2..], [None; 10]);

        let bear = main_bar(&spells, &[aura(BEAR_FORM)]);
        assert_eq!(bear[0], Some(ActionRef::Spell(MANGLE)));
        assert_eq!(bear[1..], [None; 11]);

        let caster = main_bar(&spells, &[]);
        assert_eq!(caster[0], Some(ActionRef::Spell(MOONFIRE)));
        assert_eq!(caster[1], Some(ActionRef::Spell(WRATH)));
    }

    /// Retail Battle Stance applies no shapeshift form, so a warrior's main bar stays on
    /// page 1 and playercreateinfo_action's Attack on slot 72 never shows there.
    #[test]
    fn retail_battle_stance_keeps_the_warrior_bar_on_page_one() {
        let mut spells = PlayerSpells::default();
        spells.set_bar(&[(0, ActionRef::Spell(SLAM)), (72, ActionRef::Spell(ATTACK))]);
        let bar = main_bar(&spells, &[aura(BATTLE_STANCE)]);
        assert_eq!(bar[0], Some(ActionRef::Spell(SLAM)));
        assert!(!bar.contains(&Some(ActionRef::Spell(ATTACK))));
    }
}
