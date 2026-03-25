use std::fmt;

use ui_toolkit::rsx;
use ui_toolkit::widget_def::Element;

use super::options_menu_active_sections;
use super::options_menu_sections;
use super::screen_title::framed_title;
use crate::ui::anchor::{AnchorPoint, FrameName};
use crate::ui::strata::FrameStrata;
use crate::ui::widgets::font_string::{FontColor, GameFont, JustifyH};

struct DynName(String);

impl fmt::Display for DynName {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.0)
    }
}

pub const OPTIONS_ROOT: FrameName = FrameName("OptionsRoot");
pub const OPTIONS_DRAG_HANDLE: FrameName = FrameName("OptionsDragHandle");
const OPTIONS_TITLE_FRAME: FrameName = FrameName("OptionsTitleFrame");
const OPTIONS_TITLE_LABEL: FrameName = FrameName("OptionsTitleLabel");
const OPTIONS_TAB_PANEL: FrameName = FrameName("OptionsTabPanel");
const OPTIONS_TAB_INNER: FrameName = FrameName("OptionsTabInner");
const OPTIONS_CONTENT_PANEL: FrameName = FrameName("OptionsContentPanel");
const OPTIONS_CONTENT_INNER: FrameName = FrameName("OptionsContentInner");
const OPTIONS_FOOTER: FrameName = FrameName("OptionsFooter");

const BUTTON_ATLAS_UP: &str = "defaultbutton-nineslice-up";
const BUTTON_ATLAS_PRESSED: &str = "defaultbutton-nineslice-pressed";
const BUTTON_ATLAS_HIGHLIGHT: &str = "defaultbutton-nineslice-highlight";
const BUTTON_ATLAS_DISABLED: &str = "defaultbutton-nineslice-disabled";

const OPTIONS_W: f32 = 980.0;
const OPTIONS_H: f32 = 660.0;
const OPTIONS_HEADER_H: f32 = 58.0;
const OPTIONS_TAB_W: f32 = 186.0;
const OPTIONS_CONTENT_W: f32 = 716.0;
const OPTIONS_CONTENT_H: f32 = 478.0;
const OPTIONS_CONTENT_INSET_X: f32 = 15.0;
const OPTIONS_CONTENT_INSET_TOP: f32 = 54.0;
const OPTIONS_FOOTER_RIGHT_INSET: f32 = 80.0;
const TAB_ROW_W: f32 = 164.0;
const TAB_ROW_H: f32 = 30.0;
const TAB_LABEL_W: f32 = 132.0;
const TAB_ACCENT_H: f32 = 18.0;

const TAB_TEXT_IDLE: FontColor = FontColor::new(0.83, 0.79, 0.69, 1.0);
const TAB_TEXT_SELECTED: FontColor = FontColor::new(0.96, 0.84, 0.56, 1.0);
const TAB_BG_IDLE: &str = "0.07,0.06,0.05,0.0";
const TAB_BG_SELECTED: &str = "0.18,0.14,0.08,0.55";
const TAB_BORDER_IDLE: &str = "1px solid 0.30,0.24,0.09,0.0";
const TAB_BORDER_SELECTED: &str = "1px solid 0.42,0.33,0.12,0.65";
const TAB_ACCENT_COLOR: &str = "0.95,0.76,0.14,0.95";
const TAB_DIVIDER_COLOR: &str = "0.22,0.18,0.10,0.45";

pub const ACTION_OPTIONS_BACK: &str = "options_back";
pub const ACTION_OPTIONS_APPLY: &str = "options_apply";
pub const ACTION_OPTIONS_CANCEL: &str = "options_cancel";
pub const ACTION_OPTIONS_OKAY: &str = "options_okay";
pub const ACTION_OPTIONS_DEFAULTS: &str = "options_defaults";

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OptionsCategory {
    Graphics,
    Sound,
    Camera,
    Interface,
    Hud,
    Controls,
    Accessibility,
    Keybindings,
    Macros,
    SocialAddons,
    Advanced,
    Support,
}

impl OptionsCategory {
    pub const ALL: [Self; 12] = [
        Self::Graphics,
        Self::Sound,
        Self::Camera,
        Self::Interface,
        Self::Hud,
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
pub struct SoundOptionsView {
    pub muted: bool,
    pub music_enabled: bool,
    pub master_volume: f32,
    pub music_volume: f32,
    pub ambient_volume: f32,
    pub footstep_volume: f32,
}

#[derive(Debug, Clone, PartialEq)]
pub struct CameraOptionsView {
    pub look_sensitivity: f32,
    pub invert_y: bool,
    pub zoom_speed: f32,
    pub follow_speed: f32,
    pub min_distance: f32,
    pub max_distance: f32,
}

#[derive(Debug, Clone, PartialEq)]
pub struct HudOptionsView {
    pub show_minimap: bool,
    pub show_action_bars: bool,
    pub show_nameplates: bool,
    pub show_health_bars: bool,
    pub show_target_marker: bool,
    pub show_fps_overlay: bool,
}

#[derive(Debug, Clone, PartialEq)]
pub struct OptionsViewModel {
    pub category: OptionsCategory,
    pub position: [f32; 2],
    pub sound: SoundOptionsView,
    pub camera: CameraOptionsView,
    pub hud: HudOptionsView,
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

pub fn options_view(model: &OptionsViewModel) -> Element {
    let x = model.position[0].to_string();
    let y = model.position[1].to_string();
    rsx! {
        panel {
            name: OPTIONS_ROOT,
            width: {OPTIONS_W},
            height: {OPTIONS_H},
            strata: FrameStrata::Dialog,
            frame_level: 0.0,
            anchor {
                point: AnchorPoint::Center,
                relative_point: AnchorPoint::Center,
                x: {x},
                y: {y},
            }
            r#frame {
                name: OPTIONS_DRAG_HANDLE,
                width: {OPTIONS_W},
                height: {OPTIONS_HEADER_H},
                mouse_enabled: true,
                anchor {
                    point: AnchorPoint::TopLeft,
                    relative_point: AnchorPoint::TopLeft,
                }
            }
            {title()}
            {build_tabs(model)}
            {build_content(model)}
            {build_footer()}
        }
    }
}

fn title() -> Element {
    framed_title(
        OPTIONS_TITLE_FRAME,
        OPTIONS_TITLE_LABEL,
        OPTIONS_ROOT,
        300.0,
        "Game Menu",
    )
}

fn build_tabs(model: &OptionsViewModel) -> Element {
    let buttons: Element = OptionsCategory::ALL
        .iter()
        .enumerate()
        .flat_map(|(_, category)| tab_button(*category, model.category == *category))
        .collect();
    rsx! {
        panel {
            name: OPTIONS_TAB_PANEL,
            style: "inner_plain",
            width: {OPTIONS_TAB_W},
            height: {OPTIONS_CONTENT_H},
            anchor {
                point: AnchorPoint::TopLeft,
                relative_to: OPTIONS_DRAG_HANDLE,
                relative_point: AnchorPoint::BottomLeft,
                x: "18",
                y: "-18",
            }
            {tab_stack(buttons)}
        }
    }
}

fn tab_stack(buttons: Element) -> Element {
    rsx! {
        r#frame {
            name: OPTIONS_TAB_INNER,
            width: {OPTIONS_TAB_W - 16.0},
            height: {OPTIONS_CONTENT_H - 24.0},
            layout: "flex-column",
            align: "center",
            gap: 8.0,
            anchor {
                point: AnchorPoint::TopLeft,
                relative_to: OPTIONS_TAB_PANEL,
                relative_point: AnchorPoint::TopLeft,
                x: "8",
                y: "-12",
            }
            {buttons}
        }
    }
}

fn tab_button(category: OptionsCategory, selected: bool) -> Element {
    let action = cat_action(category);
    let name = DynName(format!("OptionsTab{}", category.key()));
    let (background, border, color) = tab_style(selected);
    let accent = if selected {
        tab_accent(&name.0)
    } else {
        Vec::new()
    };
    rsx! {
        r#frame {
            name: {&name},
            width: {TAB_ROW_W},
            height: {TAB_ROW_H},
            onclick: {&action},
            background_color: background,
            border,
            {tab_label(&name.0, category.title(), color)}
            {tab_divider(&name.0)}
            {accent}
        }
    }
}

fn tab_style(selected: bool) -> (&'static str, &'static str, FontColor) {
    if selected {
        (TAB_BG_SELECTED, TAB_BORDER_SELECTED, TAB_TEXT_SELECTED)
    } else {
        (TAB_BG_IDLE, TAB_BORDER_IDLE, TAB_TEXT_IDLE)
    }
}

fn tab_label(name: &str, label: &str, color: FontColor) -> Element {
    rsx! {
        fontstring {
            name: {DynName(format!("{name}Label"))},
            width: {TAB_LABEL_W},
            height: {TAB_ROW_H},
            text: {label},
            font: GameFont::FrizQuadrata,
            font_size: 14.0,
            font_color: color,
            justify_h: JustifyH::Left,
            anchor {
                point: AnchorPoint::Left,
                relative_point: AnchorPoint::Left,
                x: "18",
            }
        }
    }
}

fn tab_divider(name: &str) -> Element {
    rsx! {
        r#frame {
            name: {DynName(format!("{name}Divider"))},
            width: {TAB_ROW_W - 22.0},
            height: 1.0,
            background_color: TAB_DIVIDER_COLOR,
            anchor {
                point: AnchorPoint::Bottom,
                relative_point: AnchorPoint::Bottom,
                y: "1",
            }
        }
    }
}

fn tab_accent(name: &str) -> Element {
    rsx! {
        r#frame {
            name: {DynName(format!("{name}Accent"))},
            width: 3.0,
            height: {TAB_ACCENT_H},
            background_color: TAB_ACCENT_COLOR,
            anchor {
                point: AnchorPoint::Left,
                relative_point: AnchorPoint::Left,
                x: "8",
            }
        }
    }
}

fn build_content(model: &OptionsViewModel) -> Element {
    rsx! {
        panel {
            name: OPTIONS_CONTENT_PANEL,
            style: "inner_plain",
            width: {OPTIONS_CONTENT_W},
            height: {OPTIONS_CONTENT_H},
            anchor {
                point: AnchorPoint::TopLeft,
                relative_to: OPTIONS_DRAG_HANDLE,
                relative_point: AnchorPoint::BottomLeft,
                x: "236",
                y: "-18",
            }
            {content_header(model.category)}
            {content_body(model)}
        }
    }
}

fn content_header(category: OptionsCategory) -> Element {
    rsx! {
        fontstring {
            name: "OptionsSectionTitle",
            width: {OPTIONS_CONTENT_W - 30.0},
            height: 24.0,
            text: {category.title()},
            font_size: 22.0,
            color: "0.96,0.84,0.56,1.0",
            justify_h: "LEFT",
            anchor {
                point: AnchorPoint::TopLeft,
                relative_to: OPTIONS_CONTENT_PANEL,
                relative_point: AnchorPoint::TopLeft,
                x: "15",
                y: "-18",
            }
        }
    }
}

fn category_body(model: &OptionsViewModel) -> Element {
    match model.category {
        OptionsCategory::Graphics => options_menu_sections::graphics_body(),
        OptionsCategory::Sound => options_menu_active_sections::sound_body(&model.sound),
        OptionsCategory::Camera => options_menu_active_sections::camera_body(&model.camera),
        OptionsCategory::Interface => options_menu_active_sections::interface_body(&model.hud),
        OptionsCategory::Hud => options_menu_active_sections::hud_body(&model.hud),
        OptionsCategory::Controls => options_menu_sections::controls_body(),
        OptionsCategory::Accessibility => options_menu_sections::accessibility_body(),
        OptionsCategory::Keybindings => options_menu_sections::keybindings_body(),
        OptionsCategory::Macros => options_menu_sections::macros_body(),
        OptionsCategory::SocialAddons => options_menu_sections::social_addons_body(),
        OptionsCategory::Advanced => options_menu_active_sections::advanced_body(&model.hud),
        OptionsCategory::Support => options_menu_sections::support_body(),
    }
}

fn content_body(model: &OptionsViewModel) -> Element {
    let x = OPTIONS_CONTENT_INSET_X.to_string();
    let y = (-OPTIONS_CONTENT_INSET_TOP).to_string();
    rsx! {
        r#frame {
            name: OPTIONS_CONTENT_INNER,
            width: {OPTIONS_CONTENT_W - OPTIONS_CONTENT_INSET_X * 2.0},
            height: {OPTIONS_CONTENT_H - OPTIONS_CONTENT_INSET_TOP - 18.0},
            layout: "flex-column",
            gap: 12.0,
            anchor {
                point: AnchorPoint::TopLeft,
                relative_to: OPTIONS_CONTENT_PANEL,
                relative_point: AnchorPoint::TopLeft,
                x: {x},
                y: {y},
            }
            {category_body(model)}
        }
    }
}

fn build_footer() -> Element {
    let footer_buttons = footer_buttons();
    rsx! {
        r#frame {
            name: OPTIONS_FOOTER,
            width: 0.0,
            height: 0.0,
            layout: "flex-row",
            justify: "start",
            align: "center",
            gap: 12.0,
            anchor {
                point: AnchorPoint::BottomRight,
                relative_to: OPTIONS_ROOT,
                relative_point: AnchorPoint::BottomRight,
                x: {(-OPTIONS_FOOTER_RIGHT_INSET).to_string()},
                y: "20",
            }
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

fn footer_specs() -> [(&'static str, &'static str, &'static str, f32); 5] {
    [
        ("OptionsBackButton", "Back", ACTION_OPTIONS_BACK, 86.0),
        (
            "OptionsDefaultsButton",
            "Defaults",
            ACTION_OPTIONS_DEFAULTS,
            116.0,
        ),
        ("OptionsApplyButton", "Apply", ACTION_OPTIONS_APPLY, 94.0),
        ("OptionsCancelButton", "Cancel", ACTION_OPTIONS_CANCEL, 94.0),
        ("OptionsOkayButton", "Okay", ACTION_OPTIONS_OKAY, 94.0),
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
            height: 30.0,
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
