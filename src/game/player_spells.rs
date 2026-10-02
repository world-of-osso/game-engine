//! Owning player's spell state from the server: known spells, active
//! specialization, action bar contents, cooldowns and charges. Also the action
//! currently dragged on the cursor.

use std::collections::HashMap;

use bevy::prelude::*;
use shared::protocol::{
    ActionBarSnapshot, ActionRef, KnownSpellsSnapshot, SpecializationChanged, SpellCastIntent,
    SpellChargesUpdate, SpellCooldownUpdate, SpellsLearned, SpellsUnlearned,
};

use crate::network_runtime::messages::MessageReceivers;
use crate::network_runtime::replication::ReplicationMirrorMap;
use crate::targeting::CurrentTarget;

/// Server action bar slots 0..=119.
pub const ACTION_SLOT_COUNT: usize = 120;
pub const SLOTS_PER_BAR: usize = 12;
/// Bars 1–5 show slots 0..60; the rest are stored but not shown.
pub const VISIBLE_BARS: usize = 5;

/// Known spell ids in server learn order.
#[derive(Resource, Default, Debug, Clone, PartialEq)]
pub struct KnownSpells {
    spells: Vec<u32>,
}

impl KnownSpells {
    pub fn new(spells: Vec<u32>) -> Self {
        Self { spells }
    }

    pub fn spells(&self) -> &[u32] {
        &self.spells
    }

    pub fn contains(&self, spell_id: u32) -> bool {
        self.spells.contains(&spell_id)
    }

    fn learn(&mut self, spells: &[u32]) {
        for &id in spells {
            if !self.spells.contains(&id) {
                self.spells.push(id);
            }
        }
    }

    fn unlearn(&mut self, spells: &[u32]) {
        self.spells.retain(|id| !spells.contains(id));
    }
}

/// `ChrSpecialization` id from `SpecializationChanged`.
#[derive(Resource, Default, Debug, Clone, Copy, PartialEq, Eq)]
pub struct ActiveSpecialization(pub Option<u32>);

#[derive(Resource, Debug, Clone, PartialEq)]
pub struct ActionBarSlots([Option<ActionRef>; ACTION_SLOT_COUNT]);

impl Default for ActionBarSlots {
    fn default() -> Self {
        Self([None; ACTION_SLOT_COUNT])
    }
}

impl ActionBarSlots {
    pub fn from_snapshot(slots: &[(u8, ActionRef)]) -> Self {
        let mut bar = Self::default();
        for &(slot, action) in slots {
            bar.set(slot as usize, Some(action));
        }
        bar
    }

    pub fn get(&self, slot: usize) -> Option<ActionRef> {
        self.0.get(slot).copied().flatten()
    }

    pub fn set(&mut self, slot: usize, action: Option<ActionRef>) {
        if let Some(entry) = self.0.get_mut(slot) {
            *entry = action;
        }
    }
}

/// Slot index of `button` (1-based) on `bar` (1-based).
pub fn bar_slot_index(bar: usize, button: usize) -> usize {
    (bar - 1) * SLOTS_PER_BAR + (button - 1)
}

/// Registry name of a visible slot's button: `ActionButton<bar>_<button>`.
pub fn action_button_name(slot: usize) -> String {
    format!(
        "ActionButton{}_{}",
        slot / SLOTS_PER_BAR + 1,
        slot % SLOTS_PER_BAR + 1
    )
}

/// Slot index of an action button or one of its children (`ActionButton2_3Icon`).
pub fn parse_action_button_name(name: &str) -> Option<usize> {
    let rest = name.strip_prefix("ActionButton")?;
    let (bar, rest) = rest.split_once('_')?;
    let digits: String = rest.chars().take_while(char::is_ascii_digit).collect();
    let bar: usize = bar.parse().ok()?;
    let button: usize = digits.parse().ok()?;
    ((1..=VISIBLE_BARS).contains(&bar) && (1..=SLOTS_PER_BAR).contains(&button))
        .then(|| bar_slot_index(bar, button))
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct CooldownTimer {
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

/// Running spell cooldowns and the global cooldown.
#[derive(Resource, Default, Debug, Clone, PartialEq)]
pub struct SpellCooldowns {
    spells: HashMap<u32, CooldownTimer>,
    gcd: Option<CooldownTimer>,
}

impl SpellCooldowns {
    pub fn apply(&mut self, update: &SpellCooldownUpdate) {
        let timer = CooldownTimer::from_ms(update.duration_ms, update.remaining_ms);
        let running = timer.remaining > 0.0;
        if update.is_gcd {
            self.gcd = running.then_some(timer);
        } else if running {
            self.spells.insert(update.spell_id, timer);
        } else {
            self.spells.remove(&update.spell_id);
        }
    }

    pub fn is_active(&self) -> bool {
        self.gcd.is_some() || !self.spells.is_empty()
    }

    pub fn tick(&mut self, dt: f32) {
        self.spells.retain(|_, timer| timer.tick(dt));
        if let Some(gcd) = &mut self.gcd
            && !gcd.tick(dt)
        {
            self.gcd = None;
        }
    }

    pub fn spell(&self, spell_id: u32) -> Option<CooldownTimer> {
        self.spells.get(&spell_id).copied()
    }

    pub fn gcd(&self) -> Option<CooldownTimer> {
        self.gcd
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct ChargeState {
    pub current: u8,
    pub max: u8,
    /// Seconds to regain one charge.
    pub recharge: f32,
    /// Seconds until the next charge.
    pub remaining: f32,
}

impl ChargeState {
    fn tick(&mut self, dt: f32) {
        if self.current >= self.max {
            return;
        }
        self.remaining -= dt;
        while self.remaining <= 0.0 && self.current < self.max {
            self.current += 1;
            self.remaining += self.recharge;
        }
        if self.current >= self.max || self.recharge <= 0.0 {
            self.current = self.current.min(self.max);
            self.remaining = 0.0;
        }
    }

    pub fn recharging(&self) -> bool {
        self.current < self.max
    }
}

#[derive(Resource, Default, Debug, Clone, PartialEq)]
pub struct SpellChargeStates(HashMap<u32, ChargeState>);

impl SpellChargeStates {
    pub fn apply(&mut self, update: &SpellChargesUpdate) {
        self.0.insert(
            update.spell_id,
            ChargeState {
                current: update.current,
                max: update.max,
                recharge: update.recharge_ms as f32 / 1000.0,
                remaining: update.remaining_ms as f32 / 1000.0,
            },
        );
    }

    pub fn get(&self, spell_id: u32) -> Option<ChargeState> {
        self.0.get(&spell_id).copied()
    }

    pub fn is_recharging(&self) -> bool {
        self.0.values().any(ChargeState::recharging)
    }

    pub fn tick(&mut self, dt: f32) {
        for charges in self.0.values_mut() {
            charges.tick(dt);
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum DragSource {
    Spellbook,
    Slot(usize),
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct DraggedAction {
    pub action: ActionRef,
    pub source: DragSource,
}

/// The action on the cursor; `Some` puts input in `UiInputMode::Drag`.
#[derive(Resource, Default, Debug, Clone, Copy, PartialEq, Eq)]
pub struct ActionDrag(pub Option<DraggedAction>);

pub struct PlayerSpellsPlugin;

impl Plugin for PlayerSpellsPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<KnownSpells>()
            .init_resource::<ActiveSpecialization>()
            .init_resource::<ActionBarSlots>()
            .init_resource::<SpellCooldowns>()
            .init_resource::<SpellChargeStates>()
            .init_resource::<ActionDrag>()
            .add_systems(Update, tick_spell_timers);
    }
}

pub fn tick_spell_timers(
    time: Res<Time>,
    mut cooldowns: ResMut<SpellCooldowns>,
    mut charges: ResMut<SpellChargeStates>,
) {
    let dt = time.delta_secs();
    if dt <= 0.0 {
        return;
    }
    if cooldowns.is_active() {
        cooldowns.tick(dt);
    }
    if charges.is_recharging() {
        charges.tick(dt);
    }
}

pub fn receive_known_spells_snapshot(
    mut receivers: MessageReceivers<KnownSpellsSnapshot>,
    mut known: ResMut<KnownSpells>,
) {
    for receiver in receivers.iter_mut() {
        for msg in receiver.receive() {
            *known = KnownSpells::new(msg.spells);
        }
    }
}

pub fn receive_spells_learned(
    mut receivers: MessageReceivers<SpellsLearned>,
    mut known: ResMut<KnownSpells>,
) {
    for receiver in receivers.iter_mut() {
        for msg in receiver.receive() {
            known.learn(&msg.spells);
        }
    }
}

pub fn receive_spells_unlearned(
    mut receivers: MessageReceivers<SpellsUnlearned>,
    mut known: ResMut<KnownSpells>,
) {
    for receiver in receivers.iter_mut() {
        for msg in receiver.receive() {
            known.unlearn(&msg.spells);
        }
    }
}

pub fn receive_specialization_changed(
    mut receivers: MessageReceivers<SpecializationChanged>,
    mut spec: ResMut<ActiveSpecialization>,
) {
    for receiver in receivers.iter_mut() {
        for msg in receiver.receive() {
            spec.set_if_neq(ActiveSpecialization(Some(msg.spec_id)));
        }
    }
}

pub fn receive_action_bar_snapshot(
    mut receivers: MessageReceivers<ActionBarSnapshot>,
    mut bar: ResMut<ActionBarSlots>,
) {
    for receiver in receivers.iter_mut() {
        for msg in receiver.receive() {
            *bar = ActionBarSlots::from_snapshot(&msg.slots);
        }
    }
}

pub fn receive_spell_cooldowns(
    mut receivers: MessageReceivers<SpellCooldownUpdate>,
    mut cooldowns: ResMut<SpellCooldowns>,
) {
    for receiver in receivers.iter_mut() {
        for msg in receiver.receive() {
            cooldowns.apply(&msg);
        }
    }
}

pub fn receive_spell_charges(
    mut receivers: MessageReceivers<SpellChargesUpdate>,
    mut charges: ResMut<SpellChargeStates>,
) {
    for receiver in receivers.iter_mut() {
        for msg in receiver.receive() {
            charges.apply(&msg);
        }
    }
}

/// Server entity bits of the current target; `None` when it is not replicated.
pub fn server_target_bits(
    current_target: Option<&CurrentTarget>,
    mirror: Option<&ReplicationMirrorMap>,
) -> Option<u64> {
    let main = current_target?.0?;
    mirror?.main_to_server(main).map(Entity::to_bits)
}

pub fn spell_cast_intent(spell_id: u32, spell_name: &str, target: Option<u64>) -> SpellCastIntent {
    SpellCastIntent {
        spell_id: Some(spell_id),
        spell: spell_name.to_string(),
        target_entity: target,
        // The preserved Bevy client finds no LOS witness ray.
        witness: None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn action_button_names_round_trip_for_visible_bars() {
        assert_eq!(action_button_name(0), "ActionButton1_1");
        assert_eq!(action_button_name(13), "ActionButton2_2");
        assert_eq!(action_button_name(59), "ActionButton5_12");
        assert_eq!(parse_action_button_name("ActionButton1_1"), Some(0));
        assert_eq!(parse_action_button_name("ActionButton2_2Icon"), Some(13));
        assert_eq!(parse_action_button_name("ActionButton5_12HotKey"), Some(59));
        assert_eq!(parse_action_button_name("ActionButton6_1"), None);
        assert_eq!(parse_action_button_name("ActionBarGuide"), None);
    }

    #[test]
    fn learn_and_unlearn_keep_learn_order_without_duplicates() {
        let mut known = KnownSpells::new(vec![35395, 20271]);
        known.learn(&[20271, 853]);
        known.unlearn(&[35395]);
        assert_eq!(known.spells(), &[20271, 853]);
    }

    #[test]
    fn cooldown_and_gcd_expire_after_their_remaining_time() {
        let mut cooldowns = SpellCooldowns::default();
        cooldowns.apply(&SpellCooldownUpdate {
            spell_id: 35395,
            category: 0,
            duration_ms: 6000,
            remaining_ms: 5500,
            is_gcd: false,
        });
        cooldowns.apply(&SpellCooldownUpdate {
            spell_id: 35395,
            category: 133,
            duration_ms: 1500,
            remaining_ms: 1500,
            is_gcd: true,
        });
        cooldowns.tick(1.5);
        assert_eq!(cooldowns.gcd(), None);
        assert_eq!(cooldowns.spell(35395).unwrap().remaining, 4.0);
        cooldowns.tick(4.0);
        assert!(!cooldowns.is_active());
    }

    #[test]
    fn charges_regain_one_at_a_time_until_full() {
        let mut charges = SpellChargeStates::default();
        charges.apply(&SpellChargesUpdate {
            spell_id: 35395,
            current: 0,
            max: 2,
            recharge_ms: 6000,
            remaining_ms: 2000,
        });
        charges.tick(2.5);
        let state = charges.get(35395).unwrap();
        assert_eq!((state.current, state.remaining), (1, 5.5));
        charges.tick(6.0);
        let state = charges.get(35395).unwrap();
        assert_eq!((state.current, state.remaining), (2, 0.0));
        assert!(!charges.is_recharging());
    }
}
