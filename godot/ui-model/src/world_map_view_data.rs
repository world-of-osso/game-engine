//! World map view model (docs/specs/world-map.md): turns the `UiMap` catalog, the
//! displayed map, the local player and the quest log into [`WorldMapFrameState`].
//! Engine-free so the Bevy and Godot hosts share navigation and marker rules.

use std::f32::consts::FRAC_PI_2;
use std::path::Path;

use shared::protocol::{QuestEntrySnapshot, QuestPoiSnapshot};

use crate::csv_util::parse_csv_line;
use crate::ui::screens::world_map_frame_component::{
    MapBreadcrumb, MapHighlight, MapPin, MapPinType, MapPlayerMarker, MapTile, WorldMapFrameState,
};
use crate::ui_map_data::{UiMapCatalog, map_type};

/// `TaxiNodes.Flags`: shown on the Alliance / Horde map.
const TAXI_ALLIANCE: u32 = 0x1;
const TAXI_HORDE: u32 = 0x2;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Faction {
    Alliance,
    Horde,
}

/// One `TaxiNodes` row that appears on a faction map.
#[derive(Debug, Clone, PartialEq)]
pub struct TaxiNodeRow {
    pub name: String,
    pub map_id: u32,
    /// World position (Retail axes).
    pub position: [f32; 3],
    pub flags: u32,
}

/// Static world-map data loaded once from the DB2 CSV exports.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct WorldMapData {
    pub catalog: UiMapCatalog,
    pub taxi_nodes: Vec<TaxiNodeRow>,
    /// `ChrRaces.Alliance` by race id: 0 Alliance, 1 Horde, 2 neutral.
    race_factions: Vec<(u8, u8)>,
}

impl WorldMapData {
    pub fn load(dir: &Path) -> Result<Self, String> {
        Ok(Self {
            catalog: UiMapCatalog::load(dir)?,
            taxi_nodes: read_taxi_nodes(dir)?,
            race_factions: read_race_factions(dir)?,
        })
    }

    pub fn race_faction(&self, race: u8) -> Option<Faction> {
        match self.race_factions.iter().find(|(id, _)| *id == race)?.1 {
            0 => Some(Faction::Alliance),
            1 => Some(Faction::Horde),
            _ => None,
        }
    }
}

/// The local player as the map sees it.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct WorldMapPlayer {
    pub map_id: u32,
    /// World position (Retail axes).
    pub position: [f32; 3],
    /// Engine facing yaw: forward is engine `(sin yaw, 0, cos yaw)`.
    pub yaw: f32,
    pub faction: Option<Faction>,
}

/// A vignette the server shows near the player (`C_VignetteInfo.GetVignetteInfo`)
/// whose `Vignette` row has `ShowOnMap` (`VignetteInfo.onWorldMap`).
#[derive(Debug, Clone, PartialEq)]
pub struct MapVignette {
    /// `Vignette.Name_lang`.
    pub name: String,
    /// `VignetteKillElite` rather than `VignetteKill`.
    pub elite: bool,
    /// `HideOnContinentMaps`.
    pub hide_on_continent_maps: bool,
    pub map_id: u32,
    /// World position (Retail axes).
    pub position: [f32; 3],
}

/// Retail world axes of an engine (Y-up) position: `x` north, `y` west, `z` up.
pub fn engine_to_world([x, y, z]: [f32; 3]) -> [f32; 3] {
    [x, -z, y]
}

/// Counter-clockwise screen rotation of the north-up arrow for an engine yaw. Engine
/// forward `(sin, cos)` is world `(sin, -cos)`, which points the arrow at map
/// `(cos yaw, -sin yaw)`.
pub fn arrow_rotation(yaw: f32) -> f32 {
    (yaw - FRAC_PI_2).rem_euclid(std::f32::consts::TAU)
}

/// What the host wants drawn.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct WorldMapRequest<'a> {
    pub visible: bool,
    pub viewport: [f32; 2],
    pub map_id: u32,
    /// Cursor position on the canvas as a map UV.
    pub hovered: Option<[f32; 2]>,
    pub player: Option<&'a WorldMapPlayer>,
    pub quests: &'a [QuestEntrySnapshot],
    /// Objective areas to outline (`QuestRuntime::watched_objective_areas`).
    pub quest_areas: &'a [&'a QuestPoiSnapshot],
    pub vignettes: &'a [MapVignette],
}

/// The map the frame opens on: the player's best map.
pub fn player_map(data: &WorldMapData, player: &WorldMapPlayer) -> Option<u32> {
    data.catalog
        .best_map_for_position(player.map_id, player.position)
}

/// Zoom out one level (zone → continent → world).
pub fn zoom_out(data: &WorldMapData, map_id: u32) -> Option<u32> {
    data.catalog.parent(map_id)
}

/// Zoom into the child map under `uv`.
pub fn zoom_in(data: &WorldMapData, map_id: u32, uv: [f32; 2]) -> Option<u32> {
    data.catalog.child_at(map_id, uv).map(|(child, _)| child)
}

pub fn world_map_frame_state(data: &WorldMapData, request: WorldMapRequest) -> WorldMapFrameState {
    let catalog = &data.catalog;
    let map_id = request.map_id;
    let kind = catalog.map(map_id).map(|map| map.kind);
    let mut pins = Vec::new();
    let mut quest_areas = Vec::new();
    if matches!(kind, Some(map_type::ZONE | map_type::CONTINENT)) {
        pins.extend(quest_pins(catalog, map_id, request.quests));
        quest_areas = quest_area_polygons(catalog, map_id, request.quest_areas);
    }
    pins.extend(vignette_pins(catalog, map_id, kind, request.vignettes));
    // Retail draws flight points on zone maps only.
    if kind == Some(map_type::ZONE) {
        let faction = request.player.and_then(|player| player.faction);
        pins.extend(flight_pins(data, map_id, faction));
    }
    WorldMapFrameState {
        visible: request.visible,
        viewport: request.viewport,
        maximized: false,
        quest_panel: None,
        map_id,
        map_name: catalog
            .map(map_id)
            .map(|map| map.name.clone())
            .unwrap_or_default(),
        breadcrumbs: breadcrumbs(catalog, map_id),
        tiles: tiles(catalog, map_id),
        highlight: request
            .hovered
            .and_then(|uv| highlight(catalog, map_id, uv)),
        pins,
        quest_areas,
        player: request
            .player
            .and_then(|player| player_marker(catalog, map_id, player)),
    }
}

fn breadcrumbs(catalog: &UiMapCatalog, map_id: u32) -> Vec<MapBreadcrumb> {
    let mut lineage = catalog.lineage(map_id);
    lineage.reverse();
    lineage
        .into_iter()
        .filter_map(|id| {
            Some(MapBreadcrumb {
                map_id: id,
                name: catalog.map(id)?.name.clone(),
            })
        })
        .collect()
}

fn tiles(catalog: &UiMapCatalog, map_id: u32) -> Vec<MapTile> {
    catalog
        .art(map_id)
        .map(|art| {
            art.placed_tiles()
                .into_iter()
                .map(|tile| MapTile {
                    fdid: tile.fdid,
                    rect: tile.rect,
                    tex_coords: tile.tex_coords,
                })
                .collect()
        })
        .unwrap_or_default()
}

fn highlight(catalog: &UiMapCatalog, map_id: u32, uv: [f32; 2]) -> Option<MapHighlight> {
    let (child, rect) = catalog.child_at(map_id, uv)?;
    Some(MapHighlight {
        map_id: child,
        name: catalog.map(child)?.name.clone(),
        fdid: catalog.art(child).map_or(0, |art| art.highlight_fdid),
        rect,
    })
}

fn player_marker(
    catalog: &UiMapCatalog,
    map_id: u32,
    player: &WorldMapPlayer,
) -> Option<MapPlayerMarker> {
    let [x, y] = catalog.map_position(map_id, player.map_id, player.position)?;
    Some(MapPlayerMarker {
        x,
        y,
        rotation: arrow_rotation(player.yaw),
    })
}

/// Map-UV outlines of the objective areas drawn on `map_id`.
pub fn quest_area_polygons(
    catalog: &UiMapCatalog,
    map_id: u32,
    areas: &[&QuestPoiSnapshot],
) -> Vec<Vec<[f32; 2]>> {
    areas
        .iter()
        .filter_map(|poi| {
            let points: Vec<[f32; 2]> = poi
                .points
                .iter()
                .map(|point| [point.x as f32, point.y as f32])
                .collect();
            catalog.map_polygon(map_id, poi.map_id, &points)
        })
        .collect()
}

/// One pin per quest: its turn-in once complete, else its first objective area.
fn quest_pins(catalog: &UiMapCatalog, map_id: u32, quests: &[QuestEntrySnapshot]) -> Vec<MapPin> {
    let mut pins = Vec::new();
    for (index, quest) in quests.iter().enumerate() {
        let poi = quest.pois.iter().find_map(|poi| {
            if !crate::quest_poi::poi_is_visible(quest, poi) {
                return None;
            }
            let count = poi.points.len() as f32;
            let x = poi.points.iter().map(|point| point.x as f32).sum::<f32>() / count;
            let y = poi.points.iter().map(|point| point.y as f32).sum::<f32>() / count;
            catalog.map_position(map_id, poi.map_id, [x, y, 0.0])
        });
        let Some([x, y]) = poi else {
            continue;
        };
        let (pin_type, badge) = if quest.completed {
            (MapPinType::QuestTurnIn, String::new())
        } else {
            (MapPinType::QuestObjective, (index + 1).to_string())
        };
        pins.push(MapPin {
            pin_type,
            label: quest.title.clone(),
            badge,
            x,
            y,
        });
    }
    pins
}

/// `VignetteDataProviderMixin`: a pin wherever `C_VignetteInfo.GetVignettePosition`
/// places the vignette, on zone and continent maps (`HideOnContinentMaps` keeps it off
/// continents).
fn vignette_pins(
    catalog: &UiMapCatalog,
    map_id: u32,
    kind: Option<u8>,
    vignettes: &[MapVignette],
) -> Vec<MapPin> {
    let shown = |vignette: &&MapVignette| match kind {
        Some(map_type::ZONE) => true,
        Some(map_type::CONTINENT) => !vignette.hide_on_continent_maps,
        _ => false,
    };
    vignettes
        .iter()
        .filter(shown)
        .filter_map(|vignette| {
            let [x, y] = catalog.map_position(map_id, vignette.map_id, vignette.position)?;
            Some(MapPin {
                pin_type: MapPinType::Vignette {
                    elite: vignette.elite,
                },
                label: vignette.name.clone(),
                badge: String::new(),
                x,
                y,
            })
        })
        .collect()
}

fn flight_pins(data: &WorldMapData, map_id: u32, faction: Option<Faction>) -> Vec<MapPin> {
    let wanted = match faction {
        Some(Faction::Alliance) => TAXI_ALLIANCE,
        Some(Faction::Horde) => TAXI_HORDE,
        None => TAXI_ALLIANCE | TAXI_HORDE,
    };
    data.taxi_nodes
        .iter()
        .filter(|node| node.flags & wanted != 0)
        .filter_map(|node| {
            let [x, y] = data
                .catalog
                .map_position(map_id, node.map_id, node.position)?;
            let pin_type = match node.flags & (TAXI_ALLIANCE | TAXI_HORDE) {
                TAXI_ALLIANCE => MapPinType::FlightAlliance,
                TAXI_HORDE => MapPinType::FlightHorde,
                _ => MapPinType::FlightNeutral,
            };
            Some(MapPin {
                pin_type,
                label: node.name.clone(),
                badge: String::new(),
                x,
                y,
            })
        })
        .collect()
}

fn csv_rows(dir: &Path, name: &str) -> Result<(Vec<String>, Vec<Vec<String>>), String> {
    let path = dir.join(format!("{name}.csv"));
    let text = std::fs::read_to_string(&path)
        .map_err(|error| format!("read {}: {error}", path.display()))?;
    let mut lines = text.lines();
    let headers = parse_csv_line(lines.next().unwrap_or_default());
    let rows = lines
        .filter(|line| !line.is_empty())
        .map(parse_csv_line)
        .collect();
    Ok((headers, rows))
}

fn column(headers: &[String], name: &str, table: &str) -> Result<usize, String> {
    headers
        .iter()
        .position(|header| header == name)
        .ok_or_else(|| format!("{table}.csv missing {name} column"))
}

fn parse<T: std::str::FromStr>(row: &[String], index: usize, table: &str) -> Result<T, String> {
    let raw = row
        .get(index)
        .ok_or_else(|| format!("{table}.csv has a short row"))?;
    raw.parse()
        .map_err(|_| format!("{table}.csv column {index}: bad value {raw:?}"))
}

fn read_taxi_nodes(dir: &Path) -> Result<Vec<TaxiNodeRow>, String> {
    let (headers, rows) = csv_rows(dir, "TaxiNodes")?;
    let col = |name| column(&headers, name, "TaxiNodes");
    let (name, x, y, z, map, flags) = (
        col("Name_lang")?,
        col("Pos_0")?,
        col("Pos_1")?,
        col("Pos_2")?,
        col("ContinentID")?,
        col("Flags")?,
    );
    let mut nodes = Vec::new();
    for row in &rows {
        let node_flags: u32 = parse(row, flags, "TaxiNodes")?;
        if node_flags & (TAXI_ALLIANCE | TAXI_HORDE) == 0 {
            continue;
        }
        nodes.push(TaxiNodeRow {
            name: row.get(name).cloned().unwrap_or_default(),
            map_id: parse(row, map, "TaxiNodes")?,
            position: [
                parse(row, x, "TaxiNodes")?,
                parse(row, y, "TaxiNodes")?,
                parse(row, z, "TaxiNodes")?,
            ],
            flags: node_flags,
        });
    }
    Ok(nodes)
}

fn read_race_factions(dir: &Path) -> Result<Vec<(u8, u8)>, String> {
    let (headers, rows) = csv_rows(dir, "ChrRaces")?;
    let (id, alliance) = (
        column(&headers, "ID", "ChrRaces")?,
        column(&headers, "Alliance", "ChrRaces")?,
    );
    rows.iter()
        .map(|row| {
            Ok((
                parse(row, id, "ChrRaces")?,
                parse(row, alliance, "ChrRaces")?,
            ))
        })
        .collect()
}

#[cfg(test)]
#[path = "world_map_view_data_tests.rs"]
mod tests;
