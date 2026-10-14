use std::fmt;

use ui_toolkit::rsx;
use ui_toolkit::widget_def::Element;

use super::options_menu_active_sections;
use super::options_menu_sections;
use crate::input_bindings::{BindingSection, InputAction};
use crate::ui::anchor::FrameName;
use crate::ui::strata::FrameStrata;
use crate::ui::widgets::font_string::{FontColor, JustifyH};
use game_engine_core::ui_layout_data::{LayoutSettings, LayoutSkin, SYSTEM_PRESETS};

#[path = "options_menu_header.rs"]
mod header;
#[path = "options_menu_scroll.rs"]
mod scroll;
pub use crate::minimal_scroll_bar::{back_stepper_name, forward_stepper_name};
use header::settings_header;
pub use scroll::{OPTIONS_CONTENT_SCROLL, OptionsPage};

struct DynName(String);

impl fmt::Display for DynName {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.0)
    }
}

pub const OPTIONS_ROOT: FrameName = FrameName("OptionsRoot");
pub const OPTIONS_DRAG_HANDLE: FrameName = FrameName("OptionsDragHandle");
const OPTIONS_TITLE_LABEL: FrameName = FrameName("OptionsTitleLabel");
const OPTIONS_TAB_PANEL: FrameName = FrameName("OptionsTabPanel");
const OPTIONS_TAB_INNER: FrameName = FrameName("OptionsTabInner");
const OPTIONS_CONTENT_PANEL: FrameName = FrameName("OptionsContentPanel");
const OPTIONS_FOOTER: FrameName = FrameName("OptionsFooter");

const BUTTON_ATLAS_UP: &str = "defaultbutton-nineslice-up";
const BUTTON_ATLAS_PRESSED: &str = "defaultbutton-nineslice-pressed";
const BUTTON_ATLAS_HIGHLIGHT: &str = "defaultbutton-nineslice-highlight";
const BUTTON_ATLAS_DISABLED: &str = "defaultbutton-nineslice-disabled";

const OPTIONS_W: f32 = 920.0;
const OPTIONS_H: f32 = 724.0;
const OPTIONS_HEADER_H: f32 = 64.0;
const OPTIONS_TAB_W: f32 = 199.0;
const OPTIONS_CONTENT_W: f32 = 665.0;
const OPTIONS_CONTENT_H: f32 = 601.0;
const OPTIONS_CONTENT_INSET_X: f32 = 15.0;
const OPTIONS_CONTENT_INSET_TOP: f32 = 54.0;
const OPTIONS_CONTENT_INSET_BOTTOM: f32 = 18.0;
const OPTIONS_FOOTER_RIGHT_INSET: f32 = 16.0;
const TAB_ROW_W: f32 = 175.0;
const TAB_ROW_H: f32 = 20.0;

pub const ACTION_OPTIONS_OKAY: &str = "options_okay";
pub const ACTION_OPTIONS_DEFAULTS: &str = "options_defaults";
pub const ACTION_RESET_WINDOW_POSITIONS: &str = "options_reset_window_positions";
pub const ACTION_RESET_LAYOUT_SETTINGS: &str = "options_reset_layout_settings";

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OptionsCategory {
    Graphics,
    Sound,
    Camera,
    Interface,
    Hud,
    ActionBars,
    Nameplates,
    Controls,
    Accessibility,
    Keybindings,
    Macros,
    SocialAddons,
    Advanced,
    Support,
}

impl OptionsCategory {
    pub const ALL: [Self; 14] = [
        Self::Graphics,
        Self::Sound,
        Self::Camera,
        Self::Interface,
        Self::Hud,
        Self::ActionBars,
        Self::Nameplates,
        Self::Controls,
        Self::Accessibility,
        Self::Keybindings,
        Self::Macros,
        Self::SocialAddons,
        Self::Advanced,
        Self::Support,
    ];

    pub fn key(self) -> &'static str {
        match self {
            Self::Graphics => "graphics",
            Self::Sound => "sound",
            Self::Camera => "camera",
            Self::Interface => "interface",
            Self::Hud => "hud",
            Self::ActionBars => "actionbars",
            Self::Nameplates => "nameplates",
            Self::Controls => "controls",
            Self::Accessibility => "accessibility",
            Self::Keybindings => "keybindings",
            Self::Macros => "macros",
            Self::SocialAddons => "socialaddons",
            Self::Advanced => "advanced",
            Self::Support => "support",
        }
    }

    pub fn title(self) -> &'static str {
        match self {
            Self::Graphics => "Graphics",
            Self::Sound => "Sound",
            Self::Camera => "Camera",
            Self::Interface => "Interface",
            Self::Hud => "HUD",
            Self::ActionBars => "Action Bars",
            Self::Nameplates => "Nameplates",
            Self::Controls => "Controls",
            Self::Accessibility => "Accessibility",
            Self::Keybindings => "Keybindings",
            Self::Macros => "Macros",
            Self::SocialAddons => "Social / AddOns",
            Self::Advanced => "Advanced / Debug",
            Self::Support => "Support / About",
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct GraphicsOptionsView {
    pub particle_density: f32,
    pub render_scale: f32,
    pub ui_scale: f32,
    pub vsync_enabled: bool,
    pub frame_rate_limit_enabled: bool,
    pub frame_rate_limit: f32,
    pub colorblind_mode: bool,
    pub bloom_enabled: bool,
    pub bloom_intensity: f32,
    pub ssao_enabled: bool,
}

#[derive(Debug, Clone, PartialEq)]
pub struct SoundOptionsView {
    pub muted: bool,
    pub music_enabled: bool,
    pub master_volume: f32,
    pub music_volume: f32,
    pub ambient_volume: f32,
    pub effects_volume: f32,
}

#[derive(Debug, Clone, PartialEq)]
pub struct CameraOptionsView {
    pub mouse_sensitivity: f32,
    pub look_sensitivity: f32,
    pub invert_y: bool,
    pub fov_degrees: f32,
    pub zoom_speed: f32,
    pub follow_speed: f32,
    pub min_distance: f32,
    pub max_distance: f32,
}

pub use crate::nameplate_style::NameplateBarThickness;

#[derive(Debug, Clone, PartialEq)]
pub struct HudOptionsView {
    pub show_minimap: bool,
    pub show_action_bars: bool,
    pub cooldown_assist: game_engine_core::client_options_data::CooldownAssistOptions,
    pub extra_action_bars: game_engine_core::client_options_data::ExtraActionBars,
    pub show_nameplates: bool,
    pub nameplate_distance: f32,
    pub nameplate_style: crate::nameplate_style::NameplateStyle,
    pub show_health_bars: bool,
    pub show_target_marker: bool,
    pub auto_loot: bool,
    pub personal_resource_display: bool,
    pub soft_target_interact: bool,
    pub interact_key_icons: crate::soft_target_data::InteractKeyIcons,
    pub show_fps_overlay: bool,
    pub encounter_timeline: bool,
    pub encounter_bars: bool,
    pub encounter_warnings: bool,
    pub encounter_sounds: bool,
    pub chat_font_size: f32,
    pub status_text_display: crate::status_text_data::StatusTextDisplay,
}

#[derive(Debug, Clone, PartialEq)]
pub struct KeybindingRowView {
    pub action: InputAction,
    pub label: String,
    /// The bound key's text; `None` when unbound.
    pub binding_text: Option<String>,
    pub capturing: bool,
}

/// The Key Bindings page's status line (Retail `SettingsPanel.OutputText`).
#[derive(Debug, Clone, PartialEq)]
pub struct BindingOutputView {
    pub text: String,
    /// Shown red, as `KEY_UNBOUND_ERROR`'s `|cffff0000` colour code.
    pub error: bool,
}

#[derive(Debug, Clone, PartialEq)]
pub struct KeybindingsView {
    pub section: BindingSection,
    pub capture_action: Option<InputAction>,
    pub output: Option<BindingOutputView>,
    pub rows: Vec<KeybindingRowView>,
}

/// An Edit Mode system with layout settings; the HUD page shows one system's settings at a
/// time, as Retail's settings dialog shows the selected system's
/// (`Blizzard_EditMode/Shared/EditModeDialogs.lua:537-540,598`).
#[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
pub enum LayoutSystem {
    #[default]
    PlayerFrame,
    TargetFrame,
    FocusFrame,
    PetFrame,
    ChatFrame,
    DamageMeter,
    PartyFrames,
}

impl LayoutSystem {
    pub const ALL: [Self; 7] = [
        Self::PlayerFrame,
        Self::TargetFrame,
        Self::FocusFrame,
        Self::PetFrame,
        Self::ChatFrame,
        Self::DamageMeter,
        Self::PartyFrames,
    ];

    /// Retail's system names (`HUD_EDIT_MODE_PLAYER_FRAME_LABEL` "Player Frame", …,
    /// `HUD_EDIT_MODE_DAMAGE_METER_LABEL` "Damage Meter") without "Frame"/"Damage", to fit
    /// six choices in one row.
    pub fn label(self) -> &'static str {
        match self {
            Self::PlayerFrame => "Player",
            Self::TargetFrame => "Target",
            Self::FocusFrame => "Focus",
            Self::PetFrame => "Pet",
            Self::ChatFrame => "Chat",
            Self::DamageMeter => "Meter",
            Self::PartyFrames => "Party",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PartyDropdown {
    Sort,
    Aura,
}

/// The character's Edit Mode layout as the HUD page shows and edits it (`ui_layout.ron`).
#[derive(Debug, Clone, PartialEq)]
pub struct LayoutOptionsView {
    /// Name of the active layout.
    pub active: String,
    /// Every selectable layout: the system presets, then the saved player layouts.
    pub names: Vec<String>,
    /// The active layout's preset.
    pub skin: LayoutSkin,
    /// The active layout's settings over that preset.
    pub settings: LayoutSettings,
    /// The system whose settings show.
    pub system: LayoutSystem,
    pub party_dropdown: Option<PartyDropdown>,
}

/// The Modern preset with no saved layouts.
impl Default for LayoutOptionsView {
    fn default() -> Self {
        let (name, skin) = SYSTEM_PRESETS[0];
        Self {
            active: name.to_string(),
            names: SYSTEM_PRESETS
                .iter()
                .map(|&(name, _)| name.to_string())
                .collect(),
            skin,
            settings: LayoutSettings::default(),
            system: LayoutSystem::default(),
            party_dropdown: None,
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct OptionsViewModel {
    pub category: OptionsCategory,
    pub search: String,
    pub addons_tab: bool,
    pub position: [f32; 2],
    pub graphics: GraphicsOptionsView,
    pub sound: SoundOptionsView,
    pub camera: CameraOptionsView,
    pub hud: HudOptionsView,
    pub bindings: KeybindingsView,
    pub layout: LayoutOptionsView,
}

pub fn cat_action(category: OptionsCategory) -> String {
    format!("options_category:{}", category.key())
}

pub fn toggle_action(key: &str) -> String {
    format!("options_toggle:{key}")
}

pub fn slider_action(key: &str) -> String {
    format!("options_slider:{key}")
}

pub fn step_action(key: &str, delta: i32) -> String {
    format!("options_step:{key}:{delta}")
}

pub fn keybinding_section_action(section: BindingSection) -> String {
    format!("options_binding_section:{}", section.key())
}

pub fn keybinding_rebind_action(action: InputAction) -> String {
    format!("options_binding_rebind:{}", action.key())
}

/// The binding button of `action`'s row: left-click listens, right-click unbinds.
pub fn keybinding_button_name(action: InputAction) -> String {
    format!("KeybindingButton{}", action.key())
}

/// The Options panel, its content scrolled down `offset` pixels (the
/// [`OPTIONS_CONTENT_SCROLL`] position).
pub fn options_view(
    model: &OptionsViewModel,
    offset: usize,
    open: Option<&crate::options_dropdown::OptionsDropdownMenu>,
) -> Element {
    let mut body = build_body(model, offset, open);
    let popups = crate::options_dropdown::detach_popup(&mut body);
    rsx! {
        r#frame {
            name: OPTIONS_ROOT,
            width: {OPTIONS_W},
            height: OPTIONS_H,
            strata: FrameStrata::Dialog,
            frame_level: 0.0,
            pos_type: "absolute",
            left: "50%",
            top: "50%",
            translate_x: "-50%",
            translate_y: "-50%",
            margin_left: {model.position[0]},
            margin_top: {-model.position[1]},
            r#frame {
                name: OPTIONS_DRAG_HANDLE,
                width: {OPTIONS_W},
                height: {OPTIONS_HEADER_H},
                mouse_enabled: true,
                pos_type: "absolute",
                left: "0%",
                top: "0%",
            }
            {settings_chrome()}
            {title()}
            {settings_header(model)}
            {body}
            {popups}
            {build_footer()}
        }
    }
}

/// Retail SettingsPanel title: `SETTINGS_TITLE` "Options" (Blizzard_SettingsPanel.lua:54,
/// GlobalStrings `SETTINGS_TITLE`).
fn title() -> Element {
    rsx! { fontstring { name: OPTIONS_TITLE_LABEL, text: "Options", font_size: 12.0,
    width: 800.0, height: 20.0, pos_type: "absolute", left: 60.0, top: 5.0, } }
}
fn build_body(
    model: &OptionsViewModel,
    offset: usize,
    open: Option<&crate::options_dropdown::OptionsDropdownMenu>,
) -> Element {
    rsx! { r#frame { name: "OptionsBody", width: 880.0, height: 602.0, pos_type: "absolute", left: 18.0, top: 76.0,
        {build_tabs(model)}
        r#frame { name: "OptionsContentMount", width: OPTIONS_CONTENT_W, height: OPTIONS_CONTENT_H,
            pos_type: "absolute", left: 215.0, top: 0.0, {build_content(model,offset,open)} }
    } }
}
fn build_tabs(model: &OptionsViewModel) -> Element {
    let categories = visible_options_categories(model);
    let count = categories.len();
    let config = format!("{count},30,20,602");
    let buttons: Element = categories
        .into_iter()
        .flat_map(|category| tab_button(category, model.category == category))
        .collect();
    rsx! { r#frame { name: OPTIONS_TAB_PANEL, width: OPTIONS_TAB_W, height: 602.0, scroll_list:config,
        r#frame { name: OPTIONS_TAB_INNER, width: TAB_ROW_W, height: 602.0, layout: "flex-column", gap: 0.0, {buttons} }
    } }
}
/// SettingsPanel category-set and Settings search apply to row text as well as headings.
pub fn visible_options_categories(model: &OptionsViewModel) -> Vec<OptionsCategory> {
    let query = model.search.trim().to_lowercase();
    OptionsCategory::ALL
        .into_iter()
        .filter(|category| {
            let in_set = (*category == OptionsCategory::SocialAddons) == model.addons_tab;
            if !in_set {
                return false;
            }
            let mut page_model = model.clone();
            page_model.category = *category;
            query.is_empty()
                || category.title().to_lowercase().contains(&query)
                || category_body(&page_model, None).matches(&query)
        })
        .collect()
}

fn tab_button(category: OptionsCategory, selected: bool) -> Element {
    let action = cat_action(category);
    let name = DynName(format!("OptionsTab{}", category.key()));
    let hide = !selected;
    let background = ui_toolkit::atlas::get_region("Options_List_Active")
        .expect("Settings category selected atlas");
    rsx! { button { name: {&name}, width: TAB_ROW_W, height: TAB_ROW_H, onclick: action, button_default_skin: false,
        texture { name: DynName(format!("{}Selected",name.0)), texture_atlas: "Options_List_Active", hidden: hide,
            width: background.width, height: background.height, pos_type: "absolute", left: "50%", top: "50%", translate_x: "-50%", translate_y: "-50%", }
        texture { name: DynName(format!("{}Hover",name.0)), texture_atlas: "Options_List_Hover", hidden: true,
            width: background.width, height: background.height, pos_type: "absolute", left: "50%", top: "50%", translate_x: "-50%", translate_y: "-50%", }
        fontstring { name: DynName(format!("{}Label",name.0)), text: category.title(), font_size: 12.0,
            font_color: {if selected { FontColor::new(1.0,1.0,1.0,1.0) } else { FontColor::new(1.0,0.82,0.0,1.0) }},
            width: 139.0, height: TAB_ROW_H, justify_h: JustifyH::Left,
            pos_type: "absolute", left: 36.0, top: -1.0, frame_level: 100.0, }
    } }
}
fn settings_chrome() -> Element {
    // SettingsFrameTemplate BG TOPLEFT7,-18 -> BOTTOMRIGHT-3,3.
    // GlobalColor191 PANEL_BACKGROUND_COLOR =0xcc1f1e21.
    let bg = ui_toolkit::atlas::get_region("uiframebackground-nineslice-cornerbottomleft")
        .expect("FlatPanelBackgroundTemplate corner");
    let bg_color = "0.121569,0.117647,0.129412,0.8";
    let mut art = rsx! {
        r#frame { name: "OptionsBackgroundTop", width: 910.0, height: {703.0 - bg.height},
            background_color: bg_color, pos_type: "absolute", left: 7.0, top: 18.0, }
        r#frame { name: "OptionsBackgroundBottom", width: {910.0 - bg.width * 2.0}, height: bg.height,
            background_color: bg_color, pos_type: "absolute", left: {7.0 + bg.width}, top: {721.0 - bg.height}, }
        texture { name: "OptionsBackgroundBottomLeft", texture_atlas: "uiframebackground-nineslice-cornerbottomleft",
            width: bg.width, height: bg.height, vertex_color: bg_color, pos_type: "absolute", left: 7.0, top: {721.0-bg.height}, }
        texture { name: "OptionsBackgroundBottomRight", texture_atlas: "uiframebackground-nineslice-cornerbottomright",
            width: bg.width, height: bg.height, vertex_color: bg_color, pos_type: "absolute", left: {917.0-bg.width}, top: {721.0-bg.height}, }
    };
    art.extend(crate::quest_art::named_atlas_texture(
        "OptionsInnerFrame".into(),
        "Options_InnerFrame",
        (17.0, 64.0, 886.0, 618.0),
    ));
    // ButtonFrameTemplateNoPortrait: source corner atlases, offsets-8/+4,+16/-3.
    let corner =
        ui_toolkit::atlas::get_region("UI-Frame-Metal-CornerTopLeft").expect("Settings corner");
    // Retail2x metal top150px=>75logical; bottom64px=>32logical.
    let cw = corner.width / 2.0;
    let ch = corner.height / 2.0;
    let bc = 32.0;
    for (part, atlas, x, y, w, h) in [
        (
            "TopLeft",
            "UI-Frame-Metal-CornerTopLeft",
            -8.0,
            -16.0,
            cw,
            ch,
        ),
        (
            "TopRight",
            "UI-Frame-Metal-CornerTopRight",
            OPTIONS_W + 4.0 - cw,
            -16.0,
            cw,
            ch,
        ),
        (
            "BottomLeft",
            "UI-Frame-Metal-CornerBottomLeft",
            -8.0,
            OPTIONS_H + 3.0 - bc,
            bc,
            bc,
        ),
        (
            "BottomRight",
            "UI-Frame-Metal-CornerBottomRight",
            OPTIONS_W + 4.0 - bc,
            OPTIONS_H + 3.0 - bc,
            bc,
            bc,
        ),
        (
            "Top",
            "_UI-Frame-Metal-EdgeTop",
            cw - 8.0,
            -16.0,
            OPTIONS_W + 12.0 - 2.0 * cw,
            ch,
        ),
        (
            "Bottom",
            "_UI-Frame-Metal-EdgeBottom",
            bc - 8.0,
            OPTIONS_H + 3.0 - bc,
            OPTIONS_W + 12.0 - 2.0 * bc,
            bc,
        ),
        (
            "Left",
            "!UI-Frame-Metal-EdgeLeft",
            -8.0,
            ch - 16.0,
            cw,
            OPTIONS_H + 19.0 - ch - bc,
        ),
        (
            "Right",
            "!UI-Frame-Metal-EdgeRight",
            OPTIONS_W + 4.0 - cw,
            ch - 16.0,
            cw,
            OPTIONS_H + 19.0 - ch - bc,
        ),
    ] {
        art.extend(crate::quest_art::named_atlas_texture(
            format!("OptionsBorder{part}"),
            atlas,
            (x, y, w, h),
        ));
    }
    art
}

/// The content area is as tall as the category list, as Retail's settings container is
/// anchored to it (`Blizzard_SettingsPanel.xml:59-74`); a taller page scrolls.
fn build_content(
    model: &OptionsViewModel,
    offset: usize,
    open: Option<&crate::options_dropdown::OptionsDropdownMenu>,
) -> Element {
    if visible_options_categories(model).is_empty() {
        return rsx! { fontstring { name:"OptionsNoResults",text:"No results found",width:OPTIONS_CONTENT_W,
        height:24.0,font_size:12.0,pos_type:"absolute",left:15.0,top:18.0 } };
    }
    rsx! {
        r#frame {
            name: OPTIONS_CONTENT_PANEL,
            width: {OPTIONS_CONTENT_W},
            height: {OPTIONS_CONTENT_H},
            layout: "flex-column",
            pos_type: "relative",
            {content_header(model.category, !model.search.trim().is_empty())}
            {content_body(model, offset, open)}
        }
    }
}

fn content_header(category: OptionsCategory, searching: bool) -> Element {
    let title = if searching {
        "Search Results"
    } else {
        category.title()
    };
    rsx! {
        fontstring {
            name: "OptionsSectionTitle",
            width: {OPTIONS_CONTENT_W - 30.0},
            height: 24.0,
            text: title,
            font_size: 22.0,
            color: "0.96,0.84,0.56,1.0",
            justify_h: "LEFT",
            pos_type: "absolute",
            left: "0%",
            top: "0%",
            margin_left: {15},
            margin_top: {18.0},
        }
    }
}

/// Pixels the Options content scrolls per stepper click on `model`'s page.
pub fn options_pan_extent(model: &OptionsViewModel) -> usize {
    filtered_category_body(model, None).pan_extent()
}

/// Pixels the Options content scrolls per wheel notch on `model`'s page.
pub fn options_wheel_extent(model: &OptionsViewModel) -> usize {
    filtered_category_body(model, None).wheel_extent()
}

fn category_body(
    model: &OptionsViewModel,
    open: Option<&crate::options_dropdown::OptionsDropdownMenu>,
) -> OptionsPage {
    match model.category {
        OptionsCategory::Graphics => options_menu_active_sections::graphics_body(&model.graphics),
        OptionsCategory::Sound => options_menu_active_sections::sound_body(&model.sound),
        OptionsCategory::Camera => options_menu_active_sections::camera_body(&model.camera),
        OptionsCategory::Interface => {
            options_menu_active_sections::interface_body(&model.hud, open)
        }
        OptionsCategory::Hud => {
            options_menu_active_sections::hud_body(&model.hud, &model.layout, open)
        }
        OptionsCategory::ActionBars => {
            options_menu_active_sections::action_bars_body(&model.hud, model.layout.skin)
        }
        OptionsCategory::Nameplates => options_menu_active_sections::nameplates_body(&model.hud),
        OptionsCategory::Controls => options_menu_sections::controls_body(),
        OptionsCategory::Accessibility => {
            options_menu_active_sections::accessibility_body(&model.graphics, &model.hud, open)
        }
        OptionsCategory::Keybindings => {
            options_menu_active_sections::keybindings_body(&model.bindings)
        }
        OptionsCategory::Macros => options_menu_sections::macros_body(),
        OptionsCategory::SocialAddons => options_menu_sections::social_addons_body(),
        OptionsCategory::Advanced => options_menu_active_sections::advanced_body(&model.hud),
        OptionsCategory::Support => options_menu_sections::support_body(),
    }
}

fn filtered_category_body(
    model: &OptionsViewModel,
    open: Option<&crate::options_dropdown::OptionsDropdownMenu>,
) -> OptionsPage {
    let query = model.search.trim();
    if query.is_empty() {
        return category_body(model, open);
    }
    // SettingsPanel.lua:575-709: aggregate matching initializers grouped by category.
    let items = visible_options_categories(model).into_iter().flat_map(|category| {
        let mut page_model = model.clone();
        page_model.category = category;
        let page = category_body(&page_model, open);
        let mut items = rsx! { fontstring { name:DynName(format!("OptionsSearchCategory{}",category.key())),
            text:category.title(),width:635.0,height:20.0,font_size:14.0,justify_h:"LEFT" } };
        let matches = if category.title().to_lowercase().contains(&query.to_lowercase()) {
            page
        } else {
            page.filter(query)
        };
        items.extend(matches.items);
        items
    }).collect();
    OptionsPage { items, gap: 14.0 }
}

fn content_body(
    model: &OptionsViewModel,
    offset: usize,
    open: Option<&crate::options_dropdown::OptionsDropdownMenu>,
) -> Element {
    let area = scroll::ScrollArea {
        row_width: OPTIONS_CONTENT_W - OPTIONS_CONTENT_INSET_X * 2.0,
        width: OPTIONS_CONTENT_W - OPTIONS_CONTENT_INSET_X,
        height: OPTIONS_CONTENT_H - OPTIONS_CONTENT_INSET_TOP - OPTIONS_CONTENT_INSET_BOTTOM,
    };
    rsx! {
        r#frame {
            name: "OptionsContentArea",
            width: {area.width},
            height: {area.height},
            pos_type: "relative",
            margin_left: {OPTIONS_CONTENT_INSET_X},
            margin_top: {OPTIONS_CONTENT_INSET_TOP},
            {scroll::scroll_area(filtered_category_body(model,open), &area, offset)}
        }
    }
}

fn build_footer() -> Element {
    let footer_buttons = footer_buttons();
    rsx! {
        r#frame {
            name: OPTIONS_FOOTER,
            width: "auto",
            height: "auto",
            layout: "flex-row",
            justify: "start",
            align: "center",
            gap: 2.0,
            pos_type: "absolute",
            left: "100%",
            top: "100%",
            translate_x: "-100%",
            translate_y: "-100%",
            margin_left: {(-OPTIONS_FOOTER_RIGHT_INSET).to_string()},
            margin_top: -16.0,
            {footer_buttons}
        }
    }
}

fn footer_buttons() -> Element {
    footer_specs()
        .into_iter()
        .flat_map(|(name, text, action, width)| small_button(name, text, action, width))
        .collect()
}

fn footer_specs() -> [(&'static str, &'static str, &'static str, f32); 2] {
    [
        (
            "OptionsDefaultsButton",
            "Defaults",
            ACTION_OPTIONS_DEFAULTS,
            96.0,
        ),
        ("OptionsDoneButton", "Done", ACTION_OPTIONS_OKAY, 96.0),
    ]
}

fn small_button(name: &str, text: &str, action: &str, width: f32) -> Element {
    let n = DynName(name.to_string());
    let text = text.to_string();
    let action = action.to_string();
    rsx! {
        button {
            name: {&n},
            width: {width},
            height: 22.0,
            text: {&text},
            font_size: 14.0,
            onclick: {&action},
            button_atlas_up: BUTTON_ATLAS_UP,
            button_atlas_pressed: BUTTON_ATLAS_PRESSED,
            button_atlas_highlight: BUTTON_ATLAS_HIGHLIGHT,
            button_atlas_disabled: BUTTON_ATLAS_DISABLED,
        }
    }
}

pub fn sync_options_art(registry: &mut ui_toolkit::registry::FrameRegistry) {
    use ui_toolkit::{frame::WidgetData, widgets::button::ButtonState};
    let categories: Vec<_> = registry
        .frames_iter()
        .filter_map(|frame| {
            let name = frame.name.as_deref()?;
            if !name.starts_with("OptionsTab") {
                return None;
            }
            let Some(WidgetData::Button(button)) = frame.widget_data.as_ref() else {
                return None;
            };
            Some((name.to_owned(), button.hovered))
        })
        .collect();
    for (name, hovered) in categories {
        let selected = registry
            .get_by_name(&format!("{name}Selected"))
            .and_then(|id| registry.get(id))
            .is_some_and(|frame| !frame.hidden);
        if let Some(id) = registry.get_by_name(&format!("{name}Hover")) {
            registry.set_hidden(id, !hovered || selected);
        }
    }
    let bindings: Vec<_> = registry
        .frames_iter()
        .filter_map(|frame| {
            let key = frame.name.as_deref()?.strip_prefix("KeybindingButton")?;
            let Some(WidgetData::Button(button)) = frame.widget_data.as_ref() else {
                return None;
            };
            Some((key.to_owned(), button.state, button.hovered))
        })
        .collect();
    for (key, state, hovered) in bindings {
        let pushed = state == ButtonState::Pushed;
        for (name, visible) in [
            (format!("KeybindingSilver{key}"), !pushed),
            (format!("KeybindingSilver{key}Pressed"), pushed),
            (format!("KeybindingHover{key}"), hovered),
        ] {
            if let Some(id) = registry.get_by_name(&name) {
                registry.set_hidden(id, !visible);
            }
        }
    }
}
