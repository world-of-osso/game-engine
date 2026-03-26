use ui_toolkit::rsx;
use ui_toolkit::screen::SharedContext;
use ui_toolkit::widget_def::Element;

use crate::ui::anchor::{AnchorPoint, FrameName};
use crate::ui::strata::FrameStrata;
use crate::ui::widgets::font_string::{FontColor, GameFont};

const TEX_LOADING_ART: &str = "/syncthing/World of Warcraft/_retail_/BlizzardInterfaceArt/Interface/GLUES/LOADINGSCREENS/Expansion10/Main/Loadscreen_Housing_ElwynnForest.blp";
const TEX_GAME_LOGO: &str = "data/glues/common/world-of-osso-logo.ktx2";
pub const TEX_LOADING_BAR_BG: &str = "/syncthing/World of Warcraft/_retail_/BlizzardInterfaceArt/Interface/GLUES/LoadingBar/Loading-BarBorder-Background-v2.blp";
pub const TEX_LOADING_BAR_FILL: &str = "/syncthing/World of Warcraft/_retail_/BlizzardInterfaceArt/Interface/GLUES/LoadingBar/Loading-BarFill.blp";
pub const TEX_LOADING_BAR_FRAME: &str = "/syncthing/World of Warcraft/_retail_/BlizzardInterfaceArt/Interface/GLUES/LoadingBar/Loading-BarBorder-Frame-v2.blp";

const COLOR_GOLD: FontColor = FontColor::new(1.0, 0.82, 0.0, 1.0);
const COLOR_SUBTLE: FontColor = FontColor::new(0.95, 0.9, 0.78, 1.0);
const COLOR_TIP: FontColor = FontColor::new(0.78, 0.74, 0.66, 1.0);
const BAR_FILL_MAX_WIDTH: f32 = 524.0;

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
        artwork_frame(),
        logo_frame(),
        zone_text(state),
        status_text(state),
        bar_background(),
        bar_fill_clip(state.progress_percent),
        bar_frame(),
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

fn artwork_frame() -> Element {
    rsx! {
        r#frame {
            name: "LoadingArtworkMatte",
            width: 1080.0,
            height: 700.0,
            background_color: "0.0,0.0,0.0,1.0",
            strata: FrameStrata::Background,
            anchor {
                point: AnchorPoint::Center,
                relative_point: AnchorPoint::Center,
                y: "12",
            }
        }
        texture {
            name: "LoadingArtwork",
            width: 1024.0,
            height: 640.0,
            texture_file: TEX_LOADING_ART,
            strata: FrameStrata::Background,
            anchor {
                point: AnchorPoint::Center,
                relative_point: AnchorPoint::Center,
                y: "24",
            }
        }
    }
}

fn logo_frame() -> Element {
    rsx! {
        texture {
            name: "LoadingLogo",
            width: 420.0,
            height: 164.0,
            texture_file: TEX_GAME_LOGO,
            strata: FrameStrata::High,
            anchor {
                point: AnchorPoint::Top,
                relative_point: AnchorPoint::Top,
                y: "-36",
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
                relative_to: FrameName("LoadingArtwork"),
                relative_point: AnchorPoint::Bottom,
                y: "92",
            }
        }
    }
}

fn status_text(state: &LoadingScreenState) -> Element {
    rsx! {
        fontstring {
            name: LOADING_STATUS_TEXT,
            width: 560.0,
            height: 24.0,
            text: state.status_text.clone(),
            font_size: 18.0,
            font: GameFont::FrizQuadrata,
            font_color: COLOR_SUBTLE,
            anchor {
                point: AnchorPoint::Bottom,
                relative_to: FrameName("LoadingArtwork"),
                relative_point: AnchorPoint::Bottom,
                y: "62",
            }
        }
    }
}

fn bar_background() -> Element {
    rsx! {
        texture {
            name: "LoadingBarBackground",
            width: 610.0,
            height: 84.0,
            texture_file: TEX_LOADING_BAR_BG,
            strata: FrameStrata::Medium,
            anchor {
                point: AnchorPoint::Bottom,
                relative_to: FrameName("LoadingArtwork"),
                relative_point: AnchorPoint::Bottom,
                y: "10",
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
            height: 22.0,
            background_color: "0.0,0.0,0.0,0.0",
            strata: FrameStrata::High,
            anchor {
                point: AnchorPoint::Bottom,
                relative_to: FrameName("LoadingBarBackground"),
                relative_point: AnchorPoint::Bottom,
                y: "19",
            }
            texture {
                name: LOADING_BAR_FILL,
                width: fill_width,
                height: 22.0,
                texture_file: TEX_LOADING_BAR_FILL,
                strata: FrameStrata::High,
                anchor {
                    point: AnchorPoint::Left,
                    relative_point: AnchorPoint::Left,
                }
            }
        }
    }
}

fn bar_frame() -> Element {
    rsx! {
        texture {
            name: "LoadingBarFrame",
            width: 610.0,
            height: 84.0,
            texture_file: TEX_LOADING_BAR_FRAME,
            strata: FrameStrata::Dialog,
            anchor {
                point: AnchorPoint::Center,
                relative_to: FrameName("LoadingBarBackground"),
                relative_point: AnchorPoint::Center,
            }
        }
    }
}

fn progress_text(progress_percent: u8) -> Element {
    let text = format!("{}%", progress_percent);
    rsx! {
        fontstring {
            name: LOADING_PROGRESS_TEXT,
            width: 90.0,
            height: 20.0,
            text,
            font_size: 16.0,
            font: GameFont::FrizQuadrata,
            font_color: COLOR_GOLD,
            anchor {
                point: AnchorPoint::Right,
                relative_to: FrameName("LoadingBarBackground"),
                relative_point: AnchorPoint::Right,
                x: "-42",
                y: "2",
            }
        }
    }
}

fn tip_text(state: &LoadingScreenState) -> Element {
    rsx! {
        fontstring {
            name: "LoadingTipText",
            width: 920.0,
            height: 22.0,
            text: state.tip_text.clone(),
            font_size: 15.0,
            font: GameFont::FrizQuadrata,
            font_color: COLOR_TIP,
            anchor {
                point: AnchorPoint::Top,
                relative_to: FrameName("LoadingBarBackground"),
                relative_point: AnchorPoint::Bottom,
                y: "-12",
            }
        }
    }
}
