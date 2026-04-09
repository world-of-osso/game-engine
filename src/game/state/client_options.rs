use std::fs;
use std::path::{Path, PathBuf};

use bevy::dev_tools::fps_overlay::FpsOverlayConfig;
use bevy::prelude::*;
use directories::ProjectDirs;
use game_engine::ui::render::UiCamera;
use serde::{Deserialize, Serialize};

use crate::sound::SoundSettings;
use game_engine::input_bindings::InputBindings;

const LEGACY_OPTIONS_PATH: &str = "data/ui/options_settings.ron";
const OPTIONS_FILE_NAME: &str = "options_settings.ron";
const LEGACY_CREDENTIALS_PATH: &str = "data/ui/credentials.ron";
const CREDENTIALS_FILE_NAME: &str = "credentials.ron";

pub struct ClientOptionsPlugin;

impl Plugin for ClientOptionsPlugin {
    fn build(&self, app: &mut App) {
        let loaded = load_options_file();
        app.insert_resource(loaded.sound.to_runtime())
            .insert_resource(CameraOptions::from_file(&loaded.camera))
            .insert_resource(GraphicsOptions::from_file(&loaded.graphics))
            .insert_resource(HudOptions::from_file(&loaded.hud))
            .insert_resource(HudVisibilityToggles::from_hud_options(
                &HudOptions::from_file(&loaded.hud),
            ))
            .insert_resource(loaded.bindings.clone())
            .insert_resource(ClientOptionsUiState {
                modal_offset: loaded.modal_offset,
                legacy_modal_position: loaded.modal_position,
            })
            .insert_resource(LoadedClientOptions {
                file: loaded,
                applied: false,
            })
            .add_systems(
                Update,
                (
                    apply_loaded_client_options,
                    sync_hud_visibility_toggles,
                    sync_ui_scale,
                ),
            );
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct LoginCredentials {
    pub username: String,
    pub password: String,
}

#[derive(Resource, Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct CameraOptions {
    pub mouse_sensitivity: f32,
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
            mouse_sensitivity: default_mouse_sensitivity(),
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
            mouse_sensitivity: file
                .mouse_sensitivity
                .clamp(MIN_MOUSE_SENSITIVITY, MAX_MOUSE_SENSITIVITY),
            look_sensitivity: file.look_sensitivity,
            invert_y: file.invert_y,
            follow_speed: file.follow_speed,
            zoom_speed: file.zoom_speed,
            min_distance: file.min_distance,
            max_distance: file.max_distance,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
pub enum AntiAliasMode {
    None,
    Msaa4x,
    #[default]
    Taa,
}

#[derive(Resource, Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct GraphicsOptions {
    pub particle_density: u8,
    pub render_scale: f32,
    pub ui_scale: f32,
    pub colorblind_mode: bool,
    pub bloom_enabled: bool,
    pub bloom_intensity: f32,
    pub depth_of_field: bool,
    pub anti_alias: AntiAliasMode,
}

impl Default for GraphicsOptions {
    fn default() -> Self {
        Self {
            particle_density: default_particle_density(),
            render_scale: default_render_scale(),
            ui_scale: default_ui_scale(),
            colorblind_mode: default_colorblind_mode(),
            bloom_enabled: default_bloom_enabled(),
            bloom_intensity: default_bloom_intensity(),
            depth_of_field: false,
            anti_alias: AntiAliasMode::default(),
        }
    }
}

impl GraphicsOptions {
    fn from_file(file: &GraphicsOptionsFile) -> Self {
        Self {
            particle_density: file.particle_density.clamp(10, 100),
            render_scale: file.render_scale.clamp(0.5, 1.0),
            ui_scale: file.ui_scale.clamp(MIN_UI_SCALE, MAX_UI_SCALE),
            colorblind_mode: file.colorblind_mode,
            bloom_enabled: file.bloom_enabled,
            bloom_intensity: file.bloom_intensity.clamp(0.0, 1.0),
            depth_of_field: false,
            anti_alias: AntiAliasMode::default(),
        }
    }

    pub fn particle_density_multiplier(&self) -> f32 {
        self.particle_density.clamp(10, 100) as f32 / 100.0
    }
}

#[derive(Resource, Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct HudOptions {
    pub show_minimap: bool,
    pub show_action_bars: bool,
    pub show_nameplates: bool,
    pub nameplate_distance: f32,
    pub show_health_bars: bool,
    pub show_target_marker: bool,
    pub show_fps_overlay: bool,
    pub chat_font_size: f32,
}

impl Default for HudOptions {
    fn default() -> Self {
        Self {
            show_minimap: true,
            show_action_bars: true,
            show_nameplates: true,
            nameplate_distance: default_nameplate_distance(),
            show_health_bars: true,
            show_target_marker: true,
            show_fps_overlay: true,
            chat_font_size: default_chat_font_size(),
        }
    }
}

impl HudOptions {
    fn from_file(file: &HudOptionsFile) -> Self {
        Self {
            show_minimap: file.show_minimap,
            show_action_bars: file.show_action_bars,
            show_nameplates: file.show_nameplates,
            nameplate_distance: file
                .nameplate_distance
                .clamp(MIN_NAMEPLATE_DISTANCE, MAX_NAMEPLATE_DISTANCE),
            show_health_bars: file.show_health_bars,
            show_target_marker: file.show_target_marker,
            show_fps_overlay: file.show_fps_overlay,
            chat_font_size: file
                .chat_font_size
                .clamp(MIN_CHAT_FONT_SIZE, MAX_CHAT_FONT_SIZE),
        }
    }
}

#[derive(Resource, Debug, Clone, PartialEq, Eq)]
pub struct HudVisibilityToggles {
    pub show_minimap: bool,
    pub show_action_bars: bool,
    pub show_player_frame: bool,
    pub show_target_frame: bool,
    pub show_nameplates: bool,
    pub show_health_bars: bool,
    pub show_fps_overlay: bool,
    pub show_target_marker: bool,
}

impl Default for HudVisibilityToggles {
    fn default() -> Self {
        Self::from_hud_options(&HudOptions::default())
    }
}

impl HudVisibilityToggles {
    pub fn from_hud_options(hud: &HudOptions) -> Self {
        Self {
            show_minimap: hud.show_minimap,
            show_action_bars: hud.show_action_bars,
            show_player_frame: hud.show_health_bars,
            show_target_frame: hud.show_health_bars,
            show_nameplates: hud.show_nameplates,
            show_health_bars: hud.show_health_bars,
            show_fps_overlay: hud.show_fps_overlay,
            show_target_marker: hud.show_target_marker,
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
    graphics: GraphicsOptionsFile,
    #[serde(default)]
    hud: HudOptionsFile,
    #[serde(default)]
    bindings: InputBindings,
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
            graphics: GraphicsOptionsFile::default(),
            hud: HudOptionsFile::default(),
            bindings: InputBindings::default(),
            modal_offset: None,
            modal_position: None,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct SoundOptionsFile {
    master_volume: f32,
    ambient_volume: f32,
    effects_volume: f32,
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
            ambient_volume: settings.ambient_volume,
            effects_volume: settings.effects_volume,
            music_volume: settings.music_volume,
            music_enabled: settings.music_enabled,
            muted: settings.muted,
        }
    }

    fn to_runtime(&self) -> SoundSettings {
        SoundSettings {
            master_volume: self.master_volume,
            ambient_volume: self.ambient_volume,
            effects_volume: self.effects_volume,
            music_volume: self.music_volume,
            music_enabled: self.music_enabled,
            muted: self.muted,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct CameraOptionsFile {
    #[serde(default = "default_mouse_sensitivity", rename = "mouseSensitivity")]
    mouse_sensitivity: f32,
    look_sensitivity: f32,
    invert_y: bool,
    follow_speed: f32,
    zoom_speed: f32,
    min_distance: f32,
    max_distance: f32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct GraphicsOptionsFile {
    #[serde(default = "default_particle_density", rename = "particleDensity")]
    particle_density: u8,
    #[serde(default = "default_render_scale", rename = "renderScale")]
    render_scale: f32,
    #[serde(default = "default_ui_scale", rename = "uiScale")]
    ui_scale: f32,
    #[serde(default = "default_colorblind_mode", rename = "colorblindMode")]
    colorblind_mode: bool,
    #[serde(default = "default_bloom_enabled", rename = "bloomEnabled")]
    bloom_enabled: bool,
    #[serde(default = "default_bloom_intensity", rename = "bloomIntensity")]
    bloom_intensity: f32,
}

impl Default for GraphicsOptionsFile {
    fn default() -> Self {
        Self {
            particle_density: default_particle_density(),
            render_scale: default_render_scale(),
            ui_scale: default_ui_scale(),
            colorblind_mode: default_colorblind_mode(),
            bloom_enabled: default_bloom_enabled(),
            bloom_intensity: default_bloom_intensity(),
        }
    }
}

impl Default for CameraOptionsFile {
    fn default() -> Self {
        let defaults = CameraOptions::default();
        Self {
            mouse_sensitivity: defaults.mouse_sensitivity,
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
    #[serde(default = "default_nameplate_distance", rename = "nameplateDistance")]
    nameplate_distance: f32,
    show_health_bars: bool,
    show_target_marker: bool,
    show_fps_overlay: bool,
    #[serde(default = "default_chat_font_size", rename = "chatFontSize")]
    chat_font_size: f32,
}

impl Default for HudOptionsFile {
    fn default() -> Self {
        let defaults = HudOptions::default();
        Self {
            show_minimap: defaults.show_minimap,
            show_action_bars: defaults.show_action_bars,
            show_nameplates: defaults.show_nameplates,
            nameplate_distance: defaults.nameplate_distance,
            show_health_bars: defaults.show_health_bars,
            show_target_marker: defaults.show_target_marker,
            show_fps_overlay: defaults.show_fps_overlay,
            chat_font_size: defaults.chat_font_size,
        }
    }
}

pub fn save_client_options(
    sound: Option<&SoundSettings>,
    camera: &CameraOptions,
    graphics: &GraphicsOptions,
    hud: &HudOptions,
    bindings: &InputBindings,
    modal_offset: [f32; 2],
) -> Result<(), String> {
    let file = build_options_file(sound, camera, graphics, hud, bindings, modal_offset);
    let path = options_path();
    save_options_file_to_path(&path, &file)
}

pub fn save_client_options_values(
    sound: &SoundSettings,
    camera: &CameraOptions,
    graphics: &GraphicsOptions,
    hud: &HudOptions,
    bindings: &InputBindings,
    modal_offset: [f32; 2],
) -> Result<(), String> {
    save_client_options(Some(sound), camera, graphics, hud, bindings, modal_offset)
}

fn build_options_file(
    sound: Option<&SoundSettings>,
    camera: &CameraOptions,
    graphics: &GraphicsOptions,
    hud: &HudOptions,
    bindings: &InputBindings,
    modal_offset: [f32; 2],
) -> ClientOptionsFile {
    ClientOptionsFile {
        sound: build_sound_options_file(sound),
        camera: build_camera_options_file(camera),
        graphics: build_graphics_options_file(graphics),
        hud: build_hud_options_file(hud),
        bindings: bindings.clone(),
        modal_offset: Some(modal_offset),
        modal_position: None,
    }
}

fn build_sound_options_file(sound: Option<&SoundSettings>) -> SoundOptionsFile {
    sound
        .map(SoundOptionsFile::from_runtime)
        .unwrap_or_default()
}

fn build_camera_options_file(camera: &CameraOptions) -> CameraOptionsFile {
    CameraOptionsFile {
        mouse_sensitivity: camera
            .mouse_sensitivity
            .clamp(MIN_MOUSE_SENSITIVITY, MAX_MOUSE_SENSITIVITY),
        look_sensitivity: camera.look_sensitivity,
        invert_y: camera.invert_y,
        follow_speed: camera.follow_speed,
        zoom_speed: camera.zoom_speed,
        min_distance: camera.min_distance,
        max_distance: camera.max_distance,
    }
}

fn build_graphics_options_file(graphics: &GraphicsOptions) -> GraphicsOptionsFile {
    GraphicsOptionsFile {
        particle_density: graphics.particle_density.clamp(10, 100),
        render_scale: graphics.render_scale.clamp(0.5, 1.0),
        ui_scale: graphics.ui_scale.clamp(MIN_UI_SCALE, MAX_UI_SCALE),
        colorblind_mode: graphics.colorblind_mode,
        bloom_enabled: graphics.bloom_enabled,
        bloom_intensity: graphics.bloom_intensity.clamp(0.0, 1.0),
    }
}

fn build_hud_options_file(hud: &HudOptions) -> HudOptionsFile {
    HudOptionsFile {
        show_minimap: hud.show_minimap,
        show_action_bars: hud.show_action_bars,
        show_nameplates: hud.show_nameplates,
        nameplate_distance: hud
            .nameplate_distance
            .clamp(MIN_NAMEPLATE_DISTANCE, MAX_NAMEPLATE_DISTANCE),
        show_health_bars: hud.show_health_bars,
        show_target_marker: hud.show_target_marker,
        show_fps_overlay: hud.show_fps_overlay,
        chat_font_size: hud
            .chat_font_size
            .clamp(MIN_CHAT_FONT_SIZE, MAX_CHAT_FONT_SIZE),
    }
}

fn save_options_file_to_path(path: &Path, file: &ClientOptionsFile) -> Result<(), String> {
    let pretty = ron::ser::PrettyConfig::new();
    let serialized = ron::ser::to_string_pretty(file, pretty)
        .map_err(|err| format!("failed to serialize client options: {err}"))?;
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)
            .map_err(|err| format!("failed to create options dir {}: {err}", parent.display()))?;
    }
    info!("Saving client options to {}", path.display());
    fs::write(path, serialized)
        .map_err(|err| format!("failed to write client options {}: {err}", path.display()))
}

const fn default_particle_density() -> u8 {
    100
}

const fn default_render_scale() -> f32 {
    1.0
}

const fn default_ui_scale() -> f32 {
    1.0
}

const fn default_colorblind_mode() -> bool {
    false
}

const fn default_bloom_enabled() -> bool {
    false
}

const fn default_bloom_intensity() -> f32 {
    0.08
}

pub const MIN_UI_SCALE: f32 = 0.75;
pub const MAX_UI_SCALE: f32 = 1.5;
pub const MIN_MOUSE_SENSITIVITY: f32 = 0.001;
pub const MAX_MOUSE_SENSITIVITY: f32 = 0.01;
pub const MIN_NAMEPLATE_DISTANCE: f32 = 20.0;
pub const MAX_NAMEPLATE_DISTANCE: f32 = 80.0;
pub const DEFAULT_NAMEPLATE_DISTANCE: f32 = 40.0;
pub const MIN_CHAT_FONT_SIZE: f32 = 8.0;
pub const MAX_CHAT_FONT_SIZE: f32 = 16.0;

const fn default_nameplate_distance() -> f32 {
    DEFAULT_NAMEPLATE_DISTANCE
}

const fn default_chat_font_size() -> f32 {
    10.0
}

const fn default_mouse_sensitivity() -> f32 {
    0.003
}

fn load_options_file() -> ClientOptionsFile {
    let path = load_options_path();
    load_options_file_from_path(&path)
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

fn load_options_file_from_path(path: &Path) -> ClientOptionsFile {
    if !path.exists() {
        info!(
            "No client options file found at {}; using defaults",
            path.display()
        );
        return ClientOptionsFile::default();
    }

    info!("Loading client options from {}", path.display());
    let Ok(raw) = fs::read_to_string(path) else {
        return ClientOptionsFile::default();
    };
    ron::de::from_str::<ClientOptionsFile>(&raw).unwrap_or_default()
}

fn apply_loaded_client_options(
    mut loaded: ResMut<LoadedClientOptions>,
    mut fps: Option<ResMut<FpsOverlayConfig>>,
) {
    if loaded.applied {
        return;
    }
    if let Some(fps) = fps.as_mut() {
        apply_fps_overlay_visibility(fps.as_mut(), loaded.file.hud.show_fps_overlay);
    }
    loaded.applied = true;
}

fn sync_hud_visibility_toggles(
    hud: Res<HudOptions>,
    mut toggles: ResMut<HudVisibilityToggles>,
    mut fps: Option<ResMut<FpsOverlayConfig>>,
) {
    if !hud.is_changed() {
        return;
    }
    let next = HudVisibilityToggles::from_hud_options(&hud);
    if *toggles != next {
        *toggles = next.clone();
    }
    if let Some(fps) = fps.as_mut() {
        apply_fps_overlay_visibility(fps.as_mut(), next.show_fps_overlay);
    }
}

fn sync_ui_scale(
    graphics: Res<GraphicsOptions>,
    mut ui_camera: Query<&mut Projection, With<UiCamera>>,
) {
    if !graphics.is_changed() {
        return;
    }
    let Ok(mut projection) = ui_camera.single_mut() else {
        return;
    };
    let Projection::Orthographic(orthographic) = projection.as_mut() else {
        return;
    };
    orthographic.scale = 1.0 / graphics.ui_scale.clamp(MIN_UI_SCALE, MAX_UI_SCALE);
}

pub fn apply_fps_overlay_visibility(fps: &mut FpsOverlayConfig, visible: bool) {
    fps.enabled = visible;
    fps.frame_time_graph_config.enabled = visible;
}

#[cfg(test)]
mod tests {
    use super::*;
    use game_engine::input_bindings::{InputAction, InputBinding};
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
        assert_eq!(file.bindings, InputBindings::default());
    }

    #[test]
    fn hud_defaults_are_visible() {
        let defaults = HudOptions::default();
        assert!(defaults.show_minimap);
        assert!(defaults.show_action_bars);
        assert!(defaults.show_target_marker);
        assert!((defaults.nameplate_distance - DEFAULT_NAMEPLATE_DISTANCE).abs() < 0.0001);
        assert!((defaults.chat_font_size - 10.0).abs() < 0.0001);
    }

    #[test]
    fn hud_visibility_toggles_follow_hud_options() {
        let toggles = HudVisibilityToggles::from_hud_options(&HudOptions {
            show_minimap: false,
            show_action_bars: true,
            show_nameplates: false,
            nameplate_distance: default_nameplate_distance(),
            show_health_bars: false,
            show_target_marker: true,
            show_fps_overlay: false,
            chat_font_size: default_chat_font_size(),
        });
        assert!(!toggles.show_minimap);
        assert!(toggles.show_action_bars);
        assert!(!toggles.show_player_frame);
        assert!(!toggles.show_target_frame);
        assert!(!toggles.show_nameplates);
        assert!(!toggles.show_health_bars);
        assert!(toggles.show_target_marker);
        assert!(!toggles.show_fps_overlay);
    }

    #[test]
    fn graphics_defaults_use_full_particle_density() {
        let defaults = GraphicsOptions::default();
        assert_eq!(defaults.particle_density, 100);
        assert!((defaults.render_scale - 1.0).abs() < 0.0001);
        assert!((defaults.ui_scale - 1.0).abs() < 0.0001);
        assert!(!defaults.colorblind_mode);
        assert!(!defaults.bloom_enabled);
        assert!((defaults.bloom_intensity - 0.08).abs() < 0.0001);
        assert!((defaults.particle_density_multiplier() - 1.0).abs() < 0.0001);
    }

    #[test]
    fn options_file_serializes_particle_density_with_cvar_name() {
        let file = ClientOptionsFile {
            graphics: GraphicsOptionsFile {
                particle_density: 80,
                render_scale: 0.67,
                ui_scale: 1.2,
                colorblind_mode: false,
                bloom_enabled: false,
                bloom_intensity: 0.12,
            },
            ..ClientOptionsFile::default()
        };

        let serialized = ron::ser::to_string(&file).unwrap();

        assert!(serialized.contains("particleDensity:80"));
        assert!(serialized.contains("renderScale:0.67"));
        assert!(serialized.contains("uiScale:1.2"));
        assert!(serialized.contains("colorblindMode:false"));
        assert!(serialized.contains("bloomEnabled:false"));
        assert!(serialized.contains("bloomIntensity:0.12"));
    }

    #[test]
    fn camera_defaults_include_mouse_sensitivity() {
        let defaults = CameraOptions::default();
        assert!((defaults.mouse_sensitivity - default_mouse_sensitivity()).abs() < 0.0001);
    }

    #[test]
    fn options_file_round_trips_target_nearest_binding() {
        let mut bindings = InputBindings::default();
        bindings.assign(
            InputAction::TargetNearest,
            InputBinding::Keyboard(KeyCode::F2),
        );

        let file = ClientOptionsFile {
            bindings: bindings.clone(),
            ..ClientOptionsFile::default()
        };

        let serialized = ron::ser::to_string(&file).unwrap();
        let parsed: ClientOptionsFile = ron::de::from_str(&serialized).unwrap();

        assert_eq!(
            parsed.bindings.binding(InputAction::TargetNearest),
            Some(InputBinding::Keyboard(KeyCode::F2))
        );
        assert!(serialized.contains("TargetNearest"));
        assert!(serialized.contains("key:F2"));
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

    #[test]
    fn save_options_file_to_path_persists_and_loads_back() {
        let test_dir = unique_test_dir("persist-options");
        let path = test_dir.join("config").join(OPTIONS_FILE_NAME);
        let mut bindings = InputBindings::default();
        bindings.assign(
            InputAction::TargetNearest,
            InputBinding::Keyboard(KeyCode::F3),
        );
        let file = ClientOptionsFile {
            sound: SoundOptionsFile {
                master_volume: 0.25,
                ambient_volume: 0.5,
                effects_volume: 0.75,
                music_volume: 0.9,
                music_enabled: false,
                muted: true,
            },
            camera: CameraOptionsFile {
                mouse_sensitivity: 0.006,
                look_sensitivity: 0.02,
                invert_y: true,
                follow_speed: 6.0,
                zoom_speed: 3.0,
                min_distance: 4.0,
                max_distance: 28.0,
            },
            graphics: GraphicsOptionsFile {
                particle_density: 60,
                render_scale: 0.8,
                ui_scale: 1.3,
                colorblind_mode: true,
                bloom_enabled: true,
                bloom_intensity: 0.2,
            },
            hud: HudOptionsFile {
                show_minimap: false,
                show_action_bars: true,
                show_nameplates: false,
                nameplate_distance: 60.0,
                show_health_bars: true,
                show_target_marker: false,
                show_fps_overlay: false,
                chat_font_size: 13.0,
            },
            bindings: bindings.clone(),
            modal_offset: Some([123.0, -45.0]),
            modal_position: None,
        };

        save_options_file_to_path(&path, &file).unwrap();
        let loaded = load_options_file_from_path(&path);

        assert_eq!(loaded.sound.master_volume, 0.25);
        assert!(!loaded.sound.music_enabled);
        assert!((loaded.camera.mouse_sensitivity - 0.006).abs() < 0.0001);
        assert!(loaded.camera.invert_y);
        assert_eq!(loaded.graphics.particle_density, 60);
        assert!((loaded.graphics.ui_scale - 1.3).abs() < 0.0001);
        assert!(loaded.graphics.colorblind_mode);
        assert!(!loaded.hud.show_minimap);
        assert!((loaded.hud.nameplate_distance - 60.0).abs() < 0.0001);
        assert!((loaded.hud.chat_font_size - 13.0).abs() < 0.0001);
        assert_eq!(loaded.modal_offset, Some([123.0, -45.0]));
        assert_eq!(
            loaded.bindings.binding(InputAction::TargetNearest),
            Some(InputBinding::Keyboard(KeyCode::F3))
        );
    }

    #[test]
    fn graphics_options_file_clamps_ui_scale_range() {
        let low = GraphicsOptionsFile {
            ui_scale: 0.1,
            ..GraphicsOptionsFile::default()
        };
        let high = GraphicsOptionsFile {
            ui_scale: 5.0,
            ..GraphicsOptionsFile::default()
        };

        assert!((GraphicsOptions::from_file(&low).ui_scale - MIN_UI_SCALE).abs() < 0.0001);
        assert!((GraphicsOptions::from_file(&high).ui_scale - MAX_UI_SCALE).abs() < 0.0001);
    }

    #[test]
    fn camera_options_file_clamps_mouse_sensitivity_range() {
        let low = CameraOptionsFile {
            mouse_sensitivity: 0.0,
            ..CameraOptionsFile::default()
        };
        let high = CameraOptionsFile {
            mouse_sensitivity: 99.0,
            ..CameraOptionsFile::default()
        };

        assert!(
            (CameraOptions::from_file(&low).mouse_sensitivity - MIN_MOUSE_SENSITIVITY).abs()
                < 0.0001
        );
        assert!(
            (CameraOptions::from_file(&high).mouse_sensitivity - MAX_MOUSE_SENSITIVITY).abs()
                < 0.0001
        );
    }

    #[test]
    fn hud_options_file_clamps_chat_font_size_range() {
        let low = HudOptionsFile {
            chat_font_size: 1.0,
            ..HudOptionsFile::default()
        };
        let high = HudOptionsFile {
            chat_font_size: 99.0,
            ..HudOptionsFile::default()
        };

        assert!((HudOptions::from_file(&low).chat_font_size - MIN_CHAT_FONT_SIZE).abs() < 0.0001);
        assert!((HudOptions::from_file(&high).chat_font_size - MAX_CHAT_FONT_SIZE).abs() < 0.0001);
    }

    #[test]
    fn hud_options_file_clamps_nameplate_distance_range() {
        let low = HudOptionsFile {
            nameplate_distance: 1.0,
            ..HudOptionsFile::default()
        };
        let high = HudOptionsFile {
            nameplate_distance: 999.0,
            ..HudOptionsFile::default()
        };

        assert!(
            (HudOptions::from_file(&low).nameplate_distance - MIN_NAMEPLATE_DISTANCE).abs()
                < 0.0001
        );
        assert!(
            (HudOptions::from_file(&high).nameplate_distance - MAX_NAMEPLATE_DISTANCE).abs()
                < 0.0001
        );
    }

    #[test]
    fn sync_ui_scale_updates_ui_camera_projection() {
        let mut app = App::new();
        app.add_plugins(MinimalPlugins);
        app.insert_resource(GraphicsOptions {
            ui_scale: 1.25,
            ..GraphicsOptions::default()
        });
        app.add_systems(Update, sync_ui_scale);
        app.world_mut().spawn((
            UiCamera,
            Projection::Orthographic(OrthographicProjection::default_2d()),
        ));

        app.update();

        let mut query = app
            .world_mut()
            .query_filtered::<&Projection, With<UiCamera>>();
        let projection = query.single(app.world()).expect("ui camera projection");
        let Projection::Orthographic(orthographic) = projection else {
            panic!("expected orthographic ui camera");
        };
        assert!((orthographic.scale - 0.8).abs() < 0.0001);
    }
}
