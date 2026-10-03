use crate::camera_control_data::{
    DEFAULT_CAMERA_FOV_DEGREES, MAX_CAMERA_FOV_DEGREES, MIN_CAMERA_FOV_DEGREES,
};
use crate::input_bindings_data::InputBindingsData;
use crate::nameplate_style_data::NameplateStyle;
use crate::realm_preset_data::{RealmPreset, default_realm_preset};
use crate::soft_target_data::SoftTargetOptions;
use crate::status_text_data::StatusTextDisplay;
use serde::{Deserialize, Serialize};
use std::{
    fs,
    path::{Path, PathBuf},
};

const OPTIONS_FILE_NAME: &str = "options_settings.ron";
const LEGACY_OPTIONS_PATH: &str = "data/ui/options_settings.ron";
const CREDENTIALS_FILE_NAME: &str = "credentials.ron";
pub const MIN_UI_SCALE: f32 = 0.75;
pub const MAX_UI_SCALE: f32 = 1.5;
pub const MIN_MOUSE_SENSITIVITY: f32 = 0.001;
pub const MAX_MOUSE_SENSITIVITY: f32 = 0.01;
pub const MIN_FRAME_RATE_LIMIT: u16 = 30;
pub const MAX_FRAME_RATE_LIMIT: u16 = 240;
pub const MIN_NAMEPLATE_DISTANCE: f32 = 20.0;
pub const MAX_NAMEPLATE_DISTANCE: f32 = 80.0;
pub const DEFAULT_NAMEPLATE_DISTANCE: f32 = 40.0;
pub const MIN_CHAT_FONT_SIZE: f32 = 8.0;
pub const MAX_CHAT_FONT_SIZE: f32 = 16.0;

const fn default_mouse_sensitivity() -> f32 {
    0.003
}
const fn default_camera_fov_degrees() -> f32 {
    DEFAULT_CAMERA_FOV_DEGREES
}
const fn default_particle_effects_enabled() -> bool {
    true
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
const fn default_vsync_enabled() -> bool {
    true
}
const fn default_frame_rate_limit_enabled() -> bool {
    false
}
const fn default_frame_rate_limit() -> u16 {
    144
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
const fn default_nameplate_distance() -> f32 {
    DEFAULT_NAMEPLATE_DISTANCE
}
const fn default_chat_font_size() -> f32 {
    10.0
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
pub enum AntiAliasMode {
    None,
    #[default]
    Msaa4x,
    Taa,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ClientOptionsFile {
    #[serde(default)]
    pub accepted_eula: bool,
    #[serde(default = "default_realm_preset", rename = "preferredRealm")]
    pub preferred_realm: RealmPreset,
    #[serde(default)]
    pub sound: SoundOptionsFile,
    #[serde(default)]
    pub camera: CameraOptionsFile,
    #[serde(default)]
    pub graphics: GraphicsOptionsFile,
    #[serde(default)]
    pub hud: HudOptionsFile,
    #[serde(default)]
    pub bindings: InputBindingsData,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub modal_offset: Option<[f32; 2]>,
    #[serde(default)]
    #[serde(rename = "modal_position", skip_serializing_if = "Option::is_none")]
    pub modal_position: Option<[f32; 2]>,
}

impl Default for ClientOptionsFile {
    fn default() -> Self {
        Self {
            accepted_eula: false,
            preferred_realm: default_realm_preset(),
            sound: SoundOptionsFile::default(),
            camera: CameraOptionsFile::default(),
            graphics: GraphicsOptionsFile::default(),
            hud: HudOptionsFile::default(),
            bindings: InputBindingsData::default(),
            modal_offset: None,
            modal_position: None,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SoundOptionsFile {
    pub master_volume: f32,
    pub ambient_volume: f32,
    pub effects_volume: f32,
    pub music_volume: f32,
    pub music_enabled: bool,
    pub muted: bool,
}

impl Default for SoundOptionsFile {
    fn default() -> Self {
        Self {
            master_volume: 1.0,
            ambient_volume: 0.3,
            effects_volume: 0.8,
            music_volume: 0.45,
            music_enabled: true,
            muted: false,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CameraOptionsFile {
    #[serde(default = "default_mouse_sensitivity", rename = "mouseSensitivity")]
    pub mouse_sensitivity: f32,
    pub look_sensitivity: f32,
    pub invert_y: bool,
    #[serde(default = "default_camera_fov_degrees", rename = "fovDegrees")]
    pub fov_degrees: f32,
    pub follow_speed: f32,
    pub zoom_speed: f32,
    pub min_distance: f32,
    pub max_distance: f32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GraphicsOptionsFile {
    #[serde(
        default = "default_particle_effects_enabled",
        rename = "particleEffectsEnabled"
    )]
    pub particle_effects_enabled: bool,
    #[serde(default, rename = "ssaoEnabled")]
    pub ssao_enabled: bool,
    #[serde(default, rename = "depthOfField")]
    pub depth_of_field: bool,
    #[serde(default, rename = "antiAlias")]
    pub anti_alias: AntiAliasMode,
    #[serde(default = "default_particle_density", rename = "particleDensity")]
    pub particle_density: u8,
    #[serde(default = "default_render_scale", rename = "renderScale")]
    pub render_scale: f32,
    #[serde(default = "default_ui_scale", rename = "uiScale")]
    pub ui_scale: f32,
    #[serde(default = "default_vsync_enabled", rename = "vsyncEnabled")]
    pub vsync_enabled: bool,
    #[serde(
        default = "default_frame_rate_limit_enabled",
        rename = "frameRateLimitEnabled"
    )]
    pub frame_rate_limit_enabled: bool,
    #[serde(default = "default_frame_rate_limit", rename = "frameRateLimit")]
    pub frame_rate_limit: u16,
    #[serde(default = "default_colorblind_mode", rename = "colorblindMode")]
    pub colorblind_mode: bool,
    #[serde(default = "default_bloom_enabled", rename = "bloomEnabled")]
    pub bloom_enabled: bool,
    #[serde(default = "default_bloom_intensity", rename = "bloomIntensity")]
    pub bloom_intensity: f32,
}

impl Default for GraphicsOptionsFile {
    fn default() -> Self {
        Self {
            particle_effects_enabled: default_particle_effects_enabled(),
            ssao_enabled: false,
            depth_of_field: false,
            anti_alias: AntiAliasMode::default(),
            particle_density: default_particle_density(),
            render_scale: default_render_scale(),
            ui_scale: default_ui_scale(),
            vsync_enabled: default_vsync_enabled(),
            frame_rate_limit_enabled: default_frame_rate_limit_enabled(),
            frame_rate_limit: default_frame_rate_limit(),
            colorblind_mode: default_colorblind_mode(),
            bloom_enabled: default_bloom_enabled(),
            bloom_intensity: default_bloom_intensity(),
        }
    }
}

impl Default for CameraOptionsFile {
    fn default() -> Self {
        Self {
            mouse_sensitivity: default_mouse_sensitivity(),
            look_sensitivity: 0.01,
            invert_y: false,
            fov_degrees: DEFAULT_CAMERA_FOV_DEGREES,
            follow_speed: 10.0,
            zoom_speed: 8.0,
            min_distance: 2.0,
            max_distance: 40.0,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HudOptionsFile {
    pub show_minimap: bool,
    pub show_action_bars: bool,
    pub show_nameplates: bool,
    #[serde(default = "default_nameplate_distance", rename = "nameplateDistance")]
    pub nameplate_distance: f32,
    #[serde(default, rename = "nameplateStyle")]
    pub nameplate_style: NameplateStyle,
    pub show_health_bars: bool,
    pub show_target_marker: bool,
    #[serde(default, rename = "autoLoot")]
    pub auto_loot: bool,
    /// Retail `softTargetInteract` keyboard bit: Controls "Enable Interact Key"
    /// (Controls.lua:72-86), off by default (`SoftTargetInteract` 1, gamepad only).
    #[serde(default, rename = "softTargetInteract")]
    pub soft_target_interact: bool,
    /// The other soft interact CVars.
    #[serde(default, rename = "softTarget")]
    pub soft_target: SoftTargetOptions,
    pub show_fps_overlay: bool,
    #[serde(default = "default_chat_font_size", rename = "chatFontSize")]
    pub chat_font_size: f32,
    /// Interface "Status Text" (`statusTextDisplay`).
    #[serde(default, rename = "statusTextDisplay")]
    pub status_text_display: StatusTextDisplay,
}

impl Default for HudOptionsFile {
    fn default() -> Self {
        Self {
            show_minimap: true,
            show_action_bars: true,
            show_nameplates: true,
            nameplate_distance: default_nameplate_distance(),
            nameplate_style: NameplateStyle::default(),
            show_health_bars: true,
            show_target_marker: true,
            auto_loot: false,
            soft_target_interact: false,
            soft_target: SoftTargetOptions::default(),
            show_fps_overlay: false,
            chat_font_size: default_chat_font_size(),
            status_text_display: StatusTextDisplay::default(),
        }
    }
}

impl CameraOptionsFile {
    pub fn clamped(mut self) -> Self {
        self.mouse_sensitivity = self
            .mouse_sensitivity
            .clamp(MIN_MOUSE_SENSITIVITY, MAX_MOUSE_SENSITIVITY);
        self.fov_degrees = self
            .fov_degrees
            .clamp(MIN_CAMERA_FOV_DEGREES, MAX_CAMERA_FOV_DEGREES);
        self
    }
}
impl GraphicsOptionsFile {
    pub fn clamped(mut self) -> Self {
        self.particle_density = self.particle_density.clamp(10, 100);
        self.render_scale = self.render_scale.clamp(0.5, 1.0);
        self.ui_scale = self.ui_scale.clamp(MIN_UI_SCALE, MAX_UI_SCALE);
        self.frame_rate_limit = self
            .frame_rate_limit
            .clamp(MIN_FRAME_RATE_LIMIT, MAX_FRAME_RATE_LIMIT);
        self.bloom_intensity = self.bloom_intensity.clamp(0.0, 1.0);
        self
    }
    pub fn validate(&self) -> Result<(), String> {
        if self.ssao_enabled && self.anti_alias == AntiAliasMode::Msaa4x {
            return Err("SSAO (ssaoEnabled) cannot be enabled with antiAlias: Msaa4x; use None or Taa, or set ssaoEnabled: false".to_string());
        }
        Ok(())
    }
}

impl HudOptionsFile {
    pub fn clamped(mut self) -> Self {
        self.nameplate_distance = self
            .nameplate_distance
            .clamp(MIN_NAMEPLATE_DISTANCE, MAX_NAMEPLATE_DISTANCE);
        self.nameplate_style = self.nameplate_style.clamped();
        self.chat_font_size = self
            .chat_font_size
            .clamp(MIN_CHAT_FONT_SIZE, MAX_CHAT_FONT_SIZE);
        self
    }
}
impl ClientOptionsFile {
    pub fn validate(&self) -> Result<(), String> {
        self.graphics.validate()?;
        self.hud.soft_target.validate()
    }
    pub fn clamped(mut self) -> Self {
        self.camera = self.camera.clamped();
        self.graphics = self.graphics.clamped();
        self.hud = self.hud.clamped();
        self
    }
}
pub fn options_path() -> PathBuf {
    world_of_osso_config_dir().join(OPTIONS_FILE_NAME)
}
/// Saved account used to prefill development-realm login.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct LoginCredentials {
    pub username: String,
    pub password: String,
}

pub fn login_credentials_path() -> PathBuf {
    world_of_osso_config_dir().join(CREDENTIALS_FILE_NAME)
}

pub fn load_login_credentials() -> Option<LoginCredentials> {
    let raw = fs::read_to_string(login_credentials_path()).ok()?;
    let creds = ron::de::from_str::<LoginCredentials>(&raw).ok()?;
    if creds.username.trim().is_empty() || creds.password.trim().is_empty() {
        return None;
    }
    Some(creds)
}

pub(crate) fn world_of_osso_config_dir() -> PathBuf {
    directories::BaseDirs::new()
        .map(|dirs| dirs.config_dir().to_path_buf())
        .unwrap_or_else(|| PathBuf::from("."))
        .join("world-of-osso")
}
pub fn select_load_options_path(config_path: &Path, legacy_path: &Path) -> PathBuf {
    if config_path.exists() {
        config_path.to_path_buf()
    } else if legacy_path.exists() {
        legacy_path.to_path_buf()
    } else {
        config_path.to_path_buf()
    }
}
pub fn load_options_file_with_legacy(legacy_path: &Path) -> ClientOptionsFile {
    let path = select_load_options_path(&options_path(), legacy_path);
    load_options_file_from_path(&path)
}
pub fn load_options_file() -> ClientOptionsFile {
    load_options_file_with_legacy(Path::new(LEGACY_OPTIONS_PATH))
}
pub fn load_options_file_from_path(path: &Path) -> ClientOptionsFile {
    if !path.exists() {
        return ClientOptionsFile::default();
    }
    let raw = fs::read_to_string(path).unwrap_or_else(|error| {
        panic!("failed to read client options {}: {error}", path.display())
    });
    let file: ClientOptionsFile = ron::de::from_str(&raw)
        .unwrap_or_else(|error| panic!("invalid client options {}: {error}", path.display()));
    file.validate()
        .unwrap_or_else(|error| panic!("invalid client options {}: {error}", path.display()));
    file
}
pub fn save_options_file_to_path(path: &Path, file: &ClientOptionsFile) -> Result<(), String> {
    file.validate()?;
    let serialized = ron::ser::to_string_pretty(file, ron::ser::PrettyConfig::new())
        .map_err(|err| format!("failed to serialize client options: {err}"))?;
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)
            .map_err(|err| format!("failed to create options dir {}: {err}", parent.display()))?;
    }
    fs::write(path, serialized)
        .map_err(|err| format!("failed to write client options {}: {err}", path.display()))
}
