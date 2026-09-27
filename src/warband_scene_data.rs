//! Bevy-free authored Warband scene records and CSV parsing.

use std::path::Path;

use crate::asset::adt::CHUNK_SIZE;
use crate::csv_util::parse_csv_line as parse_csv_fields;

/// A single warband scene entry (parsed from WarbandScene.csv).
#[derive(Debug, Clone)]
pub struct WarbandSceneEntry {
    pub id: u32,
    pub name: String,
    pub description: String,
    /// WoW world position [X, Y, Z] for the camera.
    pub position: [f32; 3],
    /// WoW world look-at point [X, Y, Z].
    pub look_at: [f32; 3],
    pub map_id: u32,
    pub fov: f32,
    pub texture_kit: u32,
}

/// Character placement slot within a warband scene.
#[derive(Debug, Clone)]
pub struct WarbandScenePlacement {
    pub id: u32,
    pub scene_id: u32,
    pub slot_type: u32,
    /// WoW world position [X, Y, Z].
    pub position: [f32; 3],
    /// Rotation in degrees.
    pub rotation: f32,
    pub slot_id: u32,
}

/// Optional authored overrides for a placement in alternate warband layouts.
#[derive(Debug, Clone)]
pub struct WarbandScenePlacementOption {
    pub placement_id: u32,
    pub layout_key: u32,
    pub position: [f32; 3],
    pub orientation: f32,
    pub scale: f32,
}

#[derive(Debug)]
pub struct WarbandSceneCatalog {
    pub scenes: Vec<WarbandSceneEntry>,
    pub placements: Vec<WarbandScenePlacement>,
    pub placement_options: Vec<WarbandScenePlacementOption>,
}

/// Read original authored rows directly; never substitute an empty catalog on file errors.
pub fn read_authored_catalog(data_root: &Path) -> Result<WarbandSceneCatalog, String> {
    Ok(WarbandSceneCatalog {
        scenes: read_rows(&data_root.join("WarbandScene.csv"), parse_scene_line)?,
        placements: read_rows(
            &data_root.join("WarbandScenePlacement.csv"),
            parse_placement_line,
        )?,
        placement_options: read_rows(
            &data_root.join("WarbandScenePlacementOption.csv"),
            parse_placement_option_line,
        )?,
    })
}

fn read_rows<T>(path: &Path, parse: fn(&str) -> Option<T>) -> Result<Vec<T>, String> {
    let contents =
        std::fs::read_to_string(path).map_err(|err| format!("read {}: {err}", path.display()))?;
    Ok(contents.lines().skip(1).filter_map(parse).collect())
}

impl WarbandSceneEntry {
    /// Compute the ADT tile coordinates for this scene's camera position.
    pub fn tile_coords(&self) -> (u32, u32) {
        // WarbandScene camera positions use standard WoW world axes for transforms,
        // but ADT filenames still map tile row from world Y and tile column from world X.
        let tile_size = CHUNK_SIZE * 16.0;
        let center = 32.0 * tile_size;
        let row = ((center - self.position[1]) / tile_size).floor() as i32;
        let col = ((center - self.position[0]) / tile_size).floor() as i32;
        (row.clamp(0, 63) as u32, col.clamp(0, 63) as u32)
    }

    /// Map name for listfile lookup (warband maps use numeric names).
    pub fn map_name(&self) -> String {
        self.map_id.to_string()
    }
}

/// Extra tiles needed to complete authored campsite backdrops across tile borders.
pub fn supplemental_terrain_tile_coords(scene: &WarbandSceneEntry) -> Vec<(u32, u32)> {
    match scene.id {
        // Adventurer's Rest waterfall occupies the western neighboring tile.
        1 => vec![(31, 36)],
        _ => Vec::new(),
    }
}

impl WarbandScenePlacement {
    pub fn is_character_slot(&self) -> bool {
        self.slot_type == 0
    }
}

pub(crate) fn parse_scene_line(line: &str) -> Option<WarbandSceneEntry> {
    // CSV with quoted strings: Name_lang,Description_lang,Position_0..2,LookAt_0..2,ID,MapID,Fov,...,Flags,...
    let fields = parse_csv_fields(line);
    if fields.len() < 13 {
        return None;
    }
    let flags: u32 = fields[12].parse().ok()?;
    // Flags & 7 means test/internal entries (values 1, 3, 7)
    if flags & 7 != 0 {
        return None;
    }
    Some(WarbandSceneEntry {
        id: fields[8].parse().ok()?,
        name: fields[0].trim_matches('"').to_string(),
        description: fields[1].trim_matches('"').to_string(),
        position: [
            fields[2].parse().ok()?,
            fields[3].parse().ok()?,
            fields[4].parse().ok()?,
        ],
        look_at: [
            fields[5].parse().ok()?,
            fields[6].parse().ok()?,
            fields[7].parse().ok()?,
        ],
        map_id: fields[9].parse().ok()?,
        fov: fields[10].parse().ok()?,
        texture_kit: fields[15].parse().ok()?,
    })
}

pub(crate) fn parse_placement_line(line: &str) -> Option<WarbandScenePlacement> {
    // Position_0,Position_1,Position_2,ID,WarbandSceneID,SlotType,Rotation,Scale,...,SlotID,...
    let fields: Vec<&str> = line.split(',').collect();
    if fields.len() < 12 {
        return None;
    }
    Some(WarbandScenePlacement {
        id: fields[3].parse().ok()?,
        scene_id: fields[4].parse().ok()?,
        slot_type: fields[5].parse().ok()?,
        position: [
            fields[0].parse().ok()?,
            fields[1].parse().ok()?,
            fields[2].parse().ok()?,
        ],
        rotation: fields[6].parse().ok()?,
        slot_id: fields[11].parse().ok()?,
    })
}

pub(crate) fn parse_placement_option_line(line: &str) -> Option<WarbandScenePlacementOption> {
    let fields: Vec<&str> = line.split(',').collect();
    if fields.len() < 8 {
        return None;
    }
    Some(WarbandScenePlacementOption {
        placement_id: fields[1].parse().ok()?,
        layout_key: fields[2].parse().ok()?,
        position: [
            fields[3].parse().ok()?,
            fields[4].parse().ok()?,
            fields[5].parse().ok()?,
        ],
        orientation: fields[6].parse().ok()?,
        scale: fields[7].parse().ok()?,
    })
}
