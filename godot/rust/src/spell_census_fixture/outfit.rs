//! Exact CharStartOutfit rows, resolved through the client's Retail item metadata.
use std::collections::{HashMap, HashSet};
use std::path::Path;

use game_engine_core::csv_util::read_db2_rows;
use shared::components::{
    EquipmentAppearance, EquipmentVisualSlot, EquippedAppearanceEntry, SheathState,
};
use shared::item_data::ItemDefinitionSource;
use shared::protocol::EquipmentSlot;

#[derive(Clone, Copy)]
struct Item {
    inventory_type: u8,
    subclass: u8,
    class: u8,
}

pub(super) struct OutfitCatalog {
    rows: HashMap<(u8, u8, u8), Vec<u32>>,
    items: HashMap<u32, Item>,
}

pub(super) struct StartingOutfit {
    pub item_ids: Vec<u32>,
    pub equipment: EquipmentAppearance,
    pub skipped: Vec<String>,
    pub weapon_subclass: Option<u8>,
    pub sheath: SheathState,
}

impl OutfitCatalog {
    pub fn load(data: &Path) -> Result<Self, String> {
        let mut rows = HashMap::new();
        let path = data.join("CharStartOutfit.csv");
        read_db2_rows(&path, |row| {
            let key = (
                row.parse("RaceID")?,
                row.parse("ClassID")?,
                row.parse("SexID")?,
            );
            let mut items = Vec::new();
            for index in 0..12 {
                let id: u32 = row.parse(&format!("ItemID_{index}"))?;
                if id != 0 {
                    items.push(id);
                }
            }
            if rows.insert(key, items).is_some() {
                return Err(format!(
                    "{}: duplicate race/class/sex {key:?}",
                    path.display()
                ));
            }
            Ok(())
        })?;
        let wanted: HashSet<u32> = rows.values().flatten().copied().collect();
        let mut items = HashMap::new();
        read_db2_rows(&data.join("db2/active/Item.csv"), |row| {
            let id: u32 = row.parse("ID")?;
            if wanted.contains(&id) {
                items.insert(
                    id,
                    Item {
                        inventory_type: row.parse("InventoryType")?,
                        subclass: row.parse("SubclassID")?,
                        class: row.parse("ClassID")?,
                    },
                );
            }
            Ok(())
        })?;
        Ok(Self { rows, items })
    }

    pub fn resolve(&self, race: u8, class: u8, sex: u8) -> Option<StartingOutfit> {
        let item_ids = self.rows.get(&(race, class, sex))?.clone();
        let mut entries: Vec<EquippedAppearanceEntry> = Vec::new();
        let mut skipped = Vec::new();
        let mut weapon_subclass = None;
        let mut sheath = SheathState::Melee;
        for &id in &item_ids {
            let Some(item) = self.items.get(&id) else {
                skipped.push(format!("item {id}: missing Item row"));
                continue;
            };
            let slots = EquipmentSlot::for_inventory_type(item.inventory_type);
            let visual_slots: Vec<_> = slots.iter().filter_map(|slot| slot.visual_slot()).collect();
            if visual_slots.is_empty() {
                continue;
            }
            let Some(slot) = visual_slots
                .into_iter()
                .find(|slot| !entries.iter().any(|entry| entry.slot == *slot))
            else {
                skipped.push(format!("item {id}: no free authored inventory-type slot"));
                continue;
            };
            if slot == EquipmentVisualSlot::MainHand && item.class == 2 {
                weapon_subclass = Some(item.subclass);
                if matches!(item.inventory_type, 15 | 25 | 26) {
                    sheath = SheathState::Ranged;
                }
            }
            entries.push(EquippedAppearanceEntry {
                slot,
                item_id: Some(id),
                definition_source: Some(ItemDefinitionSource::Retail),
                display_info_id: None,
                inventory_type: item.inventory_type,
                hidden: false,
            });
        }
        Some(StartingOutfit {
            item_ids,
            equipment: EquipmentAppearance { entries },
            skipped,
            weapon_subclass,
            sheath,
        })
    }
}
