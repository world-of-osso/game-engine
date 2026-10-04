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
    #[serde(default)]
    settings: LayoutSettings,
}

/// An Edit Mode slider's display range (`Blizzard_EditMode/Shared/EditModeSettingDisplayInfo.lua`
/// `minValue`/`maxValue`/`stepSize`). A stored value outside it is clamped when applied
/// (`ClampValue`, :1435-1437).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SettingRange {
    pub min: u16,
    pub max: u16,
    pub step: u16,
}

impl SettingRange {
    const fn new(min: u16, max: u16, step: u16) -> Self {
        Self { min, max, step }
    }

    pub fn clamp(&self, value: u16) -> u16 {
        value.clamp(self.min, self.max)
    }
}

/// `EditModeUnitFrameSetting.FrameSize`, percent (:357-362).
pub const UNIT_FRAME_SIZE_RANGE: SettingRange = SettingRange::new(100, 200, 5);
/// Unit frame text size, percent: not a Retail unit-frame setting; the range of the damage
/// meter's `TextSize` (:1357-1362).
pub const UNIT_FRAME_TEXT_SIZE_RANGE: SettingRange = SettingRange::new(50, 150, 10);
/// `EditModeChatFrameSetting` width and height (:560-591).
pub const CHAT_WIDTH_RANGE: SettingRange = SettingRange::new(250, 800, 1);
pub const CHAT_HEIGHT_RANGE: SettingRange = SettingRange::new(120, 800, 1);
/// `EditModeDamageMeterSetting.FrameWidth` / `FrameHeight` (:1278-1297).
pub const DAMAGE_METER_WIDTH_RANGE: SettingRange = SettingRange::new(200, 600, 1);
pub const DAMAGE_METER_HEIGHT_RANGE: SettingRange = SettingRange::new(120, 400, 1);

/// A font the client ships in `data/fonts`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum LayoutFont {
    FrizQuadrata,
    ArialNarrow,
}

/// One unit frame's settings; `None` keeps the layout's preset value.
#[derive(Debug, Default, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default)]
pub struct UnitFrameSettings {
    /// [`UNIT_FRAME_SIZE_RANGE`].
    pub frame_size: Option<u16>,
    /// The face of the frame's name, level, health and power text.
    pub font: Option<LayoutFont>,
    /// [`UNIT_FRAME_TEXT_SIZE_RANGE`].
    pub text_size: Option<u16>,
}

/// A frame's width and height in UI units; `None` keeps the layout's preset value.
#[derive(Debug, Default, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default)]
pub struct FrameSizeSettings {
    pub width: Option<u16>,
    pub height: Option<u16>,
}

/// A layout's per-system settings over its preset: every field absent means the preset's
/// own value, so a file saved before a setting existed loads unchanged.
#[derive(Debug, Default, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default)]
pub struct LayoutSettings {
    pub player_frame: UnitFrameSettings,
    /// Also the target-of-target frame, a child of Retail's TargetFrame.
    pub target_frame: UnitFrameSettings,
    pub focus_frame: UnitFrameSettings,
    pub pet_frame: UnitFrameSettings,
    /// [`CHAT_WIDTH_RANGE`] × [`CHAT_HEIGHT_RANGE`].
    pub chat: FrameSizeSettings,
    /// [`DAMAGE_METER_WIDTH_RANGE`] × [`DAMAGE_METER_HEIGHT_RANGE`].
    pub damage_meter: FrameSizeSettings,
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
    pub settings: LayoutSettings,
}

/// The Modern preset: the layout of a character that never chose one.
impl Default for ActiveLayout {
    fn default() -> Self {
        let (name, skin) = SYSTEM_PRESETS[0];
        Self {
            name: name.to_string(),
            skin,
            settings: LayoutSettings::default(),
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

fn preset_skin(name: &str) -> Option<LayoutSkin> {
    SYSTEM_PRESETS
        .iter()
        .find(|(preset, _)| *preset == name)
        .map(|&(_, skin)| skin)
}

/// `name` as a system preset (its own values) or a saved player layout.
fn layout_named(file: &LayoutFile, name: &str) -> Result<ActiveLayout, String> {
    let (skin, settings) = match preset_skin(name) {
        Some(skin) => (skin, LayoutSettings::default()),
        None => file
            .edit_mode
            .layouts
            .get(name)
            .map(|layout| (layout.skin, layout.settings))
            .ok_or_else(|| format!("unknown UI layout {name:?}"))?,
    };
    Ok(ActiveLayout {
        name: name.to_string(),
        skin,
        settings,
    })
}

fn active_layout_in(file: &LayoutFile, character_id: u64) -> Result<ActiveLayout, String> {
    match file.edit_mode.active_layout.get(&character_id.to_string()) {
        Some(name) => layout_named(file, name),
        None => Ok(ActiveLayout::default()),
    }
}

/// The character's active layout; a character that never chose one uses the Modern preset.
pub fn active_layout(path: &Path, character_id: u64) -> Result<ActiveLayout, String> {
    active_layout_in(&read_layout(path)?, character_id)
}

/// Save `name` (a system preset or saved layout) as the character's active layout.
pub fn set_active_layout(
    path: &Path,
    character_id: u64,
    name: &str,
) -> Result<ActiveLayout, String> {
    let mut file = read_layout(path)?;
    let layout = layout_named(&file, name)?;
    file.edit_mode
        .active_layout
        .insert(character_id.to_string(), name.to_string());
    write_layout(path, &file)?;
    Ok(layout)
}

/// The first "Layout N" no saved layout uses.
fn unused_layout_name(file: &LayoutFile) -> String {
    (1..)
        .map(|index| format!("Layout {index}"))
        .find(|name| !file.edit_mode.layouts.contains_key(name))
        .expect("an unused layout name")
}

/// Save `settings` to the character's active layout. A system preset is never written:
/// while one is active the settings go to a new player layout with the preset's skin, which
/// becomes the character's active layout (Retail `SaveLayoutChanges` opens the new-layout
/// dialog for a preset, `EditModeManager.lua:1568-1574`).
pub fn save_layout_settings(
    path: &Path,
    character_id: u64,
    settings: LayoutSettings,
) -> Result<ActiveLayout, String> {
    let mut file = read_layout(path)?;
    let active = active_layout_in(&file, character_id)?;
    let name = if preset_skin(&active.name).is_some() {
        unused_layout_name(&file)
    } else {
        active.name
    };
    let layout = file.edit_mode.layouts.entry(name.clone()).or_default();
    layout.skin = active.skin;
    layout.settings = settings;
    file.edit_mode
        .active_layout
        .insert(character_id.to_string(), name.clone());
    write_layout(path, &file)?;
    Ok(ActiveLayout {
        name,
        skin: active.skin,
        settings,
    })
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

    fn temp_layout(name: &str) -> std::path::PathBuf {
        std::env::temp_dir().join(format!("ui-layout-{name}-{}.ron", std::process::id()))
    }

    #[test]
    fn layout_saved_before_settings_existed_loads_with_preset_values() {
        let path = temp_layout("old-file");
        fs::write(
            &path,
            r#"(edit_mode: (
                layouts: {"Raid": (skin: Forever, elements: {"PlayerFrame": (anchor: Center, offset: (4.0, -8.0))})},
                active_layout: {"17": "Raid"},
            ))"#,
        )
        .unwrap();
        let layout = active_layout(&path, 17).unwrap();
        assert_eq!(layout.name, "Raid");
        assert_eq!(layout.skin, LayoutSkin::Forever);
        assert_eq!(layout.settings, LayoutSettings::default());
        assert_eq!(layout.settings.chat.width, None);
        assert_eq!(layout.settings.player_frame.font, None);
        fs::remove_file(path).unwrap();
    }

    fn custom_settings() -> LayoutSettings {
        LayoutSettings {
            player_frame: UnitFrameSettings {
                frame_size: Some(150),
                font: Some(LayoutFont::ArialNarrow),
                text_size: Some(120),
            },
            chat: FrameSizeSettings {
                width: Some(640),
                height: None,
            },
            damage_meter: FrameSizeSettings {
                width: Some(300),
                height: Some(260),
            },
            ..Default::default()
        }
    }

    #[test]
    fn editing_a_preset_saves_a_player_layout_and_leaves_the_preset_unchanged() {
        let path = temp_layout("preset-edit");
        set_active_layout(&path, 17, "Forever").unwrap();
        set_active_layout(&path, 18, "Forever").unwrap();
        let saved = save_layout_settings(&path, 17, custom_settings()).unwrap();
        assert_eq!(saved.name, "Layout 1");
        assert_eq!(saved.skin, LayoutSkin::Forever);
        // Restart: the character's layout and its values come back from the file.
        let reloaded = active_layout(&path, 17).unwrap();
        assert_eq!(reloaded, saved);
        assert_eq!(reloaded.settings, custom_settings());
        // The preset another character uses, and the preset chosen again, keep their values.
        let other = active_layout(&path, 18).unwrap();
        assert_eq!(other.name, "Forever");
        assert_eq!(other.settings, LayoutSettings::default());
        let preset = set_active_layout(&path, 17, "Forever").unwrap();
        assert_eq!(preset.settings, LayoutSettings::default());
        // The player layout is still there to return to.
        assert_eq!(set_active_layout(&path, 17, "Layout 1").unwrap(), saved);
        fs::remove_file(path).unwrap();
    }

    #[test]
    fn editing_a_player_layout_updates_it_in_place_and_a_second_preset_edit_gets_its_own() {
        let path = temp_layout("layout-edit");
        save_layout_settings(&path, 17, custom_settings()).unwrap();
        let mut changed = custom_settings();
        changed.chat.height = Some(300);
        let saved = save_layout_settings(&path, 17, changed).unwrap();
        assert_eq!(saved.name, "Layout 1");
        assert_eq!(saved.skin, LayoutSkin::Modern);
        assert_eq!(active_layout(&path, 17).unwrap().settings, changed);
        // Another character edits the Modern preset: its own layout, the first untouched.
        let second = save_layout_settings(&path, 18, LayoutSettings::default()).unwrap();
        assert_eq!(second.name, "Layout 2");
        assert_eq!(active_layout(&path, 17).unwrap().settings, changed);
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
