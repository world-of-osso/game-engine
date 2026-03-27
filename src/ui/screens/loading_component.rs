use ui_toolkit::rsx;
use ui_toolkit::screen::SharedContext;
use ui_toolkit::widget_def::Element;

use crate::ui::anchor::{AnchorPoint, FrameName};
use crate::ui::frame::NineSlice;
use crate::ui::registry::FrameRegistry;
use crate::ui::strata::FrameStrata;
use crate::ui::widgets::font_string::{FontColor, GameFont};
use crate::ui::widgets::texture::TextureSource;

const TEX_LOADING_ART: &str = "data/ui/loading-screen-cathedral-bg-v1.png";
const TEX_LOADING_FILLER_TOP: &str = "data/ui/loading-screen-parchment-band-top-v1.png";
const TEX_LOADING_FILLER_BOTTOM: &str = "data/ui/loading-screen-parchment-band-bottom-v1.png";
const TEX_GAME_LOGO: &str = "data/glues/common/world-of-osso-logo.ktx2";
pub const TEX_LOADING_BAR_LEFT: &str = "data/ui/loading-bar-steel-shell-left.png";
pub const TEX_LOADING_BAR_CENTER: &str = "data/ui/loading-bar-steel-shell-center.png";
pub const TEX_LOADING_BAR_RIGHT: &str = "data/ui/loading-bar-steel-shell-right.png";
pub const TEX_LOADING_BAR_TRACK: &str = "data/ui/loading-bar-track-steel-flat.png";
pub const TEX_LOADING_BAR_FILL: &str = "data/ui/loading-bar-fill-v2-flat.png";

const COLOR_GOLD: FontColor = FontColor::new(1.0, 0.82, 0.0, 1.0);
const COLOR_SUBTLE: FontColor = FontColor::new(0.95, 0.9, 0.78, 1.0);
const COLOR_TIP: FontColor = FontColor::new(0.78, 0.74, 0.66, 1.0);
const ART_WIDTH: f32 = 1280.0;
const ART_HEIGHT: f32 = 640.0;
const FILLER_WIDTH: f32 = 2048.0;
const FILLER_HEIGHT: f32 = 96.0;
const BAR_CAP_WIDTH: f32 = 25.0;
const BAR_WIDTH: f32 = 610.0;
const BAR_HEIGHT: f32 = 32.0;
const BAR_FILL_MAX_WIDTH: f32 = 524.0;
const BAR_FILL_HEIGHT: f32 = 21.0;
const PROGRESS_TEXT_X: f32 = -42.0;
const PROGRESS_TEXT_Y: f32 = -1.0;
const STATUS_TEXT_Y: f32 = -1.0;
const BAR_Y: f32 = -18.0;
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

pub fn loading_screen(ctx: &SharedContext) -> Element {
    let state = ctx
        .get::<LoadingScreenState>()
        .expect("LoadingScreenState must be in SharedContext");
    [
        background_frame(),
        filler_bands(),
        artwork_frame(),
        logo_frame(),
        zone_text(state),
        status_text(state),
        bar_background(),
        bar_fill_clip(state.progress_percent),
        progress_text(state.progress_percent),
        tip_text(state),
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

fn filler_bands() -> Element {
    rsx! {
        texture {
            name: "LoadingTopFiller",
            width: FILLER_WIDTH,
            height: FILLER_HEIGHT,
            texture_file: TEX_LOADING_FILLER_TOP,
            strata: FrameStrata::Background,
            anchor {
                point: AnchorPoint::Bottom,
                relative_to: FrameName("LoadingArtwork"),
                relative_point: AnchorPoint::Top,
            }
        }
        texture {
            name: "LoadingBottomFiller",
            width: FILLER_WIDTH,
            height: FILLER_HEIGHT,
            texture_file: TEX_LOADING_FILLER_BOTTOM,
            strata: FrameStrata::Background,
            anchor {
                point: AnchorPoint::Top,
                relative_to: FrameName("LoadingArtwork"),
                relative_point: AnchorPoint::Bottom,
            }
        }
    }
}

fn artwork_frame() -> Element {
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
            width: ART_WIDTH,
            height: ART_HEIGHT,
            texture_file: TEX_LOADING_ART,
            strata: FrameStrata::Background,
            anchor {
                point: AnchorPoint::Center,
                relative_point: AnchorPoint::Center,
                y: "-18",
            }
        }
    }
}

fn logo_frame() -> Element {
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
                y: "34",
            }
        }
    }
}

fn zone_text(state: &LoadingScreenState) -> Element {
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
                y: {ZONE_TEXT_Y.to_string()},
            }
        }
    }
}

fn status_text(state: &LoadingScreenState) -> Element {
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
                y: STATUS_TEXT_Y,
            }
        }
    }
}

fn bar_background() -> Element {
    rsx! {
        r#frame {
            name: "LoadingBarBackground",
            width: BAR_WIDTH,
            height: BAR_HEIGHT,
            strata: FrameStrata::Medium,
            anchor {
                point: AnchorPoint::Bottom,
                relative_to: FrameName("LoadingArtwork"),
                relative_point: AnchorPoint::Bottom,
                y: {BAR_Y.to_string()},
            }
        }
    }
}

fn bar_fill_clip(progress_percent: u8) -> Element {
    let fill_width = BAR_FILL_MAX_WIDTH * (f32::from(progress_percent).clamp(0.0, 100.0) / 100.0);
    rsx! {
        r#frame {
            name: "LoadingBarFillClip",
            width: BAR_FILL_MAX_WIDTH,
            height: BAR_FILL_HEIGHT,
            background_color: "0.0,0.0,0.0,0.0",
            strata: FrameStrata::High,
            anchor {
                point: AnchorPoint::Center,
                relative_to: FrameName("LoadingBarBackground"),
                relative_point: AnchorPoint::Center,
            }
            {bar_track_texture()}
            {bar_fill_texture(fill_width)}
        }
    }
}

fn bar_track_texture() -> Element {
    rsx! {
        texture {
            name: "LoadingBarTrack",
            width: BAR_FILL_MAX_WIDTH,
            height: BAR_FILL_HEIGHT,
            texture_file: TEX_LOADING_BAR_TRACK,
            strata: FrameStrata::High,
            anchor {
                point: AnchorPoint::Left,
                relative_point: AnchorPoint::Left,
            }
        }
    }
}

fn bar_fill_texture(fill_width: f32) -> Element {
    rsx! {
        texture {
            name: LOADING_BAR_FILL,
            width: fill_width,
            height: BAR_FILL_HEIGHT,
            texture_file: TEX_LOADING_BAR_FILL,
            strata: FrameStrata::High,
            anchor {
                point: AnchorPoint::Left,
                relative_point: AnchorPoint::Left,
            }
        }
    }
}

/// Apply nine-slice to the loading bar background frame (3-slice: left cap, center, right cap).
pub fn apply_bar_nine_slice(reg: &mut FrameRegistry) {
    let Some(id) = reg.get_by_name("LoadingBarBackground") else {
        return;
    };
    let Some(frame) = reg.get_mut(id) else {
        return;
    };
    frame.nine_slice = Some(NineSlice {
        edge_sizes: Some([BAR_CAP_WIDTH, 0.0, BAR_CAP_WIDTH, 0.0]),
        part_textures: Some(bar_shell_part_textures()),
        border_color: [1.0, 1.0, 1.0, 1.0],
        bg_color: [1.0, 1.0, 1.0, 1.0],
        ..Default::default()
    });
}

fn bar_shell_part_textures() -> [TextureSource; 9] {
    [
        TextureSource::None,                                     // TL
        TextureSource::None,                                     // T
        TextureSource::None,                                     // TR
        TextureSource::File(TEX_LOADING_BAR_LEFT.to_string()),   // L
        TextureSource::File(TEX_LOADING_BAR_CENTER.to_string()), // C
        TextureSource::File(TEX_LOADING_BAR_RIGHT.to_string()),  // R
        TextureSource::None,                                     // BL
        TextureSource::None,                                     // B
        TextureSource::None,                                     // BR
    ]
}

fn progress_text(progress_percent: u8) -> Element {
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
                x: PROGRESS_TEXT_X,
                y: PROGRESS_TEXT_Y,
            }
        }
    }
}

fn tip_text(state: &LoadingScreenState) -> Element {
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
                y: {TIP_TEXT_Y.to_string()},
            }
        }
    }
}
