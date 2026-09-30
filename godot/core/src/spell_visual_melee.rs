//! Melee swing and impact sounds (12.1.0.69933 local-CASC exports).
//!
//! An attack clip's M2 events time them (wowdev.wiki/M2 Events): `$CSS`
//! "PlayWeaponSwooshSound ... depends on CGUnit_C::GetWeaponSwingType", then `$CAH`
//! "CGUnit_C::HandleCombatAnimEvent", where the swing lands on its victim. A miss or
//! dodge is the swoosh alone.
//!
//! - Swoosh: the main-hand `ItemDisplayInfo.OverrideSwooshSoundKitID`, else
//!   `WeaponSwingSounds2` of (`ItemSubClass.WeaponSwingSize` as `SwingType`, `Crit`)
//!   (wowdev.wiki/DB/WeaponSwingSounds2: SwingType "match with
//!   ItemSubClassRec::m_WeaponSwingSize"). Sizes 0-2 are Light, Medium, Heavy; the Dagger
//!   and Fishing Pole size 8 is no `SwingType` and has no swoosh.
//! - Impact: `WeaponImpactSounds` of the attacker's (`WeaponSubClassID`,
//!   `ParrySoundType`, `ImpactSource`), its `ImpactSoundID` (`CritImpactSoundID` on a
//!   crit) at the victim's impact index. `ParrySoundType` is the weapon's
//!   PARRYMATERIAL (wowdev.wiki/DB/WeaponImpactSounds: WOOD 0, METAL 1): `Item.Material`
//!   2 (Wood) is wood, other materials metal. The index's materials are inferred from
//!   the files the columns hold (e.g. row 8, a player's one-hand sword: 0 `hit_flesh`,
//!   1 `armor_chain`, 2 `armor_plate`, 3 `shield_metal`, 4 `shield_wood`, 5
//!   `metal_parry`, 6 `wood_parry`, 7 `body_wood`, 8 `body_stone`, 9 `ethereal`). A hit
//!   lands on `CreatureSoundData.CreatureImpactType` through the client's
//!   `s_creatureIpactSounds` = {FLESH 0, STONE 8, WOOD 7, ETHEREAL 9}
//!   (wowdev.wiki/DB/CreatureSoundData); types 4-6 are not documented and play nothing.
//!   A parry lands on the parrying weapon: 5 metal, 6 wood. A block
//!   (`HITINFO_BLOCK`, a hit whose damage the shield partly stopped) lands on the victim's
//!   shield: 3 metal, 4 wood (inferred from the `shield_metal` / `shield_wood` files), by
//!   the shield's `Item.Material` as a weapon's parry material; a victim without a shield
//!   is struck like on a hit.
//! - `ImpactSource` (inferred): 1 rows hold the player sets (`*_combatrevamp`,
//!   `1h_sword_hit_*`), 0 rows the `*_npc_*` and pre-revamp sets. A player swings 1, a
//!   creature 0; a subclass without that row takes its closest row (same source, then
//!   same parry material).
//! - A bare hand swings its display's `CreatureDisplayInfo.UnarmedWeaponType` subclass
//!   (11 Bear Claws, 12 Cat Claws, ...), -1 being Fist Weapon (13), whose rows hold the
//!   `unarmed*` files; bare hands parry as wood.
//! - `Item.Sound_override_subclassID`, when set, replaces the weapon's subclass.
//!
//! Not modelled: armour (a player's chain or plate impact), the `Pierce*` columns, and the attacker's `SoundExertionID` voice (what
//! triggers it on a plain swing is not documented).

use std::collections::HashMap;
use std::path::Path;

use serde::{Deserialize, Serialize};

use super::{KitSound, SpellVisualCatalog, Table, VoiceSource};

/// `ItemClass` Weapon.
const WEAPON_CLASS: i64 = 2;
/// `ItemClass` Armor, subclass Shield.
const ARMOR_CLASS: i64 = 4;
const SHIELD_SUBCLASS: i64 = 6;
/// `Item.Material` Wood.
const MATERIAL_WOOD: i64 = 2;
/// `ItemSubClass` Fist Weapon: what a display with `UnarmedWeaponType` -1 swings.
const FIST_WEAPON: u8 = 13;
/// `WeaponImpactSounds` array length.
const IMPACT_SLOTS: usize = 11;
/// Impact indices of a parry by a metal and a wooden weapon.
const PARRY_METAL: usize = 5;
const PARRY_WOOD: usize = 6;
/// Impact indices of a block by a metal and a wooden shield.
const SHIELD_METAL: usize = 3;
const SHIELD_WOOD: usize = 4;

/// A weapon's `PARRYMATERIALS` (`WeaponImpactSounds.ParrySoundType`).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
enum ParryMaterial {
    Wood = 0,
    Metal = 1,
}

/// What a unit swings or parries with: its visible main-hand item, else its bare hands.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct MeleeHand {
    /// Main-hand `Item` ID and its `ItemDisplayInfo` ID.
    pub item_id: Option<u32>,
    pub display_info_id: Option<u32>,
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
    Parry,
    /// A hit partly stopped by the victim's shield.
    Block,
    /// Miss or dodge: nothing is struck.
    Avoided,
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
    /// Shields (`Item` class 4, subclass 6) by item ID: their material.
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
            .chain(
                self.impacts
                    .iter()
                    .flat_map(|row| row.hit.iter().chain(&row.critical).copied()),
            )
    }
}

impl SpellVisualCatalog {
    pub(super) fn read_melee(&mut self, dir: &Path) -> Result<(), String> {
        self.melee = MeleeSounds {
            swings: read_swings(dir)?,
            swing_sizes: read_swing_sizes(dir)?,
            impacts: read_impacts(dir)?,
            weapons: read_weapons(dir)?,
            shields: read_shields(dir)?,
            swooshes: read_at_least(dir, "ItemDisplayInfo", "OverrideSwooshSoundKitID", 1)?,
            unarmed: read_at_least(dir, "CreatureDisplayInfo", "UnarmedWeaponType", 0)?,
        };
        Ok(())
    }

    /// The swoosh `hand` swings with (`$CSS`).
    pub fn swing_sound(&self, hand: MeleeHand, critical: bool) -> Option<&KitSound> {
        let melee = &self.melee;
        let kit = match hand.display_info_id.and_then(|id| melee.swooshes.get(&id)) {
            Some(&kit) => kit,
            None => {
                let (subclass, _) = self.weapon_of(hand);
                let size = *melee.swing_sizes.get(&subclass)?;
                *melee.swings.get(&(size, critical))?
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
        let (index, critical) = match result {
            SwingResult::Avoided => return None,
            SwingResult::Parry => match self.weapon_of(victim).1 {
                ParryMaterial::Metal => (PARRY_METAL, false),
                ParryMaterial::Wood => (PARRY_WOOD, false),
            },
            SwingResult::Block => match victim
                .shield_item_id
                .and_then(|id| self.melee.shields.get(&id))
            {
                Some(ParryMaterial::Metal) => (SHIELD_METAL, false),
                Some(ParryMaterial::Wood) => (SHIELD_WOOD, false),
                None => (hit_index(self.impact_type(victim.unit))?, false),
            },
            SwingResult::Hit { critical } => (hit_index(self.impact_type(victim.unit))?, critical),
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
        let kits = if critical { &row.critical } else { &row.hit };
        self.playable(kits[index])
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

/// The impact index of `CreatureImpactType` (`s_creatureIpactSounds`).
fn hit_index(impact_type: u8) -> Option<usize> {
    match impact_type {
        0 => Some(0),
        1 => Some(8),
        2 => Some(7),
        3 => Some(9),
        _ => None,
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

/// Each weapon's sound subclass (`Sound_override_subclassID` when set) and material.
fn read_weapons(dir: &Path) -> Result<HashMap<u32, WeaponItem>, String> {
    let rows = Table::read(dir, "Item")?.ints([
        "ID",
        "ClassID",
        "SubclassID",
        "Material",
        "Sound_override_subclassID",
    ])?;
    Ok(rows
        .into_iter()
        .filter(|&[_, class, ..]| class == WEAPON_CLASS)
        .map(|[id, _, subclass, material, sound_subclass]| {
            let subclass = if sound_subclass >= 0 {
                sound_subclass
            } else {
                subclass
            };
            let material = if material == MATERIAL_WOOD {
                ParryMaterial::Wood
            } else {
                ParryMaterial::Metal
            };
            let weapon = WeaponItem {
                subclass: subclass as u8,
                material,
            };
            (id as u32, weapon)
        })
        .collect())
}

/// Shields by item ID: `Item.Material` 2 (Wood) is wood, other materials metal.
fn read_shields(dir: &Path) -> Result<HashMap<u32, ParryMaterial>, String> {
    let rows = Table::read(dir, "Item")?.ints(["ID", "ClassID", "SubclassID", "Material"])?;
    Ok(rows
        .into_iter()
        .filter(|&[_, class, subclass, _]| class == ARMOR_CLASS && subclass == SHIELD_SUBCLASS)
        .map(|[id, _, _, material]| {
            let material = if material == MATERIAL_WOOD {
                ParryMaterial::Wood
            } else {
                ParryMaterial::Metal
            };
            (id as u32, material)
        })
        .collect())
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
