//! WarbandScene DB2 data: camera positions + character placements for char select backgrounds.

use std::path::{Path, PathBuf};

use bevy::prelude::*;

use crate::asset::casc_resolver;
use crate::asset::m2::wow_to_bevy;
use crate::terrain::bevy_to_tile_coords;

/// A single warband scene entry (parsed from WarbandScene.csv).
#[derive(Debug, Clone)]
pub struct WarbandSceneEntry {
    pub id: u32,
    pub name: String,
    #[allow(dead_code)]
    pub description: String,
    /// WoW world position [X, Y, Z] for the camera.
    pub position: [f32; 3],
    /// WoW world look-at point [X, Y, Z].
    pub look_at: [f32; 3],
    pub map_id: u32,
    pub fov: f32,
}

/// Character placement slot within a warband scene.
#[derive(Debug, Clone)]
pub struct WarbandScenePlacement {
    pub scene_id: u32,
    /// WoW world position [X, Y, Z].
    pub position: [f32; 3],
    /// Rotation in degrees.
    pub rotation: f32,
    pub slot_id: u32,
}

/// Bevy resource holding all parsed warband scenes.
#[derive(Resource)]
pub struct WarbandScenes {
    pub scenes: Vec<WarbandSceneEntry>,
    pub placements: Vec<WarbandScenePlacement>,
}

impl WarbandScenes {
    pub fn load() -> Self {
        let scenes = load_scenes(Path::new("data/WarbandScene.csv"));
        let placements = load_placements(Path::new("data/WarbandScenePlacement.csv"));
        Self { scenes, placements }
    }

    /// Get the first character placement for a given scene (slot 0).
    pub fn first_placement(&self, scene_id: u32) -> Option<&WarbandScenePlacement> {
        self.placements
            .iter()
            .filter(|p| p.scene_id == scene_id)
            .min_by_key(|p| p.slot_id)
    }
}

/// Currently selected warband scene for the char select background.
#[derive(Resource)]
pub struct SelectedWarbandScene {
    pub scene_id: u32,
}

impl WarbandSceneEntry {
    /// Convert the WoW camera position to Bevy coordinates.
    pub fn bevy_position(&self) -> Vec3 {
        let [bx, by, bz] = wow_to_bevy(self.position[0], self.position[1], self.position[2]);
        Vec3::new(bx, by, bz)
    }

    /// Convert the WoW look-at position to Bevy coordinates.
    pub fn bevy_look_at(&self) -> Vec3 {
        let [bx, by, bz] = wow_to_bevy(self.look_at[0], self.look_at[1], self.look_at[2]);
        Vec3::new(bx, by, bz)
    }

    /// Compute the ADT tile coordinates for this scene's camera position.
    pub fn tile_coords(&self) -> (u32, u32) {
        let pos = self.bevy_position();
        bevy_to_tile_coords(pos.x, pos.z)
    }

    /// Map name for listfile lookup (warband maps use numeric names).
    pub fn map_name(&self) -> String {
        self.map_id.to_string()
    }
}

impl WarbandScenePlacement {
    /// Convert the WoW placement position to Bevy coordinates.
    pub fn bevy_position(&self) -> Vec3 {
        let [bx, by, bz] = wow_to_bevy(self.position[0], self.position[1], self.position[2]);
        Vec3::new(bx, by, bz)
    }

    /// Rotation as Bevy quaternion (WoW degrees around Y axis).
    pub fn bevy_rotation(&self) -> Quat {
        Quat::from_rotation_y(self.rotation.to_radians())
    }
}

/// Extract ADT tile + companions from CASC for a warband scene.
/// Returns the local path to the root ADT file.
pub fn ensure_warband_terrain(scene: &WarbandSceneEntry) -> Option<PathBuf> {
    let map_name = scene.map_name();
    let (tile_y, tile_x) = scene.tile_coords();
    ensure_adt_tile(&map_name, tile_y, tile_x)
}

/// Extract a single ADT tile + _tex0 + _obj0 from CASC.
fn ensure_adt_tile(map_name: &str, tile_y: u32, tile_x: u32) -> Option<PathBuf> {
    let base_wow = format!("world/maps/{map_name}/{map_name}_{tile_y}_{tile_x}.adt");
    let local = PathBuf::from(format!("data/terrain/{map_name}_{tile_y}_{tile_x}.adt"));

    let fdid = game_engine::listfile::lookup_path(&base_wow)?;
    casc_resolver::ensure_file_at_path(fdid, &local)?;

    // Also extract _tex0 and _obj0 companions
    for suffix in &["_tex0", "_obj0"] {
        let companion_wow =
            format!("world/maps/{map_name}/{map_name}_{tile_y}_{tile_x}{suffix}.adt");
        let companion_local = PathBuf::from(format!(
            "data/terrain/{map_name}_{tile_y}_{tile_x}{suffix}.adt"
        ));
        if let Some(companion_fdid) = game_engine::listfile::lookup_path(&companion_wow) {
            let _ = casc_resolver::ensure_file_at_path(companion_fdid, &companion_local);
        }
    }

    Some(local)
}

fn load_scenes(path: &Path) -> Vec<WarbandSceneEntry> {
    let Ok(data) = std::fs::read_to_string(path) else {
        eprintln!("WarbandScene.csv not found at {}", path.display());
        return Vec::new();
    };
    let mut scenes = Vec::new();
    for line in data.lines().skip(1) {
        if let Some(entry) = parse_scene_line(line) {
            // Filter out test entries: Flags field has bit 1 or 2 set (values 1,3,7),
            // or ID >= 99.
            if entry.id >= 99 {
                continue;
            }
            scenes.push(entry);
        }
    }
    scenes
}

fn parse_scene_line(line: &str) -> Option<WarbandSceneEntry> {
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
    })
}

fn load_placements(path: &Path) -> Vec<WarbandScenePlacement> {
    let Ok(data) = std::fs::read_to_string(path) else {
        eprintln!("WarbandScenePlacement.csv not found at {}", path.display());
        return Vec::new();
    };
    let mut placements = Vec::new();
    for line in data.lines().skip(1) {
        if let Some(entry) = parse_placement_line(line) {
            placements.push(entry);
        }
    }
    placements
}

fn parse_placement_line(line: &str) -> Option<WarbandScenePlacement> {
    // Position_0,Position_1,Position_2,ID,WarbandSceneID,SlotType,Rotation,Scale,...,SlotID,...
    let fields: Vec<&str> = line.split(',').collect();
    if fields.len() < 12 {
        return None;
    }
    Some(WarbandScenePlacement {
        scene_id: fields[4].parse().ok()?,
        position: [
            fields[0].parse().ok()?,
            fields[1].parse().ok()?,
            fields[2].parse().ok()?,
        ],
        rotation: fields[6].parse().ok()?,
        slot_id: fields[11].parse().ok()?,
    })
}

/// Parse a CSV line respecting quoted fields.
fn parse_csv_fields(line: &str) -> Vec<String> {
    let mut fields = Vec::new();
    let mut current = String::new();
    let mut in_quotes = false;
    for ch in line.chars() {
        match ch {
            '"' => in_quotes = !in_quotes,
            ',' if !in_quotes => {
                fields.push(std::mem::take(&mut current));
            }
            _ => current.push(ch),
        }
    }
    fields.push(current);
    fields
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_warband_scenes_csv() {
        let scenes = load_scenes(Path::new("data/WarbandScene.csv"));
        assert!(
            !scenes.is_empty(),
            "Should parse at least one warband scene"
        );
        // Adventurer's Rest should be present (ID=1, Flags=8 → 8 & 7 = 0)
        let rest = scenes
            .iter()
            .find(|s| s.id == 1)
            .expect("Adventurer's Rest");
        assert_eq!(rest.name, "Adventurer's Rest");
        assert_eq!(rest.map_id, 2703);
        assert!((rest.fov - 65.0).abs() < 0.1);
        // Test entries (ID >= 99 or Flags & 7 != 0) should be filtered
        assert!(scenes.iter().all(|s| s.id < 99));
        // Randomize entry (ID=29, Flags=25 → 25 & 7 = 1) should be filtered
        assert!(!scenes.iter().any(|s| s.id == 29));
    }

    #[test]
    fn parse_warband_placements_csv() {
        let placements = load_placements(Path::new("data/WarbandScenePlacement.csv"));
        assert!(!placements.is_empty());
        // Scene 1 should have placements
        let scene1: Vec<_> = placements.iter().filter(|p| p.scene_id == 1).collect();
        assert!(!scene1.is_empty());
    }

    #[test]
    fn csv_parser_handles_quoted_commas() {
        let fields = parse_csv_fields(r#""Hello, World",42,"test""#);
        assert_eq!(fields, vec!["Hello, World", "42", "test"]);
    }
}
