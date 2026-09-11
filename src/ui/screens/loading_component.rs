use crate::ui::anchor::FrameName;

pub const TEX_LOADING_ART: &str = "data/ui/loading-screen-cathedral-bg-v1.png";
pub const TEX_LOADING_FILLER_TOP: &str = "data/ui/loading-screen-parchment-band-top-v1.png";
pub const TEX_LOADING_FILLER_BOTTOM: &str = "data/ui/loading-screen-parchment-band-bottom-v1.png";
pub const TEX_GAME_LOGO: &str = "data/glues/common/world-of-osso-logo.ktx2";
pub const TEX_LOADING_BAR_LEFT: &str = "data/ui/loading-bar-steel-shell-left.png";
pub const TEX_LOADING_BAR_CENTER: &str = "data/ui/loading-bar-steel-shell-center.png";
pub const TEX_LOADING_BAR_RIGHT: &str = "data/ui/loading-bar-steel-shell-right.png";
pub const TEX_LOADING_BAR_FILL: &str = "data/ui/loading-bar-fill-v3-alchemical.png";

const ART_WIDTH: f32 = 1280.0;
const ART_HEIGHT: f32 = 640.0;
const FILLER_WIDTH: f32 = 2048.0;
const FILLER_HEIGHT: f32 = 160.0;
const FILLER_TOP_Y: f32 = 0.0;
const FILLER_BOTTOM_Y: f32 = 0.0;
const BAR_CAP_WIDTH: f32 = 25.0;
const BAR_FILL_START_X: f32 = 6.0;
const BAR_WIDTH: f32 = 610.0;
const BAR_HEIGHT: f32 = 32.0;
const BAR_FILL_MAX_WIDTH: f32 = BAR_WIDTH - (BAR_FILL_START_X * 2.0);
const BAR_FILL_HEIGHT: f32 = 23.0;
const PROGRESS_TEXT_X: f32 = -42.0;
const PROGRESS_TEXT_Y: f32 = -1.0;
const STATUS_TEXT_Y: f32 = -1.0;
const BAR_Y: f32 = -10.0;
const LOGO_Y: f32 = -150.0;
const ZONE_TEXT_Y: f32 = 8.0;
const TIP_TEXT_Y: f32 = -5.0;

pub const LOADING_ROOT: FrameName = FrameName("LoadingRoot");
pub const LOADING_BAR_FILL: FrameName = FrameName("LoadingBarFill");
pub const LOADING_STATUS_TEXT: FrameName = FrameName("LoadingStatusText");
pub const LOADING_PROGRESS_TEXT: FrameName = FrameName("LoadingProgressText");

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct LoadingScreenState {
    pub status_text: String,
    pub zone_text: String,
    pub tip_text: String,
    pub progress_percent: u8,
}

#[derive(Clone, Debug, PartialEq)]
pub struct LoadingScreenLayout {
    pub art_width: f32,
    pub art_height: f32,
    pub filler_width: f32,
    pub filler_height: f32,
    pub filler_top_y: f32,
    pub filler_bottom_y: f32,
    pub bar_cap_width: f32,
    pub bar_fill_start_x: f32,
    pub bar_width: f32,
    pub bar_height: f32,
    pub bar_fill_max_width: f32,
    pub bar_fill_height: f32,
    pub progress_text_x: f32,
    pub progress_text_y: f32,
    pub status_text_y: f32,
    pub bar_y: f32,
    pub logo_y: f32,
    pub zone_text_y: f32,
    pub tip_text_y: f32,
}

impl Default for LoadingScreenLayout {
    fn default() -> Self {
        Self {
            art_width: ART_WIDTH,
            art_height: ART_HEIGHT,
            filler_width: FILLER_WIDTH,
            filler_height: FILLER_HEIGHT,
            filler_top_y: FILLER_TOP_Y,
            filler_bottom_y: FILLER_BOTTOM_Y,
            bar_cap_width: BAR_CAP_WIDTH,
            bar_fill_start_x: BAR_FILL_START_X,
            bar_width: BAR_WIDTH,
            bar_height: BAR_HEIGHT,
            bar_fill_max_width: BAR_FILL_MAX_WIDTH,
            bar_fill_height: BAR_FILL_HEIGHT,
            progress_text_x: PROGRESS_TEXT_X,
            progress_text_y: PROGRESS_TEXT_Y,
            status_text_y: STATUS_TEXT_Y,
            bar_y: BAR_Y,
            logo_y: LOGO_Y,
            zone_text_y: ZONE_TEXT_Y,
            tip_text_y: TIP_TEXT_Y,
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
        name: "FILLER_WIDTH",
        setter: set_filler_width,
    },
    LayoutConstOverride {
        name: "FILLER_HEIGHT",
        setter: set_filler_height,
    },
    LayoutConstOverride {
        name: "FILLER_TOP_Y",
        setter: set_filler_top_y,
    },
    LayoutConstOverride {
        name: "FILLER_BOTTOM_Y",
        setter: set_filler_bottom_y,
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
        name: "BAR_Y",
        setter: set_bar_y,
    },
    LayoutConstOverride {
        name: "LOGO_Y",
        setter: set_logo_y,
    },
    LayoutConstOverride {
        name: "ZONE_TEXT_Y",
        setter: set_zone_text_y,
    },
    LayoutConstOverride {
        name: "TIP_TEXT_Y",
        setter: set_tip_text_y,
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
fn set_filler_width(layout: &mut LoadingScreenLayout, value: f32) {
    layout.filler_width = value;
}

#[cfg(debug_assertions)]
fn set_filler_height(layout: &mut LoadingScreenLayout, value: f32) {
    layout.filler_height = value;
}

#[cfg(debug_assertions)]
fn set_filler_top_y(layout: &mut LoadingScreenLayout, value: f32) {
    layout.filler_top_y = value;
}

#[cfg(debug_assertions)]
fn set_filler_bottom_y(layout: &mut LoadingScreenLayout, value: f32) {
    layout.filler_bottom_y = value;
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
fn set_bar_y(layout: &mut LoadingScreenLayout, value: f32) {
    layout.bar_y = value;
}

#[cfg(debug_assertions)]
fn set_logo_y(layout: &mut LoadingScreenLayout, value: f32) {
    layout.logo_y = value;
}

#[cfg(debug_assertions)]
fn set_zone_text_y(layout: &mut LoadingScreenLayout, value: f32) {
    layout.zone_text_y = value;
}

#[cfg(debug_assertions)]
fn set_tip_text_y(layout: &mut LoadingScreenLayout, value: f32) {
    layout.tip_text_y = value;
}

#[cfg(all(test, debug_assertions))]
mod tests {
    use super::*;

    #[test]
    fn apply_debug_const_override_updates_known_layout_const() {
        let mut layout = LoadingScreenLayout::default();
        apply_debug_const_override("const BAR_Y: f32 = -24.5;", &mut layout);
        assert_eq!(layout.bar_y, -24.5);
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
        apply_debug_const_override("fn bar_y() -> f32 { -10.0 }", &mut layout);
        assert_eq!(layout, before);
    }
}
