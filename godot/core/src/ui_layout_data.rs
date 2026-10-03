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
    skin: LayoutSkin,
    #[serde(default)]
    elements: BTreeMap<String, SavedElement>,
}

/// The art a layout draws its frames with: Retail atlases, or Forever's set-1 re-skin.
#[derive(Debug, Default, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum LayoutSkin {
    #[default]
    Modern,
    Forever,
}

/// Built-in layouts, listed before saved ones as Retail lists its presets
/// (`Blizzard_EditMode/Shared/EditModePresetLayoutsManager.lua:5-21`); ours also pick a skin.
pub const SYSTEM_PRESETS: [(&str, LayoutSkin); 2] = [
    ("Modern", LayoutSkin::Modern),
    ("Forever", LayoutSkin::Forever),
];

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ActiveLayout {
    pub name: String,
    pub skin: LayoutSkin,
}

/// The Modern preset: the layout of a character that never chose one.
impl Default for ActiveLayout {
    fn default() -> Self {
        let (name, skin) = SYSTEM_PRESETS[0];
        Self {
            name: name.to_string(),
            skin,
        }
    }
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

fn layout_skin(file: &LayoutFile, name: &str) -> Result<LayoutSkin, String> {
    SYSTEM_PRESETS
        .iter()
        .find(|(preset, _)| *preset == name)
        .map(|&(_, skin)| skin)
        .or_else(|| file.edit_mode.layouts.get(name).map(|layout| layout.skin))
        .ok_or_else(|| format!("unknown UI layout {name:?}"))
}

/// The character's active layout; a character that never chose one uses the Modern preset.
pub fn active_layout(path: &Path, character_id: u64) -> Result<ActiveLayout, String> {
    let file = read_layout(path)?;
    let Some(name) = file.edit_mode.active_layout.get(&character_id.to_string()) else {
        return Ok(ActiveLayout::default());
    };
    Ok(ActiveLayout {
        name: name.clone(),
        skin: layout_skin(&file, name)?,
    })
}

/// Save `name` (a system preset or saved layout) as the character's active layout.
pub fn set_active_layout(path: &Path, character_id: u64, name: &str) -> Result<LayoutSkin, String> {
    let mut file = read_layout(path)?;
    let skin = layout_skin(&file, name)?;
    file.edit_mode
        .active_layout
        .insert(character_id.to_string(), name.to_string());
    write_layout(path, &file)?;
    Ok(skin)
}

pub fn window_position(
    path: &Path,
    character_id: u64,
    root: &str,
) -> Result<Option<[f32; 2]>, String> {
    Ok(read_layout(path)?
        .window_positions
        .get(&character_id.to_string())
        .and_then(|windows| windows.get(root).copied()))
}

pub fn save_window_position(
    path: &Path,
    character_id: u64,
    root: &str,
    position: [f32; 2],
) -> Result<(), String> {
    let mut file = read_layout(path)?;
    file.window_positions
        .entry(character_id.to_string())
        .or_default()
        .insert(root.to_string(), position);
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
        assert_eq!(window_position(&path, 17, "SpellBookRoot").unwrap(), None);
        save_window_position(&path, 17, "WorldMapFrame", [210.0, 104.0]).unwrap();
        save_window_position(&path, 17, "SpellBookRoot", [40.0, 120.0]).unwrap();
        save_window_position(&path, 18, "WorldMapFrame", [75.0, 80.0]).unwrap();
        save_window_position(&path, 18, "SpellBookRoot", [70.0, 90.0]).unwrap();
        assert_eq!(
            window_position(&path, 17, "WorldMapFrame").unwrap(),
            Some([210.0, 104.0])
        );
        assert_eq!(
            window_position(&path, 17, "SpellBookRoot").unwrap(),
            Some([40.0, 120.0])
        );
        reset_window_positions(&path, Some(17)).unwrap();
        assert_eq!(window_position(&path, 17, "WorldMapFrame").unwrap(), None);
        assert_eq!(window_position(&path, 17, "SpellBookRoot").unwrap(), None);
        assert_eq!(
            window_position(&path, 18, "WorldMapFrame").unwrap(),
            Some([75.0, 80.0])
        );
        assert_eq!(
            window_position(&path, 18, "SpellBookRoot").unwrap(),
            Some([70.0, 90.0])
        );
        let raw = fs::read_to_string(&path).unwrap();
        fs::write(&path, "(window_positions: invalid)").unwrap();
        assert!(window_position(&path, 18, "SpellBookRoot").is_err());
        assert!(save_window_position(&path, 17, "SpellBookRoot", [1.0, 2.0]).is_err());
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
