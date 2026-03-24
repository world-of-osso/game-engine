use std::fs;
use std::path::Path;

use bevy::dev_tools::fps_overlay::FpsOverlayConfig;
use bevy::prelude::*;
use serde::{Deserialize, Serialize};

use crate::sound::SoundSettings;

const OPTIONS_PATH: &str = "data/ui/options_settings.ron";
const DEFAULT_MODAL_POSITION: [f32; 2] = [580.0, 190.0];

pub struct ClientOptionsPlugin;

impl Plugin for ClientOptionsPlugin {
    fn build(&self, app: &mut App) {
        let loaded = load_options_file();
        app.insert_resource(CameraOptions::from_file(&loaded.camera))
            .insert_resource(HudOptions::from_file(&loaded.hud))
            .insert_resource(ClientOptionsUiState {
                modal_position: loaded.modal_position.unwrap_or(DEFAULT_MODAL_POSITION),
            })
            .insert_resource(LoadedClientOptions {
                file: loaded,
                applied: false,
            })
            .add_systems(Update, apply_loaded_client_options);
    }
}

#[derive(Resource, Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct CameraOptions {
    pub look_sensitivity: f32,
    pub invert_y: bool,
    pub follow_speed: f32,
    pub zoom_speed: f32,
    pub min_distance: f32,
    pub max_distance: f32,
}

impl Default for CameraOptions {
    fn default() -> Self {
        Self {
            look_sensitivity: 0.01,
            invert_y: false,
            follow_speed: 10.0,
            zoom_speed: 8.0,
            min_distance: 2.0,
            max_distance: 40.0,
        }
    }
}

impl CameraOptions {
    fn from_file(file: &CameraOptionsFile) -> Self {
        Self {
            look_sensitivity: file.look_sensitivity,
            invert_y: file.invert_y,
            follow_speed: file.follow_speed,
            zoom_speed: file.zoom_speed,
            min_distance: file.min_distance,
            max_distance: file.max_distance,
        }
    }
}

#[derive(Resource, Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct HudOptions {
    pub show_minimap: bool,
    pub show_action_bars: bool,
    pub show_nameplates: bool,
    pub show_health_bars: bool,
    pub show_target_marker: bool,
    pub show_fps_overlay: bool,
}

impl Default for HudOptions {
    fn default() -> Self {
        Self {
            show_minimap: true,
            show_action_bars: true,
            show_nameplates: true,
            show_health_bars: true,
            show_target_marker: true,
            show_fps_overlay: true,
        }
    }
}

impl HudOptions {
    fn from_file(file: &HudOptionsFile) -> Self {
        Self {
            show_minimap: file.show_minimap,
            show_action_bars: file.show_action_bars,
            show_nameplates: file.show_nameplates,
            show_health_bars: file.show_health_bars,
            show_target_marker: file.show_target_marker,
            show_fps_overlay: file.show_fps_overlay,
        }
    }
}

#[derive(Resource, Debug, Clone)]
pub struct ClientOptionsUiState {
    pub modal_position: [f32; 2],
}

#[derive(Resource)]
struct LoadedClientOptions {
    file: ClientOptionsFile,
    applied: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct ClientOptionsFile {
    #[serde(default)]
    sound: SoundOptionsFile,
    #[serde(default)]
    camera: CameraOptionsFile,
    #[serde(default)]
    hud: HudOptionsFile,
    #[serde(default)]
    modal_position: Option<[f32; 2]>,
}

impl Default for ClientOptionsFile {
    fn default() -> Self {
        Self {
            sound: SoundOptionsFile::default(),
            camera: CameraOptionsFile::default(),
            hud: HudOptionsFile::default(),
            modal_position: Some(DEFAULT_MODAL_POSITION),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct SoundOptionsFile {
    master_volume: f32,
    footstep_volume: f32,
    ambient_volume: f32,
    music_volume: f32,
    music_enabled: bool,
    muted: bool,
}

impl Default for SoundOptionsFile {
    fn default() -> Self {
        Self::from_runtime(&SoundSettings::default())
    }
}

impl SoundOptionsFile {
    fn from_runtime(settings: &SoundSettings) -> Self {
        Self {
            master_volume: settings.master_volume,
            footstep_volume: settings.footstep_volume,
            ambient_volume: settings.ambient_volume,
            music_volume: settings.music_volume,
            music_enabled: settings.music_enabled,
            muted: settings.muted,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct CameraOptionsFile {
    look_sensitivity: f32,
    invert_y: bool,
    follow_speed: f32,
    zoom_speed: f32,
    min_distance: f32,
    max_distance: f32,
}

impl Default for CameraOptionsFile {
    fn default() -> Self {
        let defaults = CameraOptions::default();
        Self {
            look_sensitivity: defaults.look_sensitivity,
            invert_y: defaults.invert_y,
            follow_speed: defaults.follow_speed,
            zoom_speed: defaults.zoom_speed,
            min_distance: defaults.min_distance,
            max_distance: defaults.max_distance,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct HudOptionsFile {
    show_minimap: bool,
    show_action_bars: bool,
    show_nameplates: bool,
    show_health_bars: bool,
    show_target_marker: bool,
    show_fps_overlay: bool,
}

impl Default for HudOptionsFile {
    fn default() -> Self {
        let defaults = HudOptions::default();
        Self {
            show_minimap: defaults.show_minimap,
            show_action_bars: defaults.show_action_bars,
            show_nameplates: defaults.show_nameplates,
            show_health_bars: defaults.show_health_bars,
            show_target_marker: defaults.show_target_marker,
            show_fps_overlay: defaults.show_fps_overlay,
        }
    }
}

pub fn save_client_options(
    sound: Option<&SoundSettings>,
    camera: &CameraOptions,
    hud: &HudOptions,
    modal_position: [f32; 2],
) -> Result<(), String> {
    let file = ClientOptionsFile {
        sound: sound
            .map(SoundOptionsFile::from_runtime)
            .unwrap_or_default(),
        camera: CameraOptionsFile {
            look_sensitivity: camera.look_sensitivity,
            invert_y: camera.invert_y,
            follow_speed: camera.follow_speed,
            zoom_speed: camera.zoom_speed,
            min_distance: camera.min_distance,
            max_distance: camera.max_distance,
        },
        hud: HudOptionsFile {
            show_minimap: hud.show_minimap,
            show_action_bars: hud.show_action_bars,
            show_nameplates: hud.show_nameplates,
            show_health_bars: hud.show_health_bars,
            show_target_marker: hud.show_target_marker,
            show_fps_overlay: hud.show_fps_overlay,
        },
        modal_position: Some(modal_position),
    };
    let pretty = ron::ser::PrettyConfig::new();
    let serialized = ron::ser::to_string_pretty(&file, pretty)
        .map_err(|err| format!("failed to serialize client options: {err}"))?;
    let path = Path::new(OPTIONS_PATH);
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)
            .map_err(|err| format!("failed to create options dir {}: {err}", parent.display()))?;
    }
    fs::write(path, serialized)
        .map_err(|err| format!("failed to write client options {}: {err}", path.display()))
}

fn load_options_file() -> ClientOptionsFile {
    let path = Path::new(OPTIONS_PATH);
    let Ok(raw) = fs::read_to_string(path) else {
        return ClientOptionsFile::default();
    };
    ron::de::from_str::<ClientOptionsFile>(&raw).unwrap_or_default()
}

fn apply_loaded_client_options(
    mut loaded: ResMut<LoadedClientOptions>,
    mut sound: Option<ResMut<SoundSettings>>,
    mut fps: Option<ResMut<FpsOverlayConfig>>,
) {
    if loaded.applied {
        return;
    }
    if let Some(sound) = sound.as_mut() {
        sound.master_volume = loaded.file.sound.master_volume;
        sound.footstep_volume = loaded.file.sound.footstep_volume;
        sound.ambient_volume = loaded.file.sound.ambient_volume;
        sound.music_volume = loaded.file.sound.music_volume;
        sound.music_enabled = loaded.file.sound.music_enabled;
        sound.muted = loaded.file.sound.muted;
    }
    if let Some(fps) = fps.as_mut() {
        fps.enabled = loaded.file.hud.show_fps_overlay;
    }
    loaded.applied = true;
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_file_uses_expected_modal_position() {
        let file = ClientOptionsFile::default();
        assert_eq!(file.modal_position, Some(DEFAULT_MODAL_POSITION));
    }

    #[test]
    fn hud_defaults_are_visible() {
        let defaults = HudOptions::default();
        assert!(defaults.show_minimap);
        assert!(defaults.show_action_bars);
        assert!(defaults.show_target_marker);
    }
}
