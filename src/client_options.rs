use std::fs;
use std::path::{Path, PathBuf};

use bevy::dev_tools::fps_overlay::FpsOverlayConfig;
use bevy::prelude::*;
use directories::ProjectDirs;
use serde::{Deserialize, Serialize};

use crate::sound::SoundSettings;

const LEGACY_OPTIONS_PATH: &str = "data/ui/options_settings.ron";
const OPTIONS_FILE_NAME: &str = "options_settings.ron";
const LEGACY_CREDENTIALS_PATH: &str = "data/ui/credentials.ron";
const CREDENTIALS_FILE_NAME: &str = "credentials.ron";

pub struct ClientOptionsPlugin;

impl Plugin for ClientOptionsPlugin {
    fn build(&self, app: &mut App) {
        let loaded = load_options_file();
        app.insert_resource(CameraOptions::from_file(&loaded.camera))
            .insert_resource(HudOptions::from_file(&loaded.hud))
            .insert_resource(ClientOptionsUiState {
                modal_offset: loaded.modal_offset,
                legacy_modal_position: loaded.modal_position,
            })
            .insert_resource(LoadedClientOptions {
                file: loaded,
                applied: false,
            })
            .add_systems(Update, apply_loaded_client_options);
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct LoginCredentials {
    pub username: String,
    pub password: String,
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
    pub modal_offset: Option<[f32; 2]>,
    pub legacy_modal_position: Option<[f32; 2]>,
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
    #[serde(default, skip_serializing_if = "Option::is_none")]
    modal_offset: Option<[f32; 2]>,
    #[serde(default)]
    #[serde(rename = "modal_position", skip_serializing_if = "Option::is_none")]
    modal_position: Option<[f32; 2]>,
}

impl Default for ClientOptionsFile {
    fn default() -> Self {
        Self {
            sound: SoundOptionsFile::default(),
            camera: CameraOptionsFile::default(),
            hud: HudOptionsFile::default(),
            modal_offset: None,
            modal_position: None,
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
    modal_offset: [f32; 2],
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
        modal_offset: Some(modal_offset),
        modal_position: None,
    };
    let pretty = ron::ser::PrettyConfig::new();
    let serialized = ron::ser::to_string_pretty(&file, pretty)
        .map_err(|err| format!("failed to serialize client options: {err}"))?;
    let path = options_path();
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)
            .map_err(|err| format!("failed to create options dir {}: {err}", parent.display()))?;
    }
    info!("Saving client options to {}", path.display());
    fs::write(&path, serialized)
        .map_err(|err| format!("failed to write client options {}: {err}", path.display()))
}

pub fn save_client_options_values(
    sound: &SoundSettings,
    camera: &CameraOptions,
    hud: &HudOptions,
    modal_offset: [f32; 2],
) -> Result<(), String> {
    save_client_options(Some(sound), camera, hud, modal_offset)
}

fn load_options_file() -> ClientOptionsFile {
    let path = load_options_path();
    if !path.exists() {
        info!(
            "No client options file found at {}; using defaults",
            path.display()
        );
        return ClientOptionsFile::default();
    }

    info!("Loading client options from {}", path.display());
    let Ok(raw) = fs::read_to_string(&path) else {
        return ClientOptionsFile::default();
    };
    ron::de::from_str::<ClientOptionsFile>(&raw).unwrap_or_default()
}

fn load_options_path() -> PathBuf {
    let config_path = options_path();
    let legacy_path = PathBuf::from(LEGACY_OPTIONS_PATH);
    select_load_options_path(&config_path, &legacy_path)
}

fn options_path() -> PathBuf {
    if let Some(proj_dirs) = ProjectDirs::from("org", "WorldOfOsso", "game-engine") {
        return proj_dirs.config_dir().join(OPTIONS_FILE_NAME);
    }

    Path::new(LEGACY_OPTIONS_PATH).to_path_buf()
}

pub fn load_login_credentials() -> Option<LoginCredentials> {
    let path = login_credentials_path();
    if !path.exists() {
        return None;
    }

    let raw = fs::read_to_string(&path).ok()?;
    let creds = ron::de::from_str::<LoginCredentials>(&raw).ok()?;
    if creds.username.trim().is_empty() || creds.password.trim().is_empty() {
        return None;
    }
    Some(creds)
}

pub fn login_credentials_path() -> PathBuf {
    if let Some(proj_dirs) = ProjectDirs::from("org", "WorldOfOsso", "game-engine") {
        return proj_dirs.config_dir().join(CREDENTIALS_FILE_NAME);
    }

    Path::new(LEGACY_CREDENTIALS_PATH).to_path_buf()
}

fn select_load_options_path(config_path: &Path, legacy_path: &Path) -> PathBuf {
    if config_path.exists() {
        return config_path.to_path_buf();
    }
    if legacy_path.exists() {
        return legacy_path.to_path_buf();
    }
    config_path.to_path_buf()
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
    use std::time::{SystemTime, UNIX_EPOCH};

    fn unique_test_dir(name: &str) -> PathBuf {
        let nanos = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        Path::new("target/test-artifacts")
            .join("client_options")
            .join(format!("{name}-{nanos}"))
    }

    #[test]
    fn default_file_uses_expected_modal_position() {
        let file = ClientOptionsFile::default();
        assert_eq!(file.modal_offset, None);
        assert_eq!(file.modal_position, None);
    }

    #[test]
    fn hud_defaults_are_visible() {
        let defaults = HudOptions::default();
        assert!(defaults.show_minimap);
        assert!(defaults.show_action_bars);
        assert!(defaults.show_target_marker);
    }

    #[test]
    fn prefers_config_path_when_both_exist() {
        let test_dir = unique_test_dir("prefer-config");
        fs::create_dir_all(&test_dir).unwrap();
        let config_path = test_dir.join("config").join(OPTIONS_FILE_NAME);
        let legacy_path = test_dir.join("legacy").join(OPTIONS_FILE_NAME);
        fs::create_dir_all(config_path.parent().unwrap()).unwrap();
        fs::create_dir_all(legacy_path.parent().unwrap()).unwrap();
        fs::write(&config_path, "config").unwrap();
        fs::write(&legacy_path, "legacy").unwrap();

        let selected = select_load_options_path(&config_path, &legacy_path);

        assert_eq!(selected, config_path);
    }

    #[test]
    fn falls_back_to_legacy_path_when_config_missing() {
        let test_dir = unique_test_dir("fallback-legacy");
        fs::create_dir_all(&test_dir).unwrap();
        let config_path = test_dir.join("config").join(OPTIONS_FILE_NAME);
        let legacy_path = test_dir.join("legacy").join(OPTIONS_FILE_NAME);
        fs::create_dir_all(legacy_path.parent().unwrap()).unwrap();
        fs::write(&legacy_path, "legacy").unwrap();

        let selected = select_load_options_path(&config_path, &legacy_path);

        assert_eq!(selected, legacy_path);
    }

    #[test]
    fn returns_config_path_when_no_file_exists() {
        let test_dir = unique_test_dir("default-config");
        fs::create_dir_all(&test_dir).unwrap();
        let config_path = test_dir.join("config").join(OPTIONS_FILE_NAME);
        let legacy_path = test_dir.join("legacy").join(OPTIONS_FILE_NAME);

        let selected = select_load_options_path(&config_path, &legacy_path);

        assert_eq!(selected, config_path);
    }
}
