//! Combat animations of replicated units, driven by server combat traffic.
//!
//! Melee (TrinityCore `SMSG_ATTACKERSTATEUPDATE`, here `CombatEvent`): the attacker
//! swings its main-hand weapon class clip (Attack Unarmed/1H/2H/2HL, 16-19; WoWee
//! `resolveMeleeAnimId` picks by the equipped weapon's inventory type the same way) and
//! the victim reacts: CombatWound (9) on a hit, CombatCritical (10) on a crit, Dodge
//! (30), Parry by weapon class (20-23) or ShieldBlock (24); a miss plays nothing. A
//! reaction does not cut the victim's own swing short (`ActionPriority::Reaction`).
//! While in combat a standing unit holds its weapon class Ready stance (25-28).
//! Clips a model lacks follow `AnimationData.Fallback`.

use std::collections::HashMap;
use std::path::Path;

use game_engine_core::spell_visual::read_animation_fallbacks;
use godot::prelude::*;
use shared::components::{EquipmentAppearance, EquipmentVisualSlot};
use shared::protocol::{CombatEvent, CombatEventType};

use super::{UnitNode, WorldUnits};
use crate::animation::{ActionPriority, WowAnimationPlayer};
use crate::world_models::{UnitAppearance, WorldModels};

/// `ItemSubclassWeapon` swung with the two-handed "loose" clips: polearm and staff.
const LOOSE_TWO_HAND_SUBCLASSES: [u8; 2] = [6, 10];
/// `INVTYPE_2HWEAPON`.
const INVTYPE_TWO_HAND: u8 = 17;
/// `INVTYPE_WEAPON`, `INVTYPE_WEAPONMAINHAND`.
const INVTYPE_ONE_HAND: [u8; 2] = [13, 21];

const COMBAT_WOUND: u16 = 9;
const COMBAT_CRITICAL: u16 = 10;
const SHIELD_BLOCK: u16 = 24;
const DODGE: u16 = 30;

/// The main-hand weapon class that selects melee, parry and ready clips.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub(crate) enum MeleeWeapon {
    #[default]
    Unarmed,
    OneHand,
    TwoHand,
    TwoHandLoose,
}

impl MeleeWeapon {
    /// Class of a main-hand item of `inventory_type` with weapon `subclass`.
    pub(crate) fn classify(inventory_type: u8, subclass: Option<u8>) -> Self {
        if inventory_type == INVTYPE_TWO_HAND {
            if subclass.is_some_and(|subclass| LOOSE_TWO_HAND_SUBCLASSES.contains(&subclass)) {
                Self::TwoHandLoose
            } else {
                Self::TwoHand
            }
        } else if INVTYPE_ONE_HAND.contains(&inventory_type) {
            Self::OneHand
        } else {
            Self::Unarmed
        }
    }

    fn clip(self, unarmed: u16) -> u16 {
        unarmed
            + match self {
                Self::Unarmed => 0,
                Self::OneHand => 1,
                Self::TwoHand => 2,
                Self::TwoHandLoose => 3,
            }
    }

    /// AttackUnarmed 16, Attack1H 17, Attack2H 18, Attack2HL 19.
    pub(crate) fn attack_anim(self) -> u16 {
        self.clip(16)
    }

    /// ParryUnarmed 20, Parry1H 21, Parry2H 22, Parry2HL 23.
    pub(crate) fn parry_anim(self) -> u16 {
        self.clip(20)
    }

    /// ReadyUnarmed 25, Ready1H 26, Ready2H 27, Ready2HL 28.
    pub(crate) fn ready_anim(self) -> u16 {
        self.clip(25)
    }
}

/// The main-hand weapon class of `equipment` and the weapon's `Item.SubclassID`, which
/// `subclass` resolves from an item id.
pub(crate) fn main_hand_weapon(
    equipment: &EquipmentAppearance,
    subclass: impl Fn(u32) -> Option<u8>,
) -> (MeleeWeapon, Option<u8>) {
    equipment
        .entries
        .iter()
        .find(|entry| entry.slot == EquipmentVisualSlot::MainHand && !entry.hidden)
        .map(|entry| {
            let subclass = entry.item_id.and_then(&subclass);
            (
                MeleeWeapon::classify(entry.inventory_type, subclass),
                subclass,
            )
        })
        .unwrap_or_default()
}

/// Stand becomes the weapon class Ready stance while the unit is in combat.
pub(crate) fn combat_stance(movement_id: u16, in_combat: bool, weapon: MeleeWeapon) -> u16 {
    if in_combat && movement_id == 0 {
        weapon.ready_anim()
    } else {
        movement_id
    }
}

/// Whether the outcome is a melee swing the attacker plays.
pub(crate) fn is_melee_swing(kind: &CombatEventType) -> bool {
    matches!(
        kind,
        CombatEventType::MeleeDamage
            | CombatEventType::CriticalHit
            | CombatEventType::Miss
            | CombatEventType::Dodge
            | CombatEventType::Parry
            | CombatEventType::Block
    )
}

/// The victim's reaction clip to a melee outcome.
pub(crate) fn melee_reaction(kind: &CombatEventType, weapon: MeleeWeapon) -> Option<u16> {
    match kind {
        CombatEventType::MeleeDamage => Some(COMBAT_WOUND),
        CombatEventType::CriticalHit => Some(COMBAT_CRITICAL),
        CombatEventType::Dodge => Some(DODGE),
        CombatEventType::Parry => Some(weapon.parry_anim()),
        CombatEventType::Block => Some(SHIELD_BLOCK),
        _ => None,
    }
}

impl UnitNode {
    /// The unit's bone animation player.
    pub(super) fn animation_player(&self) -> Option<Gd<WowAnimationPlayer>> {
        let path = match self.appearance.as_ref()? {
            UnitAppearance::Player(_, _) => "M2Animation",
            UnitAppearance::Creature { .. } => "NpcModel/M2Animation",
        };
        self.visual
            .as_ref()?
            .try_get_node_as::<WowAnimationPlayer>(path)
    }

    /// The model node whose skeleton carries the unit's attachments.
    pub(super) fn model_node(&self) -> Option<Gd<Node3D>> {
        let visual = self.visual.as_ref()?;
        match self.appearance.as_ref()? {
            UnitAppearance::Player(_, _) => Some(visual.clone()),
            UnitAppearance::Creature { .. } => visual.try_get_node_as::<Node3D>("NpcModel"),
        }
    }

    /// The equipment whose main hand selects the unit's weapon clips.
    pub(super) fn equipment(&self) -> Option<&EquipmentAppearance> {
        match self.appearance.as_ref()? {
            UnitAppearance::Creature { items, .. } => Some(items),
            UnitAppearance::Player(_, equipment) => Some(equipment),
        }
    }
}

/// The main-hand weapon class and subclass of `unit`'s equipment or virtual items.
pub(super) fn unit_weapon_class(
    unit: &UnitNode,
    models: &mut WorldModels,
) -> (MeleeWeapon, Option<u8>) {
    let Some(equipment) = unit.equipment() else {
        return (MeleeWeapon::Unarmed, None);
    };
    match models.gear() {
        Ok(gear) => main_hand_weapon(equipment, |item| gear.weapon_subclass(item)),
        Err(error) => {
            godot_error!("{} weapon class: {error}", unit.name);
            main_hand_weapon(equipment, |_| None)
        }
    }
}

/// The locomotion clip a unit plays: in combat, standing becomes the weapon class
/// Ready stance, or the `AnimationData.Fallback` the model has (Ready1H → ReadyUnarmed
/// → Stand).
pub(super) fn stance_clip(
    animation: &WowAnimationPlayer,
    movement_id: u16,
    in_combat: bool,
    weapon: MeleeWeapon,
    fallbacks: &HashMap<u16, u16>,
) -> u16 {
    let stance = combat_stance(movement_id, in_combat, weapon);
    if stance == movement_id {
        return movement_id;
    }
    animation
        .resolve_clip(stance, fallbacks)
        .unwrap_or(movement_id)
}

/// `AnimationData.Fallback` of `data_root`'s export, read into `slot` once; empty
/// (every clip must exist) when the export cannot be read, which is reported.
pub(super) fn load_fallbacks<'a>(
    slot: &'a mut Option<HashMap<u16, u16>>,
    data_root: &Path,
) -> &'a HashMap<u16, u16> {
    slot.get_or_insert_with(|| {
        read_animation_fallbacks(&data_root.join("db2/12.1.0.69933")).unwrap_or_else(|error| {
            godot_error!("Animation fallbacks: {error}");
            HashMap::new()
        })
    })
}

impl WorldUnits {
    /// Play clip `anim` on unit `id` over its locomotion (once, or held while
    /// `looping`); a clip the model lacks plays its fallback or nothing.
    pub fn play_unit_action(
        &mut self,
        id: u64,
        anim: u16,
        looping: bool,
        priority: ActionPriority,
    ) -> Result<Option<u16>, String> {
        let fallbacks = load_fallbacks(&mut self.anim_fallbacks, &self.data_root);
        let Some(unit) = self.units.get(&id) else {
            return Ok(None);
        };
        if unit.death_applied {
            return Ok(None);
        }
        let Some(mut animation) = unit.animation_player() else {
            return Ok(None);
        };
        animation
            .bind_mut()
            .play_action(anim, looping, priority, fallbacks)
            .map_err(|error| format!("{} action {anim}: {error}", unit.name))
    }

    /// Fade out the held clip `anim` of unit `id`.
    pub fn stop_unit_action(&mut self, id: u64, anim: u16) {
        if let Some(mut animation) = self.units.get(&id).and_then(UnitNode::animation_player) {
            animation.bind_mut().stop_action(anim);
        }
    }

    /// The attacker's swing and the victim's reaction to one melee outcome.
    pub fn apply_combat_event(&mut self, event: &CombatEvent) -> Result<(), String> {
        if !is_melee_swing(&event.event_type) {
            return Ok(());
        }
        let mut errors = Vec::new();
        let swing = self.unit_weapon(event.attacker).attack_anim();
        if let Err(error) =
            self.play_unit_action(event.attacker, swing, false, ActionPriority::Combat)
        {
            errors.push(error);
        }
        let victim_weapon = self.unit_weapon(event.target);
        if let Some(reaction) = melee_reaction(&event.event_type, victim_weapon)
            && let Err(error) =
                self.play_unit_action(event.target, reaction, false, ActionPriority::Reaction)
        {
            errors.push(error);
        }
        if errors.is_empty() {
            Ok(())
        } else {
            Err(errors.join("; "))
        }
    }

    /// The main-hand weapon class of unit `id`.
    pub(crate) fn unit_weapon(&self, id: u64) -> MeleeWeapon {
        self.units
            .get(&id)
            .map(|unit| unit.weapon)
            .unwrap_or_default()
    }

    /// The combat/spell clip layered over unit `id`'s locomotion.
    pub fn unit_action_id(&self, id: u64) -> Option<u16> {
        let animation = self.units.get(&id)?.animation_player()?;
        u16::try_from(animation.bind().action_id()).ok()
    }

    /// (unit, identifier) of the reported M2 events (`$SCD`, `$CSS`, `$CAH`) units'
    /// action clips passed since the last call.
    pub(crate) fn take_animation_events(&mut self) -> Vec<(u64, [u8; 4])> {
        self.units
            .iter()
            .filter_map(|(&id, unit)| Some((id, unit.animation_player()?)))
            .flat_map(|(id, mut animation)| {
                let fired = animation.bind_mut().take_fired_events();
                fired.into_iter().map(move |event| (id, event))
            })
            .collect()
    }

    /// Units whose death clip started since the last call.
    pub(crate) fn take_deaths(&mut self) -> Vec<u64> {
        std::mem::take(&mut self.deaths)
    }

    /// Unit `id`'s visible main-hand (`Item` ID, `ItemDisplayInfo` ID).
    pub(crate) fn unit_main_hand(&self, id: u64) -> (Option<u32>, Option<u32>) {
        self.units
            .get(&id)
            .and_then(UnitNode::equipment)
            .and_then(|equipment| {
                equipment
                    .entries
                    .iter()
                    .find(|entry| entry.slot == EquipmentVisualSlot::MainHand && !entry.hidden)
            })
            .map_or((None, None), |entry| (entry.item_id, entry.display_info_id))
    }

    /// `Item` ID of unit `id`'s worn chest, shown or hidden.
    pub(crate) fn unit_chest_item(&self, id: u64) -> Option<u32> {
        self.units
            .get(&id)
            .and_then(UnitNode::equipment)?
            .entries
            .iter()
            .find(|entry| entry.slot == EquipmentVisualSlot::Chest)?
            .item_id
    }

    /// Unit `id`'s cast clip has a missile release event yet to fire.
    pub(crate) fn unit_awaits_missile_release(&self, id: u64) -> bool {
        self.units
            .get(&id)
            .and_then(UnitNode::animation_player)
            .is_some_and(|animation| animation.bind().awaits_missile_release())
    }

    /// `Item.SubclassID` of unit `id`'s main-hand weapon.
    pub(crate) fn unit_main_hand_subclass(&self, id: u64) -> Option<u8> {
        self.units.get(&id)?.main_hand_subclass
    }

    /// The model node of unit `id` that carries its M2 attachments.
    pub fn unit_model_node(&self, id: u64) -> Option<Gd<Node3D>> {
        self.units.get(&id)?.model_node()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use shared::components::EquippedAppearanceEntry;

    fn main_hand(inventory_type: u8, item_id: u32) -> EquipmentAppearance {
        EquipmentAppearance {
            entries: vec![EquippedAppearanceEntry {
                slot: EquipmentVisualSlot::MainHand,
                item_id: Some(item_id),
                display_info_id: Some(1),
                inventory_type,
                hidden: false,
            }],
        }
    }

    #[test]
    fn weapon_class_selects_swing_parry_and_ready_clips() {
        // Worn Shortsword (one-hand sword), Worn Greatsword (two-hand sword), a staff.
        let subclasses = |item| match item {
            25 => Some(7),
            12282 => Some(8),
            35 => Some(10),
            _ => None,
        };
        let (sword, sword_subclass) = main_hand_weapon(&main_hand(13, 25), subclasses);
        let (greatsword, _) = main_hand_weapon(&main_hand(17, 12282), subclasses);
        let (staff, _) = main_hand_weapon(&main_hand(17, 35), subclasses);
        let (fists, no_weapon) = main_hand_weapon(&EquipmentAppearance::default(), subclasses);
        assert_eq!((sword_subclass, no_weapon), (Some(7), None));
        assert_eq!(
            [sword, greatsword, staff, fists].map(MeleeWeapon::attack_anim),
            [17, 18, 19, 16]
        );
        assert_eq!(greatsword.parry_anim(), 22);
        assert_eq!(sword.ready_anim(), 26);
    }

    #[test]
    fn standing_in_combat_holds_the_ready_stance_and_moving_keeps_locomotion() {
        assert_eq!(combat_stance(0, true, MeleeWeapon::OneHand), 26);
        assert_eq!(combat_stance(0, false, MeleeWeapon::OneHand), 0);
        assert_eq!(combat_stance(5, true, MeleeWeapon::TwoHand), 5);
    }

    #[test]
    fn swing_outcomes_select_the_victim_reaction() {
        let weapon = MeleeWeapon::OneHand;
        assert_eq!(
            melee_reaction(&CombatEventType::MeleeDamage, weapon),
            Some(9)
        );
        assert_eq!(melee_reaction(&CombatEventType::Dodge, weapon), Some(30));
        assert_eq!(melee_reaction(&CombatEventType::Parry, weapon), Some(21));
        assert_eq!(melee_reaction(&CombatEventType::Miss, weapon), None);
        assert!(is_melee_swing(&CombatEventType::Miss));
        assert!(!is_melee_swing(&CombatEventType::SpellDamage));
        assert!(!is_melee_swing(&CombatEventType::Death));
    }
}
