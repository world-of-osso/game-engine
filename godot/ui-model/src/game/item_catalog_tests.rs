use super::*;

const ITEM_CSV: &str = "ID,ClassID,SubclassID,Material,InventoryType,SheatheType,Sound_override_subclassID,IconFileDataID,ItemGroupSoundsID\n\
    2447,7,9,7,0,0,-1,133939,23\n\
    2589,7,5,8,0,0,-1,132889,7\n\
    25,2,7,1,21,3,-1,135274,0\n\
    250223,4,0,3,12,0,-1,133323,24\n";

/// The ItemSparse columns the catalog reads, values from build 12.1.0.69933.
const SPARSE_CSV: &str = "ID,Description_lang,Display_lang,Stackable,MaxCount,SellPrice,ItemLevel,Bonding,RequiredLevel,InventoryType,OverallQualityID,ContainerSlots,ExpansionID\n\
    25,,\"Worn Shortsword\",1,0,3,1,2,1,21,1,0,0\n\
    2589,,\"Linen Cloth\",1000,0,13,10,0,0,0,1,0,0\n\
    2447,\"A quoted\r\nline break\",\"Peacebloom\",1000,0,5,5,0,0,0,1,0,0\n\
    250223,,\"Soulcatcher's Charm\",1,0,265845,108,1,78,12,3,0,11\n\
    99999,,\"No Item Row\",1,0,1,1,0,0,0,1,0,0\n";

fn table(name: &str, text: &str) -> CsvTable {
    CsvTable::parse(Path::new(name), text).unwrap()
}

#[test]
fn collision_catalogs_preserve_source_and_never_borrow_missing_rows() {
    use shared::item_data::ItemDefinitionSource::{Forever70205, Retail};
    let mut retail = catalog();
    let mut forever = catalog();
    for (id, name, class, kind, icon, stackable) in [
        (2947, "Small Throwing Knife", 2, 25, 135426, 1),
        (2512, "Rough Arrow", 6, 24, 132382, 200),
        (2101, "Light Quiver", 11, 18, 134409, 1),
    ] {
        retail.items.insert(
            id,
            ItemCatalogEntry {
                name: format!("Retail {id}"),
                class_id: 15,
                icon_fdid: 134400,
                stackable: 1,
                ..Default::default()
            },
        );
        forever.items.insert(
            id,
            ItemCatalogEntry {
                name: name.into(),
                class_id: class,
                inventory_type: kind,
                icon_fdid: icon,
                stackable,
                ..Default::default()
            },
        );
    }
    forever.items.remove(&2589);
    let sources = SourceItemCatalogs::new(retail.clone(), Ok(forever));
    assert_eq!(sources.catalog(Retail).unwrap(), &retail);
    for (id, name, kind, stackable) in [
        (2947, "Small Throwing Knife", 25, 1),
        (2512, "Rough Arrow", 24, 200),
        (2101, "Light Quiver", 18, 1),
    ] {
        let entry = sources.catalog(Forever70205).unwrap().get(id).unwrap();
        assert_eq!(
            (entry.name.as_str(), entry.inventory_type, entry.stackable),
            (name, kind, stackable)
        );
        assert_ne!(
            entry.icon_fdid,
            sources.catalog(Retail).unwrap().icon_fdid(id).unwrap()
        );
    }
    let mut inventory = crate::bag_data::InventoryState::default();
    for (index, (source, id)) in [
        (Retail, 2947),
        (Forever70205, 2947),
        (Retail, 2512),
        (Forever70205, 2512),
        (Retail, 2101),
        (Forever70205, 2101),
    ]
    .into_iter()
    .enumerate()
    {
        let stack = shared::protocol::ItemStack {
            item_guid: index as u64 + 1,
            item_id: id,
            definition_source: source,
            count: 1,
            durability: None,
            soulbound: false,
        };
        let slot = crate::bag_data::stack_slot_in_catalog(&stack, sources.catalog(source).ok());
        let tooltip = crate::item_tooltip::item_tooltip_in_catalog(
            &slot,
            Some(10),
            sources.catalog(source).ok(),
        );
        assert_eq!(
            tooltip.title,
            sources.catalog(source).unwrap().get(id).unwrap().name
        );
        assert_eq!(slot.definition_source, source);
        inventory.set_item(0, index, slot);
    }
    assert_ne!(
        inventory.slot(0, 0).unwrap().name,
        inventory.slot(0, 1).unwrap().name
    );
    assert_ne!(
        inventory.slot(0, 2).unwrap().icon_fdid,
        inventory.slot(0, 3).unwrap().icon_fdid
    );
    inventory.equipment.insert(
        shared::protocol::EquipmentSlot::MainHand,
        inventory.slot(0, 0).unwrap().clone(),
    );
    let comparisons = crate::game_tooltip::item::comparisons_in_catalogs(
        inventory.slot(0, 1).unwrap(),
        &inventory,
        Some(10),
        &sources,
    );
    assert_eq!(comparisons.len(), 1);
    assert_eq!(comparisons[0].tooltip.title, "Retail 2947");
    assert!(
        crate::game_tooltip::item::comparisons_in_catalogs(
            inventory.slot(0, 0).unwrap(),
            &inventory,
            Some(10),
            &sources
        )
        .is_empty()
    );
    assert!(sources.catalog(Forever70205).unwrap().get(2589).is_none());
    let error = sources.entry(Forever70205, 2589).unwrap_err();
    assert!(error.contains("Forever70205 item 2589"), "{error}");
    let absent = shared::protocol::ItemStack {
        item_guid: 20,
        item_id: 2589,
        definition_source: Forever70205,
        count: 1,
        durability: None,
        soulbound: false,
    };
    let absent_slot =
        crate::bag_data::stack_slot_in_catalog(&absent, sources.catalog(Forever70205).ok());
    assert!(absent_slot.name.is_empty());
    assert_ne!(
        absent_slot.icon_fdid,
        sources.catalog(Retail).unwrap().icon_fdid(2589).unwrap()
    );
    let missing = SourceItemCatalogs::new(retail, Err("Forever70205: missing Item.csv".into()));
    assert!(
        missing
            .catalog(Forever70205)
            .unwrap_err()
            .contains("Forever70205")
    );
    assert!(missing.catalog(Retail).unwrap().get(2589).is_some());
    let error = missing.entry(Forever70205, 2947).unwrap_err();
    assert!(error.contains("Forever70205 item 2947"), "{error}");
}

#[test]
fn authored_forever_source_survives_mixed_snapshot_tooltips_and_equipped_levels() {
    use shared::item_data::ItemDefinitionSource::{Forever70205, Retail};
    use shared::protocol::{BagContents, BagSlotItem, EquipmentSlot, InventorySnapshot, ItemStack};
    wait_for_item_catalog();
    let catalogs = item_catalogs().unwrap();
    let forever = catalogs
        .catalog(Forever70205)
        .expect("provisioned selected Forever kit");
    assert_eq!(forever.len(), 30);
    let pairs = [
        (Retail, 2947),
        (Forever70205, 2947),
        (Retail, 2512),
        (Forever70205, 2512),
        (Retail, 2101),
        (Forever70205, 2101),
    ];
    let items = pairs
        .into_iter()
        .enumerate()
        .map(|(slot, (source, id))| BagSlotItem {
            slot: slot as u8,
            item: ItemStack {
                item_guid: slot as u64 + 1,
                item_id: id,
                definition_source: source,
                count: 1,
                durability: None,
                soulbound: false,
            },
        })
        .collect();
    let mut inventory = crate::bag_data::InventoryState::default();
    inventory.apply_snapshot(&InventorySnapshot {
        bags: vec![BagContents {
            bag: 0,
            size: 16,
            items,
        }],
    });
    assert_eq!(
        inventory.slot(0, 0).unwrap().name,
        "Broken Small Throwing Knife"
    );
    assert_eq!(inventory.slot(0, 1).unwrap().name, "Small Throwing Knife");
    for (index, (source, id)) in pairs.into_iter().enumerate() {
        let slot = inventory.slot(0, index).unwrap();
        let entry = catalogs.entry(source, id).unwrap();
        assert_eq!(slot.definition_source, source);
        assert_eq!(
            slot.icon_fdid,
            catalogs.catalog(source).unwrap().icon_fdid(id).unwrap()
        );
        let tooltip = crate::item_tooltip::item_tooltip(slot, Some(10));
        assert_eq!(tooltip.title, entry.name);
        assert_eq!(slot.name, entry.name);
    }
    assert_eq!(
        (
            catalogs.entry(Retail, 2947).unwrap().class_id,
            catalogs.entry(Forever70205, 2947).unwrap().class_id
        ),
        (15, 2)
    );
    assert_eq!(
        (
            catalogs.entry(Retail, 2512).unwrap().stackable,
            catalogs.entry(Forever70205, 2512).unwrap().stackable
        ),
        (1000, 200)
    );
    assert_eq!(
        (
            catalogs.entry(Retail, 2101).unwrap().class_id,
            catalogs.entry(Forever70205, 2101).unwrap().class_id
        ),
        (1, 11)
    );
    let arrow = crate::item_tooltip::item_tooltip(inventory.slot(0, 3).unwrap(), Some(10));
    assert!(
        arrow
            .lines
            .iter()
            .any(|line| line.left_text == "Ammo" && line.right_text == "Arrow")
    );
    let quiver = crate::item_tooltip::item_tooltip(inventory.slot(0, 5).unwrap(), Some(10));
    assert!(
        quiver
            .lines
            .iter()
            .any(|line| line.left_text == "6 Slot Quiver")
    );
    inventory.equipment.insert(
        EquipmentSlot::MainHand,
        inventory.slot(0, 0).unwrap().clone(),
    );
    let comparisons =
        crate::game_tooltip::item::comparisons(inventory.slot(0, 1).unwrap(), &inventory, Some(10));
    assert_eq!(comparisons.len(), 1);
    assert_eq!(comparisons[0].tooltip.title, "Broken Small Throwing Knife");
    let level = crate::character_frame::average_equipped_item_level(&inventory, |slot| {
        catalogs
            .entry(slot.definition_source, slot.item_id)
            .ok()
            .map(|item| (item.item_level, item.inventory_type))
    });
    assert_eq!(level, 2.0 / 16.0);
    inventory.equipment.insert(
        EquipmentSlot::MainHand,
        inventory.slot(0, 1).unwrap().clone(),
    );
    let level = crate::character_frame::average_equipped_item_level(&inventory, |slot| {
        catalogs
            .entry(slot.definition_source, slot.item_id)
            .ok()
            .map(|item| (item.item_level, item.inventory_type))
    });
    assert_eq!(level, 3.0 / 16.0);
    assert!(catalogs.entry(Forever70205, 2589).is_err());
    assert!(catalogs.entry(Retail, 2589).is_ok());
    let staff = catalogs.entry(Forever70205, 35).unwrap();
    let damage = crate::item_stats::weapon_damage_for(Forever70205, staff).unwrap();
    assert!((damage.dps - 1.3212558).abs() < 0.00001, "{damage:?}");
    assert_ne!(
        damage,
        crate::item_stats::weapon_damage_for(Retail, staff).unwrap()
    );
}

#[test]
fn retail_item_scaling_fields_keep_socket_costs_aligned_to_used_stats() {
    let mut catalog = catalog();
    let headers = "ID,StatModifier_bonusStat_0,StatPercentEditor_0,StatPercentageOfSocket_0,StatModifier_bonusStat_1,StatPercentEditor_1,StatPercentageOfSocket_1,StatModifier_bonusStat_2,StatPercentEditor_2,StatPercentageOfSocket_2,StatModifier_bonusStat_3,StatPercentEditor_3,StatPercentageOfSocket_3,StatModifier_bonusStat_4,StatPercentEditor_4,StatPercentageOfSocket_4,StatModifier_bonusStat_5,StatPercentEditor_5,StatPercentageOfSocket_5,StatModifier_bonusStat_6,StatPercentEditor_6,StatPercentageOfSocket_6,StatModifier_bonusStat_7,StatPercentEditor_7,StatPercentageOfSocket_7,StatModifier_bonusStat_8,StatPercentEditor_8,StatPercentageOfSocket_8,StatModifier_bonusStat_9,StatPercentEditor_9,StatPercentageOfSocket_9";
    let sparse = table(
        "ItemSparse.csv",
        &format!(
            "{headers}\n25,-1,0,0,7,7889,0.25,-1,0,0,32,4300,0.75,-1,0,0,-1,0,0,-1,0,0,-1,0,0,-1,0,0,-1,0,0\n"
        ),
    );
    let items = table("Item.csv", "ID,ItemSquishEraID\n25,2\n");
    apply_retail_scaling_fields(&mut catalog, &items, &sparse).unwrap();
    let item = catalog.get(25).unwrap();
    assert_eq!(item.squish_era, 2);
    assert_eq!(
        item.stat_socket_multipliers,
        [0.25, 0.75, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0]
    );
    let missing_era = table("Item.csv", "ID\n25\n");
    assert!(
        apply_retail_scaling_fields(&mut catalog, &missing_era, &sparse)
            .unwrap_err()
            .contains("ItemSquishEraID")
    );
}

fn catalog() -> ItemCatalog {
    let mut catalog = parse_item_catalog(&table("Item.csv", ITEM_CSV)).unwrap();
    apply_item_sparse(&mut catalog, &table("ItemSparse.csv", SPARSE_CSV)).unwrap();
    apply_subclass_names(
        &mut catalog,
        &table(
            "ItemSubClass.csv",
            "DisplayName_lang,VerboseName_lang,ID,ClassID,SubClassID\nSword,\"One-Handed Swords\",9,2,7\n",
        ),
    )
    .unwrap();
    catalog
}

#[test]
fn parses_class_and_icon_by_item_id() {
    let catalog = catalog();

    assert_eq!(catalog.len(), 4);
    let linen = catalog.get(2589).unwrap();
    assert_eq!(
        (linen.class_id, linen.subclass_id, linen.icon_fdid),
        (7, 5, 132889)
    );
    assert_eq!(catalog.get(25).map(|item| item.class_id), Some(2));
    assert_eq!(catalog.get(25).map(|item| item.sheathe_type), Some(3));
    assert_eq!(catalog.subclass_name(2, 7), Some("Sword"));
    assert_eq!(catalog.subclass_name(2, 8), None);
    assert_eq!(catalog.get(9999), None);
}

#[test]
fn item_sparse_adds_name_quality_stack_and_sell_price() {
    let catalog = catalog();

    let sword = catalog.get(25).unwrap();
    assert_eq!(sword.name, "Worn Shortsword");
    assert_eq!(
        (
            sword.quality,
            sword.stackable,
            sword.sell_price,
            sword.bonding
        ),
        (1, 1, 3, 2)
    );
    assert_eq!(
        (sword.required_level, sword.inventory_type, sword.item_level),
        (1, 21, 1)
    );
    let linen = catalog.get(2589).unwrap();
    assert_eq!(
        (linen.name.as_str(), linen.stackable, linen.sell_price),
        ("Linen Cloth", 1000, 13)
    );
    assert!(catalog.get(99999).is_none(), "sparse rows need an Item row");
    let charm = catalog.get(250223).unwrap();
    assert_eq!(
        (charm.expansion_id, linen.expansion_id),
        (11, 0),
        "ExpansionID: Soulcatcher's Charm is a Midnight (11) item"
    );
    // A quoted description may span lines (ItemSparse 151800 "Radiant Moonlight").
    let peacebloom = catalog.get(2447).unwrap();
    assert_eq!(
        (peacebloom.name.as_str(), peacebloom.description.as_str()),
        ("Peacebloom", "A quoted\r\nline break")
    );
}

#[test]
fn missing_icon_column_is_an_error() {
    let error = parse_item_catalog(&table("x", "ID,ClassID,SubclassID\n1,2,3\n")).unwrap_err();

    assert!(error.contains("IconFileDataID"), "{error}");
}

#[test]
fn retail_tables_name_linen_and_the_grey_ruined_pelt() {
    let linen = wait_for_item_catalog()
        .get(2589)
        .expect("Linen Cloth in the pinned DB2 export");
    assert_eq!(
        (linen.name.as_str(), linen.quality, linen.sell_price),
        ("Linen Cloth", 1, 13)
    );
    let pelt = wait_for_item_catalog().get(4865).expect("Ruined Pelt");
    assert_eq!(
        (pelt.name.as_str(), pelt.quality, pelt.stackable),
        ("Ruined Pelt", 0, 20)
    );
}
