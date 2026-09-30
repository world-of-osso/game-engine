//! Flight map state (docs/specs/flight-master.md): the server's `TaxiMap` for the
//! open flight master, and the Retail continent art it is drawn on.
//!
//! [`FlightMapArt`] joins the UiMap DB2 exports for a continent: the `UiMap` of type
//! Continent whose `UiMapAssignment` covers the whole map (UiMin 0,0 / UiMax 1,1) on
//! that `MapID`, its `UiMapXMapArt` → `UiMapArt` → `UiMapArtStyleLayer` (layer 0
//! size and tile size) and the layer-0 `UiMapArtTile` textures. World positions map
//! to the art like Retail `C_Map.GetMapPosFromWorldPos`: x runs west→east along
//! world -Y, y north→south along world -X, over the assignment's `Region`.

use std::path::Path;

use bevy::prelude::*;
use shared::protocol::{TaxiMap, TaxiNodeInfo, TaxiNodeState};

use crate::spell_catalog::csv_records::CsvTable;

/// `Enum.UIMapType.Continent`.
const UI_MAP_TYPE_CONTINENT: u32 = 2;

#[derive(Resource, Debug, Clone, Default, PartialEq)]
pub struct TaxiMapState {
    /// Server entity bits of the flight master.
    pub npc: Option<u64>,
    pub continent: u32,
    pub nodes: Vec<TaxiNodeInfo>,
    /// The pin under the cursor.
    pub hovered: Option<u32>,
}

impl TaxiMapState {
    pub fn is_open(&self) -> bool {
        self.npc.is_some()
    }

    pub fn open(&mut self, map: TaxiMap) {
        *self = Self {
            npc: Some(map.npc),
            continent: map.continent,
            nodes: map.nodes,
            hovered: None,
        };
    }

    pub fn close(&mut self) {
        *self = Self::default();
    }

    pub fn node(&self, node: u32) -> Option<&TaxiNodeInfo> {
        self.nodes.iter().find(|info| info.node == node)
    }

    pub fn current(&self) -> Option<&TaxiNodeInfo> {
        self.nodes
            .iter()
            .find(|info| info.state == TaxiNodeState::Current)
    }
}

/// Flight map actions for the open flight master.
#[derive(Message, Debug, Clone, Copy, PartialEq, Eq)]
pub enum TaxiRequest {
    /// `TakeTaxiNode`.
    Fly { destination: u32 },
    /// The frame closed (`CloseTaxiMap`).
    Close,
}

/// One `UiMapArtTile` of layer 0.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct MapTile {
    pub row: u32,
    pub col: u32,
    pub fdid: u32,
}

#[derive(Resource, Debug, Clone, Default, PartialEq)]
pub struct FlightMapArt {
    pub ui_map: u32,
    /// `LayerWidth`, `LayerHeight`.
    pub size: Vec2,
    /// `TileWidth`, `TileHeight`.
    pub tile: Vec2,
    pub tiles: Vec<MapTile>,
    /// `Region_0/1` (min world x, y) and `Region_3/4` (max).
    pub region_min: Vec2,
    pub region_max: Vec2,
}

impl FlightMapArt {
    /// Normalized map position (0..1, top-left origin) of a world position.
    pub fn map_position(&self, world_x: f32, world_y: f32) -> Vec2 {
        let span = self.region_max - self.region_min;
        Vec2::new(
            (self.region_max.y - world_y) / span.y,
            (self.region_max.x - world_x) / span.x,
        )
    }

    /// The continent art of `map_id`, from the DB2 CSVs in `dir`.
    pub fn load(dir: &Path, map_id: u32) -> Result<Self, String> {
        let (ui_map, region_min, region_max) = continent_assignment(dir, map_id)?;
        let art_id = single_value(dir, "UiMapXMapArt", ("UiMapID", ui_map), "UiMapArtID")?;
        let style = single_value(dir, "UiMapArt", ("ID", art_id), "UiMapArtStyleID")?;
        let (size, tile) = style_layer(dir, style)?;
        let tiles = art_tiles(dir, art_id)?;
        Ok(Self {
            ui_map,
            size,
            tile,
            tiles,
            region_min,
            region_max,
        })
    }
}

fn read(dir: &Path, table: &str) -> Result<CsvTable, String> {
    CsvTable::read(&dir.join(format!("{table}.csv")))
}

fn field<T: std::str::FromStr>(
    table: &CsvTable,
    record: &[std::borrow::Cow<str>],
    column: usize,
) -> Result<T, String> {
    let raw = record
        .get(column)
        .ok_or_else(|| format!("{} has a short record", table.path().display()))?;
    raw.parse().map_err(|_| {
        format!(
            "{} column {column}: bad value {raw:?}",
            table.path().display()
        )
    })
}

fn continent_ui_maps(dir: &Path) -> Result<Vec<u32>, String> {
    let table = read(dir, "UiMap")?;
    let (id, kind) = (table.column("ID")?, table.column("Type")?);
    let mut maps = Vec::new();
    for record in table.records() {
        if field::<u32>(&table, &record, kind)? == UI_MAP_TYPE_CONTINENT {
            maps.push(field(&table, &record, id)?);
        }
    }
    Ok(maps)
}

fn continent_assignment(dir: &Path, map_id: u32) -> Result<(u32, Vec2, Vec2), String> {
    let continents = continent_ui_maps(dir)?;
    let table = read(dir, "UiMapAssignment")?;
    let column = |name: &str| table.column(name);
    let columns = [
        "UiMapID", "MapID", "UiMin_0", "UiMin_1", "UiMax_0", "UiMax_1", "Region_0", "Region_1",
        "Region_3", "Region_4",
    ]
    .map(column);
    let [ui_map, map, min0, min1, max0, max1, r0, r1, r3, r4] = columns;
    let (ui_map, map, min0, min1, max0, max1, r0, r1, r3, r4) = (
        ui_map?, map?, min0?, min1?, max0?, max1?, r0?, r1?, r3?, r4?,
    );
    for record in table.records() {
        let id: u32 = field(&table, &record, ui_map)?;
        let whole = [min0, min1]
            .iter()
            .all(|&c| field::<f32>(&table, &record, c) == Ok(0.0))
            && [max0, max1]
                .iter()
                .all(|&c| field::<f32>(&table, &record, c) == Ok(1.0));
        if field::<u32>(&table, &record, map)? != map_id || !whole || !continents.contains(&id) {
            continue;
        }
        let min = Vec2::new(field(&table, &record, r0)?, field(&table, &record, r1)?);
        let max = Vec2::new(field(&table, &record, r3)?, field(&table, &record, r4)?);
        return Ok((id, min, max));
    }
    Err(format!("no continent UiMapAssignment for map {map_id}"))
}

fn single_value(
    dir: &Path,
    name: &str,
    (key, value): (&str, u32),
    wanted: &str,
) -> Result<u32, String> {
    let table = read(dir, name)?;
    let (key_column, wanted_column) = (table.column(key)?, table.column(wanted)?);
    for record in table.records() {
        if field::<u32>(&table, &record, key_column)? == value {
            return field(&table, &record, wanted_column);
        }
    }
    Err(format!("{name}: no row with {key} {value}"))
}

fn style_layer(dir: &Path, style: u32) -> Result<(Vec2, Vec2), String> {
    let table = read(dir, "UiMapArtStyleLayer")?;
    let columns = [
        "UiMapArtStyleID",
        "LayerIndex",
        "LayerWidth",
        "LayerHeight",
        "TileWidth",
        "TileHeight",
    ]
    .map(|name| table.column(name));
    let [style_column, layer, width, height, tile_w, tile_h] = columns;
    let (style_column, layer, width, height, tile_w, tile_h) =
        (style_column?, layer?, width?, height?, tile_w?, tile_h?);
    for record in table.records() {
        if field::<u32>(&table, &record, style_column)? == style
            && field::<u32>(&table, &record, layer)? == 0
        {
            let size = Vec2::new(
                field(&table, &record, width)?,
                field(&table, &record, height)?,
            );
            let tile = Vec2::new(
                field(&table, &record, tile_w)?,
                field(&table, &record, tile_h)?,
            );
            return Ok((size, tile));
        }
    }
    Err(format!("UiMapArtStyleLayer: no layer 0 for style {style}"))
}

fn art_tiles(dir: &Path, art_id: u32) -> Result<Vec<MapTile>, String> {
    let table = read(dir, "UiMapArtTile")?;
    let columns = [
        "UiMapArtID",
        "LayerIndex",
        "RowIndex",
        "ColIndex",
        "FileDataID",
    ]
    .map(|name| table.column(name));
    let [art, layer, row, col, fdid] = columns;
    let (art, layer, row, col, fdid) = (art?, layer?, row?, col?, fdid?);
    let mut tiles = Vec::new();
    for record in table.records() {
        if field::<u32>(&table, &record, art)? == art_id
            && field::<u32>(&table, &record, layer)? == 0
        {
            tiles.push(MapTile {
                row: field(&table, &record, row)?,
                col: field(&table, &record, col)?,
                fdid: field(&table, &record, fdid)?,
            });
        }
    }
    tiles.sort_by_key(|tile| (tile.row, tile.col));
    Ok(tiles)
}

#[cfg(test)]
#[path = "../../tests/unit/required_asset.rs"]
mod required_asset;

#[cfg(test)]
mod tests {
    use super::*;

    use super::required_asset::require_asset;

    const DB2: &str = "data/db2/12.1.0.69933";

    fn eastern_kingdoms() -> FlightMapArt {
        let dir = Path::new(env!("CARGO_MANIFEST_DIR")).join(DB2);
        require_asset(dir.join("UiMapArtTile.csv"));
        FlightMapArt::load(&dir, 0).unwrap()
    }

    #[test]
    fn eastern_kingdoms_art_is_uimap_13_in_150_tiles() {
        let art = eastern_kingdoms();
        assert_eq!(art.ui_map, 13);
        assert_eq!(art.size, Vec2::new(3840.0, 2560.0));
        assert_eq!(art.tile, Vec2::new(256.0, 256.0));
        assert_eq!(art.tiles.len(), 150);
        assert_eq!(
            art.tiles[0],
            MapTile {
                row: 0,
                col: 0,
                fdid: 2_353_944
            }
        );
        assert_eq!(
            art.tiles[149],
            MapTile {
                row: 9,
                col: 14,
                fdid: 2_354_001
            }
        );
    }

    #[test]
    fn flight_points_land_where_the_retail_map_shows_them() {
        let art = eastern_kingdoms();
        // Stormwind (TaxiNodes 2) and Sentinel Hill (4): south-west, Westfall south of it.
        let stormwind = art.map_position(-8841.06, 489.66);
        let sentinel = art.map_position(-10551.9, 1034.39);
        assert!((stormwind.x - 0.4435).abs() < 0.001, "{stormwind}");
        assert!((stormwind.y - 0.7647).abs() < 0.001, "{stormwind}");
        assert!(sentinel.y > stormwind.y && sentinel.x < stormwind.x);
    }

    #[test]
    fn the_map_state_keeps_the_open_flight_masters_nodes() {
        let mut state = TaxiMapState::default();
        state.open(TaxiMap {
            npc: 9,
            continent: 0,
            nodes: vec![TaxiNodeInfo {
                node: 2,
                name: "Stormwind, Elwynn".into(),
                world_x: -8841.06,
                world_y: 489.66,
                state: TaxiNodeState::Current,
                cost: 0,
                route: vec![],
            }],
        });
        assert!(state.is_open());
        assert_eq!(state.current().map(|node| node.node), Some(2));
        state.close();
        assert!(!state.is_open());
    }
}
