//! The owning player's spell state from the server (Bevy `game/player_spells.rs`):
//! known spells in learn order, active specialization, the action slots, running
//! cooldowns including the global cooldown, and spell charges.

use std::collections::HashMap;

use game_engine_core::spell_catalog::SpellCatalogData;
use shared::components::AuraView;
use shared::protocol::{ActionRef, SpellChargesUpdate, SpellCooldownUpdate};

/// Server action bar slots 0..=179 (TrinityCore Player.h `MAX_ACTION_BUTTONS` 180).
pub(crate) const ACTION_SLOT_COUNT: usize = 180;
/// Retail `NUM_ACTIONBAR_BUTTONS` / `NUM_ACTIONBAR_PAGES` (ActionButtonUtil.lua).
const NUM_ACTIONBAR_BUTTONS: usize = 12;
const NUM_ACTIONBAR_PAGES: usize = 6;

/// `SPELL_AURA_ADV_FLYING` (TrinityCore SpellAuraDefines.h:533), the Skyriding aura
/// 406095's effect 0.
const AURA_ADV_FLYING: u16 = 446;
/// The bonus bar of the Skyriding abilities, action slots 120..=131: Retail's Skyriding
/// tutorial looks for Surge Forward past `(NUM_ACTIONBAR_PAGES + GetBonusBarOffset() - 1) *
/// NUM_ACTIONBAR_BUTTONS` (Blizzard_Tutorials_RPE.lua:446), the macro conditional
/// `[bonusbar:5]`.
pub(crate) const SKYRIDING_BONUS_BAR: u8 = 5;

/// `C_ActionBar.GetBonusBarOffset`: the `BonusActionBar` of the player's shapeshift
/// form, which comes from its form aura (one form at a time), or the Skyriding bar while
/// an ADV_FLYING aura lets the player skyride.
pub(crate) fn bonus_bar_offset(auras: &[AuraView], catalog: &SpellCatalogData) -> u8 {
    auras
        .iter()
        .filter_map(|aura| catalog.get(aura.spell_id))
        .map(|spell| {
            if spell
                .effects
                .iter()
                .any(|effect| effect.aura == AURA_ADV_FLYING)
            {
                SKYRIDING_BONUS_BAR
            } else {
                spell.bonus_bar
            }
        })
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

/// Charges of one charge category, recovered locally between server updates the way
/// Retail's `SpellHistory` restores one every `recharge` seconds (`C_Spell.GetSpellCharges`).
#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) struct ChargeTimer {
    pub current: u8,
    pub max: u8,
    pub recharge: f32,
    /// Seconds until the next charge returns; meaningful while `current < max`.
    pub remaining: f32,
}

impl ChargeTimer {
    fn tick(&mut self, dt: f32) {
        let mut dt = dt;
        while self.current < self.max && dt >= self.remaining {
            dt -= self.remaining;
            self.current += 1;
            self.remaining = self.recharge;
        }
        if self.current < self.max {
            self.remaining -= dt;
        }
    }

    /// How far the recovering charge is, 0..1 (a full category has none recovering).
    pub fn recovered(&self) -> f32 {
        if self.current >= self.max || self.recharge <= 0.0 {
            return 0.0;
        }
        (1.0 - self.remaining / self.recharge).clamp(0.0, 1.0)
    }
}

#[derive(Debug, Clone, PartialEq)]
pub(crate) struct PlayerSpells {
    known: Vec<u32>,
    spec: Option<u32>,
    slots: [Option<ActionRef>; ACTION_SLOT_COUNT],
    cooldowns: HashMap<u32, CooldownTimer>,
    gcd: Option<CooldownTimer>,
    /// By `ChargeCategory`.
    charges: HashMap<u32, ChargeTimer>,
}

impl Default for PlayerSpells {
    fn default() -> Self {
        Self {
            known: Vec::new(),
            spec: None,
            slots: [None; ACTION_SLOT_COUNT],
            cooldowns: HashMap::new(),
            gcd: None,
            charges: HashMap::new(),
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

    /// `SpellChargesUpdate` (`SMSG_SET_SPELL_CHARGES`): the category's charges now.
    pub fn apply_charges(&mut self, update: &SpellChargesUpdate) {
        self.charges.insert(
            update.category,
            ChargeTimer {
                current: update.current,
                max: update.max,
                recharge: update.recharge_ms as f32 / 1000.0,
                remaining: update.remaining_ms as f32 / 1000.0,
            },
        );
    }

    /// Charges of `category` the server reported, recovered to now; None before any.
    pub fn charges(&self, category: u32) -> Option<ChargeTimer> {
        self.charges.get(&category).copied()
    }

    pub fn tick(&mut self, dt: f32) {
        for charges in self.charges.values_mut() {
            charges.tick(dt);
        }
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
    use game_engine_core::spell_catalog::{CatalogEffect, CatalogSpell};

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
    /// The Skyriding aura (MountCapability 494) and its abilities.
    const SKYRIDING: u32 = 406095;
    const SURGE_FORWARD: u32 = 372608;
    const SKYWARD_ASCENT: u32 = 372610;
    const AERIAL_HALT: u32 = 403092;
    const WHIRLING_SURGE: u32 = 361584;
    const SECOND_WIND: u32 = 425782;
    const SKYRIDING_CHARGES: u32 = 2391;

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
        // Skyriding 406095 effect 0: APPLY_AURA ADV_FLYING.
        let skyriding = CatalogSpell {
            effects: Box::new([CatalogEffect {
                index: 0,
                effect: 6,
                aura: 446,
                base_points: 0.0,
                aura_period_ms: 0,
                chain_targets: 0,
                radius_yd: 0.0,
                spell_power_coefficient: 0.0,
                attack_power_coefficient: 0.0,
                level_scaled: false,
            }]),
            ..spell(SKYRIDING, 0)
        };
        let spells = vec![
            spell(CAT_FORM, 1),
            spell(BEAR_FORM, 3),
            spell(BATTLE_STANCE, 0),
            skyriding,
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

    /// The server's Skyriding bar (bonus bar 5, slots 120..=124) replaces the main bar while
    /// the Skyriding aura is up; dismounting (the aura gone) shows page 1 again.
    #[test]
    fn the_skyriding_aura_pages_the_main_bar_to_the_skyriding_bar() {
        let mut spells = PlayerSpells::default();
        let skyriding_bar = [
            SURGE_FORWARD,
            SKYWARD_ASCENT,
            AERIAL_HALT,
            WHIRLING_SURGE,
            SECOND_WIND,
        ];
        let mut slots = vec![(0, ActionRef::Spell(SLAM))];
        slots.extend(
            (120..)
                .zip(skyriding_bar)
                .map(|(slot, spell)| (slot, ActionRef::Spell(spell))),
        );
        spells.set_bar(&slots);

        let mounted = main_bar(&spells, &[aura(SKYRIDING)]);
        let expected: Vec<_> = skyriding_bar
            .iter()
            .map(|&spell| Some(ActionRef::Spell(spell)))
            .chain([None; 7])
            .collect();
        assert_eq!(mounted, expected);

        let dismounted = main_bar(&spells, &[]);
        assert_eq!(dismounted[0], Some(ActionRef::Spell(SLAM)));
        assert_eq!(dismounted[1..], [None; 11]);
    }

    /// Two Skyriding Charges spent (4 of 6 left, 10.35 s each): the client recovers them
    /// between updates, one every 10.35 s, and stops at 6.
    #[test]
    fn skyriding_charges_recover_locally_between_updates() {
        let mut spells = PlayerSpells::default();
        assert_eq!(spells.charges(SKYRIDING_CHARGES), None);
        spells.apply_charges(&SpellChargesUpdate {
            spell_id: SKYWARD_ASCENT,
            category: SKYRIDING_CHARGES,
            current: 4,
            max: 6,
            recharge_ms: 10_350,
            remaining_ms: 8_000,
        });
        spells.tick(4.0);
        let charges = spells.charges(SKYRIDING_CHARGES).unwrap();
        assert_eq!(charges.current, 4);
        assert!((charges.recovered() - 6.35 / 10.35).abs() < 1e-4);

        // The fifth returns 4 s later; the sixth starts recovering from there.
        spells.tick(5.0);
        let charges = spells.charges(SKYRIDING_CHARGES).unwrap();
        assert_eq!(charges.current, 5);
        assert!((charges.remaining - 9.35).abs() < 1e-4);

        spells.tick(30.0);
        let charges = spells.charges(SKYRIDING_CHARGES).unwrap();
        assert_eq!((charges.current, charges.recovered()), (6, 0.0));
    }
}
