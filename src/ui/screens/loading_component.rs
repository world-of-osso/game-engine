use ui_toolkit::rsx;
use ui_toolkit::screen::SharedContext;
use ui_toolkit::widget_def::Element;

use crate::ui::anchor::FrameName;
use crate::ui::strata::FrameStrata;
use crate::ui::widgets::font_string::{FontColor, GameFont};

const TEX_LOADING_ART: &str = "data/ui/loading-screen-cathedral-bg-v1.png";
const TEX_GAME_LOGO: &str = "data/glues/common/world-of-osso-logo.ktx2";
pub const TEX_LOADING_BAR_LEFT: &str = "data/ui/loading-bar-steel-shell-left.png";
pub const TEX_LOADING_BAR_CENTER: &str = "data/ui/loading-bar-steel-shell-center.png";
pub const TEX_LOADING_BAR_RIGHT: &str = "data/ui/loading-bar-steel-shell-right.png";
pub const TEX_LOADING_BAR_FILL: &str = "data/ui/loading-bar-fill-v3-alchemical.png";

pub fn loading_bar_shell() -> ui_toolkit::frame::ThreeSlice {
    use ui_toolkit::widgets::texture::TextureSource;
    ui_toolkit::frame::ThreeSlice {
        cap_width: BAR_CAP_WIDTH,
        left: TextureSource::File(TEX_LOADING_BAR_LEFT.to_owned()),
        center: TextureSource::File(TEX_LOADING_BAR_CENTER.to_owned()),
        right: TextureSource::File(TEX_LOADING_BAR_RIGHT.to_owned()),
        color: [1.0; 4],
    }
}

const COLOR_GOLD: FontColor = FontColor::new(1.0, 0.82, 0.0, 1.0);
const COLOR_SUBTLE: FontColor = FontColor::new(0.95, 0.9, 0.78, 1.0);
const COLOR_TIP: FontColor = FontColor::new(0.78, 0.74, 0.66, 1.0);
const ART_WIDTH: f32 = 1280.0;
const ART_HEIGHT: f32 = 640.0;
const BAR_CAP_WIDTH: f32 = 25.0;
const BAR_FILL_START_X: f32 = 6.0;
const BAR_WIDTH: f32 = 610.0;
const BAR_HEIGHT: f32 = 32.0;
const BAR_FILL_MAX_WIDTH: f32 = BAR_WIDTH - (BAR_FILL_START_X * 2.0);
const BAR_FILL_HEIGHT: f32 = 23.0;
const PROGRESS_TEXT_X: f32 = -42.0;
const PROGRESS_TEXT_Y: f32 = -1.0;
const STATUS_TEXT_Y: f32 = -1.0;
const BAR_BOTTOM_MARGIN: f32 = 56.0;
const LOGO_TOP: f32 = 24.0;
const ZONE_TEXT_GAP: f32 = 8.0;
const TIP_TEXT_GAP: f32 = 6.0;
const ZONE_TEXT_HEIGHT: f32 = 28.0;
const TIP_TEXT_HEIGHT: f32 = 22.0;
const LOGO_WIDTH: f32 = 360.0;
const LOGO_HEIGHT: f32 = 140.0;

pub const LOADING_ROOT: FrameName = FrameName("LoadingRoot");
pub const LOADING_BAR_FILL: FrameName = FrameName("LoadingBarFill");
pub const LOADING_STATUS_TEXT: FrameName = FrameName("LoadingStatusText");
pub const LOADING_PROGRESS_TEXT: FrameName = FrameName("LoadingProgressText");

/// Zone line shown when the host has no resolved zone name.
pub const DEFAULT_ZONE_TEXT: &str = "Entering Elwynn Forest";
pub const DEFAULT_TIP_TEXT: &str =
    "Tip: The first zone load streams terrain and replicated actors before gameplay begins.";
/// Displayed bar fill speed; readiness jumps are eased at this rate instead of snapping.
pub const LOADING_BAR_FILL_RATE_PERCENT_PER_SEC: f32 = 6.0;

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct LoadingScreenState {
    pub status_text: String,
    pub zone_text: String,
    pub tip_text: String,
    pub progress_percent: u8,
}

impl LoadingScreenState {
    /// State for a host without a resolved zone: default zone and tip lines.
    pub fn with_default_text(status_text: &str, progress_percent: u8) -> Self {
        Self {
            status_text: status_text.to_owned(),
            zone_text: DEFAULT_ZONE_TEXT.to_owned(),
            tip_text: DEFAULT_TIP_TEXT.to_owned(),
            progress_percent,
        }
    }
}

/// Move the displayed percent toward readiness at the fixed fill rate, never past it.
pub fn advance_displayed_progress(current: f32, target: f32, delta_secs: f32) -> f32 {
    if delta_secs <= 0.0 {
        return current.min(target);
    }
    if current >= target {
        return target;
    }
    let step = delta_secs * LOADING_BAR_FILL_RATE_PERCENT_PER_SEC;
    (current + step).min(target)
}

/// Window height the loading artwork fills; hosts update it on resize.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct LoadingViewportHeight(pub f32);

#[derive(Clone, Debug, PartialEq)]
pub struct LoadingScreenLayout {
    pub art_width: f32,
    pub art_height: f32,
    pub bar_cap_width: f32,
    pub bar_fill_start_x: f32,
    pub bar_width: f32,
    pub bar_height: f32,
    pub bar_fill_max_width: f32,
    pub bar_fill_height: f32,
    pub progress_text_x: f32,
    pub progress_text_y: f32,
    pub status_text_y: f32,
    pub bar_bottom_margin: f32,
    pub logo_top: f32,
    pub zone_text_gap: f32,
    pub tip_text_gap: f32,
}

impl Default for LoadingScreenLayout {
    fn default() -> Self {
        Self {
            art_width: ART_WIDTH,
            art_height: ART_HEIGHT,
            bar_cap_width: BAR_CAP_WIDTH,
            bar_fill_start_x: BAR_FILL_START_X,
            bar_width: BAR_WIDTH,
            bar_height: BAR_HEIGHT,
            bar_fill_max_width: BAR_FILL_MAX_WIDTH,
            bar_fill_height: BAR_FILL_HEIGHT,
            progress_text_x: PROGRESS_TEXT_X,
            progress_text_y: PROGRESS_TEXT_Y,
            status_text_y: STATUS_TEXT_Y,
            bar_bottom_margin: BAR_BOTTOM_MARGIN,
            logo_top: LOGO_TOP,
            zone_text_gap: ZONE_TEXT_GAP,
            tip_text_gap: TIP_TEXT_GAP,
        }
    }
}

impl LoadingScreenLayout {
    /// Offset from the window bottom to the bar's vertical centre.
    fn bar_center_from_bottom(&self) -> f32 {
        self.bar_bottom_margin + self.bar_height / 2.0
    }
}

pub fn loading_screen(ctx: &SharedContext) -> Element {
    let state = ctx
        .get::<LoadingScreenState>()
        .expect("LoadingScreenState must be in SharedContext");
    let viewport_height = ctx
        .get::<LoadingViewportHeight>()
        .expect("LoadingViewportHeight must be in SharedContext")
        .0;
    let layout = ctx
        .get::<LoadingScreenLayout>()
        .cloned()
        .unwrap_or_default();
    rsx! {
        r#frame {
            name: LOADING_ROOT,
            pos_type: "absolute",
            left: 0.0, right: 0.0, top: 0.0, bottom: 0.0,
            width: "auto", height: "auto",
            background_color: "0.0,0.0,0.0,1.0",
            strata: FrameStrata::Background,
            {artwork_frame(&layout, viewport_height)}
            {logo_frame(&layout)}
            {zone_text(state, &layout)}
            {status_text(state, &layout)}
            {bar_background(state, &layout)}
            {progress_text(state.progress_percent, &layout)}
            {tip_text(state, &layout)}
        }
    }
}

/// Artwork fills the window height at its authored aspect; wider art crops at the sides.
fn artwork_frame(layout: &LoadingScreenLayout, viewport_height: f32) -> Element {
    let width = viewport_height * layout.art_width / layout.art_height;
    rsx! {
        texture {
            name: "LoadingArtwork",
            width,
            height: viewport_height,
            texture_file: TEX_LOADING_ART,
            strata: FrameStrata::Background,
            pos_type: "absolute",
            left: "50%", top: 0.0,
            translate_x: "-50%",
        }
    }
}

fn logo_frame(layout: &LoadingScreenLayout) -> Element {
    rsx! {
        texture {
            name: "LoadingLogo",
            width: LOGO_WIDTH,
            height: LOGO_HEIGHT,
            texture_file: TEX_GAME_LOGO,
            strata: FrameStrata::High,
            pos_type: "absolute",
            left: "50%", top: layout.logo_top,
            translate_x: "-50%",
        }
    }
}

fn zone_text(state: &LoadingScreenState, layout: &LoadingScreenLayout) -> Element {
    rsx! {
        fontstring {
            name: "LoadingZoneText",
            strata: FrameStrata::Medium,
            width: 560.0,
            height: ZONE_TEXT_HEIGHT,
            text: state.zone_text.clone(),
            font_size: 22.0,
            font: GameFont::FrizQuadrata,
            font_color: COLOR_GOLD,
            pos_type: "absolute",
            left: "50%", top: "100%",
            translate_x: "-50%", translate_y: "-100%",
            margin_top: {-(layout.bar_bottom_margin + layout.bar_height + layout.zone_text_gap)},
        }
    }
}

fn status_text(state: &LoadingScreenState, layout: &LoadingScreenLayout) -> Element {
    rsx! {
        fontstring {
            name: LOADING_STATUS_TEXT,
            strata: FrameStrata::Dialog,
            width: 420.0,
            height: 20.0,
            text: state.status_text.clone(),
            font_size: 13.0,
            font: GameFont::FrizQuadrata,
            font_color: COLOR_SUBTLE,
            pos_type: "absolute",
            left: "50%", top: "100%",
            translate_x: "-50%", translate_y: "-50%",
            margin_top: {-layout.bar_center_from_bottom() - layout.status_text_y},
        }
    }
}

fn bar_background(state: &LoadingScreenState, layout: &LoadingScreenLayout) -> Element {
    rsx! {
        r#frame {
            name: "LoadingBarBackground",
            width: layout.bar_width,
            height: layout.bar_height,
            three_slice_style: "loading_bar_shell",
            strata: FrameStrata::Medium,
            pos_type: "absolute",
            left: "50%", top: "100%",
            translate_x: "-50%", translate_y: "-100%",
            margin_top: {-layout.bar_bottom_margin},
            {bar_fill_clip(state.progress_percent, layout)}
        }
    }
}

fn bar_fill_clip(progress_percent: u8, layout: &LoadingScreenLayout) -> Element {
    let fill_width =
        layout.bar_fill_max_width * (f32::from(progress_percent).clamp(0.0, 100.0) / 100.0);
    rsx! {
        r#frame {
            name: "LoadingBarFillClip",
            width: layout.bar_fill_max_width,
            height: layout.bar_fill_height,
            background_color: "0.0,0.0,0.0,0.0",
            strata: FrameStrata::High,
            pos_type: "absolute",
            pos_x: layout.bar_fill_start_x,
            top: "50%",
            translate_y: "-50%",
            {bar_fill_texture(fill_width, layout)}
        }
    }
}

fn bar_fill_texture(fill_width: f32, layout: &LoadingScreenLayout) -> Element {
    rsx! {
        texture {
            name: LOADING_BAR_FILL,
            width: fill_width,
            height: layout.bar_fill_height,
            texture_file: TEX_LOADING_BAR_FILL,
            strata: FrameStrata::High,
            pos_type: "absolute",
            pos_x: 0.0, pos_y: 0.0,
        }
    }
}

fn progress_text(progress_percent: u8, layout: &LoadingScreenLayout) -> Element {
    let text = format!("{}%", progress_percent);
    rsx! {
        fontstring {
            name: LOADING_PROGRESS_TEXT,
            strata: FrameStrata::Dialog,
            width: 90.0,
            height: 18.0,
            text,
            font_size: 15.0,
            font: GameFont::FrizQuadrata,
            font_color: COLOR_GOLD,
            pos_type: "absolute",
            left: "50%", top: "100%",
            translate_x: "-100%", translate_y: "-50%",
            margin_left: {layout.bar_width / 2.0 + layout.progress_text_x},
            margin_top: {-layout.bar_center_from_bottom() - layout.progress_text_y},
        }
    }
}

fn tip_text(state: &LoadingScreenState, layout: &LoadingScreenLayout) -> Element {
    rsx! {
        fontstring {
            name: "LoadingTipText",
            strata: FrameStrata::Medium,
            width: 980.0,
            height: TIP_TEXT_HEIGHT,
            text: state.tip_text.clone(),
            font_size: 14.0,
            font: GameFont::FrizQuadrata,
            font_color: COLOR_TIP,
            pos_type: "absolute",
            left: "50%", top: "100%",
            translate_x: "-50%",
            margin_top: {-layout.bar_bottom_margin + layout.tip_text_gap},
        }
    }
}

#[cfg(debug_assertions)]
pub fn debug_loading_layout_from_source() -> LoadingScreenLayout {
    let path = std::path::Path::new("src/ui/screens/loading_component.rs");
    let Ok(source) = std::fs::read_to_string(path) else {
        return LoadingScreenLayout::default();
    };
    let mut layout = LoadingScreenLayout::default();
    for line in source.lines() {
        apply_debug_const_override(line, &mut layout);
    }
    layout
}

#[cfg(not(debug_assertions))]
pub fn debug_loading_layout_from_source() -> LoadingScreenLayout {
    LoadingScreenLayout::default()
}

#[cfg(debug_assertions)]
fn apply_debug_const_override(line: &str, layout: &mut LoadingScreenLayout) {
    let Some((name, value)) = parse_layout_const_override(line) else {
        return;
    };
    apply_named_layout_override(name, value, layout);
}

#[cfg(debug_assertions)]
fn parse_layout_const_override(line: &str) -> Option<(&str, f32)> {
    let line = line.trim().strip_prefix("const ")?;
    let (name, value) = line.split_once(": f32 = ")?;
    let value = value.strip_suffix(';')?;
    let value = value.parse::<f32>().ok()?;
    Some((name, value))
}

#[cfg(debug_assertions)]
struct LayoutConstOverride {
    name: &'static str,
    setter: fn(&mut LoadingScreenLayout, f32),
}

#[cfg(debug_assertions)]
const LAYOUT_CONST_OVERRIDES: &[LayoutConstOverride] = &[
    LayoutConstOverride {
        name: "ART_WIDTH",
        setter: set_art_width,
    },
    LayoutConstOverride {
        name: "ART_HEIGHT",
        setter: set_art_height,
    },
    LayoutConstOverride {
        name: "BAR_CAP_WIDTH",
        setter: set_bar_cap_width,
    },
    LayoutConstOverride {
        name: "BAR_FILL_START_X",
        setter: set_bar_fill_start_x,
    },
    LayoutConstOverride {
        name: "BAR_WIDTH",
        setter: set_bar_width,
    },
    LayoutConstOverride {
        name: "BAR_HEIGHT",
        setter: set_bar_height,
    },
    LayoutConstOverride {
        name: "BAR_FILL_MAX_WIDTH",
        setter: set_bar_fill_max_width,
    },
    LayoutConstOverride {
        name: "BAR_FILL_HEIGHT",
        setter: set_bar_fill_height,
    },
    LayoutConstOverride {
        name: "PROGRESS_TEXT_X",
        setter: set_progress_text_x,
    },
    LayoutConstOverride {
        name: "PROGRESS_TEXT_Y",
        setter: set_progress_text_y,
    },
    LayoutConstOverride {
        name: "STATUS_TEXT_Y",
        setter: set_status_text_y,
    },
    LayoutConstOverride {
        name: "BAR_BOTTOM_MARGIN",
        setter: set_bar_bottom_margin,
    },
    LayoutConstOverride {
        name: "LOGO_TOP",
        setter: set_logo_top,
    },
    LayoutConstOverride {
        name: "ZONE_TEXT_GAP",
        setter: set_zone_text_gap,
    },
    LayoutConstOverride {
        name: "TIP_TEXT_GAP",
        setter: set_tip_text_gap,
    },
];

#[cfg(debug_assertions)]
fn apply_named_layout_override(name: &str, value: f32, layout: &mut LoadingScreenLayout) {
    let Some(override_entry) = LAYOUT_CONST_OVERRIDES
        .iter()
        .find(|entry| entry.name == name)
    else {
        return;
    };
    (override_entry.setter)(layout, value);
}

#[cfg(debug_assertions)]
fn set_art_width(layout: &mut LoadingScreenLayout, value: f32) {
    layout.art_width = value;
}

#[cfg(debug_assertions)]
fn set_art_height(layout: &mut LoadingScreenLayout, value: f32) {
    layout.art_height = value;
}

#[cfg(debug_assertions)]
fn set_bar_cap_width(layout: &mut LoadingScreenLayout, value: f32) {
    layout.bar_cap_width = value;
}

#[cfg(debug_assertions)]
fn set_bar_fill_start_x(layout: &mut LoadingScreenLayout, value: f32) {
    layout.bar_fill_start_x = value;
}

#[cfg(debug_assertions)]
fn set_bar_width(layout: &mut LoadingScreenLayout, value: f32) {
    layout.bar_width = value;
}

#[cfg(debug_assertions)]
fn set_bar_height(layout: &mut LoadingScreenLayout, value: f32) {
    layout.bar_height = value;
}

#[cfg(debug_assertions)]
fn set_bar_fill_max_width(layout: &mut LoadingScreenLayout, value: f32) {
    layout.bar_fill_max_width = value;
}

#[cfg(debug_assertions)]
fn set_bar_fill_height(layout: &mut LoadingScreenLayout, value: f32) {
    layout.bar_fill_height = value;
}

#[cfg(debug_assertions)]
fn set_progress_text_x(layout: &mut LoadingScreenLayout, value: f32) {
    layout.progress_text_x = value;
}

#[cfg(debug_assertions)]
fn set_progress_text_y(layout: &mut LoadingScreenLayout, value: f32) {
    layout.progress_text_y = value;
}

#[cfg(debug_assertions)]
fn set_status_text_y(layout: &mut LoadingScreenLayout, value: f32) {
    layout.status_text_y = value;
}

#[cfg(debug_assertions)]
fn set_bar_bottom_margin(layout: &mut LoadingScreenLayout, value: f32) {
    layout.bar_bottom_margin = value;
}

#[cfg(debug_assertions)]
fn set_logo_top(layout: &mut LoadingScreenLayout, value: f32) {
    layout.logo_top = value;
}

#[cfg(debug_assertions)]
fn set_zone_text_gap(layout: &mut LoadingScreenLayout, value: f32) {
    layout.zone_text_gap = value;
}

#[cfg(debug_assertions)]
fn set_tip_text_gap(layout: &mut LoadingScreenLayout, value: f32) {
    layout.tip_text_gap = value;
}

#[cfg(all(test, debug_assertions))]
mod tests {
    use super::*;

    #[test]
    fn apply_debug_const_override_updates_known_layout_const() {
        let mut layout = LoadingScreenLayout::default();
        apply_debug_const_override("const BAR_BOTTOM_MARGIN: f32 = 24.5;", &mut layout);
        assert_eq!(layout.bar_bottom_margin, 24.5);
    }

    #[test]
    fn apply_debug_const_override_ignores_unknown_layout_const() {
        let mut layout = LoadingScreenLayout::default();
        let before = layout.clone();
        apply_debug_const_override("const DOES_NOT_EXIST: f32 = 12.0;", &mut layout);
        assert_eq!(layout, before);
    }

    #[test]
    fn apply_debug_const_override_ignores_non_const_lines() {
        let mut layout = LoadingScreenLayout::default();
        let before = layout.clone();
        apply_debug_const_override("fn bar_bottom_margin() -> f32 { 10.0 }", &mut layout);
        assert_eq!(layout, before);
    }
}
