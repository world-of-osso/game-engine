//! WarbandScene DB2 data: camera positions + character placements for char select backgrounds.

use std::path::{Path, PathBuf};

use bevy::prelude::*;

use crate::asset::casc_resolver;
use crate::asset::m2::wow_to_bevy;
use crate::terrain_tile::TILE_SIZE;

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
    pub texture_kit: u32,
}

/// Character placement slot within a warband scene.
#[derive(Debug, Clone)]
pub struct WarbandScenePlacement {
    #[allow(dead_code)]
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
#[allow(dead_code)]
pub struct WarbandScenePlacementOption {
    pub placement_id: u32,
    pub layout_key: u32,
    pub position: [f32; 3],
    pub orientation: f32,
    #[allow(dead_code)]
    pub scale: f32,
}

/// Bevy resource holding all parsed warband scenes.
#[derive(Resource)]
pub struct WarbandScenes {
    pub scenes: Vec<WarbandSceneEntry>,
    pub placements: Vec<WarbandScenePlacement>,
    #[allow(dead_code)]
    pub placement_options: Vec<WarbandScenePlacementOption>,
}

impl WarbandScenes {
    pub fn load() -> Self {
        let scenes = load_scenes(Path::new("data/WarbandScene.csv"));
        let placements = load_placements(Path::new("data/WarbandScenePlacement.csv"));
        let placement_options =
            load_placement_options(Path::new("data/WarbandScenePlacementOption.csv"));
        Self {
            scenes,
            placements,
            placement_options,
        }
    }

    /// Get the first character placement for a given scene (slot 0).
    pub fn first_placement(&self, scene_id: u32) -> Option<&WarbandScenePlacement> {
        self.placements
            .iter()
            .filter(|p| p.scene_id == scene_id)
            .min_by_key(|p| p.slot_id)
    }

    pub fn first_character_placement(&self, scene_id: u32) -> Option<&WarbandScenePlacement> {
        self.placements
            .iter()
            .filter(|p| p.scene_id == scene_id && p.is_character_slot())
            .min_by_key(|p| p.slot_id)
    }

    /// Pick the authored placement to use when only one character is rendered.
    ///
    /// Warband placement options encode alternate full-layout arrangements, but this client
    /// does not yet know how to select the same authored layout retail uses. The previous
    /// "most compact" heuristic produced visibly wrong positions, so prefer the first
    /// authored character slot directly until layout selection is understood.
    pub fn solo_character_placement(
        &self,
        scene: &WarbandSceneEntry,
    ) -> Option<WarbandScenePlacement> {
        self.first_character_placement(scene.id).cloned()
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
        // WarbandScene camera positions use standard WoW world axes for transforms,
        // but ADT filenames still map tile row from world Y and tile column from world X.
        let center = 32.0 * TILE_SIZE;
        let row = ((center - self.position[1]) / TILE_SIZE).floor() as i32;
        let col = ((center - self.position[0]) / TILE_SIZE).floor() as i32;
        (row.clamp(0, 63) as u32, col.clamp(0, 63) as u32)
    }

    /// Map name for listfile lookup (warband maps use numeric names).
    pub fn map_name(&self) -> String {
        self.map_id.to_string()
    }

    pub fn preview_image_path(&self) -> Option<&'static str> {
        match self.texture_kit {
            5671 => Some("data/ui/campsites/adventurers-rest.png"),
            5672 => Some("data/ui/campsites/ohnahran-overlook.png"),
            5673 => Some("data/ui/campsites/cultists-quay.png"),
            5674 => Some("data/ui/campsites/freywold-spring.png"),
            5675 => Some("data/ui/campsites/randomize-from-favorites.png"),
            5676 => Some("data/ui/campsites/gallagio-grand-gallery.png"),
            _ => None,
        }
    }
}

impl WarbandScenePlacement {
    pub fn is_character_slot(&self) -> bool {
        self.slot_type == 0
    }

    /// Convert the WoW placement position to Bevy coordinates.
    pub fn bevy_position(&self) -> Vec3 {
        let [bx, by, bz] = wow_to_bevy(self.position[0], self.position[1], self.position[2]);
        Vec3::new(bx, by, bz)
    }

    /// Rotation as Bevy quaternion (WoW degrees around Y axis).
    pub fn bevy_rotation(&self) -> Quat {
        // Character M2s need the same base-facing correction used elsewhere in the renderer.
        Quat::from_rotation_y(self.rotation.to_radians() - std::f32::consts::FRAC_PI_2)
    }
}

/// Extract ADT tile + companions from CASC for a warband scene.
/// Returns the local path to the root ADT file.
pub fn ensure_warband_terrain(scene: &WarbandSceneEntry) -> Option<PathBuf> {
    let map_name = scene.map_name();
    let (tile_y, tile_x) = scene.tile_coords();
    ensure_adt_tile(&map_name, tile_y, tile_x)
}

/// Return the local root ADT path for a warband scene if it is already cached on disk.
pub fn local_warband_terrain(scene: &WarbandSceneEntry) -> Option<PathBuf> {
    let map_name = scene.map_name();
    let (tile_y, tile_x) = scene.tile_coords();
    let local = PathBuf::from(format!("data/terrain/{map_name}_{tile_y}_{tile_x}.adt"));
    local.exists().then_some(local)
}

/// Extract a square neighborhood of ADT tiles around the scene tile.
pub fn ensure_warband_terrain_tiles(scene: &WarbandSceneEntry, radius: i32) -> Vec<PathBuf> {
    let map_name = scene.map_name();
    let (tile_y, tile_x) = scene.tile_coords();
    let mut tiles = Vec::new();
    for dy in -radius..=radius {
        for dx in -radius..=radius {
            let ny = tile_y as i32 + dy;
            let nx = tile_x as i32 + dx;
            if !(0..=63).contains(&ny) || !(0..=63).contains(&nx) {
                continue;
            }
            if let Some(path) = ensure_adt_tile(&map_name, ny as u32, nx as u32) {
                tiles.push(path);
            }
        }
    }
    tiles
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
        texture_kit: fields[15].parse().ok()?,
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

fn load_placement_options(path: &Path) -> Vec<WarbandScenePlacementOption> {
    let Ok(data) = std::fs::read_to_string(path) else {
        eprintln!(
            "WarbandScenePlacementOption.csv not found at {}",
            path.display()
        );
        return Vec::new();
    };
    data.lines()
        .skip(1)
        .filter_map(parse_placement_option_line)
        .collect()
}

fn parse_placement_option_line(line: &str) -> Option<WarbandScenePlacementOption> {
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
        assert_eq!(rest.texture_kit, 5671);
        // Test entries (ID >= 99 or Flags & 7 != 0) should be filtered
        assert!(scenes.iter().all(|s| s.id < 99));
        // Randomize entry (ID=29, Flags=25 → 25 & 7 = 1) should be filtered
        assert!(!scenes.iter().any(|s| s.id == 29));
    }

    #[test]
    fn preview_image_paths_cover_current_char_select_scenes() {
        let scenes = load_scenes(Path::new("data/WarbandScene.csv"));
        for scene in scenes {
            assert!(
                scene.preview_image_path().is_some(),
                "missing preview image path for scene {} ({})",
                scene.id,
                scene.name
            );
        }
    }

    #[test]
    fn warband_scene_tile_coords_match_existing_tiles() {
        let scenes = load_scenes(Path::new("data/WarbandScene.csv"));
        let rest = scenes
            .iter()
            .find(|s| s.id == 1)
            .expect("Adventurer's Rest");
        let (tile_y, tile_x) = rest.tile_coords();

        assert_eq!(
            (tile_y, tile_x),
            (31, 37),
            "Adventurer's Rest should resolve to the existing warband terrain tile"
        );

        let map_name = rest.map_name();
        let adt_path = format!("world/maps/{map_name}/{map_name}_{tile_y}_{tile_x}.adt");
        assert!(
            game_engine::listfile::lookup_path(&adt_path).is_some(),
            "expected listfile entry for {adt_path}"
        );
    }

    #[test]
    fn warband_scene_bevy_position_maps_back_to_loaded_tile() {
        let scenes = load_scenes(Path::new("data/WarbandScene.csv"));
        let rest = scenes
            .iter()
            .find(|s| s.id == 1)
            .expect("Adventurer's Rest");
        let pos = rest.bevy_position();

        assert_eq!(
            crate::terrain::bevy_to_tile_coords(pos.x, pos.z),
            rest.tile_coords()
        );
    }

    #[test]
    fn solo_character_placement_uses_first_authored_character_slot() {
        let warband = WarbandScenes::load();
        let rest = warband
            .scenes
            .iter()
            .find(|s| s.id == 1)
            .expect("Adventurer's Rest");
        let placement = warband
            .solo_character_placement(rest)
            .expect("expected at least one placement");

        assert_eq!(
            placement.slot_id, 0,
            "single-character rendering should start from the first authored character slot"
        );
        assert!(
            (placement.position[0] - (-2981.8201)).abs() < 0.01
                && (placement.position[1] - 457.3500).abs() < 0.01,
            "single-character rendering should use the base authored placement until layout selection is understood"
        );
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
    fn parse_warband_placement_options_csv() {
        let options = load_placement_options(Path::new("data/WarbandScenePlacementOption.csv"));
        assert!(!options.is_empty());
        assert!(options.iter().any(|option| {
            option.placement_id == 1 && option.layout_key == 4 && option.orientation == 108.0
        }));
    }

    #[test]
    fn csv_parser_handles_quoted_commas() {
        let fields = parse_csv_fields(r#""Hello, World",42,"test""#);
        assert_eq!(fields, vec!["Hello, World", "42", "test"]);
    }
}
