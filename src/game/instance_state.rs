//! Dungeon difficulty and saved instances on the client (docs/specs/instances.md):
//! the player's dungeon difficulty (`GetDungeonDifficultyID`), the difficulty of the map
//! copy it is in (`GetInstanceInfo`), its saved instances (`GetSavedInstanceInfo`) and
//! the `Difficulty.db2` / `Map.db2` names and flags the UI shows them with
//! (`GetDifficultyInfo`, `GetRealZoneText`).

use std::collections::HashMap;
use std::path::Path;

use bevy::prelude::*;
use shared::protocol::InstanceLockInfo;

use crate::csv_util::{header_index, parse_csv_records};

pub const DIFFICULTY_CSV: &str = "data/db2/12.1.0.69933/Difficulty.csv";
pub const MAP_CSV: &str = "data/db2/12.1.0.69933/Map.csv";
pub const MAP_DIFFICULTY_CSV: &str = "data/db2/12.1.0.69933/MapDifficulty.csv";

/// `MapDifficultyFlags::DisableLockExtension`.
const MAP_DIFFICULTY_FLAG_DISABLE_LOCK_EXTENSION: u32 = 0x10;

/// The dungeon difficulties of the unit menu (`UnitPopupDungeonDifficulty1..3ButtonMixin`):
/// Normal, Heroic, Mythic.
pub const MENU_DUNGEON_DIFFICULTIES: [u32; 3] = [1, 2, 23];

/// `DifficultyFlags` the minimap banner reads (`GetDifficultyInfo` isHeroic,
/// displayHeroic, displayMythic).
const DIFFICULTY_FLAG_HEROIC_STYLE_LOCKOUTS: u32 = 0x01;
const DIFFICULTY_FLAG_DISPLAY_HEROIC: u32 = 0x40;
const DIFFICULTY_FLAG_DISPLAY_MYTHIC: u32 = 0x80;

#[derive(Clone, Debug, PartialEq)]
pub struct DifficultyInfo {
    pub name: String,
    pub flags: u32,
    pub max_players: u32,
}

/// Which `ui-hud-minimap-guildbanner-*-large` texture the instance banner shows
/// (`InstanceDifficultyMixin:GetDifficultyTexture`).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum BannerTexture {
    Normal,
    Heroic,
    Mythic,
}

impl DifficultyInfo {
    pub fn banner_texture(&self) -> BannerTexture {
        if self.flags & DIFFICULTY_FLAG_DISPLAY_MYTHIC != 0 {
            BannerTexture::Mythic
        } else if self.flags
            & (DIFFICULTY_FLAG_HEROIC_STYLE_LOCKOUTS | DIFFICULTY_FLAG_DISPLAY_HEROIC)
            != 0
        {
            BannerTexture::Heroic
        } else {
            BannerTexture::Normal
        }
    }
}

/// `Difficulty.db2` and `Map.db2` names.
#[derive(Resource, Clone, Debug, Default)]
pub struct InstanceCatalog {
    pub difficulties: HashMap<u32, DifficultyInfo>,
    pub map_names: HashMap<u32, String>,
    /// (map, difficulty) rows of `MapDifficulty.db2` whose lock cannot be extended
    /// (`GetSavedInstanceInfo` extendDisabled).
    pub extension_disabled: std::collections::HashSet<(u32, u32)>,
}

impl InstanceCatalog {
    pub fn load() -> Result<Self, String> {
        Ok(Self {
            difficulties: load_difficulties(Path::new(DIFFICULTY_CSV))?,
            map_names: load_map_names(Path::new(MAP_CSV))?,
            extension_disabled: load_extension_disabled(Path::new(MAP_DIFFICULTY_CSV))?,
        })
    }

    pub fn difficulty_name(&self, difficulty_id: u32) -> &str {
        self.difficulties
            .get(&difficulty_id)
            .map_or("", |info| info.name.as_str())
    }

    pub fn map_name(&self, map_id: u32) -> &str {
        self.map_names.get(&map_id).map_or("", String::as_str)
    }
}

fn read_rows(path: &Path, columns: &[&str]) -> Result<Vec<Vec<String>>, String> {
    let text =
        std::fs::read_to_string(path).map_err(|err| format!("read {}: {err}", path.display()))?;
    let mut records = parse_csv_records(&text).into_iter();
    let headers = records
        .next()
        .ok_or_else(|| format!("{} is empty", path.display()))?;
    let indices = columns
        .iter()
        .map(|column| header_index(&headers, column, path))
        .collect::<Result<Vec<_>, _>>()?;
    records
        .map(|fields| {
            indices
                .iter()
                .map(|&i| {
                    fields
                        .get(i)
                        .cloned()
                        .ok_or_else(|| format!("{}: short row {fields:?}", path.display()))
                })
                .collect()
        })
        .collect()
}

fn number(value: &str, path: &Path) -> Result<u32, String> {
    value
        .parse::<i64>()
        .map(|value| value as u32)
        .map_err(|err| format!("{}: bad number {value:?}: {err}", path.display()))
}

fn load_difficulties(path: &Path) -> Result<HashMap<u32, DifficultyInfo>, String> {
    read_rows(path, &["ID", "Name_lang", "Flags", "MaxPlayers"])?
        .into_iter()
        .map(|row| {
            Ok((
                number(&row[0], path)?,
                DifficultyInfo {
                    name: row[1].clone(),
                    flags: number(&row[2], path)?,
                    max_players: number(&row[3], path)?,
                },
            ))
        })
        .collect()
}

fn load_extension_disabled(path: &Path) -> Result<std::collections::HashSet<(u32, u32)>, String> {
    let mut disabled = std::collections::HashSet::new();
    for row in read_rows(path, &["MapID", "DifficultyID", "Flags"])? {
        if number(&row[2], path)? & MAP_DIFFICULTY_FLAG_DISABLE_LOCK_EXTENSION != 0 {
            disabled.insert((number(&row[0], path)?, number(&row[1], path)?));
        }
    }
    Ok(disabled)
}

fn load_map_names(path: &Path) -> Result<HashMap<u32, String>, String> {
    read_rows(path, &["ID", "MapName_lang"])?
        .into_iter()
        .map(|row| Ok((number(&row[0], path)?, row[1].clone())))
        .collect()
}

/// What the server told the client about difficulties and saved instances.
#[derive(Resource, Clone, Debug, Default, PartialEq)]
pub struct InstanceState {
    /// `GetDungeonDifficultyID`: `None` until the server sends it.
    pub dungeon_difficulty: Option<u32>,
    /// (map id, difficulty id) of the copy the player is in; difficulty 0 on a continent.
    pub current_map: Option<(u32, u32)>,
    /// `GetSavedInstanceInfo` rows in server order.
    pub saved: Vec<InstanceLockInfo>,
    /// Elapsed seconds when `saved` arrived.
    pub saved_at: f64,
    /// Players in the current copy (the banner's group size).
    pub instance_group_size: u32,
}

impl InstanceState {
    /// `IsInInstance`: in a dungeon copy (a map difficulty other than 0).
    pub fn in_instance(&self) -> bool {
        self.current_map
            .is_some_and(|(_, difficulty)| difficulty != 0)
    }

    /// The seconds left on `lock` at elapsed time `now`.
    pub fn time_remaining(&self, lock: &InstanceLockInfo, now: f64) -> u32 {
        let passed = (now - self.saved_at).max(0.0) as u32;
        lock.time_remaining_secs.saturating_sub(passed)
    }
}

/// What the UI asks the server for.
#[derive(Message, Clone, Debug, PartialEq, Eq)]
pub enum InstanceCommand {
    /// `SetDungeonDifficultyID`.
    SetDungeonDifficulty(u32),
    /// `RequestRaidInfo`.
    RequestRaidInfo,
    /// `SetSavedInstanceExtend(index, extend)`.
    SetExtended {
        map_id: u32,
        difficulty_id: u32,
        extend: bool,
    },
    /// `ResetInstances`.
    ResetInstances,
}

/// `DifficultyUtil.IsDungeonDifficultyEnabled`: not inside an instance, and in a group
/// only for its leader.
pub fn dungeon_difficulty_enabled(state: &InstanceState, in_group: bool, leader: bool) -> bool {
    !state.in_instance() && (!in_group || leader)
}

/// `SecondsToTime(seconds, noSeconds = true, notAbbreviated = nil, maxCount = 3)`:
/// "1 Day 3 Hr 5 Min" (`DAYS_ABBR`, `HOURS_ABBR`, `MINUTES_ABBR`).
pub fn seconds_to_time(seconds: u32) -> String {
    const UNITS: [(u32, &str, &str); 3] = [
        (86_400, "Day", "Days"),
        (3_600, "Hr", "Hr"),
        (60, "Min", "Min"),
    ];
    let mut parts = Vec::new();
    let mut left = seconds;
    for (size, one, many) in UNITS {
        if parts.len() < 3 && left >= size {
            let count = left / size;
            left %= size;
            parts.push(format!("{count} {}", if count == 1 { one } else { many }));
        }
    }
    parts.join(" ")
}

/// `ERR_DUNGEON_DIFFICULTY_CHANGED_S`.
pub fn difficulty_changed_text(catalog: &InstanceCatalog, difficulty_id: u32) -> String {
    format!(
        "Dungeon Difficulty set to {}.",
        catalog.difficulty_name(difficulty_id)
    )
}

#[cfg(test)]
#[path = "instance_state_tests.rs"]
mod tests;
