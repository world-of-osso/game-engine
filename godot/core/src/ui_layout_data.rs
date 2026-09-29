//! Canonical UI layout file operation shared with the legacy client schema.
use serde::{Deserialize, Serialize};
use std::{collections::BTreeMap, fs, path::Path};

#[derive(Default, Serialize, Deserialize)]
struct LayoutFile {
    #[serde(default)]
    window_positions: BTreeMap<String, BTreeMap<String, [f32; 2]>>,
    #[serde(default)]
    edit_mode: EditModeLayoutsFile,
}

#[derive(Default, Serialize, Deserialize)]
struct EditModeLayoutsFile {
    #[serde(default)]
    layouts: BTreeMap<String, EditLayout>,
    #[serde(default)]
    active_layout: BTreeMap<String, String>,
}

#[derive(Default, Serialize, Deserialize)]
struct EditLayout {
    #[serde(default)]
    elements: BTreeMap<String, SavedElement>,
}

#[derive(Serialize, Deserialize)]
struct SavedElement {
    anchor: HudAnchor,
    offset: [f32; 2],
}

#[derive(Serialize, Deserialize)]
enum HudAnchor {
    TopLeft,
    Top,
    TopRight,
    Left,
    Center,
    Right,
    BottomLeft,
    Bottom,
    BottomRight,
}

const WORLD_MAP_KEY: &str = "WorldMapFrame";

fn read_layout(path: &Path) -> Result<LayoutFile, String> {
    let raw = match fs::read_to_string(path) {
        Ok(raw) => raw,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
            return Ok(LayoutFile::default());
        }
        Err(error) => {
            return Err(format!(
                "failed to read UI layout {}: {error}",
                path.display()
            ));
        }
    };
    ron::from_str(&raw).map_err(|error| format!("invalid UI layout {}: {error}", path.display()))
}

fn write_layout(path: &Path, file: &LayoutFile) -> Result<(), String> {
    let serialized = ron::ser::to_string_pretty(file, ron::ser::PrettyConfig::new())
        .map_err(|error| format!("failed to serialize UI layout {}: {error}", path.display()))?;
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).map_err(|error| {
            format!(
                "failed to create UI layout directory {}: {error}",
                parent.display()
            )
        })?;
    }
    fs::write(path, serialized)
        .map_err(|error| format!("failed to write UI layout {}: {error}", path.display()))
}

pub fn world_map_position(path: &Path, character_id: u64) -> Result<Option<[f32; 2]>, String> {
    Ok(read_layout(path)?
        .window_positions
        .get(&character_id.to_string())
        .and_then(|windows| windows.get(WORLD_MAP_KEY).copied()))
}

pub fn save_world_map_position(
    path: &Path,
    character_id: u64,
    position: [f32; 2],
) -> Result<(), String> {
    let mut file = read_layout(path)?;
    file.window_positions
        .entry(character_id.to_string())
        .or_default()
        .insert(WORLD_MAP_KEY.to_string(), position);
    write_layout(path, &file)
}

/// Remove only the server-selected character's managed window placements.
/// Missing files are empty; existing unreadable/invalid files fail without replacement.
pub fn reset_window_positions(
    path: &Path,
    selected_character_id: Option<u64>,
) -> Result<(), String> {
    let id = selected_character_id
        .ok_or("Reset Window Positions requires a selected server character ID")?;
    let mut file = read_layout(path)?;
    if file.window_positions.remove(&id.to_string()).is_none() {
        return Ok(());
    }
    write_layout(path, &file)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn invalid_layout_and_missing_character_never_replace_saved_data() {
        let path = std::env::temp_dir().join(format!(
            "ui-layout-reset-{}-{}-test.ron",
            std::process::id(),
            std::thread::current().name().unwrap_or("worker")
        ));
        let malformed = "(window_positions: not valid)";
        fs::write(&path, malformed).expect("seed invalid layout");
        let no_character = reset_window_positions(&path, None).unwrap_err();
        assert!(no_character.contains("selected server character ID"));
        let invalid_layout = reset_window_positions(&path, Some(17)).unwrap_err();
        assert!(invalid_layout.contains("invalid UI layout"));
        assert_eq!(fs::read_to_string(&path).unwrap(), malformed);
        fs::remove_file(&path).expect("remove isolated test layout");
    }

    #[test]
    fn map_position_round_trips_by_character_and_reset_preserves_others() {
        let path = std::env::temp_dir().join(format!(
            "ui-layout-map-{}-{}-test.ron",
            std::process::id(),
            std::thread::current().name().unwrap_or("worker")
        ));
        assert_eq!(world_map_position(&path, 17).unwrap(), None);
        save_world_map_position(&path, 17, [210.0, 104.0]).unwrap();
        save_world_map_position(&path, 18, [75.0, 80.0]).unwrap();
        assert_eq!(world_map_position(&path, 17).unwrap(), Some([210.0, 104.0]));
        assert_eq!(world_map_position(&path, 18).unwrap(), Some([75.0, 80.0]));
        reset_window_positions(&path, Some(17)).unwrap();
        assert_eq!(world_map_position(&path, 17).unwrap(), None);
        assert_eq!(world_map_position(&path, 18).unwrap(), Some([75.0, 80.0]));
        let raw = fs::read_to_string(&path).unwrap();
        fs::write(&path, "(window_positions: invalid)").unwrap();
        assert!(world_map_position(&path, 18).is_err());
        assert!(save_world_map_position(&path, 17, [1.0, 2.0]).is_err());
        assert_eq!(
            fs::read_to_string(&path).unwrap(),
            "(window_positions: invalid)"
        );
        fs::write(&path, raw).unwrap();
        fs::remove_file(path).unwrap();
    }

    #[test]
    fn unreadable_layout_reports_path_without_creating_replacement() {
        let path = std::env::temp_dir().join(format!("ui-layout-reset-{}-dir", std::process::id()));
        fs::create_dir(&path).expect("create unreadable layout path");
        let error = reset_window_positions(&path, Some(17)).unwrap_err();
        assert!(error.contains("failed to read UI layout"));
        assert!(error.contains(path.to_str().unwrap()));
        assert!(path.is_dir());
        fs::remove_dir(&path).expect("remove isolated test directory");
    }
}
