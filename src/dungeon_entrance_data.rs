//! Dungeon entrances and the difficulty bar shown at them, after the Plumber addon's
//! "Instance Difficulty Selector" (`Modules/RaidCheck`; docs/specs/instances.md,
//! Entrance difficulty bar).
//!
//! Entrances are `JournalInstanceEntrance.db2` world positions of `JournalInstance.db2`
//! instances (`C_EncounterJournal.GetDungeonEntrancesForMap`); only dungeon
//! (`Map.InstanceType` 1) entrances are kept, the server has no raids. The choices are
//! Plumber's dungeon difficulties the map has a `MapDifficulty.db2` row for
//! (`EJ_IsValidInstanceDifficulty`), labelled `ENCOUNTER_JOURNAL_DIFF_TEXT` "(%s) %s" from
//! `Difficulty.db2`, with the `DungeonEncounter.db2` bosses killed on the player's lock.

use std::collections::{HashMap, HashSet};
use std::path::Path;

use shared::protocol::InstanceLockInfo;

use crate::csv_util::{header_index, parse_csv_records};

pub const JOURNAL_INSTANCE_ENTRANCE_CSV: &str = "JournalInstanceEntrance.csv";
const JOURNAL_INSTANCE_CSV: &str = "JournalInstance.csv";
const MAP_CSV: &str = "Map.csv";
const MAP_DIFFICULTY_CSV: &str = "MapDifficulty.csv";
const DUNGEON_ENCOUNTER_CSV: &str = "DungeonEncounter.csv";
const DIFFICULTY_CSV: &str = "Difficulty.csv";

/// `Map.InstanceType` of a dungeon (`MAP_INSTANCE`).
const INSTANCE_TYPE_DUNGEON: u32 = 1;

/// Plumber `VALID_DIFFUICULTY_OPEN_WORLD` dungeon ids in its order: Normal, Heroic, Mythic.
/// Mythic Keystone, Timewalking and Follower are never listed.
pub const SELECTOR_DIFFICULTIES: [u32; 3] = [1, 2, 23];

/// Plumber `DEFAULT_RANGE`: the bar shows within 31 yards (2D) of an entrance.
const SHOW_RANGE_YD: f32 = 31.0;
/// Within 60 yards the proximity check runs every 0.5 s, else every 1 s.
const NEAR_RANGE_YD: f32 = 60.0;

/// Plumber `ColorCodes`: `c6c6c6` while bosses remain, `f55a4f` once all are killed.
pub const PROGRESS_OPEN_RGBA: [f32; 4] = [198.0 / 255.0, 198.0 / 255.0, 198.0 / 255.0, 1.0];
pub const PROGRESS_CLEARED_RGBA: [f32; 4] = [245.0 / 255.0, 90.0 / 255.0, 79.0 / 255.0, 1.0];

/// One `JournalInstanceEntrance` row: a world position on a continent map.
#[derive(Clone, Debug, PartialEq)]
pub struct Entrance {
    pub map_id: u32,
    /// Retail world coordinates (x, y, z).
    pub position: [f32; 3],
    pub journal_instance_id: u32,
}

#[derive(Clone, Debug, PartialEq)]
pub struct JournalInstance {
    pub name: String,
    /// The instance's own map.
    pub map_id: u32,
}

#[derive(Clone, Debug, PartialEq)]
struct Encounter {
    /// 0 for every difficulty.
    difficulty_id: u32,
    bit: u32,
}

#[derive(Clone, Debug, PartialEq)]
struct Difficulty {
    name: String,
    max_players: u32,
}

/// The closest dungeon entrance to the player.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct NearestEntrance {
    pub journal_instance_id: u32,
    /// 2D yards; height is ignored.
    pub distance: f32,
}

impl NearestEntrance {
    pub fn shows_bar(&self) -> bool {
        self.distance < SHOW_RANGE_YD
    }

    /// Seconds until the next proximity check.
    pub fn recheck_secs(&self) -> f32 {
        if self.distance < NEAR_RANGE_YD {
            0.5
        } else {
            1.0
        }
    }
}

/// One difficulty button: "(5) Normal" and its `(killed/total)` boss count.
#[derive(Clone, Debug, PartialEq)]
pub struct DifficultyChoice {
    pub difficulty_id: u32,
    pub label: String,
    pub killed: usize,
    pub total: usize,
}

impl DifficultyChoice {
    pub fn progress_text(&self) -> String {
        format!("({}/{})", self.killed, self.total)
    }

    pub fn progress_color(&self) -> [f32; 4] {
        if self.killed < self.total {
            PROGRESS_OPEN_RGBA
        } else {
            PROGRESS_CLEARED_RGBA
        }
    }
}

/// What the bar shows for one instance.
#[derive(Clone, Debug, PartialEq)]
pub struct EntranceSelector {
    pub journal_instance_id: u32,
    pub map_id: u32,
    pub title: String,
    pub choices: Vec<DifficultyChoice>,
    /// The player's dungeon difficulty when the instance offers it.
    pub selected: Option<u32>,
}

#[derive(Clone, Debug, Default)]
pub struct EntranceCatalog {
    entrances: Vec<Entrance>,
    journal: HashMap<u32, JournalInstance>,
    map_difficulties: HashMap<u32, HashSet<u32>>,
    encounters: HashMap<u32, Vec<Encounter>>,
    difficulties: HashMap<u32, Difficulty>,
}

impl EntranceCatalog {
    /// Read the tables from a `data/db2/<build>` directory.
    pub fn load(dir: &Path) -> Result<Self, String> {
        let journal = load_journal(&dir.join(JOURNAL_INSTANCE_CSV))?;
        let dungeon_maps = load_dungeon_maps(&dir.join(MAP_CSV))?;
        let entrances = load_entrances(&dir.join(JOURNAL_INSTANCE_ENTRANCE_CSV))?
            .into_iter()
            .filter(|entrance| {
                journal
                    .get(&entrance.journal_instance_id)
                    .is_some_and(|instance| dungeon_maps.contains(&instance.map_id))
            })
            .collect();
        Ok(Self {
            entrances,
            journal,
            map_difficulties: load_map_difficulties(&dir.join(MAP_DIFFICULTY_CSV))?,
            encounters: load_encounters(&dir.join(DUNGEON_ENCOUNTER_CSV))?,
            difficulties: load_difficulties(&dir.join(DIFFICULTY_CSV))?,
        })
    }

    /// Dungeon entrances.
    pub fn entrances(&self) -> &[Entrance] {
        &self.entrances
    }

    pub fn journal_instance(&self, journal_instance_id: u32) -> Option<&JournalInstance> {
        self.journal.get(&journal_instance_id)
    }

    /// The closest dungeon entrance on `map_id` to world position `position`.
    pub fn nearest_entrance(&self, map_id: u32, position: [f32; 3]) -> Option<NearestEntrance> {
        self.entrances
            .iter()
            .filter(|entrance| entrance.map_id == map_id)
            .map(|entrance| NearestEntrance {
                journal_instance_id: entrance.journal_instance_id,
                distance: (entrance.position[0] - position[0])
                    .hypot(entrance.position[1] - position[1]),
            })
            .min_by(|a, b| a.distance.total_cmp(&b.distance))
    }

    /// The bar for `journal_instance_id`, or None when it offers no difficulty.
    pub fn selector(
        &self,
        journal_instance_id: u32,
        dungeon_difficulty: Option<u32>,
        locks: &[InstanceLockInfo],
    ) -> Option<EntranceSelector> {
        let instance = self.journal.get(&journal_instance_id)?;
        let offered = self.map_difficulties.get(&instance.map_id)?;
        let choices: Vec<_> = SELECTOR_DIFFICULTIES
            .into_iter()
            .filter(|id| offered.contains(id))
            .filter_map(|id| self.choice(instance.map_id, id, locks))
            .collect();
        if choices.is_empty() {
            return None;
        }
        let selected = dungeon_difficulty
            .filter(|id| choices.iter().any(|choice| choice.difficulty_id == *id));
        Some(EntranceSelector {
            journal_instance_id,
            map_id: instance.map_id,
            title: instance.name.clone(),
            choices,
            selected,
        })
    }

    fn choice(
        &self,
        map_id: u32,
        difficulty_id: u32,
        locks: &[InstanceLockInfo],
    ) -> Option<DifficultyChoice> {
        let difficulty = self.difficulties.get(&difficulty_id)?;
        let bosses: Vec<_> = self
            .encounters
            .get(&map_id)
            .into_iter()
            .flatten()
            .filter(|encounter| {
                encounter.difficulty_id == 0 || encounter.difficulty_id == difficulty_id
            })
            .collect();
        let completed = locks
            .iter()
            .find(|lock| {
                lock.locked && lock.map_id == map_id && lock.difficulty_id == difficulty_id
            })
            .map_or(0, |lock| lock.completed_mask);
        let killed = bosses
            .iter()
            .filter(|encounter| encounter.bit < 32 && completed & (1 << encounter.bit) != 0)
            .count();
        Some(DifficultyChoice {
            difficulty_id,
            label: format!("({}) {}", difficulty.max_players, difficulty.name),
            killed,
            total: bosses.len(),
        })
    }
}

/// The named columns of every row of CSV `path`.
fn read_rows(path: &Path, columns: &[&str]) -> Result<Vec<Vec<String>>, String> {
    let text = std::fs::read_to_string(path)
        .map_err(|error| format!("read {}: {error}", path.display()))?;
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
                .map(|&index| {
                    fields
                        .get(index)
                        .cloned()
                        .ok_or_else(|| format!("{}: short row {fields:?}", path.display()))
                })
                .collect()
        })
        .collect()
}

fn parse<T: std::str::FromStr>(value: &str, path: &Path) -> Result<T, String> {
    value
        .parse()
        .map_err(|_| format!("{}: bad value {value:?}", path.display()))
}

fn load_entrances(path: &Path) -> Result<Vec<Entrance>, String> {
    let columns = ["MapID", "Pos_0", "Pos_1", "Pos_2", "JournalInstanceID"];
    read_rows(path, &columns)?
        .iter()
        .map(|row| {
            Ok(Entrance {
                map_id: parse(&row[0], path)?,
                position: [
                    parse(&row[1], path)?,
                    parse(&row[2], path)?,
                    parse(&row[3], path)?,
                ],
                journal_instance_id: parse(&row[4], path)?,
            })
        })
        .collect()
}

fn load_journal(path: &Path) -> Result<HashMap<u32, JournalInstance>, String> {
    read_rows(path, &["ID", "Name_lang", "MapID"])?
        .iter()
        .map(|row| {
            let instance = JournalInstance {
                name: row[1].clone(),
                map_id: parse(&row[2], path)?,
            };
            Ok((parse(&row[0], path)?, instance))
        })
        .collect()
}

fn load_dungeon_maps(path: &Path) -> Result<HashSet<u32>, String> {
    let mut maps = HashSet::new();
    for row in read_rows(path, &["ID", "InstanceType"])? {
        if parse::<u32>(&row[1], path)? == INSTANCE_TYPE_DUNGEON {
            maps.insert(parse(&row[0], path)?);
        }
    }
    Ok(maps)
}

fn load_map_difficulties(path: &Path) -> Result<HashMap<u32, HashSet<u32>>, String> {
    let mut maps: HashMap<u32, HashSet<u32>> = HashMap::new();
    for row in read_rows(path, &["MapID", "DifficultyID"])? {
        maps.entry(parse(&row[0], path)?)
            .or_default()
            .insert(parse(&row[1], path)?);
    }
    Ok(maps)
}

fn load_encounters(path: &Path) -> Result<HashMap<u32, Vec<Encounter>>, String> {
    let mut maps: HashMap<u32, Vec<Encounter>> = HashMap::new();
    for row in read_rows(path, &["MapID", "DifficultyID", "Bit"])? {
        maps.entry(parse(&row[0], path)?)
            .or_default()
            .push(Encounter {
                difficulty_id: parse(&row[1], path)?,
                bit: parse(&row[2], path)?,
            });
    }
    Ok(maps)
}

fn load_difficulties(path: &Path) -> Result<HashMap<u32, Difficulty>, String> {
    read_rows(path, &["ID", "Name_lang", "MaxPlayers"])?
        .iter()
        .map(|row| {
            let difficulty = Difficulty {
                name: row[1].clone(),
                max_players: parse(&row[2], path)?,
            };
            Ok((parse(&row[0], path)?, difficulty))
        })
        .collect()
}

#[cfg(test)]
#[path = "dungeon_entrance_data_tests.rs"]
mod tests;
