use bevy::prelude::*;
use rusqlite::{Connection, OpenFlags};
use std::path::{Path, PathBuf};

/// Texture FDIDs for the encounter journal.
pub mod textures {
    // Boss portraits
    pub const BOSS_VANCLEEF: u32 = 5875507;
    pub const BOSS_COOKIE: u32 = 5875506;
    pub const BOSS_GODFREY: u32 = 522247;
    pub const BOSS_RAGNAROS: u32 = 522261;
    pub const BOSS_DEFAULT: u32 = 521744;
    // Instance / journal chrome
    pub const BACKGROUND: u32 = 521743;
    pub const JOURNAL_BG: u32 = 521750;
    pub const ICONS: u32 = 521749;
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
pub enum Difficulty {
    #[default]
    Normal,
    Heroic,
    Mythic,
    MythicPlus,
    LFR,
}

impl Difficulty {
    pub fn label(self) -> &'static str {
        match self {
            Self::Normal => "Normal",
            Self::Heroic => "Heroic",
            Self::Mythic => "Mythic",
            Self::MythicPlus => "Mythic+",
            Self::LFR => "LFR",
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum InstanceType {
    Dungeon,
    Raid,
}

#[derive(Clone, Debug, PartialEq)]
pub struct InstanceDef {
    pub id: u32,
    pub name: &'static str,
    pub instance_type: InstanceType,
    pub tier: &'static str,
    pub map_id: u32,
    pub boss_entries: &'static [u32],
}

#[derive(Clone, Debug, PartialEq)]
pub struct BossDef {
    pub entry: u32,
    pub name: &'static str,
    pub instance_id: u32,
}

#[derive(Clone, Debug, PartialEq)]
pub struct AbilityDef {
    pub name: &'static str,
    pub description: &'static str,
    pub icon_fdid: u32,
    pub boss_entry: u32,
}

#[derive(Clone, Debug, PartialEq)]
pub struct LootEntry {
    pub item_name: &'static str,
    pub slot: &'static str,
    pub icon_fdid: u32,
    pub drop_pct: u8,
    pub boss_entry: u32,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct EncounterJournalBossData {
    pub entry: u32,
    pub name: String,
    pub min_level: u16,
    pub max_level: u16,
    pub rank: u8,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct EncounterJournalInstanceData {
    pub instance_id: u32,
    pub name: String,
    pub instance_type: InstanceType,
    pub tier: String,
    pub source: String,
    pub bosses: Vec<EncounterJournalBossData>,
}

const DEADMINES_BOSS_ENTRIES: &[u32] = &[644, 646, 642, 1763, 647, 639, 645];
const SHADOWFANG_KEEP_BOSS_ENTRIES: &[u32] = &[3886, 4278, 4274, 3887, 4275];
const MOLTEN_CORE_BOSS_ENTRIES: &[u32] = &[
    12118, 11982, 12259, 12057, 12056, 11988, 12264, 12098, 11502,
];

pub static INSTANCES: &[InstanceDef] = &[
    InstanceDef {
        id: 1,
        name: "The Deadmines",
        instance_type: InstanceType::Dungeon,
        tier: "Classic",
        map_id: 36,
        boss_entries: DEADMINES_BOSS_ENTRIES,
    },
    InstanceDef {
        id: 2,
        name: "Shadowfang Keep",
        instance_type: InstanceType::Dungeon,
        tier: "Classic",
        map_id: 33,
        boss_entries: SHADOWFANG_KEEP_BOSS_ENTRIES,
    },
    InstanceDef {
        id: 3,
        name: "Molten Core",
        instance_type: InstanceType::Raid,
        tier: "Classic",
        map_id: 409,
        boss_entries: MOLTEN_CORE_BOSS_ENTRIES,
    },
];

pub static BOSSES: &[BossDef] = &[
    BossDef {
        entry: 639,
        name: "Edwin VanCleef",
        instance_id: 1,
    },
    BossDef {
        entry: 645,
        name: "Cookie",
        instance_id: 1,
    },
    BossDef {
        entry: 4275,
        name: "Archmage Arugal",
        instance_id: 2,
    },
    BossDef {
        entry: 11502,
        name: "Ragnaros",
        instance_id: 3,
    },
];

pub static ABILITIES: &[AbilityDef] = &[
    AbilityDef {
        name: "Deadly Poison",
        description: "Coats weapons with deadly poison.",
        icon_fdid: 136067,
        boss_entry: 639,
    },
    AbilityDef {
        name: "Summon Pirates",
        description: "Calls pirates to aid in battle.",
        icon_fdid: 136243,
        boss_entry: 639,
    },
    AbilityDef {
        name: "Cookie's Cooking",
        description: "Throws food that damages.",
        icon_fdid: 136243,
        boss_entry: 645,
    },
    AbilityDef {
        name: "Void Bolt",
        description: "Hurls shadowy bolts at nearby enemies.",
        icon_fdid: 136197,
        boss_entry: 4275,
    },
    AbilityDef {
        name: "Hand of Ragnaros",
        description: "Knocks back all nearby enemies.",
        icon_fdid: 135819,
        boss_entry: 11502,
    },
];

pub static LOOT: &[LootEntry] = &[
    LootEntry {
        item_name: "Cruel Barb",
        slot: "One-Hand Sword",
        icon_fdid: 135274,
        drop_pct: 15,
        boss_entry: 639,
    },
    LootEntry {
        item_name: "Cape of the Brotherhood",
        slot: "Back",
        icon_fdid: 133762,
        drop_pct: 18,
        boss_entry: 639,
    },
    LootEntry {
        item_name: "Cookie's Stirring Rod",
        slot: "Wand",
        icon_fdid: 135474,
        drop_pct: 20,
        boss_entry: 645,
    },
    LootEntry {
        item_name: "Robes of Arugal",
        slot: "Chest",
        icon_fdid: 132666,
        drop_pct: 8,
        boss_entry: 4275,
    },
    LootEntry {
        item_name: "Sulfuras, Hand of Ragnaros",
        slot: "Two-Hand Mace",
        icon_fdid: 135819,
        drop_pct: 2,
        boss_entry: 11502,
    },
];

pub fn instances_by_type(itype: InstanceType) -> Vec<&'static InstanceDef> {
    INSTANCES
        .iter()
        .filter(|i| i.instance_type == itype)
        .collect()
}

pub fn bosses_for_instance(instance_id: u32) -> Vec<&'static BossDef> {
    BOSSES
        .iter()
        .filter(|b| b.instance_id == instance_id)
        .collect()
}

pub fn abilities_for_boss(boss_entry: u32) -> Vec<&'static AbilityDef> {
    ABILITIES
        .iter()
        .filter(|a| a.boss_entry == boss_entry)
        .collect()
}

pub fn loot_for_boss(boss_entry: u32) -> Vec<&'static LootEntry> {
    LOOT.iter().filter(|l| l.boss_entry == boss_entry).collect()
}

/// Filter loot for a boss by slot name (case-insensitive substring match).
pub fn loot_for_boss_by_slot(boss_entry: u32, slot_filter: &str) -> Vec<&'static LootEntry> {
    let q = slot_filter.to_lowercase();
    LOOT.iter()
        .filter(|l| l.boss_entry == boss_entry && l.slot.to_lowercase().contains(&q))
        .collect()
}

/// Full tree traversal: instance → bosses → abilities for all bosses in an instance.
pub fn instance_ability_tree(
    instance_id: u32,
) -> Vec<(&'static BossDef, Vec<&'static AbilityDef>)> {
    bosses_for_instance(instance_id)
        .into_iter()
        .map(|boss| (boss, abilities_for_boss(boss.entry)))
        .collect()
}

pub fn load_encounter_journal() -> Result<Vec<EncounterJournalInstanceData>, String> {
    load_encounter_journal_from_world_db(&encounter_world_db_path())
}

pub fn builtin_encounter_journal() -> Vec<EncounterJournalInstanceData> {
    INSTANCES
        .iter()
        .map(|instance| EncounterJournalInstanceData {
            instance_id: instance.id,
            name: instance.name.to_string(),
            instance_type: instance.instance_type,
            tier: instance.tier.to_string(),
            source: "builtin".to_string(),
            bosses: fallback_bosses(instance),
        })
        .collect()
}

fn encounter_world_db_path() -> PathBuf {
    if let Some(path) = std::env::var_os("GAME_SERVER_WORLD_DB") {
        PathBuf::from(path)
    } else {
        crate::paths::shared_repo_root()
            .parent()
            .unwrap_or_else(|| Path::new("."))
            .join("game-server")
            .join("data")
            .join("world.db")
    }
}

fn load_encounter_journal_from_world_db(
    db_path: &Path,
) -> Result<Vec<EncounterJournalInstanceData>, String> {
    let conn = Connection::open_with_flags(
        db_path,
        OpenFlags::SQLITE_OPEN_READ_ONLY | OpenFlags::SQLITE_OPEN_NO_MUTEX,
    )
    .map_err(|err| format!("open {}: {err}", db_path.display()))?;
    let mut instances = Vec::with_capacity(INSTANCES.len());
    for instance in INSTANCES {
        let bosses = load_world_db_bosses(&conn, instance)?;
        let (bosses, source) = if bosses.is_empty() {
            (fallback_bosses(instance), "builtin".to_string())
        } else {
            (bosses, format!("world.db:{}", db_path.display()))
        };
        instances.push(EncounterJournalInstanceData {
            instance_id: instance.id,
            name: instance.name.to_string(),
            instance_type: instance.instance_type,
            tier: instance.tier.to_string(),
            source,
            bosses,
        });
    }
    Ok(instances)
}

fn load_world_db_bosses(
    conn: &Connection,
    instance: &InstanceDef,
) -> Result<Vec<EncounterJournalBossData>, String> {
    let mut stmt = conn
        .prepare(
            "SELECT entry, name, minlevel, maxlevel, rank
             FROM creature_template
             WHERE entry = ?1
             LIMIT 1",
        )
        .map_err(|err| format!("prepare encounter journal boss query: {err}"))?;
    let mut bosses = Vec::with_capacity(instance.boss_entries.len());
    for entry in instance.boss_entries {
        let row = stmt.query_row([entry], |row| {
            Ok(EncounterJournalBossData {
                entry: row.get::<_, u32>(0)?,
                name: row.get::<_, String>(1)?,
                min_level: row.get::<_, u16>(2)?,
                max_level: row.get::<_, u16>(3)?,
                rank: row.get::<_, u8>(4)?,
            })
        });
        match row {
            Ok(boss) => bosses.push(boss),
            Err(rusqlite::Error::QueryReturnedNoRows) => {}
            Err(err) => return Err(format!("query encounter journal boss {entry}: {err}")),
        }
    }
    Ok(bosses)
}

fn fallback_bosses(instance: &InstanceDef) -> Vec<EncounterJournalBossData> {
    bosses_for_instance(instance.id)
        .into_iter()
        .map(|boss| EncounterJournalBossData {
            entry: boss.entry,
            name: boss.name.to_string(),
            min_level: 0,
            max_level: 0,
            rank: 0,
        })
        .collect()
}

/// Runtime encounter journal state.
#[derive(Resource, Clone, Debug, PartialEq, Default)]
pub struct EJState {
    pub selected_instance: Option<u32>,
    pub selected_boss: Option<u32>,
    pub difficulty: Difficulty,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn instances_by_type_filters() {
        let dungeons = instances_by_type(InstanceType::Dungeon);
        assert_eq!(dungeons.len(), 2);
        let raids = instances_by_type(InstanceType::Raid);
        assert_eq!(raids.len(), 1);
    }

    #[test]
    fn bosses_for_deadmines() {
        let bosses = bosses_for_instance(1);
        assert_eq!(bosses.len(), 2);
        assert_eq!(bosses[0].name, "Edwin VanCleef");
    }

    #[test]
    fn abilities_for_vancleef() {
        let abilities = abilities_for_boss(639);
        assert_eq!(abilities.len(), 2);
    }

    #[test]
    fn loot_for_vancleef() {
        let loot = loot_for_boss(639);
        assert_eq!(loot.len(), 2);
        assert_eq!(loot[0].item_name, "Cruel Barb");
    }

    #[test]
    fn difficulty_labels() {
        assert_eq!(Difficulty::Normal.label(), "Normal");
        assert_eq!(Difficulty::Heroic.label(), "Heroic");
        assert_eq!(Difficulty::MythicPlus.label(), "Mythic+");
    }

    #[test]
    fn default_ej_state() {
        let state = EJState::default();
        assert!(state.selected_instance.is_none());
        assert!(state.selected_boss.is_none());
        assert_eq!(state.difficulty, Difficulty::Normal);
    }

    #[test]
    fn texture_fdids_are_nonzero() {
        assert_ne!(textures::BOSS_VANCLEEF, 0);
        assert_ne!(textures::BOSS_RAGNAROS, 0);
        assert_ne!(textures::BOSS_DEFAULT, 0);
        assert_ne!(textures::BACKGROUND, 0);
        assert_ne!(textures::JOURNAL_BG, 0);
    }

    // --- Instance/boss/ability tree ---

    #[test]
    fn instance_ability_tree_deadmines() {
        let tree = instance_ability_tree(1);
        assert_eq!(tree.len(), 2); // VanCleef + Cookie
        assert_eq!(tree[0].0.name, "Edwin VanCleef");
        assert_eq!(tree[0].1.len(), 2); // Deadly Poison + Summon Pirates
        assert_eq!(tree[1].0.name, "Cookie");
        assert_eq!(tree[1].1.len(), 1); // Cookie's Cooking
    }

    #[test]
    fn instance_ability_tree_single_boss() {
        let tree = instance_ability_tree(3); // Molten Core
        assert_eq!(tree.len(), 1);
        assert_eq!(tree[0].0.name, "Ragnaros");
        assert_eq!(tree[0].1[0].name, "Hand of Ragnaros");
    }

    #[test]
    fn instance_ability_tree_nonexistent() {
        let tree = instance_ability_tree(999);
        assert!(tree.is_empty());
    }

    // --- Loot filter matching ---

    #[test]
    fn loot_filter_by_slot_sword() {
        let loot = loot_for_boss_by_slot(639, "sword");
        assert_eq!(loot.len(), 1);
        assert_eq!(loot[0].item_name, "Cruel Barb");
    }

    #[test]
    fn loot_filter_by_slot_back() {
        let loot = loot_for_boss_by_slot(639, "back");
        assert_eq!(loot.len(), 1);
        assert_eq!(loot[0].item_name, "Cape of the Brotherhood");
    }

    #[test]
    fn loot_filter_no_match() {
        let loot = loot_for_boss_by_slot(639, "plate");
        assert!(loot.is_empty());
    }

    #[test]
    fn loot_filter_case_insensitive() {
        let loot = loot_for_boss_by_slot(639, "SWORD");
        assert_eq!(loot.len(), 1);
    }

    // --- Edge cases ---

    #[test]
    fn bosses_for_nonexistent_instance() {
        assert!(bosses_for_instance(999).is_empty());
    }

    #[test]
    fn abilities_for_nonexistent_boss() {
        assert!(abilities_for_boss(999).is_empty());
    }

    #[test]
    fn loot_for_nonexistent_boss() {
        assert!(loot_for_boss(999).is_empty());
    }

    #[test]
    fn all_bosses_have_at_least_one_ability() {
        for boss in BOSSES {
            let abilities = abilities_for_boss(boss.entry);
            assert!(!abilities.is_empty(), "boss {} has no abilities", boss.name);
        }
    }

    #[test]
    fn all_bosses_have_loot() {
        for boss in BOSSES {
            let loot = loot_for_boss(boss.entry);
            assert!(!loot.is_empty(), "boss {} has no loot", boss.name);
        }
    }

    #[test]
    fn load_encounter_journal_uses_world_db_boss_rows() {
        let dir = std::env::temp_dir().join(format!(
            "encounter-journal-worlddb-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .expect("clock")
                .as_nanos()
        ));
        std::fs::create_dir_all(&dir).expect("temp dir");
        let db_path = dir.join("world.db");
        let conn = Connection::open(&db_path).expect("open temp world db");
        conn.execute_batch(
            "CREATE TABLE creature_template (
                entry INTEGER PRIMARY KEY,
                name TEXT NOT NULL,
                minlevel INTEGER NOT NULL,
                maxlevel INTEGER NOT NULL,
                rank INTEGER NOT NULL
            );
            INSERT INTO creature_template (entry, name, minlevel, maxlevel, rank) VALUES
                (639, 'Edwin VanCleef', 20, 20, 1),
                (645, 'Cookie', 20, 20, 1),
                (4275, 'Archmage Arugal', 26, 26, 1),
                (11502, 'Ragnaros', 63, 63, 3);",
        )
        .expect("seed temp world db");

        let instances = load_encounter_journal_from_world_db(&db_path).expect("load journal");
        let deadmines = instances
            .iter()
            .find(|instance| instance.instance_id == 1)
            .expect("deadmines");
        assert_eq!(deadmines.source, format!("world.db:{}", db_path.display()));
        assert_eq!(deadmines.bosses[0].entry, 639);
        assert_eq!(deadmines.bosses[0].name, "Edwin VanCleef");
        assert_eq!(deadmines.bosses[0].min_level, 20);

        let molten_core = instances
            .iter()
            .find(|instance| instance.instance_id == 3)
            .expect("molten core");
        assert_eq!(molten_core.bosses[0].entry, 11502);
        assert_eq!(molten_core.bosses[0].rank, 3);

        std::fs::remove_file(&db_path).ok();
        std::fs::remove_dir(&dir).ok();
    }

    #[test]
    fn load_encounter_journal_falls_back_when_world_db_missing() {
        let dir = std::env::temp_dir().join(format!(
            "encounter-journal-missing-worlddb-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .expect("clock")
                .as_nanos()
        ));
        std::fs::create_dir_all(&dir).expect("temp dir");
        let db_path = dir.join("missing.db");

        let instances = load_encounter_journal_from_world_db(&db_path);
        assert!(instances.is_err());
        std::fs::remove_dir(&dir).ok();
    }
}
