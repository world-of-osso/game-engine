//! Retail minimap (`Blizzard_Minimap/Mainline/Minimap.lua`) without an engine: which
//! `world/minimaps/<map>/mapXX_YY.blp` tiles cover the player, the north-up circular
//! composite of those tiles, blip placement, zoom steps, the zone text and its PvP
//! colour, and the clock ticker text.
//!
//! Engine axes as the terrain uses them: engine `x` is Retail world X (north), engine
//! `z` is Retail world −Y (east). The minimap is north-up (`rotateMinimap` 0 in the
//! Modern edit-mode preset, `EditModePresetLayouts.lua:374`): screen up is +x, screen
//! right is +z.

use std::collections::HashMap;
use std::io::BufRead;
use std::path::Path;

use crate::asset::adt_format::adt::CHUNK_SIZE;
use crate::csv_util::{header_index, parse_csv_line_trimmed};

/// One ADT tile edge in yards.
pub const TILE_YARDS: f32 = CHUNK_SIZE * 16.0;
/// `Minimap` frame size (`Minimap.xml:179`).
pub const MINIMAP_SIZE: f32 = 198.0;
/// `Minimap:GetZoomLevels()`: zoom 0 (farthest) to 5.
pub const ZOOM_LEVELS: u8 = 6;
/// Outdoor view diameter in yards per zoom level. The engine does not expose these;
/// they are the values addons measure (HereBeDragons-Pins `minimap_size.outdoor`).
pub const OUTDOOR_DIAMETERS: [f32; ZOOM_LEVELS as usize] = [
    466.0 + 2.0 / 3.0,
    400.0,
    333.0 + 1.0 / 3.0,
    266.0 + 2.0 / 3.0,
    200.0,
    133.0 + 1.0 / 3.0,
];
/// Composite drawn where the local install has no minimap tile.
pub const MISSING_TILE_COLOR: [u8; 4] = [20, 20, 20, 255];

/// Decoded minimap tile: row-major RGBA8.
pub struct TileImage {
    pub pixels: Vec<u8>,
    pub width: u32,
    pub height: u32,
}

/// Minimap tile key `(first, second)` of `mapFF_SS.blp`; the same pair as the ADT
/// `<map>_FF_SS.adt` and `terrain_height_data::bevy_to_tile_coords`.
pub type TileKey = (u32, u32);

/// Listfile path of one minimap tile.
pub fn tile_path(map: &str, (first, second): TileKey) -> String {
    format!("world/minimaps/{map}/map{first:02}_{second:02}.blp")
}

/// What the minimap shows: centre (engine `x`, `z`) and view diameter in yards.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct MinimapView {
    pub center: [f32; 2],
    pub diameter: f32,
}

impl MinimapView {
    pub fn new(center: [f32; 2], zoom: u8) -> Self {
        Self {
            center,
            diameter: OUTDOOR_DIAMETERS[usize::from(zoom.min(ZOOM_LEVELS - 1))],
        }
    }

    /// Tile under an engine position and the position inside it: `u` runs west to
    /// east (the tile image's x), `v` north to south (its y).
    pub fn tile_uv(&self, [x, z]: [f32; 2]) -> Option<(TileKey, [f32; 2])> {
        let first = 32.0 + z / TILE_YARDS;
        let second = 32.0 - x / TILE_YARDS;
        if !(0.0..64.0).contains(&first) || !(0.0..64.0).contains(&second) {
            return None;
        }
        let key = (first.floor() as u32, second.floor() as u32);
        Some((key, [first.fract(), second.fract()]))
    }

    /// Tiles the view circle touches, in key order.
    pub fn tiles(&self) -> Vec<TileKey> {
        let radius = self.diameter / 2.0;
        let [x, z] = self.center;
        let span =
            |low: f32, high: f32| (low.floor().max(0.0) as u32)..=(high.floor().min(63.0) as u32);
        let firsts = span(
            32.0 + (z - radius) / TILE_YARDS,
            32.0 + (z + radius) / TILE_YARDS,
        );
        let seconds = span(
            32.0 - (x + radius) / TILE_YARDS,
            32.0 - (x - radius) / TILE_YARDS,
        );
        firsts
            .flat_map(|first| seconds.clone().map(move |second| (first, second)))
            .collect()
    }

    /// Engine position under composite pixel `(px, py)` of a `size`² image.
    pub fn pixel_position(&self, px: u32, py: u32, size: u32) -> [f32; 2] {
        let yards = self.diameter / size as f32;
        let half = size as f32 / 2.0;
        [
            self.center[0] + (half - py as f32 - 0.5) * yards,
            self.center[1] + (px as f32 + 0.5 - half) * yards,
        ]
    }

    /// Offset of `point` from the minimap centre as a fraction of the minimap size
    /// (right, down), or None outside the circle.
    pub fn blip_offset(&self, [x, z]: [f32; 2]) -> Option<[f32; 2]> {
        let right = (z - self.center[1]) / self.diameter;
        let down = (self.center[0] - x) / self.diameter;
        (right.hypot(down) <= 0.5).then_some([right, down])
    }
}

/// North-up composite of the view: `size`² RGBA8, bilinear within each tile, clipped to
/// the round `Minimap` mask with a one-pixel soft edge.
pub fn compose<'a>(
    view: &MinimapView,
    size: u32,
    tile: impl Fn(TileKey) -> Option<&'a TileImage>,
) -> Vec<u8> {
    let mut out = vec![0u8; (size * size * 4) as usize];
    let radius = size as f32 / 2.0;
    for py in 0..size {
        for px in 0..size {
            let dx = px as f32 + 0.5 - radius;
            let dy = py as f32 + 0.5 - radius;
            let coverage = (radius - dx.hypot(dy)).clamp(0.0, 1.0);
            if coverage == 0.0 {
                continue;
            }
            let color = view
                .tile_uv(view.pixel_position(px, py, size))
                .and_then(|(key, uv)| Some(sample(tile(key)?, uv)))
                .unwrap_or(MISSING_TILE_COLOR);
            let offset = ((py * size + px) * 4) as usize;
            out[offset..offset + 3].copy_from_slice(&color[..3]);
            out[offset + 3] = (f32::from(color[3]) * coverage).round() as u8;
        }
    }
    out
}

/// Tints the composite inside each quest objective polygon (engine `(x, z)` points,
/// three or more) with the shared quest area overlay; returns how many pixels changed.
pub fn tint_quest_areas(
    view: &MinimapView,
    size: u32,
    pixels: &mut [u8],
    areas: &[Vec<[f32; 2]>],
) -> usize {
    let scale = size as f32 / view.diameter;
    let half = size as f32 / 2.0;
    // Engine (x, z) to composite pixels: right is +z, down is -x.
    let polygons: Vec<Vec<[f32; 2]>> = areas
        .iter()
        .map(|area| {
            area.iter()
                .map(|[x, z]| {
                    [
                        half + (z - view.center[1]) * scale,
                        half + (view.center[0] - x) * scale,
                    ]
                })
                .collect()
        })
        .collect();
    let overlay = crate::quest_area_data::quest_area_overlay(size, size, &polygons);
    crate::quest_area_data::blend_overlay(pixels, &overlay)
}

/// Bilinear sample at `uv`, clamped to the tile edge.
pub fn sample(image: &TileImage, [u, v]: [f32; 2]) -> [u8; 4] {
    let (w, h) = (image.width as usize, image.height as usize);
    let fx = (u * w as f32 - 0.5).clamp(0.0, (w - 1) as f32);
    let fy = (v * h as f32 - 0.5).clamp(0.0, (h - 1) as f32);
    let (x0, y0) = (fx.floor() as usize, fy.floor() as usize);
    let (x1, y1) = ((x0 + 1).min(w - 1), (y0 + 1).min(h - 1));
    let (tx, ty) = (fx - x0 as f32, fy - y0 as f32);
    let texel = |x: usize, y: usize, c: usize| f32::from(image.pixels[(y * w + x) * 4 + c]);
    let mut color = [0u8; 4];
    for (c, value) in color.iter_mut().enumerate() {
        let top = texel(x0, y0, c) * (1.0 - tx) + texel(x1, y0, c) * tx;
        let bottom = texel(x0, y1, c) * (1.0 - tx) + texel(x1, y1, c) * tx;
        *value = (top * (1.0 - ty) + bottom * ty).round() as u8;
    }
    color
}

/// `MinimapZoomInButtonMixin:OnClick` (`Minimap.lua:300-310`).
pub fn zoom_in(zoom: u8) -> u8 {
    (zoom + 1).min(ZOOM_LEVELS - 1)
}

/// `MinimapZoomOutButtonMixin:OnClick` (`Minimap.lua:325-334`).
pub fn zoom_out(zoom: u8) -> u8 {
    zoom.saturating_sub(1)
}

/// `TimeManagerClockTicker` with the default `timeMgrUseMilitaryTime` 0:
/// `TIMEMANAGER_TICKER_12HOUR` "%d:%02d" (`GameTime.lua:36-42`).
pub fn clock_text(hour: u32, minute: u32) -> String {
    let hour = match hour % 24 {
        0 => 12,
        h if h > 12 => h - 12,
        h => h,
    };
    format!("{hour}:{minute:02}")
}

/// `C_PvP.GetZonePVPInfo` types the minimap colours (`Minimap_Update`, `Minimap.lua:183-199`).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ZonePvp {
    Sanctuary,
    Friendly,
    Hostile,
    /// No controlling faction: `NORMAL_FONT_COLOR`.
    Normal,
}

impl ZonePvp {
    pub fn text_color(self) -> [f32; 4] {
        match self {
            Self::Sanctuary => [0.41, 0.8, 0.94, 1.0],
            Self::Friendly => [0.1, 1.0, 0.1, 1.0],
            Self::Hostile => [1.0, 0.1, 0.1, 1.0],
            Self::Normal => [1.0, 0.82, 0.0, 1.0],
        }
    }
}

/// `AreaTable` `FactionGroupMask` bits.
pub const FACTION_GROUP_ALLIANCE: u32 = 2;
pub const FACTION_GROUP_HORDE: u32 = 4;
/// `AreaTable.Flags[0]` sanctuary bit (TrinityCore `AREA_FLAG_SANCTUARY`).
const AREA_FLAG_SANCTUARY: u32 = 0x800;

struct AreaRow {
    name: String,
    parent: u32,
    faction_mask: u32,
    flags: u32,
}

/// `AreaTable` names, parents, controlling faction and flags.
#[derive(Default)]
pub struct AreaCatalog {
    areas: HashMap<u32, AreaRow>,
}

impl AreaCatalog {
    pub fn parse<R: BufRead>(mut reader: R, path: &Path) -> Result<Self, String> {
        let mut header = String::new();
        reader
            .read_line(&mut header)
            .map_err(|err| format!("read {} header: {err}", path.display()))?;
        let headers = parse_csv_line_trimmed(header.trim_end_matches(['\r', '\n']));
        let column = |name| header_index(&headers, name, path);
        let (id, name, parent) = (
            column("ID")?,
            column("AreaName_lang")?,
            column("ParentAreaID")?,
        );
        let (mask, flags) = (column("FactionGroupMask")?, column("Flags_0")?);
        let mut areas = HashMap::new();
        for line in reader.lines() {
            let line = line.map_err(|err| format!("read {} row: {err}", path.display()))?;
            let fields = parse_csv_line_trimmed(&line);
            let number = |col: usize| fields.get(col).and_then(|value| value.parse::<u32>().ok());
            let Some(area_id) = number(id) else { continue };
            areas.insert(
                area_id,
                AreaRow {
                    name: fields.get(name).cloned().unwrap_or_default(),
                    parent: number(parent).unwrap_or(0),
                    faction_mask: number(mask).unwrap_or(0),
                    flags: number(flags).unwrap_or(0),
                },
            );
        }
        Ok(Self { areas })
    }

    /// `GetMinimapZoneText`: the area the player stands in (its subzone when it has one).
    pub fn name(&self, area_id: u32) -> Option<&str> {
        self.areas.get(&area_id).map(|area| area.name.as_str())
    }

    /// `GetZoneText`: the zone the area belongs to, its top-level ancestor.
    pub fn zone(&self, area_id: u32) -> Option<&str> {
        self.lineage(area_id).last().map(|area| area.name.as_str())
    }

    /// Area then its ancestors, bounded against cyclic data.
    fn lineage(&self, area_id: u32) -> impl Iterator<Item = &AreaRow> {
        let mut next = Some(area_id);
        std::iter::from_fn(move || {
            let area = self.areas.get(&next?)?;
            next = (area.parent != 0).then_some(area.parent);
            Some(area)
        })
        .take(16)
    }

    /// Zone PvP type for a player of `player_faction` (`FACTION_GROUP_*`): the nearest
    /// area in the lineage with a controlling faction decides.
    pub fn pvp(&self, area_id: u32, player_faction: u32) -> ZonePvp {
        if self
            .lineage(area_id)
            .any(|area| area.flags & AREA_FLAG_SANCTUARY != 0)
        {
            return ZonePvp::Sanctuary;
        }
        match self.lineage(area_id).find(|area| area.faction_mask != 0) {
            Some(area) if area.faction_mask & player_faction != 0 => ZonePvp::Friendly,
            Some(_) => ZonePvp::Hostile,
            None => ZonePvp::Normal,
        }
    }
}

/// `ChrRaces.Alliance` (0 Alliance, 1 Horde) per race as `FACTION_GROUP_*`; races of
/// neither side (Pandaren before choosing) are left out.
pub fn parse_race_faction_groups<R: BufRead>(
    mut reader: R,
    path: &Path,
) -> Result<HashMap<u8, u32>, String> {
    let mut header = String::new();
    reader
        .read_line(&mut header)
        .map_err(|err| format!("read {} header: {err}", path.display()))?;
    let headers = parse_csv_line_trimmed(header.trim_end_matches(['\r', '\n']));
    let (id, alliance) = (
        header_index(&headers, "ID", path)?,
        header_index(&headers, "Alliance", path)?,
    );
    let mut races = HashMap::new();
    for line in reader.lines() {
        let line = line.map_err(|err| format!("read {} row: {err}", path.display()))?;
        let fields = parse_csv_line_trimmed(&line);
        let race = fields.get(id).and_then(|value| value.parse::<u8>().ok());
        let group = match fields.get(alliance).map(String::as_str) {
            Some("0") => FACTION_GROUP_ALLIANCE,
            Some("1") => FACTION_GROUP_HORDE,
            _ => continue,
        };
        if let Some(race) = race {
            races.insert(race, group);
        }
    }
    Ok(races)
}

/// One `Vignette` DB2 row as the maps read it: `Name_lang` and `Flags`
/// (TrinityCore `VignetteFlags`, DBCEnums.h).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct VignetteRow {
    pub name: String,
    pub flags: u32,
}

const VIGNETTE_SHOW_ON_MAP: u32 = 0x2;
const VIGNETTE_DONT_SHOW_ON_MINIMAP: u32 = 0x200;
const VIGNETTE_HIDE_ON_CONTINENT_MAPS: u32 = 0x1_0000;

impl VignetteRow {
    /// `VignetteInfo.onMinimap`: every vignette without `DontShowOnMinimap`.
    pub fn on_minimap(&self) -> bool {
        self.flags & VIGNETTE_DONT_SHOW_ON_MINIMAP == 0
    }

    /// `VignetteInfo.onWorldMap` (`VignetteDataProviderMixin:ShouldShowVignette`):
    /// `ShowOnMap`.
    pub fn on_world_map(&self) -> bool {
        self.flags & VIGNETTE_SHOW_ON_MAP != 0
    }

    /// `HideOnContinentMaps`: drawn on zone maps only.
    pub fn hide_on_continent_maps(&self) -> bool {
        self.flags & VIGNETTE_HIDE_ON_CONTINENT_MAPS != 0
    }
}

/// `Vignette.csv` rows by `ID`.
pub fn parse_vignettes<R: BufRead>(
    mut reader: R,
    path: &Path,
) -> Result<HashMap<u32, VignetteRow>, String> {
    let mut header = String::new();
    reader
        .read_line(&mut header)
        .map_err(|err| format!("read {} header: {err}", path.display()))?;
    let headers = parse_csv_line_trimmed(header.trim_end_matches(['\r', '\n']));
    let (id, name, flags) = (
        header_index(&headers, "ID", path)?,
        header_index(&headers, "Name_lang", path)?,
        header_index(&headers, "Flags", path)?,
    );
    let mut vignettes = HashMap::new();
    for line in reader.lines() {
        let line = line.map_err(|err| format!("read {} row: {err}", path.display()))?;
        let fields = parse_csv_line_trimmed(&line);
        let parse = |index: usize| {
            fields
                .get(index)
                .and_then(|value| value.parse::<u32>().ok())
                .ok_or_else(|| format!("{}: bad row {line}", path.display()))
        };
        let row = VignetteRow {
            name: fields.get(name).cloned().unwrap_or_default(),
            flags: parse(flags)?,
        };
        vignettes.insert(parse(id)?, row);
    }
    Ok(vignettes)
}
