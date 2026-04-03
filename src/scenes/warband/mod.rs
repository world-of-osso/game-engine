//! WarbandScene DB2 data: camera positions + character placements for char select backgrounds.

use std::path::{Path, PathBuf};

use bevy::prelude::*;

use crate::asset::asset_cache;
use crate::asset::m2::wow_to_bevy;
use crate::terrain_tile::TILE_SIZE;

#[path = "cache.rs"]
mod cache;

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
        let (scenes, placements, placement_options) = match cache::load_cached_warband_scenes(
            Path::new("data/WarbandScene.csv"),
            Path::new("data/WarbandScenePlacement.csv"),
            Path::new("data/WarbandScenePlacementOption.csv"),
        ) {
            Ok(data) => data,
            Err(err) => {
                eprintln!("Failed to load warband scene cache: {err}");
                cache::load_warband_scenes_uncached(
                    Path::new("data/WarbandScene.csv"),
                    Path::new("data/WarbandScenePlacement.csv"),
                    Path::new("data/WarbandScenePlacementOption.csv"),
                )
                .unwrap_or_default()
            }
        };
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
            5671 => Some("data/ui/campsites/adventurers-rest.ktx2"),
            5672 => Some("data/ui/campsites/ohnahran-overlook.ktx2"),
            5673 => Some("data/ui/campsites/cultists-quay.ktx2"),
            5674 => Some("data/ui/campsites/freywold-spring.ktx2"),
            5675 => Some("data/ui/campsites/randomize-from-favorites.ktx2"),
            5676 => Some("data/ui/campsites/gallagio-grand-gallery.ktx2"),
            _ => None,
        }
    }

    pub fn authored_light_params_id(&self) -> Option<u32> {
        crate::light_lookup::resolve_light_params_id(self.map_id, self.position)
    }

    pub fn authored_light_skybox_id(&self) -> Option<u32> {
        let light_params_id =
            crate::light_lookup::resolve_skybox_light_params_id(self.map_id, self.position)?;
        crate::light_lookup::resolve_light_skybox_id(light_params_id)
    }

    pub fn authored_skybox_model_wow_path(&self) -> Option<&'static str> {
        let light_skybox_id = self.authored_light_skybox_id()?;
        let fdid = crate::light_lookup::resolve_light_skybox_fdid(light_skybox_id)?;
        let wow_path = game_engine::listfile::lookup_fdid(fdid)?;
        wow_path.ends_with(".m2").then_some(wow_path)
    }

    pub fn skybox_model_wow_path(&self) -> Option<&'static str> {
        if let Some(path) = self.authored_skybox_model_wow_path() {
            return Some(path);
        }
        match self.texture_kit {
            5671..=5676 => Some("environments/stars/costalislandskybox.m2"),
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

pub fn ensure_warband_skybox(scene: &WarbandSceneEntry) -> Option<PathBuf> {
    let wow_path = scene.skybox_model_wow_path()?;
    let filename = Path::new(wow_path).file_name()?;
    let local = PathBuf::from("data/models/skyboxes").join(filename);
    let fdid = game_engine::listfile::lookup_path(wow_path)?;
    asset_cache::file_at_path(fdid, &local)
}

/// Return the local root ADT path for a warband scene if it is already cached on disk.
#[allow(dead_code)]
pub fn local_warband_terrain(scene: &WarbandSceneEntry) -> Option<PathBuf> {
    let map_name = scene.map_name();
    let (tile_y, tile_x) = scene.tile_coords();
    let local = PathBuf::from(format!("data/terrain/{map_name}_{tile_y}_{tile_x}.adt"));
    local.exists().then_some(local)
}

/// Extra tiles needed to complete authored campsite backdrops that cross tile borders.
pub fn supplemental_terrain_tile_coords(scene: &WarbandSceneEntry) -> Vec<(u32, u32)> {
    match scene.id {
        // Adventurer's Rest waterfall sits on the tile immediately west of the campsite tile.
        1 => vec![(31, 36)],
        _ => Vec::new(),
    }
}

/// Extract the specific set of ADT tiles needed for a warband scene background.
#[allow(dead_code)]
pub fn ensure_warband_terrain_tiles(scene: &WarbandSceneEntry) -> Vec<PathBuf> {
    let map_name = scene.map_name();
    let primary = scene.tile_coords();
    let mut tiles = vec![primary];
    for supplemental in supplemental_terrain_tile_coords(scene) {
        if supplemental != primary && !tiles.contains(&supplemental) {
            tiles.push(supplemental);
        }
    }
    tiles
        .into_iter()
        .filter_map(|(tile_y, tile_x)| ensure_adt_tile(&map_name, tile_y, tile_x))
        .collect()
}

/// Extract a single ADT tile + _tex0 + _obj0 from CASC.
fn ensure_adt_tile(map_name: &str, tile_y: u32, tile_x: u32) -> Option<PathBuf> {
    let base_wow = format!("world/maps/{map_name}/{map_name}_{tile_y}_{tile_x}.adt");
    let local = PathBuf::from(format!("data/terrain/{map_name}_{tile_y}_{tile_x}.adt"));

    let fdid = game_engine::listfile::lookup_path(&base_wow)?;
    let local = asset_cache::file_at_path(fdid, &local)?;

    // Also extract _tex0 and _obj0 companions
    for suffix in &["_tex0", "_obj0"] {
        let companion_wow =
            format!("world/maps/{map_name}/{map_name}_{tile_y}_{tile_x}{suffix}.adt");
        let companion_local = PathBuf::from(format!(
            "data/terrain/{map_name}_{tile_y}_{tile_x}{suffix}.adt"
        ));
        if let Some(companion_fdid) = game_engine::listfile::lookup_path(&companion_wow) {
            let _ = asset_cache::file_at_path(companion_fdid, &companion_local);
        }
    }

    Some(local)
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

fn load_scenes(path: &Path) -> Vec<WarbandSceneEntry> {
    cache::load_warband_scenes_uncached(
        path,
        Path::new("data/WarbandScenePlacement.csv"),
        Path::new("data/WarbandScenePlacementOption.csv"),
    )
    .map(|(scenes, _, _)| scenes)
    .unwrap_or_default()
}

fn load_placements(path: &Path) -> Vec<WarbandScenePlacement> {
    cache::load_warband_scenes_uncached(
        Path::new("data/WarbandScene.csv"),
        path,
        Path::new("data/WarbandScenePlacementOption.csv"),
    )
    .map(|(_, placements, _)| placements)
    .unwrap_or_default()
}

fn load_placement_options(path: &Path) -> Vec<WarbandScenePlacementOption> {
    cache::load_warband_scenes_uncached(
        Path::new("data/WarbandScene.csv"),
        Path::new("data/WarbandScenePlacement.csv"),
        path,
    )
    .map(|(_, _, options)| options)
    .unwrap_or_default()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn load_all_scenes_for_audit() -> Vec<WarbandSceneEntry> {
        let data =
            std::fs::read_to_string(Path::new("data/WarbandScene.csv")).expect("WarbandScene.csv");
        data.lines()
            .skip(1)
            .filter_map(|line| {
                let fields = parse_csv_fields(line);
                if fields.len() < 16 {
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
            })
            .collect()
    }

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
        // Randomize entry (ID=29, Flags=25 → 25 & 7 = 1) should be filtered.
        // Scene IDs above 99 are present in current retail data, so the filter contract
        // here is specifically the Flags-based exclusion.
        assert!(!scenes.iter().any(|s| s.id == 29));
    }

    #[test]
    fn preview_image_paths_cover_current_char_select_scenes() {
        let scenes = load_scenes(Path::new("data/WarbandScene.csv"));
        for scene_id in [1, 4, 5, 7, 25] {
            let scene = scenes
                .iter()
                .find(|scene| scene.id == scene_id)
                .unwrap_or_else(|| panic!("missing expected campsite scene {scene_id}"));
            assert!(
                scene.preview_image_path().is_some(),
                "missing preview image path for scene {} ({})",
                scene.id,
                scene.name
            );
        }
        assert!(
            scenes
                .iter()
                .any(|scene| scene.preview_image_path().is_none())
        );
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
    fn warband_scene_supplemental_tiles_cover_waterfall_neighbor() {
        let scenes = load_scenes(Path::new("data/WarbandScene.csv"));
        let rest = scenes
            .iter()
            .find(|s| s.id == 1)
            .expect("Adventurer's Rest");
        let tiles = supplemental_terrain_tile_coords(rest);

        assert_eq!(tiles, vec![(31, 36)], "expected only the waterfall tile");
    }

    #[test]
    fn warband_scene_primary_tile_stays_first_when_loading_all_tiles() {
        let scenes = load_scenes(Path::new("data/WarbandScene.csv"));
        let rest = scenes
            .iter()
            .find(|s| s.id == 1)
            .expect("Adventurer's Rest");
        let tiles = ensure_warband_terrain_tiles(rest);

        assert!(
            !tiles.is_empty(),
            "expected at least the primary ADT tile to be extracted"
        );
        assert!(
            tiles[0].ends_with("data/terrain/2703_31_37.adt"),
            "primary campsite tile should load first, got {}",
            tiles[0].display()
        );
        assert!(
            tiles
                .iter()
                .any(|path| path.ends_with("data/terrain/2703_31_36.adt")),
            "expected waterfall supplemental tile to remain included"
        );
    }

    #[test]
    fn active_warband_scenes_have_skybox_mappings() {
        let warband = WarbandScenes::load();
        for scene in &warband.scenes {
            assert!(
                scene.skybox_model_wow_path().is_some(),
                "scene {} ({}) should map to a skybox model",
                scene.id,
                scene.name
            );
        }
    }

    #[test]
    fn authored_light_scenes_resolve_authored_skybox_paths() {
        let warband = WarbandScenes::load();
        for (scene_id, expected_path) in [
            (1_u32, "environments/stars/deathskybox.m2"),
            (4_u32, "environments/stars/10gsl_sky01.m2"),
            (7_u32, "environments/stars/11xp_cloudsky01.m2"),
            (25_u32, "environments/stars/deathskybox.m2"),
        ] {
            let scene = warband
                .scenes
                .iter()
                .find(|scene| scene.id == scene_id)
                .expect("known scene");
            let path = scene
                .authored_skybox_model_wow_path()
                .expect("authored skybox path");

            assert_eq!(path, expected_path, "scene {scene_id} path mismatch");
        }
    }

    #[test]
    fn active_warband_scenes_now_resolve_authored_skybox_paths() {
        let warband = WarbandScenes::load();
        for scene in &warband.scenes {
            let path = scene
                .authored_skybox_model_wow_path()
                .expect("active scene should resolve authored skybox path");
            assert!(
                path.ends_with(".m2"),
                "scene {} ({}) should resolve authored m2 skybox path, got {path}",
                scene.id,
                scene.name
            );
        }
    }

    #[test]
    fn non_active_warband_scene_rows_also_resolve_authored_skybox_paths() {
        let scenes = load_all_scenes_for_audit();
        for (scene_id, expected_path) in [
            (119_u32, "environments/stars/11krs_mainskybox01.m2"),
            (145_u32, "environments/stars/deathskybox.m2"),
            (146_u32, "environments/stars/deathskybox.m2"),
        ] {
            let scene = scenes
                .iter()
                .find(|scene| scene.id == scene_id)
                .expect("known scene");
            let path = scene
                .authored_skybox_model_wow_path()
                .expect("authored skybox path");
            assert_eq!(path, expected_path, "scene {scene_id} path mismatch");
        }
    }

    #[test]
    fn scenes_missing_primary_lightparams_rows_can_still_resolve_authored_skyboxes() {
        let warband = WarbandScenes::load();
        let scene = warband
            .scenes
            .iter()
            .find(|scene| scene.id == 25)
            .expect("known scene");

        assert_eq!(scene.authored_light_params_id(), Some(6412));
        assert_eq!(
            scene.authored_skybox_model_wow_path(),
            Some("environments/stars/deathskybox.m2")
        );
    }

    #[test]
    fn authored_star_skyboxes_override_shared_star_fallback() {
        let warband = WarbandScenes::load();
        let scene = warband
            .scenes
            .iter()
            .find(|scene| scene.id == 1)
            .expect("known scene");

        assert_eq!(scene.texture_kit, 5671);
        assert_eq!(
            scene.skybox_model_wow_path(),
            Some("environments/stars/deathskybox.m2")
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
            (placement.position[0] - (-2_981.82)).abs() < 0.01
                && (placement.position[1] - 457.35).abs() < 0.01,
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
