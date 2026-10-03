//! Retail `MirrorTimerContainer` (`Blizzard_MirrorTimer/MirrorTimer.xml`): up to three
//! 206×32 timer bars stacked down from TOP of UIParent at y -100.

use ui_toolkit::rsx;
use ui_toolkit::screen::SharedContext;
use ui_toolkit::widget_def::Element;

use crate::mirror_timer_data::{
    MIRROR_TIMER_FRAMES, MirrorTimer, MirrorTimerKind, MirrorTimersData,
};
use crate::ui::anchor::FrameName;
use crate::ui::screens::inworld_unit_frames_component::inworld_unit_frames_art::AtlasArt;
use crate::ui::widgets::font_string::{FontColor, GameFont};

pub const MIRROR_TIMER_CONTAINER: FrameName = FrameName("MirrorTimerContainer");

/// `MirrorTimerTemplate` `Size x=206 y=32`; container anchor `TOP` y -100.
const TIMER_W: f32 = 206.0;
const TIMER_H: f32 = 32.0;
const CONTAINER_TOP: f32 = 100.0;
/// `StatusBar` `Size x=195 y=13` at `TOP` y -2.
const BAR_W: f32 = 195.0;
const BAR_H: f32 = 13.0;
const BAR_X: f32 = (TIMER_W - BAR_W) / 2.0;
const BAR_Y: f32 = 2.0;
/// `TextBorder` spans the bar and 14 below it; `Text` fills it under the bar, 2 short.
const TEXT_BORDER_H: f32 = BAR_H + 14.0;
const TEXT_H: f32 = TEXT_BORDER_H - BAR_H - 2.0;
/// `GameFontHighlightSmall`: 10 pt white.
const TEXT_SIZE: f32 = 10.0;

/// UiTextureAtlas 1942 (`interface/castingbar/uicastingbar.blp`, FDID 4505182, 512×256).
const fn casting_bar(rect: (f32, f32, f32, f32)) -> AtlasArt {
    AtlasArt {
        fdid: 4_505_182,
        atlas: (512.0, 256.0),
        rect,
    }
}

/// `ui-castingbar-textbox` (14274).
const TEXT_BORDER: AtlasArt = casting_bar((1.0, 211.0, 1.0, 29.0));
/// `ui-castingbar-background` (14231).
const BACKGROUND: AtlasArt = casting_bar((57.0, 266.0, 85.0, 96.0));
/// `ui-castingbar-frame` (14235).
const BORDER: AtlasArt = casting_bar((1.0, 215.0, 31.0, 47.0));

/// `MirrorTimerAtlas` fills: `ui-castingbar-filling-standard` (14234),
/// `-applyingcrafting` (14232), `-channel` (14233).
pub fn fill_art(kind: MirrorTimerKind) -> AtlasArt {
    match kind {
        MirrorTimerKind::Exhaustion => casting_bar((268.0, 477.0, 124.0, 135.0)),
        MirrorTimerKind::Breath => casting_bar((268.0, 477.0, 111.0, 122.0)),
        MirrorTimerKind::FeignDeath => casting_bar((57.0, 266.0, 124.0, 135.0)),
    }
}

struct DynName(String);

pub fn mirror_timer_name(index: usize) -> String {
    format!("MirrorTimer{}", index + 1)
}

pub fn mirror_timer_screen(ctx: &SharedContext) -> Element {
    let data = ctx
        .get::<MirrorTimersData>()
        .expect("MirrorTimersData must be in SharedContext");
    let mut shown = 0;
    let timers: Element = (0..MIRROR_TIMER_FRAMES)
        .flat_map(|index| {
            let timer = data.frames[index];
            let row = shown;
            shown += usize::from(timer.is_some());
            timer_frame(index, row, timer)
        })
        .collect();
    let height = TIMER_H * shown.max(1) as f32;
    rsx! {
        r#frame {
            name: MIRROR_TIMER_CONTAINER,
            width: TIMER_W,
            height,
            hidden: {shown == 0},
            pos_type: "absolute",
            left: "50%",
            translate_x: "-50%",
            top: CONTAINER_TOP,
            {timers}
        }
    }
}

/// One `MirrorTimerTemplate`; shown timers stack in frame order (`VerticalLayoutFrame`).
fn timer_frame(index: usize, row: usize, timer: Option<MirrorTimer>) -> Element {
    let name = mirror_timer_name(index);
    let kind = timer.map_or(MirrorTimerKind::Breath, |timer| timer.kind);
    let fraction = timer.map_or(0.0, |timer| timer.fraction());
    let label = timer.map_or("", |timer| timer.kind.label());
    let parts: Element = [
        art(
            format!("{name}TextBorder"),
            &TEXT_BORDER,
            (0.0, 0.0, BAR_W, TEXT_BORDER_H),
            1.0,
        ),
        art(
            format!("{name}Background"),
            &BACKGROUND,
            (-1.0, -1.0, BAR_W + 2.0, BAR_H + 2.0),
            1.0,
        ),
        art(
            format!("{name}StatusBar"),
            &fill_art(kind),
            (0.0, 0.0, BAR_W * fraction, BAR_H),
            fraction,
        ),
        art(
            format!("{name}Border"),
            &BORDER,
            (-2.0, -2.0, BAR_W + 4.0, BAR_H + 4.0),
            1.0,
        ),
        text(format!("{name}Text"), label),
    ]
    .into_iter()
    .flatten()
    .collect();
    rsx! {
        r#frame {
            name: {DynName(name)},
            width: TIMER_W,
            height: TIMER_H,
            hidden: {timer.is_none()},
            pos_type: "absolute",
            pos_x: 0.0,
            pos_y: {TIMER_H * row as f32},
            r#frame {
                name: {DynName(format!("{}Bar", mirror_timer_name(index)))},
                width: BAR_W,
                height: BAR_H,
                pos_type: "absolute",
                pos_x: BAR_X,
                pos_y: BAR_Y,
                {parts}
            }
        }
    }
}

/// An atlas crop at `(x, y, width, height)` from the bar's TOPLEFT, revealing the leftmost
/// `fraction` of the crop (a `StatusBar` texture; 1 for plain textures).
fn art(
    name: String,
    art: &AtlasArt,
    (x, y, width, height): (f32, f32, f32, f32),
    fraction: f32,
) -> Element {
    let coords = art.tex_coords(fraction);
    rsx! {
        texture {
            name: {DynName(name)},
            width,
            height,
            hidden: {width <= 0.0},
            texture_fdid: {art.fdid},
            tex_coords: {coords.as_str()},
            pos_type: "absolute",
            pos_x: x,
            pos_y: y,
        }
    }
}

fn text(name: String, label: &str) -> Element {
    rsx! {
        fontstring {
            name: {DynName(name)},
            width: BAR_W,
            height: TEXT_H,
            text: label,
            font: GameFont::FrizQuadrata,
            font_size: TEXT_SIZE,
            font_color: FontColor::new(1.0, 1.0, 1.0, 1.0),
            justify_h: "CENTER",
            pos_type: "absolute",
            pos_x: 0.0,
            pos_y: BAR_H,
        }
    }
}
