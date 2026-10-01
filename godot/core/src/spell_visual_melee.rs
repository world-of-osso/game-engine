//! Melee swing, impact and vocal sounds (12.1.0.69933 local-CASC exports).
//!
//! An attack clip's M2 events time them (wowdev.wiki/M2 Events): `$CSS`
//! "PlayWeaponSwooshSound ... depends on CGUnit_C::GetWeaponSwingType", then `$CAH`
//! "CGUnit_C::HandleCombatAnimEvent", where the swing lands on its victim.
//!
//! No retail source gives the rules beyond the table layouts. The 1.12.1 client's are
//! reverse-engineered by benilla (github.com/samwhosung/benilla `4772489a`,
//! `crates/benilla-app/src/sound/combat.rs`, cited by its reference addresses), and this
//! applies them to the retail tables. Where retail's tables outgrow 1.12, that is noted.
//!
//! - Swoosh (`0x624ca0`): a miss or dodge swings `(DONOTRENAME)Combat Miss 1H/2H`, SoundKits
//!   7080/7081 by whether the weapon's subclass is two-handed (retail keeps both kits, now
//!   holding `fx_misswhoosh_revamp_*`). A swing that connects swooshes the main-hand
//!   `ItemDisplayInfo.OverrideSwooshSoundKitID`, else `WeaponSwingSounds2` of
//!   (`ItemSubClass.WeaponSwingSize` as `SwingType`, crit) (wowdev.wiki/DB/WeaponSwingSounds2:
//!   SwingType "match with ItemSubClassRec::m_WeaponSwingSize"); bare hands are Light
//!   (`0x623892`). 1.12 plays nothing past its three types (`0x457f63`); retail's Dagger
//!   and Fishing Pole size 8 is no `SwingType` (0-5: Light, Medium, Heavy, Agile, Pierce,
//!   Large Monster, WoWDBDefs `WeaponSwingType`), so they swoosh only when they miss. How
//!   retail maps size 8 is unknown.
//! - Impact (`0x6247d0`): `WeaponImpactSounds` of the attacker's (`WeaponSubClassID`,
//!   `ParrySoundType`, `ImpactSource`), its `ImpactSoundID` (`CritImpactSoundID` on a crit)
//!   at the victim's slot (benilla `weapon_impact.rs:15-25`: 0 flesh, 1 chain, 2 plate, 3/4
//!   metal/wood shield, 5/6 metal/wood parry, 7 wood, 8 stone, 9 ethereal; retail's slot 10
//!   holds `*_hit_leatherarmor_*` files and no rule reaches it).
//!   - `ParrySoundType` (wowdev.wiki/DB/WeaponImpactSounds: WOOD 0, METAL 1) is the weapon's
//!     `Material.Flags & 1` (`0x457e80`): leather and cloth weapons are wood.
//!   - A player victim presents its chest item's `Material.Flags` (`0x62fb70`): 0x2 plate,
//!     else 0x4 chain, else flesh (leather, cloth). 1.12 reads only its own player's
//!     inventory; retail replicates every player's visible items, so every player does.
//!   - A creature's `CreatureSoundData.CreatureImpactType` maps through
//!     `s_creatureIpactSounds` = {FLESH 0, STONE 8, WOOD 7, ETHEREAL 9}
//!     (wowdev.wiki/DB/CreatureSoundData); 1.12 refuses types from 4 (`0x6238f0`), which
//!     then land on flesh. Retail's types 4-6 (142 rows: 4 is mechanical models such as
//!     mechagnomes and golems, 5-6 a few oddities) are undocumented.
//!   - A parry strikes the parrying weapon (`0x457dc0`), not crit-tiered: 5 metal, 6 wood;
//!     bare hands are not metal.
//!   - A block (`HITINFO_BLOCK`, a hit the shield partly stopped) strikes the victim's
//!     off-hand shield, not crit-tiered: 3 metal, 4 wood by the shield's `Material.Flags & 1`
//!     as for a weapon. benilla does not cover blocks; the slots follow the files (`shield_metal`,
//!     `shield_wood`). A victim without a shield is struck as on a hit.
//!   - `ImpactSource` (inferred from the files): 1 rows hold the player sets
//!     (`*_combatrevamp`, `1h_sword_hit_*`), 0 rows the `*_npc_*` and pre-revamp sets. A
//!     player swings 1, a creature 0; a subclass without that row takes its closest row
//!     (same source, then same parry material; 1.12's lookup also takes the other
//!     material, `WeaponImpactCatalog::get`).
//!   - The `Pierce*` columns (7.3.0, with `ImpactSource`) are unknown and not played; 12 of
//!     45 rows set them, nearly all equal to the plain columns.
//! - Bare hands strike with the display's `CreatureDisplayInfo.UnarmedWeaponType` subclass
//!   (11 Bear Claws, 12 Cat Claws, ...), -1 being Fist Weapon (13), 1.12's unarmed row.
//! - `Item.Sound_override_subclassID`, when set, replaces the weapon's subclass.
//! - Vocals (`0x623520`: a roll in 0..=100 passes at most its class threshold): the
//!   attacker's `SoundExertionID` when the swing arrives, unless it missed (`0x62476a`),
//!   70 for a creature and 35 for a player; a crit plays `SoundExertionCriticalID` always.
//!   A wounded victim's injury (`spell_visual_voice`) 60 for a creature and 30 for a
//!   player; a crit always.

use std::collections::HashMap;
use std::path::Path;

use serde::{Deserialize, Serialize};

use super::{KitSound, SpellVisualCatalog, Table, UnitSound, VoiceSource};

/// `ItemClass` Weapon and Armor.
const WEAPON_CLASS: i64 = 2;
const ARMOR_CLASS: i64 = 4;
/// `ItemSubClass` Shield of class Armor.
const SHIELD_SUBCLASS: i64 = 6;
/// `Material.Flags` (WoWDBDefs `MaterialFlags`; benilla `material.rs:30-51`).
const MATERIAL_METAL: i64 = 0x1;
const MATERIAL_PLATE: i64 = 0x2;
const MATERIAL_CHAIN: i64 = 0x4;
/// `ItemSubClass` Fist Weapon: what a display with `UnarmedWeaponType` -1 strikes with.
const FIST_WEAPON: u8 = 13;
/// Two-handed subclasses, whose misses swing the 2H miss whoosh (benilla `combat.rs:77`):
/// Axe 2H, Mace 2H, Polearm, Sword 2H, Staff, Spear.
const TWO_HANDED: [u8; 6] = [1, 5, 6, 8, 10, 17];
/// `(DONOTRENAME)Combat Miss 1H/2H` (benilla `combat.rs:52-53`).
const COMBAT_MISS_1H: u32 = 7080;
const COMBAT_MISS_2H: u32 = 7081;
/// `WeaponSwingSounds2.SwingType` Light, an empty hand's swing (`0x623892`).
const SWING_LIGHT: u8 = 0;
/// `WeaponImpactSounds` array length.
const IMPACT_SLOTS: usize = 11;
/// Impact slots (benilla `weapon_impact.rs:15-25`).
const SLOT_FLESH: usize = 0;
const SLOT_CHAIN: usize = 1;
const SLOT_PLATE: usize = 2;
const SLOT_SHIELD_METAL: usize = 3;
const SLOT_SHIELD_WOOD: usize = 4;
const SLOT_PARRY_METAL: usize = 5;
const SLOT_PARRY_WOOD: usize = 6;
const SLOT_WOOD: usize = 7;
const SLOT_STONE: usize = 8;
const SLOT_ETHEREAL: usize = 9;

/// A weapon's `PARRYMATERIALS` (`WeaponImpactSounds.ParrySoundType`).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
enum ParryMaterial {
    Wood = 0,
    Metal = 1,
}

/// What a unit swings or parries with: its visible main-hand item, else its bare hands,
/// and the chest item a player victim presents.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct MeleeHand {
    /// Main-hand `Item` ID and its `ItemDisplayInfo` ID.
    pub item_id: Option<u32>,
    pub display_info_id: Option<u32>,
    /// Worn chest `Item` ID.
    pub chest_item_id: Option<u32>,
    /// Off-hand shield `Item` ID.
    pub shield_item_id: Option<u32>,
    /// The unit's voice, whose display gives bare hands and players `ImpactSource` 1.
    pub unit: VoiceSource,
}

/// How a melee swing ended.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SwingResult {
    Hit {
        critical: bool,
    },
    /// A hit partly stopped by the victim's shield.
    Block,
    Parry,
    Dodge,
    Miss,
}

impl SwingResult {
    fn critical(self) -> bool {
        self == Self::Hit { critical: true }
    }
}

/// A melee vocal and its chance class (benilla `kit.rs:150-174`).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum MeleeVocal {
    /// The attacker's, when its swing arrives.
    Exertion,
    /// The victim's, when the swing lands a hit.
    Injury,
}

impl MeleeVocal {
    /// The class threshold: creature, player (`0x8626d4`, `0x86424c`).
    fn threshold(self, source: VoiceSource) -> u32 {
        let player = matches!(source, VoiceSource::Player { .. });
        match (self, player) {
            (Self::Exertion, false) => 70,
            (Self::Exertion, true) => 35,
            (Self::Injury, false) => 60,
            (Self::Injury, true) => 30,
        }
    }
}

#[derive(Clone, Copy, Debug, Serialize, Deserialize)]
struct WeaponItem {
    subclass: u8,
    material: ParryMaterial,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
struct ImpactRow {
    id: u32,
    subclass: u8,
    parry: ParryMaterial,
    source: u8,
    hit: [u32; IMPACT_SLOTS],
    critical: [u32; IMPACT_SLOTS],
}

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub(super) struct MeleeSounds {
    /// `WeaponSwingSounds2` sound kit of each (`SwingType`, `Crit`).
    swings: HashMap<(u8, bool), u32>,
    /// `ItemSubClass.WeaponSwingSize` of each weapon subclass.
    swing_sizes: HashMap<u8, u8>,
    impacts: Vec<ImpactRow>,
    /// Weapons (`Item` class 2) by item ID.
    weapons: HashMap<u32, WeaponItem>,
    /// Chain or plate impact slot of the armour (`Item` class 4) that presents one.
    armour: HashMap<u32, usize>,
    /// Shields (`Item` class 4, subclass 6): their material.
    shields: HashMap<u32, ParryMaterial>,
    /// `ItemDisplayInfo.OverrideSwooshSoundKitID` of the displays that set it.
    swooshes: HashMap<u32, u32>,
    /// `CreatureDisplayInfo.UnarmedWeaponType` of the displays that set it.
    unarmed: HashMap<u32, u8>,
}

impl MeleeSounds {
    pub(super) fn sound_kits(&self) -> impl Iterator<Item = u32> + '_ {
        self.swings
            .values()
            .chain(self.swooshes.values())
            .copied()
            .chain([COMBAT_MISS_1H, COMBAT_MISS_2H])
            .chain(
                self.impacts
                    .iter()
                    .flat_map(|row| row.hit.iter().chain(&row.critical).copied()),
            )
    }
}

impl SpellVisualCatalog {
    pub(super) fn read_melee(&mut self, dir: &Path) -> Result<(), String> {
        let flags = read_material_flags(dir)?;
        let (weapons, armour, shields) = read_items(dir, &flags)?;
        self.melee = MeleeSounds {
            swings: read_swings(dir)?,
            swing_sizes: read_swing_sizes(dir)?,
            impacts: read_impacts(dir)?,
            weapons,
            armour,
            shields,
            swooshes: read_at_least(dir, "ItemDisplayInfo", "OverrideSwooshSoundKitID", 1)?,
            unarmed: read_at_least(dir, "CreatureDisplayInfo", "UnarmedWeaponType", 0)?,
        };
        Ok(())
    }

    /// The swoosh `hand` swings with (`$CSS`) for a swing that ended in `result`.
    pub fn swing_sound(&self, hand: MeleeHand, result: SwingResult) -> Option<&KitSound> {
        let melee = &self.melee;
        let kit = match result {
            SwingResult::Miss | SwingResult::Dodge => {
                if TWO_HANDED.contains(&self.weapon_of(hand).0) {
                    COMBAT_MISS_2H
                } else {
                    COMBAT_MISS_1H
                }
            }
            SwingResult::Hit { .. } | SwingResult::Block | SwingResult::Parry => {
                match hand.display_info_id.and_then(|id| melee.swooshes.get(&id)) {
                    Some(&kit) => kit,
                    None => *melee
                        .swings
                        .get(&(self.swing_type(hand)?, result.critical()))?,
                }
            }
        };
        self.playable(kit)
    }

    /// The impact of `attacker`'s swing on `victim` (`$CAH`); a parrying victim parries
    /// with what its hand holds.
    pub fn impact_sound(
        &self,
        attacker: MeleeHand,
        victim: MeleeHand,
        result: SwingResult,
    ) -> Option<&KitSound> {
        let slot = match result {
            SwingResult::Miss | SwingResult::Dodge => return None,
            SwingResult::Parry => match self.weapon_of(victim).1 {
                ParryMaterial::Metal => SLOT_PARRY_METAL,
                ParryMaterial::Wood => SLOT_PARRY_WOOD,
            },
            SwingResult::Block => match victim
                .shield_item_id
                .and_then(|id| self.melee.shields.get(&id))
            {
                Some(ParryMaterial::Metal) => SLOT_SHIELD_METAL,
                Some(ParryMaterial::Wood) => SLOT_SHIELD_WOOD,
                None => self.hit_slot(victim),
            },
            SwingResult::Hit { .. } => self.hit_slot(victim),
        };
        let (subclass, parry) = self.weapon_of(attacker);
        let source = u8::from(matches!(attacker.unit, VoiceSource::Player { .. }));
        let row = self
            .melee
            .impacts
            .iter()
            .filter(|row| row.subclass == subclass)
            .max_by_key(|row| {
                (
                    row.source == source,
                    row.parry == parry,
                    std::cmp::Reverse(row.id),
                )
            })?;
        let kits = if result.critical() {
            &row.critical
        } else {
            &row.hit
        };
        self.playable(kits[slot])
    }

    /// The exertion `attacker` voices when its swing arrives, if `roll` (any `u32`) passes
    /// its chance; a miss voices none.
    pub fn exertion_sound(
        &self,
        attacker: VoiceSource,
        result: SwingResult,
        roll: u32,
    ) -> Option<&KitSound> {
        if result == SwingResult::Miss
            || !vocal_passes(MeleeVocal::Exertion, attacker, result, roll)
        {
            return None;
        }
        let sound = if result.critical() {
            UnitSound::ExertionCritical
        } else {
            UnitSound::Exertion
        };
        self.unit_sound(attacker, sound)
    }

    /// The injury `victim` voices when a swing lands a hit on it, if `roll` passes its
    /// chance.
    pub fn injury_sound(
        &self,
        victim: VoiceSource,
        result: SwingResult,
        roll: u32,
    ) -> Option<&KitSound> {
        let critical = match result {
            SwingResult::Hit { critical } => critical,
            // A blocked swing still hits (`VICTIMSTATE_HIT`, Unit.cpp:1461).
            SwingResult::Block => false,
            _ => return None,
        };
        vocal_passes(MeleeVocal::Injury, victim, result, roll)
            .then(|| self.wound_sound(victim, critical))
            .flatten()
    }

    /// The `WeaponSwingSounds2.SwingType` `hand` swings: its weapon's size, Light bare.
    /// A held non-weapon swings nothing (`0x6238b7`).
    fn swing_type(&self, hand: MeleeHand) -> Option<u8> {
        let Some(item) = hand.item_id else {
            return Some(SWING_LIGHT);
        };
        let weapon = self.melee.weapons.get(&item)?;
        self.melee.swing_sizes.get(&weapon.subclass).copied()
    }

    /// The impact slot a hit on `victim` strikes.
    fn hit_slot(&self, victim: MeleeHand) -> usize {
        match victim.unit {
            VoiceSource::Player { .. } => victim
                .chest_item_id
                .and_then(|id| self.melee.armour.get(&id).copied())
                .unwrap_or(SLOT_FLESH),
            VoiceSource::Creature { .. } => creature_slot(self.impact_type(victim.unit)),
        }
    }

    /// The (sound subclass, parry material) of what `hand` holds.
    fn weapon_of(&self, hand: MeleeHand) -> (u8, ParryMaterial) {
        if let Some(weapon) = hand.item_id.and_then(|id| self.melee.weapons.get(&id)) {
            return (weapon.subclass, weapon.material);
        }
        let subclass = self
            .display_of(hand.unit)
            .and_then(|display| self.melee.unarmed.get(&display).copied())
            .unwrap_or(FIST_WEAPON);
        (subclass, ParryMaterial::Wood)
    }

    fn playable(&self, kit: u32) -> Option<&KitSound> {
        self.sound_kits
            .get(&kit)
            .filter(|sound| !sound.files.is_empty())
    }
}

/// Whether `roll` passes `vocal`'s chance for `source` (`0x623520`: `MulHi32(101, roll)` in
/// 0..=100 at most the threshold); a crit always does.
fn vocal_passes(vocal: MeleeVocal, source: VoiceSource, result: SwingResult, roll: u32) -> bool {
    result.critical() || ((101 * u64::from(roll)) >> 32) as u32 <= vocal.threshold(source)
}

/// The impact slot of `CreatureImpactType` (`s_creatureIpactSounds`; types from 4 flesh).
fn creature_slot(impact_type: u8) -> usize {
    match impact_type {
        1 => SLOT_STONE,
        2 => SLOT_WOOD,
        3 => SLOT_ETHEREAL,
        _ => SLOT_FLESH,
    }
}

fn read_swings(dir: &Path) -> Result<HashMap<(u8, bool), u32>, String> {
    Ok(Table::read(dir, "WeaponSwingSounds2")?
        .ints(["SwingType", "Crit", "SoundID"])?
        .into_iter()
        .map(|[swing_type, crit, kit]| ((swing_type as u8, crit != 0), kit as u32))
        .collect())
}

fn read_swing_sizes(dir: &Path) -> Result<HashMap<u8, u8>, String> {
    Ok(Table::read(dir, "ItemSubClass")?
        .ints(["ClassID", "SubClassID", "WeaponSwingSize"])?
        .into_iter()
        .filter(|&[class, ..]| class == WEAPON_CLASS)
        .map(|[_, subclass, size]| (subclass as u8, size as u8))
        .collect())
}

/// `Material.Flags` by material ID.
fn read_material_flags(dir: &Path) -> Result<HashMap<i64, i64>, String> {
    Ok(Table::read(dir, "Material")?
        .ints(["ID", "Flags"])?
        .into_iter()
        .map(|[id, flags]| (id, flags))
        .collect())
}

/// Each weapon's sound subclass (`Sound_override_subclassID` when set) and parry material,
/// each chain or plate armour's impact slot, and each shield's material.
type ItemTables = (
    HashMap<u32, WeaponItem>,
    HashMap<u32, usize>,
    HashMap<u32, ParryMaterial>,
);

fn read_items(dir: &Path, flags: &HashMap<i64, i64>) -> Result<ItemTables, String> {
    let rows = Table::read(dir, "Item")?.ints([
        "ID",
        "ClassID",
        "SubclassID",
        "Material",
        "Sound_override_subclassID",
    ])?;
    let mut weapons = HashMap::new();
    let mut armour = HashMap::new();
    let mut shields = HashMap::new();
    for [id, class, subclass, material, sound_subclass] in rows {
        let flags = flags.get(&material).copied().unwrap_or(0);
        if class == WEAPON_CLASS {
            weapons.insert(id as u32, weapon_item(subclass, sound_subclass, flags));
        } else if class == ARMOR_CLASS {
            armour.extend(armour_slot(flags).map(|slot| (id as u32, slot)));
            if subclass == SHIELD_SUBCLASS {
                shields.insert(id as u32, weapon_item(subclass, -1, flags).material);
            }
        }
    }
    Ok((weapons, armour, shields))
}

/// A weapon's sound subclass (`Sound_override_subclassID` when set) and parry material.
fn weapon_item(subclass: i64, sound_subclass: i64, material_flags: i64) -> WeaponItem {
    let subclass = if sound_subclass >= 0 {
        sound_subclass
    } else {
        subclass
    };
    let material = if material_flags & MATERIAL_METAL != 0 {
        ParryMaterial::Metal
    } else {
        ParryMaterial::Wood
    };
    WeaponItem {
        subclass: subclass as u8,
        material,
    }
}

/// The plate or chain impact slot armour of `material_flags` presents, if any.
fn armour_slot(material_flags: i64) -> Option<usize> {
    if material_flags & MATERIAL_PLATE != 0 {
        Some(SLOT_PLATE)
    } else if material_flags & MATERIAL_CHAIN != 0 {
        Some(SLOT_CHAIN)
    } else {
        None
    }
}

/// `column` of `table`'s rows whose value is at least `min`, by ID.
fn read_at_least<T: TryFrom<i64>>(
    dir: &Path,
    table: &str,
    column: &str,
    min: i64,
) -> Result<HashMap<u32, T>, String> {
    Ok(Table::read(dir, table)?
        .ints(["ID", column])?
        .into_iter()
        .filter(|&[_, value]| value >= min)
        .filter_map(|[id, value]| Some((id as u32, T::try_from(value).ok()?)))
        .collect())
}

fn read_impacts(dir: &Path) -> Result<Vec<ImpactRow>, String> {
    let table = Table::read(dir, "WeaponImpactSounds")?;
    let keys = table.ints(["ID", "WeaponSubClassID", "ParrySoundType", "ImpactSource"])?;
    let hits = impact_columns(&table, "ImpactSoundID")?;
    let crits = impact_columns(&table, "CritImpactSoundID")?;
    Ok(keys
        .into_iter()
        .zip(hits)
        .zip(crits)
        .map(
            |(([id, subclass, parry, source], hit), critical)| ImpactRow {
                id: id as u32,
                subclass: subclass as u8,
                parry: if parry == 0 {
                    ParryMaterial::Wood
                } else {
                    ParryMaterial::Metal
                },
                source: source as u8,
                hit,
                critical,
            },
        )
        .collect())
}

fn impact_columns(table: &Table, name: &str) -> Result<Vec<[u32; IMPACT_SLOTS]>, String> {
    let names: Vec<String> = (0..IMPACT_SLOTS)
        .map(|slot| format!("{name}_{slot}"))
        .collect();
    let names: [&str; IMPACT_SLOTS] = std::array::from_fn(|slot| names[slot].as_str());
    Ok(table
        .ints(names)?
        .into_iter()
        .map(|row| row.map(|kit| kit as u32))
        .collect())
}
