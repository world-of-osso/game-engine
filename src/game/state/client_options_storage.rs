use super::*;

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
        bindings: bindings.0.clone(),
        modal_offset: Some(modal_offset),
        modal_position: None,
    }
}

pub(super) fn sound_options_file_from_runtime(
    settings: &SoundSettings,
) -> portable::SoundOptionsFile {
    portable::SoundOptionsFile {
        master_volume: settings.master_volume,
        ambient_volume: settings.ambient_volume,
        effects_volume: settings.effects_volume,
        music_volume: settings.music_volume,
        music_enabled: settings.music_enabled,
        muted: settings.muted,
    }
}
pub(super) fn sound_options_file_to_runtime(file: &portable::SoundOptionsFile) -> SoundSettings {
    SoundSettings {
        master_volume: file.master_volume,
        ambient_volume: file.ambient_volume,
        effects_volume: file.effects_volume,
        music_volume: file.music_volume,
        music_enabled: file.music_enabled,
        muted: file.muted,
    }
}
fn build_sound_options_file(sound: Option<&SoundSettings>) -> portable::SoundOptionsFile {
    sound
        .map(sound_options_file_from_runtime)
        .unwrap_or_default()
}

fn build_camera_options_file(camera: &CameraOptions) -> CameraOptionsFile {
    CameraOptionsFile {
        mouse_sensitivity: camera
            .mouse_sensitivity
            .clamp(MIN_MOUSE_SENSITIVITY, MAX_MOUSE_SENSITIVITY),
        look_sensitivity: camera.look_sensitivity,
        invert_y: camera.invert_y,
        fov_degrees: camera
            .fov_degrees
            .clamp(MIN_CAMERA_FOV_DEGREES, MAX_CAMERA_FOV_DEGREES),
        follow_speed: camera.follow_speed,
        zoom_speed: camera.zoom_speed,
        min_distance: camera.min_distance,
        max_distance: camera.max_distance,
    }
}

fn build_graphics_options_file(graphics: &GraphicsOptions) -> GraphicsOptionsFile {
    GraphicsOptionsFile {
        particle_effects_enabled: graphics.particle_effects_enabled,
        ssao_enabled: graphics.ssao_enabled,
        depth_of_field: graphics.depth_of_field,
        anti_alias: graphics.anti_alias,
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
        nameplate_style: hud.nameplate_style.clamped(),
        show_health_bars: hud.show_health_bars,
        show_target_marker: hud.show_target_marker,
        auto_loot: hud.auto_loot,
        soft_target_interact: hud.soft_target_interact,
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
    portable::save_options_file_to_path(path, file)
}
pub(super) fn load_options_file() -> ClientOptionsFile {
    portable::load_options_file()
}
fn load_options_path() -> PathBuf {
    select_load_options_path(&options_path(), Path::new(LEGACY_OPTIONS_PATH))
}
fn options_path() -> PathBuf {
    portable::options_path()
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

pub(super) fn ui_layout_path() -> PathBuf {
    portable::world_of_osso_config_dir().join(UI_LAYOUT_FILE_NAME)
}

pub(super) fn select_load_options_path(config_path: &Path, legacy_path: &Path) -> PathBuf {
    portable::select_load_options_path(config_path, legacy_path)
}
pub(super) fn load_options_file_from_path(path: &Path) -> ClientOptionsFile {
    portable::load_options_file_from_path(path)
}
