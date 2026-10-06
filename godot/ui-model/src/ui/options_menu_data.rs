//! Shared Options UI policy. Runtime resources, input events and persistence remain host-owned.
use crate::client_options_data::{
    self, CameraOptionsFile, GraphicsOptionsFile, HudOptionsFile, SoundOptionsFile,
};
use crate::input_bindings_data::{
    BindingSection, InputAction, InputBinding, InputBindingsData, actions_for_section,
};
use crate::nameplate_style_data::{NameplateStyle, StyleSlider};
use crate::soft_target_data::{InteractKeyIcons, SoftTargetOptions};
use crate::status_text_data::StatusTextDisplay;
use crate::ui::screens::game_menu_component::{GameMenuView, GameMenuViewModel};
use crate::ui::screens::options_menu_active_sections::{
    LAYOUT_CHOICE_KEY, LAYOUT_FONT_KEY, LAYOUT_SYSTEM_KEY,
};
use crate::ui::screens::options_menu_component::{
    ACTION_RESET_LAYOUT_SETTINGS, BindingOutputView, CameraOptionsView, GraphicsOptionsView,
    HudOptionsView, KeybindingRowView, KeybindingsView, LayoutOptionsView, LayoutSystem,
    OptionsCategory, OptionsViewModel, PartyDropdown, SoundOptionsView,
};
use game_engine_core::ui_layout_data::{
    CHAT_HEIGHT_RANGE, CHAT_WIDTH_RANGE, DAMAGE_METER_HEIGHT_RANGE, DAMAGE_METER_WIDTH_RANGE,
    LayoutFont, LayoutSettings, PARTY_DEFENSIVE_RANGE, PARTY_HEIGHT_RANGE, PARTY_ICON_RANGE,
    PARTY_OPACITY_RANGE, PARTY_WIDTH_RANGE, PartyAuraOrganization, PartySort, SettingRange,
    UNIT_FRAME_SIZE_RANGE, UNIT_FRAME_TEXT_SIZE_RANGE, UnitFrameSettings,
};

/// A slider of the HUD page's "Layout Settings" group. `FrameSize` and `TextSize` edit the
/// selected unit frame.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LayoutSlider {
    FrameSize,
    TextSize,
    ChatWidth,
    ChatHeight,
    MeterWidth,
    MeterHeight,
    PartyWidth,
    PartyHeight,
    PartySize,
    PartyOpacity,
    PartyDebuff,
    PartyBuff,
    PartyDefensive,
}

impl LayoutSlider {
    pub fn range(self) -> SettingRange {
        match self {
            Self::FrameSize => UNIT_FRAME_SIZE_RANGE,
            Self::TextSize => UNIT_FRAME_TEXT_SIZE_RANGE,
            Self::ChatWidth => CHAT_WIDTH_RANGE,
            Self::ChatHeight => CHAT_HEIGHT_RANGE,
            Self::MeterWidth => DAMAGE_METER_WIDTH_RANGE,
            Self::MeterHeight => DAMAGE_METER_HEIGHT_RANGE,
            Self::PartyWidth => PARTY_WIDTH_RANGE,
            Self::PartyHeight => PARTY_HEIGHT_RANGE,
            Self::PartySize => UNIT_FRAME_SIZE_RANGE,
            Self::PartyOpacity => PARTY_OPACITY_RANGE,
            Self::PartyDebuff | Self::PartyBuff => PARTY_ICON_RANGE,
            Self::PartyDefensive => PARTY_DEFENSIVE_RANGE,
        }
    }

    /// The setting this slider writes; none for a unit-frame slider while the selected
    /// system is not a unit frame.
    fn setting(self, layout: &mut LayoutOptionsView) -> Option<&mut Option<u16>> {
        let settings = &mut layout.settings;
        Some(match self {
            Self::FrameSize => &mut unit_frame_settings(settings, layout.system)?.frame_size,
            Self::TextSize => &mut unit_frame_settings(settings, layout.system)?.text_size,
            Self::ChatWidth => &mut settings.chat.width,
            Self::ChatHeight => &mut settings.chat.height,
            Self::MeterWidth => &mut settings.damage_meter.width,
            Self::MeterHeight => &mut settings.damage_meter.height,
            Self::PartyWidth => &mut settings.party.width,
            Self::PartyHeight => &mut settings.party.height,
            Self::PartySize => &mut settings.party.frame_size,
            Self::PartyOpacity => &mut settings.party.opacity,
            Self::PartyDebuff => &mut settings.party.debuff_size,
            Self::PartyBuff => &mut settings.party.buff_size,
            Self::PartyDefensive => &mut settings.party.defensive_size,
        })
    }
}

/// `system`'s unit-frame settings; none for the chat frame and the damage meter.
pub fn unit_frame_settings(
    settings: &mut LayoutSettings,
    system: LayoutSystem,
) -> Option<&mut UnitFrameSettings> {
    match system {
        LayoutSystem::PlayerFrame => Some(&mut settings.player_frame),
        LayoutSystem::TargetFrame => Some(&mut settings.target_frame),
        LayoutSystem::FocusFrame => Some(&mut settings.focus_frame),
        LayoutSystem::PetFrame => Some(&mut settings.pet_frame),
        LayoutSystem::ChatFrame | LayoutSystem::DamageMeter | LayoutSystem::PartyFrames => None,
    }
}

/// The fonts the "Font" dropdown offers, in its order.
pub const LAYOUT_FONTS: [LayoutFont; 2] = [LayoutFont::FrizQuadrata, LayoutFont::ArialNarrow];

/// What an Options action does to the character's Edit Mode layout.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LayoutAction {
    /// The "Layout" dropdown's choice, an index into [`LayoutOptionsView::names`].
    Select(usize),
    System(LayoutSystem),
    Font(LayoutFont),
    ToggleMicroMenu,
    PartyToggle(PartyToggle),
    PartySort(PartySort),
    PartyAura(PartyAuraOrganization),
    PartyDropdown(PartyDropdown),
    /// "Reset to Preset": clear every setting of the layout.
    Reset,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PartyToggle {
    Compact,
    Background,
    Horizontal,
    Border,
    Pets,
}

impl PartyToggle {
    pub const ALL: [Self; 5] = [
        Self::Compact,
        Self::Background,
        Self::Horizontal,
        Self::Border,
        Self::Pets,
    ];

    pub fn key(self) -> &'static str {
        match self {
            Self::Compact => "party_compact",
            Self::Background => "party_background",
            Self::Horizontal => "party_horizontal",
            Self::Border => "party_border",
            Self::Pets => "party_pets",
        }
    }

    pub fn value(self, settings: &LayoutSettings) -> bool {
        match self {
            Self::Compact => settings.use_raid_style_party_frames.unwrap_or(true),
            Self::Background => settings.party.background.unwrap_or(false),
            Self::Horizontal => settings.party.horizontal.unwrap_or(false),
            Self::Border => settings.party.border.unwrap_or(false),
            Self::Pets => settings.party.show_pets.unwrap_or(false),
        }
    }

    fn toggle(self, settings: &mut LayoutSettings) {
        let value = Some(!self.value(settings));
        match self {
            Self::Compact => settings.use_raid_style_party_frames = value,
            Self::Background => settings.party.background = value,
            Self::Horizontal => settings.party.horizontal = value,
            Self::Border => settings.party.border = value,
            Self::Pets => settings.party.show_pets = value,
        }
    }
}

pub const PARTY_SORTS: [PartySort; 3] =
    [PartySort::Role, PartySort::Group, PartySort::Alphabetical];
pub const PARTY_AURAS: [PartyAuraOrganization; 3] = [
    PartyAuraOrganization::Legacy,
    PartyAuraOrganization::BuffsTop,
    PartyAuraOrganization::BuffsRight,
];

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SliderField {
    Layout(LayoutSlider),
    MouseSensitivity,
    FovDegrees,
    ParticleDensity,
    FrameRateLimit,
    RenderScale,
    UiScale,
    NameplateDistance,
    ChatFontSize,
    BloomIntensity,
    MasterVolume,
    MusicVolume,
    AmbientVolume,
    EffectsVolume,
    LookSensitivity,
    ZoomSpeed,
    FollowSpeed,
    MinDistance,
    MaxDistance,
    Nameplate(StyleSlider),
}

#[derive(Debug, Clone)]
pub struct OptionsModel {
    pub logged_in: bool,
    pub view: GameMenuView,
    pub category: OptionsCategory,
    pub modal_position: [f32; 2],
    pub draft_graphics: GraphicsDraft,
    pub draft_sound: SoundDraft,
    pub draft_camera: CameraDraft,
    pub draft_hud: HudDraft,
    pub committed_graphics: GraphicsDraft,
    pub committed_sound: SoundDraft,
    pub committed_camera: CameraDraft,
    pub committed_hud: HudDraft,
    pub draft_bindings: InputBindingsData,
    pub committed_bindings: InputBindingsData,
    pub binding_section: BindingSection,
    pub binding_capture: BindingCapture,
    /// The character's Edit Mode layout. Its settings are saved to `ui_layout.ron` by the
    /// host on every change, not drafted with the options file.
    pub layout: LayoutOptionsView,
}

#[derive(Debug, Clone)]
pub struct SoundDraft {
    pub muted: bool,
    pub music_enabled: bool,
    pub master_volume: f32,
    pub music_volume: f32,
    pub ambient_volume: f32,
    pub effects_volume: f32,
}

#[derive(Debug, Clone)]
pub struct GraphicsDraft {
    pub particle_density: f32,
    pub render_scale: f32,
    pub ui_scale: f32,
    pub vsync_enabled: bool,
    pub frame_rate_limit_enabled: bool,
    pub frame_rate_limit: f32,
    pub colorblind_mode: bool,
    pub bloom_enabled: bool,
    pub bloom_intensity: f32,
}

#[derive(Debug, Clone)]
pub struct CameraDraft {
    pub mouse_sensitivity: f32,
    pub look_sensitivity: f32,
    pub invert_y: bool,
    pub fov_degrees: f32,
    pub zoom_speed: f32,
    pub follow_speed: f32,
    pub min_distance: f32,
    pub max_distance: f32,
}

#[derive(Debug, Clone)]
pub struct HudDraft {
    pub show_minimap: bool,
    pub show_action_bars: bool,
    pub show_nameplates: bool,
    pub nameplate_distance: f32,
    pub nameplate_style: NameplateStyle,
    pub show_health_bars: bool,
    pub show_target_marker: bool,
    pub auto_loot: bool,
    pub personal_resource_display: bool,
    pub soft_target_interact: bool,
    pub soft_target: SoftTargetOptions,
    pub show_fps_overlay: bool,
    pub chat_font_size: f32,
    pub status_text_display: StatusTextDisplay,
}

#[derive(Clone)]
pub struct ApplySnapshot {
    pub graphics: GraphicsDraft,
    pub sound: SoundDraft,
    pub camera: CameraDraft,
    pub hud: HudDraft,
    pub bindings: InputBindingsData,
    pub modal_position: [f32; 2],
}

/// Key Bindings page state: listening for a key, or the result of the last capture, which
/// Retail shows as the settings panel's output text (`SettingsPanelMixin:OnKeybind*`,
/// `Blizzard_Settings_Shared/Blizzard_SettingsPanel.lua:944-982`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BindingCapture {
    None,
    /// The next key or mouse button binds this action; Escape cancels.
    Listening(InputAction),
    /// The captured key was free (`KEY_BOUND`).
    Bound,
    /// The captured key was taken from this action, which is now unbound
    /// (`KEY_UNBOUND_ERROR`).
    Unbound(InputAction),
}

/// Left-click on an action's binding button: listen for its key
/// (`Blizzard_Keybindings.lua:425-433`).
pub fn listen_for_binding(model: &mut OptionsModel, action: InputAction) {
    model.binding_capture = BindingCapture::Listening(action);
}

/// Bind the captured key to the listening action, moving it from the action that held it
/// (`KeybindListener:ProcessInput`, `Blizzard_Keybindings.lua:76-119`). Returns false when
/// nothing listens.
pub fn capture_binding(model: &mut OptionsModel, binding: InputBinding) -> bool {
    let BindingCapture::Listening(action) = model.binding_capture else {
        return false;
    };
    model.binding_capture = match model.draft_bindings.assign(action, binding) {
        Some(unbound) => BindingCapture::Unbound(unbound),
        None => BindingCapture::Bound,
    };
    true
}

/// Escape while listening stops listening and changes nothing (`ProcessInput`,
/// `Blizzard_Keybindings.lua:89-92`). Returns false when nothing listens.
pub fn cancel_binding_capture(model: &mut OptionsModel) -> bool {
    if !matches!(model.binding_capture, BindingCapture::Listening(_)) {
        return false;
    }
    model.binding_capture = BindingCapture::None;
    true
}

/// Right-click on an action's binding button unbinds it and clears the output text
/// (`Blizzard_Keybindings.lua:434-440`).
pub fn unbind_action(model: &mut OptionsModel, action: InputAction) {
    model.draft_bindings.clear(action);
    model.binding_capture = BindingCapture::None;
}

pub fn sound_draft_from_file(s: &SoundOptionsFile) -> SoundDraft {
    SoundDraft {
        muted: s.muted,
        music_enabled: s.music_enabled,
        master_volume: s.master_volume,
        music_volume: s.music_volume,
        ambient_volume: s.ambient_volume,
        effects_volume: s.effects_volume,
    }
}

pub fn graphics_draft_from_file(graphics: &GraphicsOptionsFile) -> GraphicsDraft {
    GraphicsDraft {
        particle_density: graphics.particle_density as f32,
        render_scale: graphics.render_scale,
        ui_scale: graphics.ui_scale,
        vsync_enabled: graphics.vsync_enabled,
        frame_rate_limit_enabled: graphics.frame_rate_limit_enabled,
        frame_rate_limit: f32::from(graphics.frame_rate_limit),
        colorblind_mode: graphics.colorblind_mode,
        bloom_enabled: graphics.bloom_enabled,
        bloom_intensity: graphics.bloom_intensity,
    }
}

pub fn camera_draft_from_file(camera: &CameraOptionsFile) -> CameraDraft {
    CameraDraft {
        mouse_sensitivity: camera.mouse_sensitivity,
        look_sensitivity: camera.look_sensitivity,
        invert_y: camera.invert_y,
        fov_degrees: camera.fov_degrees,
        zoom_speed: camera.zoom_speed,
        follow_speed: camera.follow_speed,
        min_distance: camera.min_distance,
        max_distance: camera.max_distance,
    }
}

pub fn hud_draft_from_file(hud: &HudOptionsFile) -> HudDraft {
    HudDraft {
        show_minimap: hud.show_minimap,
        show_action_bars: hud.show_action_bars,
        show_nameplates: hud.show_nameplates,
        nameplate_distance: hud.nameplate_distance,
        nameplate_style: hud.nameplate_style,
        show_health_bars: hud.show_health_bars,
        show_target_marker: hud.show_target_marker,
        auto_loot: hud.auto_loot,
        personal_resource_display: hud.personal_resource_display,
        soft_target_interact: hud.soft_target_interact,
        soft_target: hud.soft_target,
        show_fps_overlay: hud.show_fps_overlay,
        chat_font_size: hud.chat_font_size,
        status_text_display: hud.status_text_display,
    }
}

fn graphics_to_view(g: &GraphicsDraft) -> GraphicsOptionsView {
    GraphicsOptionsView {
        particle_density: g.particle_density,
        render_scale: g.render_scale,
        ui_scale: g.ui_scale,
        vsync_enabled: g.vsync_enabled,
        frame_rate_limit_enabled: g.frame_rate_limit_enabled,
        frame_rate_limit: g.frame_rate_limit,
        colorblind_mode: g.colorblind_mode,
        bloom_enabled: g.bloom_enabled,
        bloom_intensity: g.bloom_intensity,
    }
}

fn sound_to_view(s: &SoundDraft) -> SoundOptionsView {
    SoundOptionsView {
        muted: s.muted,
        music_enabled: s.music_enabled,
        master_volume: s.master_volume,
        music_volume: s.music_volume,
        ambient_volume: s.ambient_volume,
        effects_volume: s.effects_volume,
    }
}

fn camera_to_view(c: &CameraDraft) -> CameraOptionsView {
    CameraOptionsView {
        mouse_sensitivity: c.mouse_sensitivity,
        look_sensitivity: c.look_sensitivity,
        invert_y: c.invert_y,
        fov_degrees: c.fov_degrees,
        zoom_speed: c.zoom_speed,
        follow_speed: c.follow_speed,
        min_distance: c.min_distance,
        max_distance: c.max_distance,
    }
}

fn hud_to_view(h: &HudDraft) -> HudOptionsView {
    HudOptionsView {
        show_minimap: h.show_minimap,
        show_action_bars: h.show_action_bars,
        show_nameplates: h.show_nameplates,
        nameplate_distance: h.nameplate_distance,
        nameplate_style: h.nameplate_style,
        show_health_bars: h.show_health_bars,
        show_target_marker: h.show_target_marker,
        auto_loot: h.auto_loot,
        personal_resource_display: h.personal_resource_display,
        soft_target_interact: h.soft_target_interact,
        interact_key_icons: h.soft_target.interact_key_icons(),
        show_fps_overlay: h.show_fps_overlay,
        chat_font_size: h.chat_font_size,
        status_text_display: h.status_text_display,
    }
}

pub fn build_view_model(model: &OptionsModel) -> GameMenuViewModel {
    GameMenuViewModel {
        logged_in: model.logged_in,
        view: model.view,
        options: OptionsViewModel {
            category: model.category,
            position: model.modal_position,
            graphics: graphics_to_view(&model.draft_graphics),
            sound: sound_to_view(&model.draft_sound),
            camera: camera_to_view(&model.draft_camera),
            hud: hud_to_view(&model.draft_hud),
            bindings: bindings_view(
                &model.draft_bindings,
                model.binding_section,
                model.binding_capture,
            ),
            layout: model.layout.clone(),
        },
    }
}

fn bindings_view(
    bindings: &InputBindingsData,
    section: BindingSection,
    capture: BindingCapture,
) -> KeybindingsView {
    let capture_action = current_capture_action(capture);
    let rows = actions_for_section(section)
        .iter()
        .map(|action| KeybindingRowView {
            action: *action,
            label: action.label().to_string(),
            binding_text: bindings.binding(*action).map(InputBinding::display),
            capturing: capture_action == Some(*action),
        })
        .collect();
    KeybindingsView {
        section,
        capture_action,
        output: binding_output(capture),
        rows,
    }
}

/// Retail's output text for the binding state: `SETTINGS_BIND_KEY_TO_COMMAND_OR_CANCEL`
/// with `GetBindingText("ESCAPE")` while listening, `KEY_BOUND` after a free key, and the
/// red `KEY_UNBOUND_ERROR` naming the action a moved key came from
/// (`Blizzard_SettingsPanel.lua:966,971-982`; GlobalStrings 49041, 15521, 10748, 12026).
fn binding_output(capture: BindingCapture) -> Option<BindingOutputView> {
    let (text, error) = match capture {
        BindingCapture::None => return None,
        BindingCapture::Listening(action) => (
            format!(
                "Assign Binding for \"{}\" or Press Escape to Cancel",
                action.label()
            ),
            false,
        ),
        BindingCapture::Bound => ("Key Bound Successfully".to_string(), false),
        BindingCapture::Unbound(action) => {
            (format!("Action {} is Now Unbound!", action.label()), true)
        }
    };
    Some(BindingOutputView { text, error })
}

pub fn parse_slider_action(action: &str) -> Option<SliderField> {
    let key = action.strip_prefix(SLIDER_ACTION_PREFIX)?;
    slider_field_from_key(key)
}

const SLIDER_ACTION_PREFIX: &str = "options_slider:";
const SLIDER_ACTION_FIELDS: &[(&str, SliderField)] = &[
    ("party_width", SliderField::Layout(LayoutSlider::PartyWidth)),
    (
        "party_height",
        SliderField::Layout(LayoutSlider::PartyHeight),
    ),
    ("party_size", SliderField::Layout(LayoutSlider::PartySize)),
    (
        "party_opacity",
        SliderField::Layout(LayoutSlider::PartyOpacity),
    ),
    (
        "party_debuff",
        SliderField::Layout(LayoutSlider::PartyDebuff),
    ),
    ("party_buff", SliderField::Layout(LayoutSlider::PartyBuff)),
    (
        "party_defensive",
        SliderField::Layout(LayoutSlider::PartyDefensive),
    ),
    ("mouse_sensitivity", SliderField::MouseSensitivity),
    ("fov_degrees", SliderField::FovDegrees),
    ("particle_density", SliderField::ParticleDensity),
    ("frame_rate_limit", SliderField::FrameRateLimit),
    ("render_scale", SliderField::RenderScale),
    ("ui_scale", SliderField::UiScale),
    ("nameplate_distance", SliderField::NameplateDistance),
    ("chat_font_size", SliderField::ChatFontSize),
    ("bloom_intensity", SliderField::BloomIntensity),
    ("master_volume", SliderField::MasterVolume),
    ("music_volume", SliderField::MusicVolume),
    ("ambient_volume", SliderField::AmbientVolume),
    ("effects_volume", SliderField::EffectsVolume),
    ("look_sensitivity", SliderField::LookSensitivity),
    ("zoom_speed", SliderField::ZoomSpeed),
    ("follow_speed", SliderField::FollowSpeed),
    ("min_distance", SliderField::MinDistance),
    ("max_distance", SliderField::MaxDistance),
    (
        "layout_frame_size",
        SliderField::Layout(LayoutSlider::FrameSize),
    ),
    (
        "layout_text_size",
        SliderField::Layout(LayoutSlider::TextSize),
    ),
    (
        "layout_chat_width",
        SliderField::Layout(LayoutSlider::ChatWidth),
    ),
    (
        "layout_chat_height",
        SliderField::Layout(LayoutSlider::ChatHeight),
    ),
    (
        "layout_meter_width",
        SliderField::Layout(LayoutSlider::MeterWidth),
    ),
    (
        "layout_meter_height",
        SliderField::Layout(LayoutSlider::MeterHeight),
    ),
];

fn slider_field_from_key(key: &str) -> Option<SliderField> {
    SLIDER_ACTION_FIELDS
        .iter()
        .find_map(|(candidate, field)| (*candidate == key).then_some(*field))
        .or_else(|| StyleSlider::from_key(key).map(SliderField::Nameplate))
}

/// The key that names a slider's `Slider{key}` frame and `options_slider:{key}` action.
pub fn slider_key(field: SliderField) -> String {
    if let SliderField::Nameplate(slider) = field {
        return slider.key();
    }
    SLIDER_ACTION_FIELDS
        .iter()
        .find_map(|(key, candidate)| (*candidate == field).then(|| (*key).to_owned()))
        .expect("every non-nameplate slider field has a key")
}

pub fn slider_bounds(field: SliderField) -> (f32, f32) {
    match field {
        SliderField::MouseSensitivity => mouse_sensitivity_range(),
        SliderField::FovDegrees => (
            crate::camera_control_data::MIN_CAMERA_FOV_DEGREES,
            crate::camera_control_data::MAX_CAMERA_FOV_DEGREES,
        ),
        SliderField::ParticleDensity => (10.0, 100.0),
        SliderField::FrameRateLimit => frame_rate_limit_range(),
        SliderField::RenderScale => (0.5, 1.0),
        SliderField::UiScale => ui_scale_range(),
        SliderField::NameplateDistance => nameplate_distance_range(),
        SliderField::ChatFontSize => chat_font_size_range(),
        SliderField::BloomIntensity => (0.0, 1.0),
        SliderField::MasterVolume
        | SliderField::MusicVolume
        | SliderField::AmbientVolume
        | SliderField::EffectsVolume => (0.0, 1.0),
        SliderField::LookSensitivity => (0.002, 0.03),
        SliderField::ZoomSpeed | SliderField::FollowSpeed => (2.0, 20.0),
        SliderField::MinDistance => (1.0, 10.0),
        SliderField::MaxDistance => (10.0, 60.0),
        SliderField::Nameplate(slider) => slider.bounds(),
        SliderField::Layout(slider) => {
            let range = slider.range();
            (f32::from(range.min), f32::from(range.max))
        }
    }
}

fn mouse_sensitivity_range() -> (f32, f32) {
    (
        client_options_data::MIN_MOUSE_SENSITIVITY,
        client_options_data::MAX_MOUSE_SENSITIVITY,
    )
}

fn frame_rate_limit_range() -> (f32, f32) {
    (
        f32::from(client_options_data::MIN_FRAME_RATE_LIMIT),
        f32::from(client_options_data::MAX_FRAME_RATE_LIMIT),
    )
}

fn ui_scale_range() -> (f32, f32) {
    (
        client_options_data::MIN_UI_SCALE,
        client_options_data::MAX_UI_SCALE,
    )
}

fn nameplate_distance_range() -> (f32, f32) {
    (
        client_options_data::MIN_NAMEPLATE_DISTANCE,
        client_options_data::MAX_NAMEPLATE_DISTANCE,
    )
}

fn chat_font_size_range() -> (f32, f32) {
    (
        client_options_data::MIN_CHAT_FONT_SIZE,
        client_options_data::MAX_CHAT_FONT_SIZE,
    )
}

pub fn apply_slider_value(field: SliderField, value: f32, model: &mut OptionsModel) {
    match field {
        SliderField::MouseSensitivity => model.draft_camera.mouse_sensitivity = value,
        SliderField::FovDegrees => model.draft_camera.fov_degrees = value,
        SliderField::ParticleDensity => model.draft_graphics.particle_density = value.round(),
        SliderField::FrameRateLimit => model.draft_graphics.frame_rate_limit = value.round(),
        SliderField::RenderScale => model.draft_graphics.render_scale = value,
        SliderField::UiScale => model.draft_graphics.ui_scale = value,
        SliderField::NameplateDistance => model.draft_hud.nameplate_distance = value.round(),
        SliderField::ChatFontSize => model.draft_hud.chat_font_size = value.round(),
        SliderField::BloomIntensity => model.draft_graphics.bloom_intensity = value,
        SliderField::MasterVolume => model.draft_sound.master_volume = value,
        SliderField::MusicVolume => model.draft_sound.music_volume = value,
        SliderField::AmbientVolume => model.draft_sound.ambient_volume = value,
        SliderField::EffectsVolume => model.draft_sound.effects_volume = value,
        SliderField::LookSensitivity => model.draft_camera.look_sensitivity = value,
        SliderField::ZoomSpeed => model.draft_camera.zoom_speed = value,
        SliderField::FollowSpeed => model.draft_camera.follow_speed = value,
        SliderField::MinDistance => {
            model.draft_camera.min_distance = value;
            normalize_camera_limits(&mut model.draft_camera);
        }
        SliderField::MaxDistance => {
            model.draft_camera.max_distance = value;
            normalize_camera_limits(&mut model.draft_camera);
        }
        SliderField::Nameplate(slider) => slider.set(&mut model.draft_hud.nameplate_style, value),
        SliderField::Layout(slider) => apply_layout_slider(slider, value, &mut model.layout),
    }
}

/// Store a layout slider's position, snapped to the setting's step.
pub fn apply_layout_slider(slider: LayoutSlider, value: f32, layout: &mut LayoutOptionsView) {
    if let Some(setting) = slider.setting(layout) {
        *setting = Some(slider.range().snap(value));
    }
}

pub fn parse_category_action(action: &str) -> Option<OptionsCategory> {
    let key = action.strip_prefix("options_category:")?;
    OptionsCategory::ALL
        .iter()
        .find(|c| c.key() == key)
        .copied()
}

pub fn parse_binding_section_action(action: &str) -> Option<BindingSection> {
    BindingSection::from_key(action.strip_prefix("options_binding_section:")?)
}

pub fn parse_binding_rebind_action(action: &str) -> Option<InputAction> {
    InputAction::from_key(action.strip_prefix("options_binding_rebind:")?)
}

pub fn parse_step_action(action: &str) -> Option<(&str, i32)> {
    let mut parts = action.strip_prefix("options_step:")?.split(':');
    let key = parts.next()?;
    let delta = parts.next()?.parse().ok()?;
    Some((key, delta))
}

pub fn parse_toggle_action(action: &str) -> Option<&str> {
    action.strip_prefix("options_toggle:")
}

/// The HUD page's layout actions: `options_toggle:ui_layout:<index>` (the "Layout"
/// dropdown), `options_toggle:layout_system:<index>`, `options_toggle:layout_font:<index>`
/// and the "Reset to Preset" button.
pub fn parse_layout_action(action: &str) -> Option<LayoutAction> {
    if let Some(toggle) = PartyToggle::ALL
        .into_iter()
        .find(|toggle| parse_toggle_action(action) == Some(toggle.key()))
    {
        return Some(LayoutAction::PartyToggle(toggle));
    }
    if action == "options_toggle:party_sort_open" {
        return Some(LayoutAction::PartyDropdown(PartyDropdown::Sort));
    }
    if action == "options_toggle:party_aura_open" {
        return Some(LayoutAction::PartyDropdown(PartyDropdown::Aura));
    }
    if action == "options_toggle:layout_show_micro_menu" {
        return Some(LayoutAction::ToggleMicroMenu);
    }
    if action == ACTION_RESET_LAYOUT_SETTINGS {
        return Some(LayoutAction::Reset);
    }
    let (key, index) = parse_toggle_action(action)?.rsplit_once(':')?;
    let index: usize = index.parse().ok()?;
    match key {
        LAYOUT_CHOICE_KEY => Some(LayoutAction::Select(index)),
        LAYOUT_SYSTEM_KEY => LayoutSystem::ALL
            .get(index)
            .copied()
            .map(LayoutAction::System),
        LAYOUT_FONT_KEY => LAYOUT_FONTS.get(index).copied().map(LayoutAction::Font),
        "party_sort" => PARTY_SORTS.get(index).copied().map(LayoutAction::PartySort),
        "party_aura" => PARTY_AURAS.get(index).copied().map(LayoutAction::PartyAura),
        _ => None,
    }
}

/// Apply a layout action to the shown layout. `Select` changes nothing here: the host
/// loads the chosen layout. `Font` sets the selected unit frame's face.
pub fn apply_layout_action(action: LayoutAction, layout: &mut LayoutOptionsView) {
    match action {
        LayoutAction::Select(_) => {}
        LayoutAction::System(system) => layout.system = system,
        LayoutAction::Font(font) => {
            if let Some(frame) = unit_frame_settings(&mut layout.settings, layout.system) {
                frame.font = Some(font);
            }
        }
        LayoutAction::ToggleMicroMenu => {
            layout.settings.show_micro_menu =
                Some(!layout.settings.show_micro_menu.unwrap_or(false));
        }
        LayoutAction::PartyToggle(toggle) => toggle.toggle(&mut layout.settings),
        LayoutAction::PartySort(sort) => {
            layout.settings.party.sort = Some(sort);
            layout.party_dropdown = None;
        }
        LayoutAction::PartyAura(aura) => {
            layout.settings.party.aura_organization = Some(aura);
            layout.party_dropdown = None;
        }
        LayoutAction::PartyDropdown(dropdown) => {
            layout.party_dropdown = if layout.party_dropdown == Some(dropdown) {
                None
            } else {
                Some(dropdown)
            };
        }
        LayoutAction::Reset => layout.settings = LayoutSettings::default(),
    }
}

pub fn apply_step(key: &str, delta: i32, model: &mut OptionsModel) {
    let step = delta as f32;
    if apply_graphics_step(key, step, &mut model.draft_graphics) {
        return;
    }
    if apply_hud_step(key, step, &mut model.draft_hud) {
        return;
    }
    if apply_sound_step(key, step, &mut model.draft_sound) {
        return;
    }
    apply_camera_step(key, step, &mut model.draft_camera);
}

fn apply_graphics_step(key: &str, step: f32, g: &mut GraphicsDraft) -> bool {
    match key {
        "particle_density" => {
            g.particle_density = clamp_step(g.particle_density, 5.0 * step, 10.0, 100.0).round()
        }
        "frame_rate_limit" => {
            g.frame_rate_limit = clamp_step(
                g.frame_rate_limit,
                10.0 * step,
                f32::from(client_options_data::MIN_FRAME_RATE_LIMIT),
                f32::from(client_options_data::MAX_FRAME_RATE_LIMIT),
            )
            .round()
        }
        "render_scale" => g.render_scale = clamp_step(g.render_scale, 0.05 * step, 0.5, 1.0),
        "ui_scale" => {
            g.ui_scale = clamp_step(
                g.ui_scale,
                0.05 * step,
                client_options_data::MIN_UI_SCALE,
                client_options_data::MAX_UI_SCALE,
            )
        }
        "bloom_intensity" => {
            g.bloom_intensity = clamp_step(g.bloom_intensity, 0.05 * step, 0.0, 1.0)
        }
        _ => return false,
    }
    true
}

fn apply_sound_step(key: &str, step: f32, sound: &mut SoundDraft) -> bool {
    let field = match key {
        "master_volume" => &mut sound.master_volume,
        "music_volume" => &mut sound.music_volume,
        "ambient_volume" => &mut sound.ambient_volume,
        "effects_volume" => &mut sound.effects_volume,
        _ => return false,
    };
    *field = clamp_step(*field, 0.05 * step, 0.0, 1.0);
    true
}

fn apply_hud_step(key: &str, step: f32, hud: &mut HudDraft) -> bool {
    match key {
        "nameplate_distance" => {
            hud.nameplate_distance = clamp_step(
                hud.nameplate_distance,
                5.0 * step,
                client_options_data::MIN_NAMEPLATE_DISTANCE,
                client_options_data::MAX_NAMEPLATE_DISTANCE,
            )
            .round();
            true
        }
        "chat_font_size" => {
            hud.chat_font_size = clamp_step(
                hud.chat_font_size,
                step,
                client_options_data::MIN_CHAT_FONT_SIZE,
                client_options_data::MAX_CHAT_FONT_SIZE,
            )
            .round();
            true
        }
        _ => false,
    }
}

fn apply_camera_step(key: &str, step: f32, c: &mut CameraDraft) {
    match key {
        "mouse_sensitivity" => {
            c.mouse_sensitivity = clamp_step(
                c.mouse_sensitivity,
                0.0005 * step,
                client_options_data::MIN_MOUSE_SENSITIVITY,
                client_options_data::MAX_MOUSE_SENSITIVITY,
            )
        }
        "fov_degrees" => {
            c.fov_degrees = clamp_step(
                c.fov_degrees,
                step,
                crate::camera_control_data::MIN_CAMERA_FOV_DEGREES,
                crate::camera_control_data::MAX_CAMERA_FOV_DEGREES,
            )
        }
        "look_sensitivity" => {
            c.look_sensitivity = clamp_step(c.look_sensitivity, 0.001 * step, 0.002, 0.03)
        }
        "zoom_speed" => c.zoom_speed = clamp_step(c.zoom_speed, 0.5 * step, 2.0, 20.0),
        "follow_speed" => c.follow_speed = clamp_step(c.follow_speed, 0.5 * step, 2.0, 20.0),
        "min_distance" => c.min_distance = clamp_step(c.min_distance, 0.5 * step, 1.0, 10.0),
        "max_distance" => c.max_distance = clamp_step(c.max_distance, step, 10.0, 60.0),
        _ => return,
    }
    normalize_camera_limits(c);
}

pub fn apply_toggle(key: &str, model: &mut OptionsModel) -> bool {
    let toggled = match key {
        "bloom_enabled" => {
            model.draft_graphics.bloom_enabled = !model.draft_graphics.bloom_enabled;
            true
        }
        "vsync_enabled" => {
            model.draft_graphics.vsync_enabled = !model.draft_graphics.vsync_enabled;
            true
        }
        "frame_rate_limit_enabled" => {
            model.draft_graphics.frame_rate_limit_enabled =
                !model.draft_graphics.frame_rate_limit_enabled;
            true
        }
        "colorblind_mode" => {
            model.draft_graphics.colorblind_mode = !model.draft_graphics.colorblind_mode;
            true
        }
        "muted" => {
            model.draft_sound.muted = !model.draft_sound.muted;
            true
        }
        "music_enabled" => {
            model.draft_sound.music_enabled = !model.draft_sound.music_enabled;
            true
        }
        "invert_y" => {
            model.draft_camera.invert_y = !model.draft_camera.invert_y;
            true
        }
        _ => apply_hud_toggle(key, &mut model.draft_hud),
    };
    toggled
}

fn apply_hud_toggle(key: &str, hud: &mut HudDraft) -> bool {
    match key {
        "nameplate_health_thickness" => {
            let style = &mut hud.nameplate_style;
            style.apply_health_preset(style.health_preset().toggled())
        }
        "nameplate_spellbar_thickness" => {
            let style = &mut hud.nameplate_style;
            style.apply_cast_preset(style.cast_preset().toggled())
        }
        "nameplate_show_border" => {
            hud.nameplate_style.show_border = !hud.nameplate_style.show_border
        }
        "nameplate_show_health_value" => {
            hud.nameplate_style.show_health_value = !hud.nameplate_style.show_health_value
        }
        "nameplate_class_colors" => {
            hud.nameplate_style.class_colored_players = !hud.nameplate_style.class_colored_players
        }
        "show_minimap" => hud.show_minimap = !hud.show_minimap,
        "show_action_bars" => hud.show_action_bars = !hud.show_action_bars,
        "show_nameplates" => hud.show_nameplates = !hud.show_nameplates,
        "show_health_bars" => hud.show_health_bars = !hud.show_health_bars,
        "show_target_marker" => hud.show_target_marker = !hud.show_target_marker,
        "auto_loot" => hud.auto_loot = !hud.auto_loot,
        "personal_resource_display" => {
            hud.personal_resource_display = !hud.personal_resource_display
        }
        "soft_target_interact" => hud.soft_target_interact = !hud.soft_target_interact,
        "show_fps_overlay" => hud.show_fps_overlay = !hud.show_fps_overlay,
        _ => {
            if let Some(choice) = interact_key_icons_choice(key) {
                hud.soft_target.set_interact_key_icons(choice);
            } else if let Some(choice) = status_text_display_choice(key) {
                hud.status_text_display = choice;
            } else {
                return false;
            }
        }
    }
    true
}

/// `interact_key_icons:<1|2|3>`: a choice of the "Interact Key Icons" dropdown.
fn interact_key_icons_choice(key: &str) -> Option<InteractKeyIcons> {
    let value = key.strip_prefix("interact_key_icons:")?.parse().ok()?;
    InteractKeyIcons::from_value(value)
}

/// `status_text_display:<1|2|3|4>`: a choice of the "Status Text" dropdown.
fn status_text_display_choice(key: &str) -> Option<StatusTextDisplay> {
    let value = key.strip_prefix("status_text_display:")?.parse().ok()?;
    StatusTextDisplay::from_value(value)
}

pub fn reset_category_defaults(model: &mut OptionsModel) {
    match model.category {
        OptionsCategory::Graphics => {
            model.draft_graphics = graphics_draft_from_file(&GraphicsOptionsFile::default())
        }
        OptionsCategory::Sound => {
            model.draft_sound = sound_draft_from_file(&SoundOptionsFile::default())
        }
        OptionsCategory::Camera => {
            model.draft_camera = camera_draft_from_file(&CameraOptionsFile::default())
        }
        OptionsCategory::Interface | OptionsCategory::Hud => {
            model.draft_hud = hud_draft_from_file(&HudOptionsFile::default())
        }
        OptionsCategory::Nameplates => model.draft_hud.nameplate_style = NameplateStyle::default(),
        OptionsCategory::Keybindings => model.draft_bindings.reset_section(model.binding_section),
        _ => {}
    }
}

pub fn apply_snapshot(model: &mut OptionsModel) -> ApplySnapshot {
    model.committed_graphics = model.draft_graphics.clone();
    model.committed_sound = model.draft_sound.clone();
    model.committed_camera = model.draft_camera.clone();
    model.committed_hud = model.draft_hud.clone();
    model.committed_bindings = model.draft_bindings.clone();
    ApplySnapshot {
        graphics: model.draft_graphics.clone(),
        sound: model.draft_sound.clone(),
        camera: model.draft_camera.clone(),
        hud: model.draft_hud.clone(),
        bindings: model.draft_bindings.clone(),
        modal_position: model.modal_position,
    }
}

pub fn apply_graphics_file_snapshot(graphics: &mut GraphicsOptionsFile, draft: &GraphicsDraft) {
    graphics.particle_density = draft.particle_density.round().clamp(10.0, 100.0) as u8;
    graphics.render_scale = draft.render_scale.clamp(0.5, 1.0);
    graphics.ui_scale = draft.ui_scale.clamp(
        client_options_data::MIN_UI_SCALE,
        client_options_data::MAX_UI_SCALE,
    );
    graphics.vsync_enabled = draft.vsync_enabled;
    graphics.frame_rate_limit_enabled = draft.frame_rate_limit_enabled;
    graphics.frame_rate_limit = draft.frame_rate_limit.round().clamp(
        f32::from(client_options_data::MIN_FRAME_RATE_LIMIT),
        f32::from(client_options_data::MAX_FRAME_RATE_LIMIT),
    ) as u16;
    graphics.colorblind_mode = draft.colorblind_mode;
    graphics.bloom_enabled = draft.bloom_enabled;
    graphics.bloom_intensity = draft.bloom_intensity.clamp(0.0, 1.0);
}

pub fn apply_sound_file_snapshot(s: &mut SoundOptionsFile, d: &SoundDraft) {
    s.muted = d.muted;
    s.music_enabled = d.music_enabled;
    s.master_volume = d.master_volume;
    s.music_volume = d.music_volume;
    s.ambient_volume = d.ambient_volume;
    s.effects_volume = d.effects_volume;
}

pub fn apply_camera_file_snapshot(c: &mut CameraOptionsFile, d: &CameraDraft) {
    c.mouse_sensitivity = d.mouse_sensitivity.clamp(
        client_options_data::MIN_MOUSE_SENSITIVITY,
        client_options_data::MAX_MOUSE_SENSITIVITY,
    );
    c.look_sensitivity = d.look_sensitivity;
    c.invert_y = d.invert_y;
    c.fov_degrees = d.fov_degrees.clamp(
        crate::camera_control_data::MIN_CAMERA_FOV_DEGREES,
        crate::camera_control_data::MAX_CAMERA_FOV_DEGREES,
    );
    c.zoom_speed = d.zoom_speed;
    c.follow_speed = d.follow_speed;
    c.min_distance = d.min_distance;
    c.max_distance = d.max_distance;
}

pub fn apply_hud_file_snapshot(h: &mut HudOptionsFile, d: &HudDraft) {
    h.show_minimap = d.show_minimap;
    h.show_action_bars = d.show_action_bars;
    h.show_nameplates = d.show_nameplates;
    h.nameplate_style = d.nameplate_style.clamped();
    h.nameplate_distance = d
        .nameplate_distance
        .clamp(
            client_options_data::MIN_NAMEPLATE_DISTANCE,
            client_options_data::MAX_NAMEPLATE_DISTANCE,
        )
        .round();
    h.show_health_bars = d.show_health_bars;
    h.show_target_marker = d.show_target_marker;
    h.auto_loot = d.auto_loot;
    h.personal_resource_display = d.personal_resource_display;
    h.soft_target_interact = d.soft_target_interact;
    h.soft_target = d.soft_target;
    h.show_fps_overlay = d.show_fps_overlay;
    h.status_text_display = d.status_text_display;
    h.chat_font_size = d
        .chat_font_size
        .clamp(
            client_options_data::MIN_CHAT_FONT_SIZE,
            client_options_data::MAX_CHAT_FONT_SIZE,
        )
        .round();
}

pub fn current_capture_action(capture: BindingCapture) -> Option<InputAction> {
    match capture {
        BindingCapture::Listening(action) => Some(action),
        BindingCapture::None | BindingCapture::Bound | BindingCapture::Unbound(_) => None,
    }
}

fn clamp_step(value: f32, delta: f32, min: f32, max: f32) -> f32 {
    (value + delta).clamp(min, max)
}

fn normalize_camera_limits(camera: &mut CameraDraft) {
    camera.max_distance = camera.max_distance.max(camera.min_distance + 1.0);
}
