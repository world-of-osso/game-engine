use super::*;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub(super) struct ClientOptionsFile {
    #[serde(default)]
    pub(super) accepted_eula: bool,
    #[serde(default = "default_realm_preset", rename = "preferredRealm")]
    pub(super) preferred_realm: RealmPreset,
    #[serde(default)]
    pub(super) sound: SoundOptionsFile,
    #[serde(default)]
    pub(super) camera: CameraOptionsFile,
    #[serde(default)]
    pub(super) graphics: GraphicsOptionsFile,
    #[serde(default)]
    pub(super) hud: HudOptionsFile,
    #[serde(default)]
    pub(super) bindings: InputBindings,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub(super) modal_offset: Option<[f32; 2]>,
    #[serde(default)]
    #[serde(rename = "modal_position", skip_serializing_if = "Option::is_none")]
    pub(super) modal_position: Option<[f32; 2]>,
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
            bindings: InputBindings::default(),
            modal_offset: None,
            modal_position: None,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub(super) struct SoundOptionsFile {
    pub(super) master_volume: f32,
    pub(super) ambient_volume: f32,
    pub(super) effects_volume: f32,
    pub(super) music_volume: f32,
    pub(super) music_enabled: bool,
    pub(super) muted: bool,
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

    pub(super) fn to_runtime(&self) -> SoundSettings {
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
pub(super) struct CameraOptionsFile {
    #[serde(default = "default_mouse_sensitivity", rename = "mouseSensitivity")]
    pub(super) mouse_sensitivity: f32,
    pub(super) look_sensitivity: f32,
    pub(super) invert_y: bool,
    pub(super) follow_speed: f32,
    pub(super) zoom_speed: f32,
    pub(super) min_distance: f32,
    pub(super) max_distance: f32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub(super) struct GraphicsOptionsFile {
    #[serde(default = "default_particle_density", rename = "particleDensity")]
    pub(super) particle_density: u8,
    #[serde(default = "default_render_scale", rename = "renderScale")]
    pub(super) render_scale: f32,
    #[serde(default = "default_ui_scale", rename = "uiScale")]
    pub(super) ui_scale: f32,
    #[serde(default = "default_vsync_enabled", rename = "vsyncEnabled")]
    pub(super) vsync_enabled: bool,
    #[serde(
        default = "default_frame_rate_limit_enabled",
        rename = "frameRateLimitEnabled"
    )]
    pub(super) frame_rate_limit_enabled: bool,
    #[serde(default = "default_frame_rate_limit", rename = "frameRateLimit")]
    pub(super) frame_rate_limit: u16,
    #[serde(default = "default_colorblind_mode", rename = "colorblindMode")]
    pub(super) colorblind_mode: bool,
    #[serde(default = "default_bloom_enabled", rename = "bloomEnabled")]
    pub(super) bloom_enabled: bool,
    #[serde(default = "default_bloom_intensity", rename = "bloomIntensity")]
    pub(super) bloom_intensity: f32,
}

impl Default for GraphicsOptionsFile {
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
pub(super) struct HudOptionsFile {
    pub(super) show_minimap: bool,
    pub(super) show_action_bars: bool,
    pub(super) show_nameplates: bool,
    #[serde(default = "default_nameplate_distance", rename = "nameplateDistance")]
    pub(super) nameplate_distance: f32,
    pub(super) show_health_bars: bool,
    pub(super) show_target_marker: bool,
    pub(super) show_fps_overlay: bool,
    #[serde(default = "default_chat_font_size", rename = "chatFontSize")]
    pub(super) chat_font_size: f32,
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
    let existing = load_options_file();
    build_options_file_from_existing(
        &existing,
        sound,
        camera,
        graphics,
        hud,
        bindings,
        modal_offset,
    )
}

pub(super) fn build_options_file_from_existing(
    existing: &ClientOptionsFile,
    sound: Option<&SoundSettings>,
    camera: &CameraOptions,
    graphics: &GraphicsOptions,
    hud: &HudOptions,
    bindings: &InputBindings,
    modal_offset: [f32; 2],
) -> ClientOptionsFile {
    ClientOptionsFile {
        accepted_eula: existing.accepted_eula,
        preferred_realm: existing.preferred_realm,
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
        vsync_enabled: graphics.vsync_enabled,
        frame_rate_limit_enabled: graphics.frame_rate_limit_enabled,
        frame_rate_limit: graphics
            .frame_rate_limit
            .clamp(MIN_FRAME_RATE_LIMIT, MAX_FRAME_RATE_LIMIT),
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

pub(super) fn save_options_file_to_path(
    path: &Path,
    file: &ClientOptionsFile,
) -> Result<(), String> {
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

pub(super) fn load_options_file() -> ClientOptionsFile {
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

pub fn load_preferred_realm() -> RealmPreset {
    load_options_file().preferred_realm
}

pub fn load_eula_accepted() -> bool {
    load_options_file().accepted_eula
}

pub fn save_preferred_realm(realm: RealmPreset) -> Result<(), String> {
    let path = load_options_path();
    let mut file = load_options_file_from_path(&path);
    if file.preferred_realm == realm {
        return Ok(());
    }
    file.preferred_realm = realm;
    save_options_file_to_path(&path, &file)
}

pub fn save_eula_accepted(accepted: bool) -> Result<(), String> {
    let path = load_options_path();
    let mut file = load_options_file_from_path(&path);
    if file.accepted_eula == accepted {
        return Ok(());
    }
    file.accepted_eula = accepted;
    save_options_file_to_path(&path, &file)
}

pub fn login_credentials_path() -> PathBuf {
    if let Some(proj_dirs) = ProjectDirs::from("org", "WorldOfOsso", "game-engine") {
        return proj_dirs.config_dir().join(CREDENTIALS_FILE_NAME);
    }

    Path::new(LEGACY_CREDENTIALS_PATH).to_path_buf()
}

pub(super) fn select_load_options_path(config_path: &Path, legacy_path: &Path) -> PathBuf {
    if config_path.exists() {
        return config_path.to_path_buf();
    }
    if legacy_path.exists() {
        return legacy_path.to_path_buf();
    }
    config_path.to_path_buf()
}

pub(super) fn load_options_file_from_path(path: &Path) -> ClientOptionsFile {
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
