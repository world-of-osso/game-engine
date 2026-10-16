//! Retail Journal DB2 cache, imported from the pinned local CASC build.
mod difficulty;
mod encounter_locks;
mod presentation;
pub use encounter_locks::DungeonEncounter;
pub use presentation::{JourneyPresentation, read_journey_presentation};
use rusqlite::{Connection, OpenFlags, Row};
use std::{collections::BTreeMap, path::Path};

#[derive(Clone, Debug, PartialEq)]
pub struct Tier {
    pub id: u32,
    pub name: String,
    pub expansion: u32,
}
#[derive(Clone, Debug, PartialEq)]
pub struct Instance {
    pub id: u32,
    pub name: String,
    pub description: String,
    pub map: u32,
    pub background: u32,
    pub button: u32,
    pub lore: u32,
    pub dungeon: bool,
    pub tiers: Vec<u32>,
}
#[derive(Clone, Debug, PartialEq)]
pub struct Encounter {
    pub id: u32,
    pub name: String,
    pub description: String,
    pub instance: u32,
    pub dungeon_encounter: u32,
    pub order: u32,
    pub root: u32,
}
#[derive(Clone, Debug, PartialEq)]
pub struct Creature {
    pub id: u32,
    pub name: String,
    pub description: String,
    pub encounter: u32,
    pub display: u32,
    /// JournalEncounterCreature.FileDataID: the authored EncounterBossButton portrait.
    pub portrait: u32,
    pub order: u32,
}
#[derive(Clone, Debug, PartialEq)]
pub struct Section {
    pub id: u32,
    pub title: String,
    pub body: String,
    pub encounter: u32,
    pub parent: u32,
    pub child: u32,
    pub next: u32,
    pub kind: u32,
    pub spell: u32,
    pub icon: u32,
    pub roles: u32,
}
#[derive(Clone, Debug, PartialEq)]
pub struct Loot {
    pub id: u32,
    pub encounter: u32,
    pub item: u32,
}
#[derive(Clone, Debug, PartialEq)]
pub struct Item {
    pub id: u32,
    pub name: String,
    pub icon: u32,
    pub slot: u32,
    pub class_mask: i32,
    pub quality: u32,
    pub level: u32,
}
#[derive(Clone, Debug, PartialEq)]
pub struct ItemSet {
    pub id: u32,
    pub name: String,
    pub flags: u32,
    pub items: Vec<u32>,
}

#[derive(Clone, Debug, PartialEq)]
pub struct SetBonus {
    pub spec: u32,
    pub spell: u32,
    pub trait_subtree: u32,
    pub threshold: u32,
}
#[derive(Clone, Debug, PartialEq)]
pub struct Specialization {
    pub id: u32,
    pub name: String,
    pub class: u32,
}
#[derive(Clone, Debug, PartialEq)]
pub struct ChallengeDungeon {
    pub id: u32,
    pub map: u32,
    pub name: String,
    pub time_limit_ms: u64,
}

#[derive(Clone, Debug, Default, PartialEq)]
pub struct Catalog {
    pub tiers: Vec<Tier>,
    pub instances: Vec<Instance>,
    pub encounters: Vec<Encounter>,
    pub creatures: Vec<Creature>,
    pub sections: BTreeMap<u32, Section>,
    pub loot: Vec<Loot>,
    pub items: BTreeMap<u32, Item>,
    pub sets: Vec<ItemSet>,
    pub set_bonuses: BTreeMap<u32, Vec<SetBonus>>,
    pub challenge_dungeons: BTreeMap<u32, ChallengeDungeon>,
    pub specs: Vec<Specialization>,
    pub section_difficulties: BTreeMap<u32, Vec<u32>>,
    pub encounter_difficulties: BTreeMap<u32, Vec<u32>>,
    pub encounter_locks: BTreeMap<u32, DungeonEncounter>,
    pub loot_difficulties: BTreeMap<u32, Vec<u32>>,
    pub item_specs: BTreeMap<u32, Vec<u32>>,
    pub classes: BTreeMap<u32, String>,
    pub difficulty_max_players: BTreeMap<u32, u32>,
    pub map_difficulties: BTreeMap<u32, Vec<u32>>,
    pub text: BTreeMap<String, String>,
}
fn query<T>(
    db: &Connection,
    sql: &str,
    read: impl FnMut(&Row<'_>) -> rusqlite::Result<T>,
) -> Result<Vec<T>, String> {
    db.prepare(sql)
        .and_then(|mut statement| statement.query_map([], read)?.collect())
        .map_err(|e| format!("Adventure Guide {sql}: {e}"))
}
fn relations(db: &Connection, sql: &str) -> Result<BTreeMap<u32, Vec<u32>>, String> {
    let mut result = BTreeMap::<u32, Vec<u32>>::new();
    for (parent, value) in query(db, sql, |r| Ok((r.get(0)?, r.get(1)?)))? {
        result.entry(parent).or_default().push(value);
    }
    Ok(result)
}
impl Catalog {
    pub fn load(path: &Path) -> Result<Self, String> {
        let data = path
            .parent()
            .and_then(Path::parent)
            .ok_or_else(|| format!("Journal cache has no data root: {}", path.display()))?;
        let db = Connection::open_with_flags(path, OpenFlags::SQLITE_OPEN_READ_ONLY)
            .map_err(|e| format!("{}: {e}", path.display()))?;
        let builds = query(&db, "SELECT DISTINCT build FROM source", |row| {
            row.get::<_, String>(0)
        })?;
        if builds.len() != 1 || builds[0].is_empty() {
            return Err("Adventure Guide requires one declared source build".into());
        }
        let tiers = query(
            &db,
            "SELECT ID,Name_lang,Expansion FROM JournalTier ORDER BY Expansion",
            |r| {
                Ok(Tier {
                    id: r.get(0)?,
                    name: r.get(1)?,
                    expansion: r.get(2)?,
                })
            },
        )?;
        let tier_links = relations(
            &db,
            "SELECT JournalInstanceID,JournalTierID FROM JournalTierXInstance ORDER BY OrderIndex,ID",
        )?;
        let instances = query(
            &db,
            "SELECT j.ID,j.Name_lang,j.Description_lang,j.MapID,j.BackgroundFileDataID,j.ButtonFileDataID,j.LoreFileDataID,m.instance_type FROM JournalInstance j JOIN map_type m ON m.ID=j.MapID ORDER BY j.ID",
            |r| {
                let id = r.get(0)?;
                Ok(Instance {
                    id,
                    name: r.get(1)?,
                    description: r.get(2)?,
                    map: r.get(3)?,
                    background: r.get(4)?,
                    button: r.get(5)?,
                    lore: r.get(6)?,
                    dungeon: r.get::<_, u32>(7)? == 1,
                    tiers: tier_links.get(&id).cloned().unwrap_or_default(),
                })
            },
        )?;
        let encounters = query(
            &db,
            "SELECT ID,Name_lang,Description_lang,JournalInstanceID,DungeonEncounterID,OrderIndex,FirstSectionID FROM JournalEncounter ORDER BY OrderIndex,ID",
            |r| {
                Ok(Encounter {
                    id: r.get(0)?,
                    name: r.get(1)?,
                    description: r.get(2)?,
                    instance: r.get(3)?,
                    dungeon_encounter: r.get(4)?,
                    order: r.get(5)?,
                    root: r.get(6)?,
                })
            },
        )?;
        let creatures = query(
            &db,
            "SELECT ID,Name_lang,JournalEncounterID,CreatureDisplayInfoID,OrderIndex,FileDataID,Description_lang FROM JournalEncounterCreature ORDER BY OrderIndex,ID",
            |r| {
                Ok(Creature {
                    id: r.get(0)?,
                    name: r.get(1)?,
                    description: r.get(6)?,
                    encounter: r.get(2)?,
                    display: r.get(3)?,
                    order: r.get(4)?,
                    portrait: r.get(5)?,
                })
            },
        )?;
        let sections = query(
            &db,
            "SELECT ID,Title_lang,BodyText_lang,JournalEncounterID,ParentSectionID,FirstChildSectionID,NextSiblingSectionID,Type,SpellID,IconFileDataID,IconFlags FROM JournalEncounterSection",
            |row| Ok(Section {
                id: row.get(0)?, title: row.get(1)?, body: row.get(2)?,
                encounter: row.get(3)?, parent: row.get(4)?, child: row.get(5)?,
                next: row.get(6)?, kind: row.get(7)?, spell: row.get(8)?,
                icon: row.get(9)?, roles: row.get(10)?,
            }),
        )?.into_iter().map(|section| (section.id, section)).collect();
        let loot = query(
            &db,
            "SELECT ID,JournalEncounterID,ItemID FROM JournalEncounterItem",
            |r| {
                Ok(Loot {
                    id: r.get(0)?,
                    encounter: r.get(1)?,
                    item: r.get(2)?,
                })
            },
        )?;
        let items = query(
            &db,
            "SELECT ID,name,icon,slot,class_mask,quality,item_level FROM item",
            |r| {
                Ok(Item {
                    id: r.get(0)?,
                    name: r.get(1)?,
                    icon: r.get(2)?,
                    slot: r.get(3)?,
                    class_mask: r.get(4)?,
                    quality: r.get(5)?,
                    level: r.get(6)?,
                })
            },
        )?
        .into_iter()
        .map(|i| (i.id, i))
        .collect();
        let sets = query(&db, "SELECT * FROM ItemSet ORDER BY Name_lang", |r| {
            let items = (5..22)
                .map(|i| r.get(i))
                .collect::<rusqlite::Result<Vec<u32>>>()?;
            Ok(ItemSet {
                id: r.get(0)?,
                name: r.get(1)?,
                flags: r.get(2)?,
                items: items.into_iter().filter(|i| *i != 0).collect(),
            })
        })?;
        let specs = query(
            &db,
            "SELECT ID,name,class_id FROM specialization ORDER BY class_id,ID",
            |r| {
                Ok(Specialization {
                    id: r.get(0)?,
                    name: r.get(1)?,
                    class: r.get(2)?,
                })
            },
        )?;
        let mut set_bonuses: BTreeMap<u32, Vec<SetBonus>> = BTreeMap::new();
        let bonuses = query(
            &db,
            "SELECT ItemSetID,ChrSpecID,SpellID,TraitSubTreeID,Threshold FROM ItemSetSpell ORDER BY Threshold,ID",
            |r| {
                Ok((
                    r.get::<_, u32>(0)?,
                    SetBonus {
                        spec: r.get(1)?,
                        spell: r.get(2)?,
                        trait_subtree: r.get(3)?,
                        threshold: r.get(4)?,
                    },
                ))
            },
        )?;
        for (set, bonus) in bonuses {
            set_bonuses.entry(set).or_default().push(bonus);
        }
        Ok(Self {
            tiers,
            instances,
            encounters,
            creatures,
            sections,
            loot,
            items,
            sets,
            set_bonuses,
            challenge_dungeons: query(
                &db,
                "SELECT ID,map_id,name,time_limit_ms FROM challenge_dungeon",
                |r| {
                    Ok(ChallengeDungeon {
                        id: r.get(0)?,
                        map: r.get(1)?,
                        name: r.get(2)?,
                        time_limit_ms: r.get(3)?,
                    })
                },
            )?
            .into_iter()
            .map(|dungeon| (dungeon.id, dungeon))
            .collect(),
            specs,
            encounter_difficulties: relations(
                &db,
                "SELECT JournalEncounterID,DifficultyID FROM JournalEncounterXDifficulty",
            )?,
            section_difficulties: relations(
                &db,
                "SELECT JournalEncounterSectionID,DifficultyID FROM JournalSectionXDifficulty",
            )?,
            loot_difficulties: relations(
                &db,
                "SELECT JournalEncounterItemID,DifficultyID FROM JournalItemXDifficulty",
            )?,
            item_specs: relations(&db, "SELECT item_id,spec_id FROM item_specialization")?,
            difficulty_max_players: difficulty::read_max_players(data)?,
            encounter_locks: encounter_locks::read_encounter_locks(data)?,
            map_difficulties: relations(
                &db,
                "SELECT map_id,difficulty_id FROM map_difficulty ORDER BY difficulty_id",
            )?,
            classes: query(&db, "SELECT ID,name FROM class", |r| {
                Ok((r.get(0)?, r.get(1)?))
            })?
            .into_iter()
            .collect(),
            text: query(&db, "SELECT tag,text FROM journal_text", |r| {
                Ok((r.get(0)?, r.get(1)?))
            })?
            .into_iter()
            .collect(),
        })
    }
    pub fn encounter(&self, id: u32) -> Option<&Encounter> {
        self.encounters.iter().find(|e| e.id == id)
    }
    pub fn instance(&self, id: u32) -> Option<&Instance> {
        self.instances.iter().find(|e| e.id == id)
    }
    pub fn display(&self, encounter: u32) -> Option<u32> {
        self.creatures
            .iter()
            .find(|c| c.encounter == encounter)
            .map(|c| c.display)
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn published_70391_catalog_resolves_authored_instances_and_encounters() {
        let path = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("../../data/cache/adventure_guide.sqlite");
        let catalog = Catalog::load(&path).expect(
            "published70391 cache must load, not remain stuck behind an obsolete build gate",
        );
        let wailing = catalog.instance(240).expect("authored Wailing Caverns");
        assert_eq!(
            (wailing.name.as_str(), wailing.map, wailing.dungeon),
            ("Wailing Caverns", 43, true)
        );
        let molten = catalog.instance(741).expect("authored Molten Core");
        assert_eq!(
            (molten.name.as_str(), molten.map, molten.dungeon),
            ("Molten Core", 409, false)
        );
        let lucifron = catalog.encounter(1519).expect("authored Lucifron");
        assert_eq!(
            (
                lucifron.name.as_str(),
                lucifron.instance,
                lucifron.dungeon_encounter
            ),
            ("Lucifron", 741, 663)
        );
        assert!(catalog.instances.len() > 200 && catalog.encounters.len() > 1100);
    }
}

pub fn difficulty_matches(
    restrictions: &BTreeMap<u32, Vec<u32>>,
    id: u32,
    difficulty: u32,
) -> bool {
    restrictions
        .get(&id)
        .is_none_or(|ids| ids.contains(&difficulty))
}
