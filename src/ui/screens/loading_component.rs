use ui_toolkit::rsx;
use ui_toolkit::screen::SharedContext;
use ui_toolkit::widget_def::Element;

use crate::ui::anchor::{AnchorPoint, FrameName};
use crate::ui::strata::FrameStrata;
use crate::ui::widgets::font_string::{FontColor, GameFont};

const TEX_LOADING_ART: &str = "data/ui/loading-screen-cathedral-bg-v1.png";
const TEX_LOADING_FILLER_TOP: &str = "data/ui/loading-screen-parchment-band-top-v1.png";
const TEX_LOADING_FILLER_BOTTOM: &str = "data/ui/loading-screen-parchment-band-bottom-v1.png";
const TEX_GAME_LOGO: &str = "data/glues/common/world-of-osso-logo.ktx2";
pub const TEX_LOADING_BAR_LEFT: &str = "data/ui/loading-bar-steel-shell-left.png";
pub const TEX_LOADING_BAR_CENTER: &str = "data/ui/loading-bar-steel-shell-center.png";
pub const TEX_LOADING_BAR_RIGHT: &str = "data/ui/loading-bar-steel-shell-right.png";
pub const TEX_LOADING_BAR_FILL: &str = "data/ui/loading-bar-fill-v2-flat.png";

const COLOR_GOLD: FontColor = FontColor::new(1.0, 0.82, 0.0, 1.0);
const COLOR_SUBTLE: FontColor = FontColor::new(0.95, 0.9, 0.78, 1.0);
const COLOR_TIP: FontColor = FontColor::new(0.78, 0.74, 0.66, 1.0);
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

pub fn loading_screen(ctx: &SharedContext) -> Element {
    let state = ctx
        .get::<LoadingScreenState>()
        .expect("LoadingScreenState must be in SharedContext");
    let layout = ctx
        .get::<LoadingScreenLayout>()
        .cloned()
        .unwrap_or_default();
    [
        background_frame(),
        filler_bands(&layout),
        artwork_frame(&layout),
        logo_frame(&layout),
        zone_text(state, &layout),
        status_text(state, &layout),
        bar_background(&layout),
        bar_fill_clip(state.progress_percent, &layout),
        progress_text(state.progress_percent, &layout),
        tip_text(state, &layout),
    ]
    .into_iter()
    .flatten()
    .collect()
}

fn background_frame() -> Element {
    rsx! {
        r#frame {
            name: LOADING_ROOT,
            stretch: true,
            background_color: "0.0,0.0,0.0,1.0",
            strata: FrameStrata::Background,
        }
    }
}

fn filler_bands(layout: &LoadingScreenLayout) -> Element {
    rsx! {
        texture {
            name: "LoadingTopFiller",
            width: layout.filler_width,
            height: layout.filler_height,
            texture_file: TEX_LOADING_FILLER_TOP,
            strata: FrameStrata::Background,
            anchor {
                point: AnchorPoint::Bottom,
                relative_to: FrameName("LoadingArtwork"),
                relative_point: AnchorPoint::Top,
                y: {layout.filler_top_y.to_string()},
            }
        }
        texture {
            name: "LoadingBottomFiller",
            width: layout.filler_width,
            height: layout.filler_height,
            texture_file: TEX_LOADING_FILLER_BOTTOM,
            strata: FrameStrata::Background,
            anchor {
                point: AnchorPoint::Top,
                relative_to: FrameName("LoadingArtwork"),
                relative_point: AnchorPoint::Bottom,
                y: {layout.filler_bottom_y.to_string()},
            }
        }
    }
}

fn artwork_frame(layout: &LoadingScreenLayout) -> Element {
    rsx! {
        r#frame {
            name: "LoadingArtworkMatte",
            width: 1328.0,
            height: 704.0,
            background_color: "0.0,0.0,0.0,0.82",
            strata: FrameStrata::Background,
            anchor {
                point: AnchorPoint::Center,
                relative_point: AnchorPoint::Center,
                y: "-18",
            }
        }
        texture {
            name: "LoadingArtwork",
            width: layout.art_width,
            height: layout.art_height,
            texture_file: TEX_LOADING_ART,
            strata: FrameStrata::Background,
            anchor {
                point: AnchorPoint::Center,
                relative_point: AnchorPoint::Center,
                y: "-50",
            }
        }
    }
}

fn logo_frame(layout: &LoadingScreenLayout) -> Element {
    rsx! {
        texture {
            name: "LoadingLogo",
            width: 360.0,
            height: 140.0,
            texture_file: TEX_GAME_LOGO,
            strata: FrameStrata::High,
            anchor {
                point: AnchorPoint::Bottom,
                relative_to: FrameName("LoadingArtwork"),
                relative_point: AnchorPoint::Top,
                y: {layout.logo_y.to_string()},
            }
        }
    }
}

fn zone_text(state: &LoadingScreenState, layout: &LoadingScreenLayout) -> Element {
    rsx! {
        fontstring {
            name: "LoadingZoneText",
            width: 560.0,
            height: 28.0,
            text: state.zone_text.clone(),
            font_size: 22.0,
            font: GameFont::FrizQuadrata,
            font_color: COLOR_GOLD,
            anchor {
                point: AnchorPoint::Bottom,
                relative_to: FrameName("LoadingBarBackground"),
                relative_point: AnchorPoint::Top,
                y: {layout.zone_text_y.to_string()},
            }
        }
    }
}

fn status_text(state: &LoadingScreenState, layout: &LoadingScreenLayout) -> Element {
    rsx! {
        fontstring {
            name: LOADING_STATUS_TEXT,
            width: 420.0,
            height: 20.0,
            text: state.status_text.clone(),
            font_size: 13.0,
            font: GameFont::FrizQuadrata,
            font_color: COLOR_SUBTLE,
            anchor {
                point: AnchorPoint::Center,
                relative_to: FrameName("LoadingBarBackground"),
                relative_point: AnchorPoint::Center,
                y: {layout.status_text_y.to_string()},
            }
        }
    }
}

fn bar_background(layout: &LoadingScreenLayout) -> Element {
    rsx! {
        r#frame {
            name: "LoadingBarBackground",
            width: layout.bar_width,
            height: layout.bar_height,
            three_slice_style: "loading_bar_shell",
            strata: FrameStrata::Medium,
            anchor {
                point: AnchorPoint::Bottom,
                relative_to: FrameName("LoadingArtwork"),
                relative_point: AnchorPoint::Bottom,
                y: {layout.bar_y.to_string()},
            }
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
            anchor {
                point: AnchorPoint::Left,
                relative_to: FrameName("LoadingBarBackground"),
                relative_point: AnchorPoint::Left,
                x: {layout.bar_fill_start_x.to_string()},
            }
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
            anchor {
                point: AnchorPoint::Left,
                relative_point: AnchorPoint::Left,
            }
        }
    }
}

fn progress_text(progress_percent: u8, layout: &LoadingScreenLayout) -> Element {
    let text = format!("{}%", progress_percent);
    rsx! {
        fontstring {
            name: LOADING_PROGRESS_TEXT,
            width: 90.0,
            height: 18.0,
            text,
            font_size: 15.0,
            font: GameFont::FrizQuadrata,
            font_color: COLOR_GOLD,
            anchor {
                point: AnchorPoint::Right,
                relative_to: FrameName("LoadingBarBackground"),
                relative_point: AnchorPoint::Right,
                x: {layout.progress_text_x.to_string()},
                y: {layout.progress_text_y.to_string()},
            }
        }
    }
}

fn tip_text(state: &LoadingScreenState, layout: &LoadingScreenLayout) -> Element {
    rsx! {
        fontstring {
            name: "LoadingTipText",
            width: 980.0,
            height: 22.0,
            text: state.tip_text.clone(),
            font_size: 14.0,
            font: GameFont::FrizQuadrata,
            font_color: COLOR_TIP,
            anchor {
                point: AnchorPoint::Top,
                relative_to: FrameName("LoadingBarBackground"),
                relative_point: AnchorPoint::Bottom,
                y: {layout.tip_text_y.to_string()},
            }
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
    let Some(line) = line.trim().strip_prefix("const ") else {
        return;
    };
    let Some((name, value)) = line.split_once(": f32 = ") else {
        return;
    };
    let Some(value) = value.strip_suffix(';') else {
        return;
    };
    let Ok(value) = value.parse::<f32>() else {
        return;
    };
    match name {
        "ART_WIDTH" => layout.art_width = value,
        "ART_HEIGHT" => layout.art_height = value,
        "FILLER_WIDTH" => layout.filler_width = value,
        "FILLER_HEIGHT" => layout.filler_height = value,
        "FILLER_TOP_Y" => layout.filler_top_y = value,
        "FILLER_BOTTOM_Y" => layout.filler_bottom_y = value,
        "BAR_CAP_WIDTH" => layout.bar_cap_width = value,
        "BAR_FILL_START_X" => layout.bar_fill_start_x = value,
        "BAR_WIDTH" => layout.bar_width = value,
        "BAR_HEIGHT" => layout.bar_height = value,
        "BAR_FILL_MAX_WIDTH" => layout.bar_fill_max_width = value,
        "BAR_FILL_HEIGHT" => layout.bar_fill_height = value,
        "PROGRESS_TEXT_X" => layout.progress_text_x = value,
        "PROGRESS_TEXT_Y" => layout.progress_text_y = value,
        "STATUS_TEXT_Y" => layout.status_text_y = value,
        "BAR_Y" => layout.bar_y = value,
        "LOGO_Y" => layout.logo_y = value,
        "ZONE_TEXT_Y" => layout.zone_text_y = value,
        "TIP_TEXT_Y" => layout.tip_text_y = value,
        _ => {}
    }
}
