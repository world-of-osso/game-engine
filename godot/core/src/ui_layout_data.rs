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

/// Remove only the server-selected character's managed window placements.
/// Missing files are empty; existing unreadable/invalid files fail without replacement.
pub fn reset_window_positions(
    path: &Path,
    selected_character_id: Option<u64>,
) -> Result<(), String> {
    let id = selected_character_id
        .ok_or("Reset Window Positions requires a selected server character ID")?;
    let raw = match fs::read_to_string(path) {
        Ok(raw) => raw,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(()),
        Err(error) => {
            return Err(format!(
                "failed to read UI layout {}: {error}",
                path.display()
            ));
        }
    };
    let mut file: LayoutFile = ron::from_str(&raw)
        .map_err(|error| format!("invalid UI layout {}: {error}", path.display()))?;
    if file.window_positions.remove(&id.to_string()).is_none() {
        return Ok(());
    }
    let serialized = ron::ser::to_string_pretty(&file, ron::ser::PrettyConfig::new())
        .map_err(|error| format!("failed to serialize UI layout {}: {error}", path.display()))?;
    fs::write(path, serialized)
        .map_err(|error| format!("failed to write UI layout {}: {error}", path.display()))
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
