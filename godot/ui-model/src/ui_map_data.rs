//! Retail `UiMap` DB2 catalog for the world map (docs/specs/world-map.md), free of
//! engine types so the Bevy and Godot hosts share it.
//!
//! Joins the exported CSVs: `UiMap` (hierarchy, type, system), `UiMapAssignment`
//! (world regions ↔ map UV rects), `UiMapXMapArt` → `UiMapArt` → `UiMapArtStyleLayer`
//! (layer-0 size and tile size) → `UiMapArtTile` (layer-0 tile textures).
//!
//! World positions are Retail world coordinates (x north, y west). A map UV has x
//! west→east along world -Y and y north→south along world -X, like
//! `C_Map.GetMapPosFromWorldPos`.

use std::collections::HashMap;
use std::path::Path;

use crate::csv_util::parse_csv_line;

/// `Enum.UIMapType`.
pub mod map_type {
    pub const COSMIC: u8 = 0;
    pub const WORLD: u8 = 1;
    pub const CONTINENT: u8 = 2;
    pub const ZONE: u8 = 3;
}

/// `Enum.UIMapSystem.World`; Taxi and Adventure maps are separate hierarchies.
const SYSTEM_WORLD: u8 = 0;

#[derive(Debug, Clone, PartialEq)]
pub struct UiMapInfo {
    pub id: u32,
    pub name: String,
    pub parent: u32,
    pub kind: u8,
    pub system: u8,
}

/// One `UiMapAssignment`: world `region` of `map_id` drawn at `ui_min..ui_max` of `ui_map`.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct UiMapAssignment {
    pub ui_map: u32,
    pub map_id: u32,
    pub order: u32,
    pub ui_min: [f32; 2],
    pub ui_max: [f32; 2],
    pub region_min: [f32; 3],
    pub region_max: [f32; 3],
    pub wmo_group: u32,
}

impl UiMapAssignment {
    fn covers_whole_map(&self) -> bool {
        self.ui_min == [0.0, 0.0] && self.ui_max == [1.0, 1.0]
    }

    fn contains(&self, [x, y, z]: [f32; 3]) -> bool {
        (self.region_min[0]..=self.region_max[0]).contains(&x)
            && (self.region_min[1]..=self.region_max[1]).contains(&y)
            && (self.region_min[2]..=self.region_max[2]).contains(&z)
    }

    fn contains_uv(&self, [u, v]: [f32; 2]) -> bool {
        (self.ui_min[0]..=self.ui_max[0]).contains(&u)
            && (self.ui_min[1]..=self.ui_max[1]).contains(&v)
    }

    fn area(&self) -> f32 {
        (self.region_max[0] - self.region_min[0]) * (self.region_max[1] - self.region_min[1])
    }

    /// Map UV of world `x`, `y` (unclamped).
    pub fn uv(&self, x: f32, y: f32) -> [f32; 2] {
        let fu = (self.region_max[1] - y) / (self.region_max[1] - self.region_min[1]);
        let fv = (self.region_max[0] - x) / (self.region_max[0] - self.region_min[0]);
        [
            self.ui_min[0] + fu * (self.ui_max[0] - self.ui_min[0]),
            self.ui_min[1] + fv * (self.ui_max[1] - self.ui_min[1]),
        ]
    }

    /// World `x`, `y` at map UV `u`, `v`.
    fn world(&self, [u, v]: [f32; 2]) -> [f32; 2] {
        let fu = (u - self.ui_min[0]) / (self.ui_max[0] - self.ui_min[0]);
        let fv = (v - self.ui_min[1]) / (self.ui_max[1] - self.ui_min[1]);
        [
            self.region_max[0] - fv * (self.region_max[0] - self.region_min[0]),
            self.region_max[1] - fu * (self.region_max[1] - self.region_min[1]),
        ]
    }
}

/// One layer-0 `UiMapArtTile`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct UiMapTile {
    pub row: u32,
    pub col: u32,
    pub fdid: u32,
}

/// Layer-0 art of one map: `size` = `LayerWidth/Height`, `tile` = `TileWidth/Height`.
#[derive(Debug, Clone, PartialEq)]
pub struct UiMapArtLayout {
    pub size: [f32; 2],
    pub tile: [f32; 2],
    pub tiles: Vec<UiMapTile>,
    /// `UiMapArt.HighlightFileDataID`, drawn over the parent map when hovered (0 = none).
    pub highlight_fdid: u32,
}

/// A tile placed on the map canvas: normalized `rect` `[x, y, w, h]` and the
/// `[left, right, top, bottom]` texture crop that trims tiles past the layer edge.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct PlacedTile {
    pub fdid: u32,
    pub rect: [f32; 4],
    pub tex_coords: [f32; 4],
}

impl UiMapArtLayout {
    pub fn placed_tiles(&self) -> Vec<PlacedTile> {
        let [width, height] = self.size;
        let [tile_w, tile_h] = self.tile;
        self.tiles
            .iter()
            .filter_map(|tile| {
                let (x, y) = (tile.col as f32 * tile_w, tile.row as f32 * tile_h);
                let (w, h) = ((width - x).min(tile_w), (height - y).min(tile_h));
                (w > 0.0 && h > 0.0).then(|| PlacedTile {
                    fdid: tile.fdid,
                    rect: [x / width, y / height, w / width, h / height],
                    tex_coords: [0.0, w / tile_w, 0.0, h / tile_h],
                })
            })
            .collect()
    }
}

#[derive(Debug, Clone, Default, PartialEq)]
pub struct UiMapCatalog {
    maps: HashMap<u32, UiMapInfo>,
    assignments: HashMap<u32, Vec<UiMapAssignment>>,
    arts: HashMap<u32, UiMapArtLayout>,
}

impl UiMapCatalog {
    /// The world-system maps and their art from the DB2 CSV exports in `dir`.
    pub fn load(dir: &Path) -> Result<Self, String> {
        let maps = read_maps(dir)?;
        let assignments = read_assignments(dir)?;
        let arts = read_arts(dir)?;
        Ok(Self {
            maps,
            assignments,
            arts,
        })
    }

    pub fn map(&self, id: u32) -> Option<&UiMapInfo> {
        self.maps.get(&id)
    }

    pub fn art(&self, id: u32) -> Option<&UiMapArtLayout> {
        self.arts.get(&id)
    }

    fn world_map(&self, id: u32) -> Option<&UiMapInfo> {
        self.map(id).filter(|map| map.system == SYSTEM_WORLD)
    }

    fn assignments(&self, id: u32) -> impl Iterator<Item = &UiMapAssignment> {
        self.assignments
            .get(&id)
            .into_iter()
            .flatten()
            .filter(|assignment| assignment.wmo_group == 0)
    }

    /// `C_Map.GetBestMapForUnit` without WMO groups: the smallest world-system zone
    /// whose assignment on `map_id` contains `position`, else the containing continent.
    pub fn best_map_for_position(&self, map_id: u32, position: [f32; 3]) -> Option<u32> {
        let mut best: Option<(u8, f32, u32)> = None;
        for (&id, rows) in &self.assignments {
            let Some(map) = self.world_map(id) else {
                continue;
            };
            let rank = match map.kind {
                map_type::ZONE if map.parent != 0 => 0,
                map_type::CONTINENT => 1,
                _ => continue,
            };
            if !self.arts.contains_key(&id) {
                continue;
            }
            for row in rows.iter().filter(|row| row.wmo_group == 0) {
                if row.map_id != map_id || !row.contains(position) {
                    continue;
                }
                let key = (rank, row.area(), id);
                if best.is_none_or(|current| key < current) {
                    best = Some(key);
                }
            }
        }
        best.map(|(_, _, id)| id)
    }

    /// UV of world `x`, `y` on `ui_map`, from the assignment of `map_id` containing it.
    pub fn map_position(&self, ui_map: u32, map_id: u32, position: [f32; 3]) -> Option<[f32; 2]> {
        self.assignments(ui_map)
            .filter(|row| row.map_id == map_id && row.contains(position))
            .min_by_key(|row| row.order)
            .map(|row| row.uv(position[0], position[1]))
    }

    /// UVs of a world-space polygon (`[x, y]` yards) on `ui_map`, through the
    /// assignment that holds its centroid, so edge points outside it still map.
    pub fn map_polygon(
        &self,
        ui_map: u32,
        map_id: u32,
        points: &[[f32; 2]],
    ) -> Option<Vec<[f32; 2]>> {
        let count = points.len() as f32;
        let centroid = points.iter().fold([0.0, 0.0], |[x, y], [px, py]| {
            [x + px / count, y + py / count]
        });
        let row = self
            .assignments(ui_map)
            .filter(|row| row.map_id == map_id && row.contains([centroid[0], centroid[1], 0.0]))
            .min_by_key(|row| row.order)?;
        Some(points.iter().map(|[x, y]| row.uv(*x, *y)).collect())
    }

    /// The navigable parent (world system, with art) of `id`.
    pub fn parent(&self, id: u32) -> Option<u32> {
        let parent = self.world_map(id)?.parent;
        (self.world_map(parent).is_some() && self.arts.contains_key(&parent)).then_some(parent)
    }

    /// `id`, then each navigable parent up to the root.
    pub fn lineage(&self, id: u32) -> Vec<u32> {
        let mut lineage = vec![id];
        while let Some(parent) = self.parent(*lineage.last().unwrap()) {
            if lineage.contains(&parent) {
                break;
            }
            lineage.push(parent);
        }
        lineage
    }

    fn children(&self, parent: u32) -> impl Iterator<Item = &UiMapInfo> {
        self.maps.values().filter(move |map| {
            map.parent == parent
                && map.system == SYSTEM_WORLD
                && matches!(map.kind, map_type::CONTINENT | map_type::ZONE)
                && self.arts.contains_key(&map.id)
        })
    }

    /// The child map drawn at `uv` of `parent` (`C_Map.GetMapInfoAtPosition` over
    /// assignment rectangles), with its rect on `parent`.
    pub fn child_at(&self, parent: u32, uv: [f32; 2]) -> Option<(u32, [f32; 4])> {
        let mut best: Option<(f32, u32, [f32; 4])> = None;
        for host in self.assignments(parent).filter(|row| row.contains_uv(uv)) {
            let [x, y] = host.world(uv);
            for child in self.children(parent) {
                for row in self.assignments(child.id) {
                    if row.map_id != host.map_id || !row.covers_whole_map() {
                        continue;
                    }
                    if !row.contains([x, y, 0.0]) {
                        continue;
                    }
                    let key = (row.area(), child.id);
                    if best.is_none_or(|(area, id, _)| key < (area, id)) {
                        best = Some((key.0, key.1, child_rect(host, row)));
                    }
                }
            }
        }
        best.map(|(_, id, rect)| (id, rect))
    }
}

/// Normalized `[x, y, w, h]` of `child`'s region on the `host` assignment, clipped
/// to the host's UV rect (a continent's art region extends past its land).
fn child_rect(host: &UiMapAssignment, child: &UiMapAssignment) -> [f32; 4] {
    let [u0, v0] = host.uv(child.region_max[0], child.region_max[1]);
    let [u1, v1] = host.uv(child.region_min[0], child.region_min[1]);
    let left = u0.min(u1).max(host.ui_min[0]);
    let right = u0.max(u1).min(host.ui_max[0]);
    let top = v0.min(v1).max(host.ui_min[1]);
    let bottom = v0.max(v1).min(host.ui_max[1]);
    [left, top, right - left, bottom - top]
}

struct Table {
    name: String,
    headers: Vec<String>,
    rows: Vec<Vec<String>>,
}

impl Table {
    fn read(dir: &Path, name: &str) -> Result<Self, String> {
        let path = dir.join(format!("{name}.csv"));
        let text = std::fs::read_to_string(&path)
            .map_err(|error| format!("read {}: {error}", path.display()))?;
        let mut lines = text.lines();
        let headers = parse_csv_line(lines.next().unwrap_or_default());
        let rows = lines
            .filter(|line| !line.is_empty())
            .map(parse_csv_line)
            .collect();
        Ok(Self {
            name: name.to_owned(),
            headers,
            rows,
        })
    }

    fn column(&self, name: &str) -> Result<usize, String> {
        self.headers
            .iter()
            .position(|header| header == name)
            .ok_or_else(|| format!("{}.csv missing {name} column", self.name))
    }

    fn columns<const N: usize>(&self, names: [&str; N]) -> Result<[usize; N], String> {
        let mut columns = [0; N];
        for (column, name) in columns.iter_mut().zip(names) {
            *column = self.column(name)?;
        }
        Ok(columns)
    }

    fn parse<T: std::str::FromStr>(&self, row: &[String], column: usize) -> Result<T, String> {
        let raw = row
            .get(column)
            .ok_or_else(|| format!("{}.csv has a short row", self.name))?;
        raw.parse()
            .map_err(|_| format!("{}.csv column {column}: bad value {raw:?}", self.name))
    }
}

fn read_maps(dir: &Path) -> Result<HashMap<u32, UiMapInfo>, String> {
    let table = Table::read(dir, "UiMap")?;
    let [name, id, parent, system, kind] =
        table.columns(["Name_lang", "ID", "ParentUiMapID", "System", "Type"])?;
    let mut maps = HashMap::new();
    for row in &table.rows {
        let info = UiMapInfo {
            id: table.parse(row, id)?,
            name: row.get(name).cloned().unwrap_or_default(),
            parent: table.parse(row, parent)?,
            kind: table.parse(row, kind)?,
            system: table.parse(row, system)?,
        };
        maps.insert(info.id, info);
    }
    Ok(maps)
}

fn read_assignments(dir: &Path) -> Result<HashMap<u32, Vec<UiMapAssignment>>, String> {
    let table = Table::read(dir, "UiMapAssignment")?;
    let [ui_map, map_id, order, wmo_group] =
        table.columns(["UiMapID", "MapID", "OrderIndex", "WMOGroupID"])?;
    let ui = table.columns(["UiMin_0", "UiMin_1", "UiMax_0", "UiMax_1"])?;
    let region = table.columns([
        "Region_0", "Region_1", "Region_2", "Region_3", "Region_4", "Region_5",
    ])?;
    let mut assignments: HashMap<u32, Vec<UiMapAssignment>> = HashMap::new();
    for row in &table.rows {
        // MapID -1 marks assignments with no world map.
        let Ok(map) = u32::try_from(table.parse::<i64>(row, map_id)?) else {
            continue;
        };
        let f = |column| table.parse::<f32>(row, column);
        let assignment = UiMapAssignment {
            ui_map: table.parse(row, ui_map)?,
            map_id: map,
            order: table.parse(row, order)?,
            ui_min: [f(ui[0])?, f(ui[1])?],
            ui_max: [f(ui[2])?, f(ui[3])?],
            region_min: [f(region[0])?, f(region[1])?, f(region[2])?],
            region_max: [f(region[3])?, f(region[4])?, f(region[5])?],
            wmo_group: table.parse(row, wmo_group)?,
        };
        assignments
            .entry(assignment.ui_map)
            .or_default()
            .push(assignment);
    }
    Ok(assignments)
}

fn read_phase_zero_art_links(dir: &Path) -> Result<HashMap<u32, u32>, String> {
    let links = Table::read(dir, "UiMapXMapArt")?;
    let [phase, link_art, link_map] = links.columns(["PhaseID", "UiMapArtID", "UiMapID"])?;
    let mut art_of_map = HashMap::new();
    for row in &links.rows {
        if links.parse::<u32>(row, phase)? == 0 {
            art_of_map.insert(
                links.parse::<u32>(row, link_map)?,
                links.parse(row, link_art)?,
            );
        }
    }
    Ok(art_of_map)
}

fn read_art_styles(dir: &Path) -> Result<HashMap<u32, (u32, u32)>, String> {
    let arts = Table::read(dir, "UiMapArt")?;
    let [art_id, highlight, style] =
        arts.columns(["ID", "HighlightFileDataID", "UiMapArtStyleID"])?;
    let mut art_style = HashMap::new();
    for row in &arts.rows {
        let highlight: u32 = arts.parse(row, highlight)?;
        art_style.insert(
            arts.parse::<u32>(row, art_id)?,
            (arts.parse::<u32>(row, style)?, highlight),
        );
    }
    Ok(art_style)
}

/// Phase-0 art of each map with its layer-0 layout and tiles.
fn read_arts(dir: &Path) -> Result<HashMap<u32, UiMapArtLayout>, String> {
    let art_of_map = read_phase_zero_art_links(dir)?;
    let art_style = read_art_styles(dir)?;
    let layers = read_style_layers(dir)?;
    let tiles = read_tiles(dir)?;
    let mut result = HashMap::new();
    for (map, art) in art_of_map {
        let Some(&(style, highlight_fdid)) = art_style.get(&art) else {
            continue;
        };
        let (Some(&(size, tile)), Some(tiles)) = (layers.get(&style), tiles.get(&art)) else {
            continue;
        };
        result.insert(
            map,
            UiMapArtLayout {
                size,
                tile,
                tiles: tiles.clone(),
                highlight_fdid,
            },
        );
    }
    Ok(result)
}

type LayerSize = ([f32; 2], [f32; 2]);

fn read_style_layers(dir: &Path) -> Result<HashMap<u32, LayerSize>, String> {
    let table = Table::read(dir, "UiMapArtStyleLayer")?;
    let [style, layer, width, height, tile_w, tile_h] = table.columns([
        "UiMapArtStyleID",
        "LayerIndex",
        "LayerWidth",
        "LayerHeight",
        "TileWidth",
        "TileHeight",
    ])?;
    let mut layers = HashMap::new();
    for row in &table.rows {
        if table.parse::<u32>(row, layer)? != 0 {
            continue;
        }
        let f = |column| table.parse::<f32>(row, column);
        layers.insert(
            table.parse::<u32>(row, style)?,
            ([f(width)?, f(height)?], [f(tile_w)?, f(tile_h)?]),
        );
    }
    Ok(layers)
}

fn read_tiles(dir: &Path) -> Result<HashMap<u32, Vec<UiMapTile>>, String> {
    let table = Table::read(dir, "UiMapArtTile")?;
    let [row_index, col_index, layer, fdid, art] = table.columns([
        "RowIndex",
        "ColIndex",
        "LayerIndex",
        "FileDataID",
        "UiMapArtID",
    ])?;
    let mut tiles: HashMap<u32, Vec<UiMapTile>> = HashMap::new();
    for row in &table.rows {
        if table.parse::<u32>(row, layer)? != 0 {
            continue;
        }
        tiles
            .entry(table.parse(row, art)?)
            .or_default()
            .push(UiMapTile {
                row: table.parse(row, row_index)?,
                col: table.parse(row, col_index)?,
                fdid: table.parse(row, fdid)?,
            });
    }
    for list in tiles.values_mut() {
        list.sort_by_key(|tile| (tile.row, tile.col));
    }
    Ok(tiles)
}

#[cfg(test)]
#[path = "ui_map_data_tests.rs"]
mod tests;
