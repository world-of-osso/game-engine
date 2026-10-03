use std::path::PathBuf;
use std::sync::OnceLock;

use super::*;

const EASTERN_KINGDOMS_MAP: u32 = 0;
const ELWYNN: u32 = 37;
const NORTHSHIRE: u32 = 425;
const EASTERN_KINGDOMS: u32 = 13;
const AZEROTH: u32 = 947;
/// TaxiNodes 582 "Goldshire, Elwynn".
const GOLDSHIRE: [f32; 3] = [-9433.99, 85.149, 57.0];
/// Inside Northshire Abbey's grounds.
const NORTHSHIRE_ABBEY: [f32; 3] = [-8914.0, -133.0, 81.0];

fn catalog() -> &'static UiMapCatalog {
    static CATALOG: OnceLock<UiMapCatalog> = OnceLock::new();
    CATALOG.get_or_init(|| {
        let root = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        let dir = [
            root.join("data/db2/12.1.0.69933"),
            root.join("../../data/db2/12.1.0.69933"),
        ]
        .into_iter()
        .find(|dir| dir.join("UiMapArtTile.csv").exists())
        .expect("UiMapArtTile.csv export (scripts/export_db2_csv.py)");
        UiMapCatalog::load(&dir).unwrap()
    })
}

fn close(actual: [f32; 2], expected: [f32; 2]) -> bool {
    (actual[0] - expected[0]).abs() < 0.002 && (actual[1] - expected[1]).abs() < 0.002
}

#[test]
fn best_map_is_the_smallest_zone_under_the_player() {
    let catalog = catalog();
    assert_eq!(
        catalog.best_map_for_position(EASTERN_KINGDOMS_MAP, GOLDSHIRE),
        Some(ELWYNN)
    );
    // Retail shows the Northshire starting-area map inside the abbey grounds.
    assert_eq!(
        catalog.best_map_for_position(EASTERN_KINGDOMS_MAP, NORTHSHIRE_ABBEY),
        Some(NORTHSHIRE)
    );
    // Open sea west of the continent still resolves to the continent map.
    assert_eq!(
        catalog.best_map_for_position(EASTERN_KINGDOMS_MAP, [-9000.0, 4000.0, 0.0]),
        Some(EASTERN_KINGDOMS)
    );
}

#[test]
fn zone_zooms_out_to_continent_then_world() {
    let catalog = catalog();
    let lineage = catalog.lineage(ELWYNN);
    assert_eq!(&lineage[..3], &[ELWYNN, EASTERN_KINGDOMS, AZEROTH]);
    assert_eq!(catalog.map(ELWYNN).unwrap().name, "Elwynn Forest");
    assert_eq!(catalog.map(AZEROTH).unwrap().kind, map_type::WORLD);
}

#[test]
fn map_position_matches_retail_coordinates() {
    let catalog = catalog();
    // Retail shows the Goldshire flight master at 41.8, 64.6 on Elwynn Forest.
    let zone = catalog
        .map_position(ELWYNN, EASTERN_KINGDOMS_MAP, GOLDSHIRE)
        .unwrap();
    assert!(close(zone, [0.4178, 0.6456]), "{zone:?}");
    let continent = catalog
        .map_position(EASTERN_KINGDOMS, EASTERN_KINGDOMS_MAP, GOLDSHIRE)
        .unwrap();
    assert!(close(continent, [0.4532, 0.7860]), "{continent:?}");
    let world = catalog
        .map_position(AZEROTH, EASTERN_KINGDOMS_MAP, GOLDSHIRE)
        .unwrap();
    assert!(close(world, [0.8555, 0.6381]), "{world:?}");
    assert_eq!(
        catalog.map_position(ELWYNN, EASTERN_KINGDOMS_MAP, [0.0, 0.0, 0.0]),
        None
    );
}

#[test]
fn child_at_finds_the_zone_and_continent_under_the_cursor() {
    let catalog = catalog();
    let on_continent = catalog
        .map_position(EASTERN_KINGDOMS, EASTERN_KINGDOMS_MAP, GOLDSHIRE)
        .unwrap();
    let (zone, rect) = catalog.child_at(EASTERN_KINGDOMS, on_continent).unwrap();
    assert_eq!(zone, ELWYNN);
    // Elwynn's region projected onto the continent.
    assert!(close([rect[0], rect[1]], [0.4184, 0.7322]), "{rect:?}");
    assert!(close([rect[2], rect[3]], [0.0834, 0.0834]), "{rect:?}");

    let on_world = catalog
        .map_position(AZEROTH, EASTERN_KINGDOMS_MAP, GOLDSHIRE)
        .unwrap();
    let (continent, rect) = catalog.child_at(AZEROTH, on_world).unwrap();
    assert_eq!(continent, EASTERN_KINGDOMS);
    // Clipped to where Azeroth draws map 0.
    assert!(close([rect[0], rect[1]], [0.775, 0.2817]), "{rect:?}");
    assert!(close([rect[2], rect[3]], [0.22, 0.4483]), "{rect:?}");
    assert_eq!(catalog.child_at(AZEROTH, [0.02, 0.02]), None);
}

#[test]
fn zone_art_tiles_are_trimmed_to_the_layer() {
    let catalog = catalog();
    let art = catalog.art(ELWYNN).unwrap();
    assert_eq!(art.size, [1002.0, 668.0]);
    assert_ne!(art.highlight_fdid, 0);
    let tiles = art.placed_tiles();
    assert_eq!(tiles.len(), 12);
    let corner = tiles.last().unwrap();
    let (w, h) = (1002.0 - 768.0, 668.0 - 512.0);
    assert_eq!(
        corner.rect,
        [768.0 / 1002.0, 512.0 / 668.0, w / 1002.0, h / 668.0]
    );
    assert_eq!(corner.tex_coords, [0.0, w / 256.0, 0.0, h / 256.0]);
    assert_eq!(
        catalog.art(EASTERN_KINGDOMS).unwrap().placed_tiles().len(),
        150
    );
}
