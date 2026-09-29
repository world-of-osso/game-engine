//! Retail spell visuals from the local-CASC DB2 exports (`scripts/export_db2_csv.py`):
//! `SpellXSpellVisual` picks a spell's `SpellVisual` by priority among the rows whose
//! caster `PlayerCondition` holds (TrinityCore `ConditionMgr::IsPlayerMeetingCondition`);
//! `SpellVisualEvent` starts a `SpellVisualKit` on the caster or target at a cast event;
//! the kit's `SpellVisualKitEffect` rows are model attachments
//! (`SpellVisualKitModelAttach` → `SpellVisualEffectName` model) and a unit animation
//! (`SpellVisualAnim`, directly or through `AnimKit` → `AnimKitSegment`). Missiles come
//! from the visual's `SpellVisualMissile` set. `AnimationData.Fallback` substitutes a
//! clip a model lacks.

use std::collections::HashMap;
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

use crate::csv_util::{parse_csv_line, parse_csv_records};
use crate::db2_cache::{CacheKey, load_or_build};

const DB2_BUILD: &str = "12.1.0.69933";
/// Bump when the cached catalog layout or its build rules change.
const CACHE_FORMAT: u32 = 1;

const SOURCE_TABLES: [&str; 11] = [
    "SpellXSpellVisual",
    "SpellVisual",
    "SpellVisualEvent",
    "SpellVisualKitEffect",
    "SpellVisualKitModelAttach",
    "SpellVisualEffectName",
    "SpellVisualAnim",
    "SpellVisualMissile",
    "AnimKitSegment",
    "AnimationData",
    "PlayerCondition",
];

/// `SpellVisualKitEffect.EffectType` of a model attachment and a unit animation.
const EFFECT_MODEL_ATTACH: u32 = 2;
const EFFECT_ANIM: u32 = 6;

/// `PlayerCondition.Flags` (TrinityCore `PlayerConditionFlags`).
const CONDITION_INVERT: i64 = 0x0008;
const CONDITION_DISABLED: i64 = 0x0100;

/// `SpellVisualEvent.StartEvent` / `EndEvent`.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum VisualEvent {
    PrecastStart,
    PrecastEnd,
    Cast,
    TravelStart,
    TravelEnd,
    Impact,
    AuraStart,
    AuraEnd,
    ChannelStart,
    ChannelEnd,
    OneShot,
    Other(u32),
}

impl VisualEvent {
    fn from_db2(value: u32) -> Self {
        match value {
            1 => Self::PrecastStart,
            2 => Self::PrecastEnd,
            3 => Self::Cast,
            4 => Self::TravelStart,
            5 => Self::TravelEnd,
            6 => Self::Impact,
            7 => Self::AuraStart,
            8 => Self::AuraEnd,
            11 => Self::ChannelStart,
            12 => Self::ChannelEnd,
            13 => Self::OneShot,
            other => Self::Other(other),
        }
    }
}

/// Unit a kit plays on (`SpellVisualEvent.TargetType`): 1 is the caster; 2 and 4 are
/// hit units (4 dominates melee impacts). 3 (area/destination) and 5 (missile) are
/// not unit kits.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum KitTarget {
    Caster,
    Target,
    Other(u32),
}

impl KitTarget {
    fn from_db2(value: u32) -> Self {
        match value {
            1 => Self::Caster,
            2 | 4 => Self::Target,
            other => Self::Other(other),
        }
    }
}

/// What a caster's `PlayerCondition` is evaluated against.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct CasterContext {
    pub race: u8,
    pub class: u8,
    pub gender: u8,
    pub level: u32,
    /// `ChrSpecialization.OrderIndex` of the primary spec; `None` passes spec checks.
    pub spec_order_index: Option<u8>,
    /// `Item.SubclassID` of the equipped main-hand weapon.
    pub main_hand_subclass: Option<u8>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
struct PlayerCondition {
    flags: i64,
    class_mask: i64,
    race_mask: u64,
    gender: i64,
    min_level: i64,
    max_level: i64,
    spec_index: i64,
    weapon_subclass_mask: i64,
    /// Some other requirement the client cannot evaluate here is set.
    unsupported: bool,
}

impl PlayerCondition {
    /// `ConditionMgr::IsPlayerMeetingCondition` over the evaluable fields, inverted by
    /// `Invert`. A condition with an unsupported requirement never holds.
    fn holds(&self, caster: &CasterContext) -> bool {
        if self.unsupported {
            return false;
        }
        self.meets(caster) != (self.flags & CONDITION_INVERT != 0)
    }

    fn meets(&self, caster: &CasterContext) -> bool {
        if self.flags & CONDITION_DISABLED != 0 {
            return true;
        }
        let level = i64::from(caster.level);
        let race_bit = 1u64.checked_shl(u32::from(caster.race).wrapping_sub(1));
        let checks = [
            self.min_level == 0 || level >= self.min_level,
            self.max_level == 0 || level <= self.max_level,
            self.race_mask == 0 || race_bit.is_some_and(|bit| self.race_mask & bit != 0),
            self.class_mask == 0 || self.class_mask & (1 << (caster.class.max(1) - 1)) != 0,
            self.gender < 0 || self.gender == i64::from(caster.gender),
            self.spec_index < 0
                || caster
                    .spec_order_index
                    .is_none_or(|index| i64::from(index) == self.spec_index),
            self.weapon_subclass_mask == 0
                || caster
                    .main_hand_subclass
                    .is_some_and(|subclass| self.weapon_subclass_mask & (1 << subclass) != 0),
        ];
        checks.into_iter().all(|check| check)
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
struct VisualChoice {
    visual_id: u32,
    priority: i64,
    difficulty: i64,
    caster_condition: u32,
    /// A unit or viewer condition is required; those are not evaluated.
    other_condition: bool,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
struct EventRow {
    start: VisualEvent,
    end: VisualEvent,
    target: KitTarget,
    kit_id: u32,
}

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
struct VisualRow {
    missile_set: u32,
    events: Vec<EventRow>,
}

/// One model a kit attaches to its unit.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct KitModel {
    pub model_fdid: u32,
    /// M2 attachment id; `None` (-1) places the model at the unit's origin.
    pub attachment: Option<u8>,
    /// Offset in the attachment's WoW-space frame (yards).
    pub offset: [f32; 3],
    /// Radians about the attachment's up, right and forward axes.
    pub yaw: f32,
    pub pitch: f32,
    pub roll: f32,
    /// `SpellVisualKitModelAttach.Scale` × `SpellVisualEffectName.Scale`.
    pub scale: f32,
    /// Seconds after the kit starts before the model appears.
    pub start_delay: f32,
    /// Model sequence to play; `None` plays its Stand (0) clip.
    pub anim_id: Option<u16>,
}

/// The unit animation a kit requests.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct KitAnimation {
    pub anim_id: u16,
    /// Held until the kit ends (a precast or channel loop) rather than played once.
    pub looping: bool,
}

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
struct KitRow {
    models: Vec<KitModel>,
    /// (`SpellVisualAnim.InitialAnimID`, `LoopAnimID`, `AnimKitID`).
    anim: Option<(i32, i32, u32)>,
}

#[derive(Clone, Copy, Debug, Serialize, Deserialize)]
struct SegmentRow {
    order: u32,
    anim_id: i32,
    loop_to: i32,
}

/// A kit started at one visual event.
#[derive(Clone, Debug, PartialEq)]
pub struct VisualKit {
    pub kit_id: u32,
    pub target: KitTarget,
    /// The event that ends the kit (`OneShot` when it plays out once).
    pub end: VisualEvent,
    pub models: Vec<KitModel>,
    pub animation: Option<KitAnimation>,
}

/// A missile the visual launches from the caster at the cast event.
#[derive(Clone, Debug, PartialEq)]
pub struct VisualMissile {
    pub model_fdid: u32,
    /// Yards per second (`SpellVisualEffectName.BaseMissileSpeed`).
    pub speed: f32,
    pub scale: f32,
    /// Caster attachment it leaves from and target attachment it flies to (`None`: origin).
    pub cast_attachment: Option<u8>,
    pub impact_attachment: Option<u8>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
struct MissileRow {
    effect_name: u32,
    attachment: i32,
    destination: i32,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
struct EffectName {
    model_fdid: u32,
    scale: f32,
    missile_speed: f32,
}

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct SpellVisualCatalog {
    spells: HashMap<u32, Vec<VisualChoice>>,
    visuals: HashMap<u32, VisualRow>,
    kits: HashMap<u32, KitRow>,
    anim_kits: HashMap<u32, Vec<SegmentRow>>,
    missiles: HashMap<u32, Vec<MissileRow>>,
    effect_names: HashMap<u32, EffectName>,
    conditions: HashMap<u32, PlayerCondition>,
    anim_fallbacks: HashMap<u16, u16>,
}

/// Rows of one exported CSV with named column access.
struct Table {
    path: PathBuf,
    columns: HashMap<String, usize>,
    rows: Vec<Vec<String>>,
}

impl Table {
    fn read(dir: &Path, name: &str) -> Result<Self, String> {
        let path = dir.join(format!("{name}.csv"));
        let text = std::fs::read_to_string(&path)
            .map_err(|error| format!("read {}: {error}", path.display()))?;
        // PlayerCondition's failure text may be quoted over several lines.
        let mut rows = if name == "PlayerCondition" {
            parse_csv_records(&text)
        } else {
            text.lines().map(parse_csv_line).collect()
        };
        if rows.is_empty() {
            return Err(format!("{} is empty", path.display()));
        }
        let header = rows.remove(0);
        let columns = header
            .into_iter()
            .enumerate()
            .map(|(index, column)| (column, index))
            .collect();
        Ok(Self {
            path,
            columns,
            rows,
        })
    }

    fn column(&self, name: &str) -> Result<usize, String> {
        self.columns
            .get(name)
            .copied()
            .ok_or_else(|| format!("{} missing {name} column", self.path.display()))
    }

    /// Integer cells of `names`, per row; a malformed cell fails the load.
    fn ints<const N: usize>(&self, names: [&str; N]) -> Result<Vec<[i64; N]>, String> {
        let columns = names.map(|name| self.column(name));
        let mut indices = [0; N];
        for (slot, column) in indices.iter_mut().zip(columns) {
            *slot = column?;
        }
        self.rows
            .iter()
            .enumerate()
            .map(|(line, row)| {
                let mut values = [0i64; N];
                for (value, &index) in values.iter_mut().zip(&indices) {
                    let cell = row.get(index).map(String::as_str).unwrap_or_default();
                    *value = parse_number(cell).ok_or_else(|| {
                        format!(
                            "{} row {}: bad value {cell:?}",
                            self.path.display(),
                            line + 2
                        )
                    })?;
                }
                Ok(values)
            })
            .collect()
    }

    fn floats<const N: usize>(&self, names: [&str; N]) -> Result<Vec<[f32; N]>, String> {
        let mut indices = [0; N];
        for (slot, name) in indices.iter_mut().zip(names) {
            *slot = self.column(name)?;
        }
        self.rows
            .iter()
            .enumerate()
            .map(|(line, row)| {
                let mut values = [0f32; N];
                for (value, &index) in values.iter_mut().zip(&indices) {
                    let cell = row.get(index).map(String::as_str).unwrap_or_default();
                    *value = cell.parse().map_err(|_| {
                        format!(
                            "{} row {}: bad float {cell:?}",
                            self.path.display(),
                            line + 2
                        )
                    })?;
                }
                Ok(values)
            })
            .collect()
    }
}

fn parse_number(cell: &str) -> Option<i64> {
    if cell.is_empty() {
        return Some(0);
    }
    cell.parse::<i64>()
        .ok()
        .or_else(|| cell.parse::<f64>().ok().map(|value| value as i64))
}

fn attachment(value: i64) -> Option<u8> {
    u8::try_from(value).ok()
}

fn anim_id(value: i64) -> Option<u16> {
    u16::try_from(value).ok().filter(|&id| id != u16::MAX)
}

impl SpellVisualCatalog {
    /// The catalog of `db2_dir`'s CSVs, cached (bincode) at `cache_path`.
    pub fn load(db2_dir: &Path, cache_path: &Path) -> Result<Self, String> {
        let sources = SOURCE_TABLES
            .iter()
            .map(|table| (table.to_string(), db2_dir.join(format!("{table}.csv"))));
        let key = CacheKey::new(CACHE_FORMAT, DB2_BUILD, sources)?;
        load_or_build(cache_path, &key, || Self::build(db2_dir))
    }

    pub fn build(dir: &Path) -> Result<Self, String> {
        let mut catalog = Self::default();
        catalog.read_spell_visuals(dir)?;
        catalog.read_visuals(dir)?;
        catalog.read_kits(dir)?;
        catalog.read_anims(dir)?;
        catalog.read_missiles(dir)?;
        catalog.read_conditions(dir)?;
        Ok(catalog)
    }

    fn read_spell_visuals(&mut self, dir: &Path) -> Result<(), String> {
        let table = Table::read(dir, "SpellXSpellVisual")?;
        for [
            spell,
            visual,
            difficulty,
            priority,
            caster,
            caster_unit,
            viewer_unit,
            viewer,
        ] in table.ints([
            "SpellID",
            "SpellVisualID",
            "DifficultyID",
            "Priority",
            "CasterPlayerConditionID",
            "CasterUnitConditionID",
            "ViewerUnitConditionID",
            "ViewerPlayerConditionID",
        ])? {
            self.spells
                .entry(spell as u32)
                .or_default()
                .push(VisualChoice {
                    visual_id: visual as u32,
                    priority,
                    difficulty,
                    caster_condition: caster as u32,
                    other_condition: caster_unit != 0 || viewer_unit != 0 || viewer != 0,
                });
        }
        Ok(())
    }

    fn read_visuals(&mut self, dir: &Path) -> Result<(), String> {
        let visuals = Table::read(dir, "SpellVisual")?;
        for [id, missile_set] in visuals.ints(["ID", "SpellVisualMissileSetID"])? {
            self.visuals.entry(id as u32).or_default().missile_set = missile_set as u32;
        }
        let events = Table::read(dir, "SpellVisualEvent")?;
        for [visual, start, end, target, kit] in events.ints([
            "SpellVisualID",
            "StartEvent",
            "EndEvent",
            "TargetType",
            "SpellVisualKitID",
        ])? {
            self.visuals
                .entry(visual as u32)
                .or_default()
                .events
                .push(EventRow {
                    start: VisualEvent::from_db2(start as u32),
                    end: VisualEvent::from_db2(end as u32),
                    target: KitTarget::from_db2(target as u32),
                    kit_id: kit as u32,
                });
        }
        Ok(())
    }

    fn read_kits(&mut self, dir: &Path) -> Result<(), String> {
        let names = Table::read(dir, "SpellVisualEffectName")?;
        let name_ints = names.ints(["ID", "ModelFileDataID"])?;
        let name_floats = names.floats(["Scale", "BaseMissileSpeed"])?;
        for ([id, model], [scale, missile_speed]) in name_ints.into_iter().zip(name_floats) {
            self.effect_names.insert(
                id as u32,
                EffectName {
                    model_fdid: model as u32,
                    scale,
                    missile_speed,
                },
            );
        }
        let attaches = Table::read(dir, "SpellVisualKitModelAttach")?;
        let attach_ints =
            attaches.ints(["ID", "SpellVisualEffectNameID", "AttachmentID", "AnimID"])?;
        let attach_floats = attaches.floats([
            "Offset_0",
            "Offset_1",
            "Offset_2",
            "Yaw",
            "Pitch",
            "Roll",
            "Scale",
            "StartDelay",
        ])?;
        let mut models = HashMap::new();
        for ([id, name, attach, anim], [x, y, z, yaw, pitch, roll, scale, delay]) in
            attach_ints.into_iter().zip(attach_floats)
        {
            let Some(effect) = self.effect_names.get(&(name as u32)) else {
                continue;
            };
            if effect.model_fdid == 0 {
                continue;
            }
            models.insert(
                id as u32,
                KitModel {
                    model_fdid: effect.model_fdid,
                    attachment: attachment(attach),
                    offset: [x, y, z],
                    yaw,
                    pitch,
                    roll,
                    scale: scale * effect.scale,
                    start_delay: delay,
                    anim_id: anim_id(anim),
                },
            );
        }
        let anims = Table::read(dir, "SpellVisualAnim")?;
        let anims: HashMap<u32, (i32, i32, u32)> = anims
            .ints(["ID", "InitialAnimID", "LoopAnimID", "AnimKitID"])?
            .into_iter()
            .map(|[id, initial, looped, kit]| {
                (id as u32, (initial as i32, looped as i32, kit as u32))
            })
            .collect();
        let effects = Table::read(dir, "SpellVisualKitEffect")?;
        let mut rows = effects.ints(["ParentSpellVisualKitID", "EffectType", "Effect", "ID"])?;
        // Kit effects in ID order, as authored.
        rows.sort_by_key(|&[kit, _, _, id]| (kit, id));
        for [kit, kind, effect, _] in rows {
            let kit = self.kits.entry(kit as u32).or_default();
            match kind as u32 {
                EFFECT_MODEL_ATTACH => kit.models.extend(models.get(&(effect as u32)).cloned()),
                EFFECT_ANIM if kit.anim.is_none() => {
                    kit.anim = anims.get(&(effect as u32)).copied()
                }
                _ => {}
            }
        }
        Ok(())
    }

    fn read_anims(&mut self, dir: &Path) -> Result<(), String> {
        let segments = Table::read(dir, "AnimKitSegment")?;
        for [kit, order, anim, loop_to] in segments.ints([
            "ParentAnimKitID",
            "OrderIndex",
            "AnimID",
            "LoopToSegmentIndex",
        ])? {
            self.anim_kits
                .entry(kit as u32)
                .or_default()
                .push(SegmentRow {
                    order: order as u32,
                    anim_id: anim as i32,
                    loop_to: loop_to as i32,
                });
        }
        for segments in self.anim_kits.values_mut() {
            segments.sort_by_key(|segment| segment.order);
        }
        let data = Table::read(dir, "AnimationData")?;
        for [id, fallback] in data.ints(["ID", "Fallback"])? {
            if id != fallback {
                self.anim_fallbacks.insert(id as u16, fallback as u16);
            }
        }
        Ok(())
    }

    fn read_missiles(&mut self, dir: &Path) -> Result<(), String> {
        let missiles = Table::read(dir, "SpellVisualMissile")?;
        let mut rows = missiles.ints([
            "SpellVisualMissileSetID",
            "ID",
            "SpellVisualEffectNameID",
            "Attachment",
            "DestinationAttachment",
        ])?;
        rows.sort_by_key(|&[set, id, ..]| (set, id));
        for [set, _, effect_name, attachment, destination] in rows {
            self.missiles
                .entry(set as u32)
                .or_default()
                .push(MissileRow {
                    effect_name: effect_name as u32,
                    attachment: attachment as i32,
                    destination: destination as i32,
                });
        }
        Ok(())
    }

    fn read_conditions(&mut self, dir: &Path) -> Result<(), String> {
        let referenced: std::collections::HashSet<u32> = self
            .spells
            .values()
            .flatten()
            .map(|choice| choice.caster_condition)
            .filter(|&id| id != 0)
            .collect();
        let table = Table::read(dir, "PlayerCondition")?;
        let handled = [
            "ID",
            "Failure_description_lang",
            "Flags",
            "ClassMask",
            "RaceMasks_0",
            "RaceMasks_1",
            "Gender",
            "MinLevel",
            "MaxLevel",
            "ChrSpecializationIndex",
            "WeaponSubclassMask",
        ];
        // Columns whose "no requirement" value is -1; every other column's is 0.
        let unset_minus_one = [
            "NativeGender",
            "MinExpansionLevel",
            "MaxExpansionLevel",
            "ChrSpecializationRole",
            "PowerType",
            "MaxExpansionTier",
            "MinExpansionTier",
        ];
        let id = table.column("ID")?;
        let wanted: Vec<&Vec<String>> = table
            .rows
            .iter()
            .filter(|row| {
                row.get(id)
                    .and_then(|cell| cell.parse::<u32>().ok())
                    .is_some_and(|id| referenced.contains(&id))
            })
            .collect();
        let column = |name: &str| table.column(name);
        let cell = |row: &[String], index: usize| -> i64 {
            row.get(index)
                .and_then(|cell| parse_number(cell))
                .unwrap_or(0)
        };
        let fields = handled.map(column);
        let mut indices = [0; 11];
        for (slot, field) in indices.iter_mut().zip(fields) {
            *slot = field?;
        }
        let others: Vec<(usize, i64)> = table
            .columns
            .iter()
            .filter(|(name, _)| !handled.contains(&name.as_str()) && !name.ends_with("Logic"))
            .map(|(name, &index)| {
                let unset = if unset_minus_one.contains(&name.as_str()) {
                    -1
                } else {
                    0
                };
                (index, unset)
            })
            .collect();
        for row in wanted {
            let value = |slot: usize| cell(row, indices[slot]);
            let race_mask = (value(4) as u32 as u64) | ((value(5) as u32 as u64) << 32);
            self.conditions.insert(
                value(0) as u32,
                PlayerCondition {
                    flags: value(2),
                    class_mask: value(3),
                    race_mask,
                    gender: value(6),
                    min_level: value(7),
                    max_level: value(8),
                    spec_index: value(9),
                    weapon_subclass_mask: value(10),
                    unsupported: others
                        .iter()
                        .any(|&(index, unset)| cell(row, index) != unset),
                },
            );
        }
        Ok(())
    }

    /// The `SpellVisual` a caster shows for `spell_id` in open-world difficulty: the
    /// highest-priority `SpellXSpellVisual` row whose caster condition holds.
    pub fn visual_for_spell(&self, spell_id: u32, caster: &CasterContext) -> Option<u32> {
        self.spells
            .get(&spell_id)?
            .iter()
            .filter(|choice| choice.difficulty == 0 && !choice.other_condition)
            .filter(|choice| {
                choice.caster_condition == 0
                    || self
                        .conditions
                        .get(&choice.caster_condition)
                        .is_some_and(|condition| condition.holds(caster))
            })
            .max_by_key(|choice| choice.priority)
            .map(|choice| choice.visual_id)
    }

    /// Kits `visual_id` starts at `event`, in authored order.
    pub fn kits(&self, visual_id: u32, event: VisualEvent) -> Vec<VisualKit> {
        let Some(visual) = self.visuals.get(&visual_id) else {
            return Vec::new();
        };
        visual
            .events
            .iter()
            .filter(|row| row.start == event)
            .map(|row| {
                let kit = self.kits.get(&row.kit_id);
                VisualKit {
                    kit_id: row.kit_id,
                    target: row.target,
                    end: row.end,
                    models: kit.map(|kit| kit.models.clone()).unwrap_or_default(),
                    animation: kit
                        .and_then(|kit| kit.anim)
                        .and_then(|anim| self.kit_animation(anim, row.end)),
                }
            })
            .collect()
    }

    /// `SpellVisualAnim`: an `AnimKit`'s first segment (looping when a segment loops
    /// back), else the loop clip (held until the kit ends unless the kit is a
    /// one-shot), else the initial clip once.
    fn kit_animation(
        &self,
        (initial, looped, kit): (i32, i32, u32),
        end: VisualEvent,
    ) -> Option<KitAnimation> {
        let held = end != VisualEvent::OneShot;
        if kit != 0 {
            let segments = self.anim_kits.get(&kit)?;
            let first = segments.first()?;
            return Some(KitAnimation {
                anim_id: anim_id(i64::from(first.anim_id))?,
                looping: held && segments.iter().any(|segment| segment.loop_to >= 0),
            });
        }
        if let Some(id) = anim_id(i64::from(looped)) {
            return Some(KitAnimation {
                anim_id: id,
                looping: held,
            });
        }
        anim_id(i64::from(initial)).map(|anim_id| KitAnimation {
            anim_id,
            looping: false,
        })
    }

    /// The first missile of `visual_id`'s missile set.
    pub fn missile(&self, visual_id: u32) -> Option<VisualMissile> {
        let set = self.visuals.get(&visual_id)?.missile_set;
        let row = self.missiles.get(&set)?.first()?;
        let name = self.effect_names.get(&row.effect_name)?;
        (name.model_fdid != 0).then(|| VisualMissile {
            model_fdid: name.model_fdid,
            speed: name.missile_speed,
            scale: name.scale,
            cast_attachment: attachment(i64::from(row.attachment)),
            impact_attachment: attachment(i64::from(row.destination)),
        })
    }

    /// `AnimationData.Fallback` of `anim_id`, when it has one.
    pub fn anim_fallback(&self, anim_id: u16) -> Option<u16> {
        self.anim_fallbacks.get(&anim_id).copied()
    }

    /// `AnimationData.Fallback` pairs (clip → substitute).
    pub fn anim_fallbacks(&self) -> &HashMap<u16, u16> {
        &self.anim_fallbacks
    }
}
