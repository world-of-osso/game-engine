use std::fs;
use std::path::{Path, PathBuf};
use std::thread;
use std::time::{Duration, Instant};

use bevy::dev_tools::fps_overlay::FpsOverlayConfig;
use bevy::prelude::*;
use bevy::window::{PresentMode, PrimaryWindow, Window};
use directories::ProjectDirs;
use game_engine::ui::render::UiCamera;
use serde::{Deserialize, Serialize};

use crate::cli_args::{RealmPreset, default_realm_preset};
use crate::sound::SoundSettings;
use game_engine::input_bindings::InputBindings;

const LEGACY_OPTIONS_PATH: &str = "data/ui/options_settings.ron";
const OPTIONS_FILE_NAME: &str = "options_settings.ron";
const LEGACY_CREDENTIALS_PATH: &str = "data/ui/credentials.ron";
const CREDENTIALS_FILE_NAME: &str = "credentials.ron";

#[path = "client_options_storage.rs"]
mod storage;

use storage::{
    CameraOptionsFile, ClientOptionsFile, GraphicsOptionsFile, HudOptionsFile, load_options_file,
};

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
                    sync_window_present_mode,
                ),
            )
            .add_systems(First, limit_frame_rate);
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
    #[default]
    Msaa4x,
    Taa,
}

#[derive(Resource, Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct GraphicsOptions {
    pub particle_density: u8,
    pub render_scale: f32,
    pub ui_scale: f32,
    pub vsync_enabled: bool,
    pub frame_rate_limit_enabled: bool,
    pub frame_rate_limit: u16,
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
            vsync_enabled: default_vsync_enabled(),
            frame_rate_limit_enabled: default_frame_rate_limit_enabled(),
            frame_rate_limit: default_frame_rate_limit(),
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
            vsync_enabled: file.vsync_enabled,
            frame_rate_limit_enabled: file.frame_rate_limit_enabled,
            frame_rate_limit: file
                .frame_rate_limit
                .clamp(MIN_FRAME_RATE_LIMIT, MAX_FRAME_RATE_LIMIT),
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

    pub fn present_mode(&self) -> PresentMode {
        if self.vsync_enabled {
            PresentMode::AutoVsync
        } else {
            PresentMode::AutoNoVsync
        }
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

const fn default_nameplate_distance() -> f32 {
    DEFAULT_NAMEPLATE_DISTANCE
}

const fn default_chat_font_size() -> f32 {
    10.0
}

const fn default_mouse_sensitivity() -> f32 {
    0.003
}

pub fn save_client_options(
    sound: Option<&SoundSettings>,
    camera: &CameraOptions,
    graphics: &GraphicsOptions,
    hud: &HudOptions,
    bindings: &InputBindings,
    modal_offset: [f32; 2],
) -> Result<(), String> {
    storage::save_client_options(sound, camera, graphics, hud, bindings, modal_offset)
}

pub fn save_client_options_values(
    sound: &SoundSettings,
    camera: &CameraOptions,
    graphics: &GraphicsOptions,
    hud: &HudOptions,
    bindings: &InputBindings,
    modal_offset: [f32; 2],
) -> Result<(), String> {
    storage::save_client_options_values(sound, camera, graphics, hud, bindings, modal_offset)
}

pub fn load_login_credentials() -> Option<LoginCredentials> {
    storage::load_login_credentials()
}

pub fn load_preferred_realm() -> RealmPreset {
    storage::load_preferred_realm()
}

pub fn load_eula_accepted() -> bool {
    storage::load_eula_accepted()
}

pub fn save_preferred_realm(realm: RealmPreset) -> Result<(), String> {
    storage::save_preferred_realm(realm)
}

pub fn save_eula_accepted(accepted: bool) -> Result<(), String> {
    storage::save_eula_accepted(accepted)
}

pub fn login_credentials_path() -> PathBuf {
    storage::login_credentials_path()
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

fn sync_window_present_mode(
    graphics: Res<GraphicsOptions>,
    mut primary_window: Query<&mut Window, With<PrimaryWindow>>,
) {
    if !graphics.is_changed() {
        return;
    }
    let Ok(mut window) = primary_window.single_mut() else {
        return;
    };
    let present_mode = graphics.present_mode();
    if window.present_mode != present_mode {
        window.present_mode = present_mode;
    }
}

#[derive(Default)]
struct FrameLimiterState {
    last_frame_started: Option<Instant>,
    last_interval: Option<Duration>,
}

fn limit_frame_rate(graphics: Res<GraphicsOptions>, mut state: Local<FrameLimiterState>) {
    let now = Instant::now();
    let next_interval =
        frame_limit_interval(graphics.frame_rate_limit_enabled, graphics.frame_rate_limit);
    if state.last_interval != next_interval {
        state.last_interval = next_interval;
        state.last_frame_started = Some(now);
        return;
    }
    let Some(target_interval) = next_interval else {
        state.last_frame_started = Some(now);
        return;
    };
    if let Some(last_frame_started) = state.last_frame_started {
        let elapsed = now.saturating_duration_since(last_frame_started);
        if elapsed < target_interval {
            thread::sleep(target_interval - elapsed);
        }
    }
    state.last_frame_started = Some(Instant::now());
}

fn frame_limit_interval(enabled: bool, frame_rate_limit: u16) -> Option<Duration> {
    if !enabled {
        return None;
    }
    let clamped_limit = frame_rate_limit.clamp(MIN_FRAME_RATE_LIMIT, MAX_FRAME_RATE_LIMIT);
    Some(Duration::from_secs_f64(1.0 / f64::from(clamped_limit)))
}

pub fn apply_fps_overlay_visibility(fps: &mut FpsOverlayConfig, visible: bool) {
    fps.enabled = visible;
    fps.frame_time_graph_config.enabled = visible;
}

#[cfg(test)]
#[path = "client_options_tests.rs"]
mod tests;
