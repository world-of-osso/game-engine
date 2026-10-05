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
        (2947, "Small Throwing Knife", 2, 25, 135641, 200),
        (2512, "Rough Arrow", 6, 24, 132382, 200),
        (2101, "Light Quiver", 11, 27, 134409, 1),
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
        (2947, "Small Throwing Knife", 25, 200),
        (2512, "Rough Arrow", 24, 200),
        (2101, "Light Quiver", 27, 1),
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
    assert!(sources.catalog(Forever70205).unwrap().get(2589).is_none());
    let missing = SourceItemCatalogs::new(retail, Err("Forever70205: missing Item.csv".into()));
    assert!(
        missing
            .catalog(Forever70205)
            .unwrap_err()
            .contains("Forever70205")
    );
    assert!(missing.catalog(Retail).unwrap().get(2589).is_some());
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
