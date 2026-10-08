use std::path::Path;

use game_engine_core::{
    creature_display_cache::import_creature_display_cache, creature_display_data::query_display,
};
use rusqlite::Connection;

#[test]
fn forever_npc_displays_resolve_real_ailee_and_ventaari_without_retail_changes() {
    let source = Path::new("data");
    let root = std::env::temp_dir().join(format!("forever-npc-real-{}", std::process::id()));
    let forever = root.join("db2/1.60.1.70205");
    std::fs::create_dir_all(&forever).unwrap();
    for table in ["CreatureDisplayInfo", "CreatureModelData"] {
        let name = format!("{table}.csv");
        std::fs::copy(
            source.join("db2/12.1.0.69933").join(&name),
            root.join(&name),
        )
        .unwrap();
        std::fs::copy(
            source.join("db2/1.60.1.70205").join(&name),
            forever.join(&name),
        )
        .unwrap();
    }
    // First capture the real Retail cache, then enable the product overlay.
    let held = root.join("forever-held");
    std::fs::rename(&forever, &held).unwrap();
    let cache = import_creature_display_cache(&root).unwrap();
    let connection = Connection::open(&cache).unwrap();
    let retail_ids: Vec<u32> = connection
        .prepare("SELECT display_id FROM creature_displays ORDER BY display_id")
        .unwrap()
        .query_map([], |row| row.get(0))
        .unwrap()
        .collect::<Result<_, _>>()
        .unwrap();
    let retail: Vec<_> = retail_ids
        .iter()
        .map(|&id| (id, query_display(&connection, id).unwrap().unwrap()))
        .collect();
    drop(connection);
    std::fs::rename(&held, &forever).unwrap();
    let cache = import_creature_display_cache(&root).unwrap();
    let connection = Connection::open(&cache).unwrap();
    for (id, display) in retail {
        assert_eq!(
            query_display(&connection, id).unwrap(),
            Some(display),
            "Retail display {id}"
        );
    }
    for id in [136968, 139694] {
        let display = query_display(&connection, id).unwrap().unwrap();
        assert_eq!(display.model_fdid, 7_478_494);
        assert_eq!(display.skin_fdids, [0; 4]);
        assert_eq!(display.scale_milli, 1000);
    }
    drop(connection);
    std::fs::remove_dir_all(root).unwrap();
}
