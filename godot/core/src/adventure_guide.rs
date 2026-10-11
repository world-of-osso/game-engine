//! Retail Journal DB2 cache, imported from the pinned local CASC build.
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
    pub encounter: u32,
    pub display: u32,
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
    pub items: Vec<u32>,
}
#[derive(Clone, Debug, PartialEq)]
pub struct Specialization {
    pub id: u32,
    pub name: String,
    pub class: u32,
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
    pub specs: Vec<Specialization>,
    pub section_difficulties: BTreeMap<u32, Vec<u32>>,
    pub loot_difficulties: BTreeMap<u32, Vec<u32>>,
    pub item_specs: BTreeMap<u32, Vec<u32>>,
    pub classes: BTreeMap<u32, String>,
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
        let db = Connection::open_with_flags(path, OpenFlags::SQLITE_OPEN_READ_ONLY)
            .map_err(|e| format!("{}: {e}", path.display()))?;
        let build: String = db
            .query_row("SELECT build FROM source LIMIT 1", [], |r| r.get(0))
            .map_err(|e| e.to_string())?;
        if build != "12.1.0.69933" {
            return Err(format!("Adventure Guide unsupported build {build}"));
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
            "SELECT ID,Name_lang,Description_lang,MapID,BackgroundFileDataID,ButtonFileDataID,LoreFileDataID,Flags FROM JournalInstance ORDER BY ID",
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
                    dungeon: r.get::<_, u32>(7)? & 1 != 0,
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
            "SELECT ID,Name_lang,JournalEncounterID,CreatureDisplayInfoID,OrderIndex FROM JournalEncounterCreature ORDER BY OrderIndex,ID",
            |r| {
                Ok(Creature {
                    id: r.get(0)?,
                    name: r.get(1)?,
                    encounter: r.get(2)?,
                    display: r.get(3)?,
                    order: r.get(4)?,
                })
            },
        )?;
        let sections=query(&db,"SELECT ID,Title_lang,BodyText_lang,JournalEncounterID,ParentSectionID,FirstChildSectionID,NextSiblingSectionID,Type,SpellID,IconFileDataID,IconFlags FROM JournalEncounterSection",|r|Ok(Section{id:r.get(0)?,title:r.get(1)?,body:r.get(2)?,encounter:r.get(3)?,parent:r.get(4)?,child:r.get(5)?,next:r.get(6)?,kind:r.get(7)?,spell:r.get(8)?,icon:r.get(9)?,roles:r.get(10)?}))?.into_iter().map(|s|(s.id,s)).collect();
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
        Ok(Self {
            tiers,
            instances,
            encounters,
            creatures,
            sections,
            loot,
            items,
            sets,
            specs,
            section_difficulties: relations(
                &db,
                "SELECT JournalEncounterSectionID,DifficultyID FROM JournalSectionXDifficulty",
            )?,
            loot_difficulties: relations(
                &db,
                "SELECT JournalEncounterItemID,DifficultyID FROM JournalItemXDifficulty",
            )?,
            item_specs: relations(&db, "SELECT item_id,spec_id FROM item_specialization")?,
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
pub fn difficulty_matches(
    restrictions: &BTreeMap<u32, Vec<u32>>,
    id: u32,
    difficulty: u32,
) -> bool {
    restrictions
        .get(&id)
        .is_none_or(|ids| ids.contains(&difficulty))
}
