//! Client-side UI layout persistence (`ui_layout.ron` next to the options file):
//! per-character window positions.

use std::collections::BTreeMap;
use std::fs;
use std::path::{Path, PathBuf};

use bevy::prelude::*;
use serde::{Deserialize, Serialize};

use crate::networking::SelectedCharacterId;

#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct UiLayoutFile {
    /// Character key → window key → saved top-left in UI units.
    #[serde(default)]
    pub window_positions: BTreeMap<String, BTreeMap<String, [f32; 2]>>,
}

#[derive(Resource, Debug)]
pub struct UiLayoutStore {
    path: PathBuf,
    pub file: UiLayoutFile,
}

impl UiLayoutStore {
    pub fn load(path: PathBuf) -> Self {
        let file = load_layout_file(&path);
        Self { path, file }
    }

    pub fn save(&self) {
        if let Err(err) = save_layout_file(&self.path, &self.file) {
            warn!("{err}");
        }
    }

    pub fn window_position(&self, character: &str, window: &str) -> Option<Vec2> {
        self.file
            .window_positions
            .get(character)?
            .get(window)
            .map(|[x, y]| Vec2::new(*x, *y))
    }

    pub fn set_window_position(&mut self, character: &str, window: &str, pos: Vec2) {
        self.file
            .window_positions
            .entry(character.to_string())
            .or_default()
            .insert(window.to_string(), [pos.x, pos.y]);
    }

    /// Returns whether the character had any saved window position.
    pub fn clear_window_positions(&mut self, character: &str) -> bool {
        self.file.window_positions.remove(character).is_some()
    }
}

/// Stable per-character key: the server character id.
pub fn character_key(selected: Option<&SelectedCharacterId>) -> Option<String> {
    selected?.character_id.map(|id| id.to_string())
}

fn load_layout_file(path: &Path) -> UiLayoutFile {
    if !path.exists() {
        return UiLayoutFile::default();
    }
    let raw = fs::read_to_string(path)
        .unwrap_or_else(|err| panic!("failed to read UI layout {}: {err}", path.display()));
    ron::de::from_str(&raw)
        .unwrap_or_else(|err| panic!("invalid UI layout {}: {err}", path.display()))
}

fn save_layout_file(path: &Path, file: &UiLayoutFile) -> Result<(), String> {
    let serialized = ron::ser::to_string_pretty(file, ron::ser::PrettyConfig::new())
        .map_err(|err| format!("failed to serialize UI layout: {err}"))?;
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)
            .map_err(|err| format!("failed to create UI layout dir {}: {err}", parent.display()))?;
    }
    fs::write(path, serialized)
        .map_err(|err| format!("failed to write UI layout {}: {err}", path.display()))
}

pub struct UiLayoutStorePlugin;

impl Plugin for UiLayoutStorePlugin {
    fn build(&self, app: &mut App) {
        app.insert_resource(UiLayoutStore::load(crate::client_options::ui_layout_path()));
    }
}
