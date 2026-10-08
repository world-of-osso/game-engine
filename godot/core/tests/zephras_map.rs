use game_engine_core::map_catalog::MapCatalog;

#[test]
fn zephras_map_identity_joins_forever_without_replacing_retail() {
    let catalog = MapCatalog::parse(
        "ID,Directory,MapName_lang,WdtFileDataID\n0,Azeroth,Retail,775971\n",
        "ID,Directory,MapName_lang,WdtFileDataID\n0,Azeroth,Forever conflict,1\n2991,2991,Zephras Isle,7198644\n",
    ).unwrap();
    let map = catalog.by_directory("2991").unwrap();
    assert_eq!(map.id, 2991);
    assert_eq!(map.wdt_fdid, 7198644);
    assert_eq!(map.name, "Zephras Isle");
    assert_eq!(catalog.by_directory("AZEROTH").unwrap().wdt_fdid, 775971);
    assert_eq!(catalog.by_id(0).unwrap().name, "Retail");
    assert!(catalog.by_directory("missing").is_none());
}

#[test]
fn zephras_map_identity_reads_real_exports() {
    let catalog = MapCatalog::read(std::path::Path::new("data")).unwrap();
    assert_eq!(catalog.by_directory("2991").unwrap().wdt_fdid, 7198644);
    assert_eq!(catalog.by_directory("stormwindjail").unwrap().id, 34);
}
