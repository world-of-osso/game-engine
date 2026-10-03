//! Retail Skyriding vigor display: the `FillUpFrames` UI widget (`Blizzard_UIWidgets/
//! Blizzard_UIWidgetTemplateFillUpFrames.lua` and `.xml`) with texture kit
//! `dragonriding_vigor`, one frame per Skyriding Charge, in `UIWidgetPowerBarContainerFrame`
//! (`EncounterBar`, a bottom managed frame over the main action bar).
//!
//! Layout (`UIWidgetTemplateFillUpFrames`, a `HorizontalLayoutFrame`): `DecorLeft`
//! (mirrored), the frames at the fixed 42×45 of `fixedSizeByTextureKit`, the first and the
//! last padded -20 inward (`firstAndLastPadding`), `DecorRight`; the decor sits 8 lower
//! (`decorTopPadding`). Each `UIWidgetFillUpFrameTemplate` centres its `BG`, the vertical
//! `Bar` (`_fillfull` when full, else `_fill` up to the recovering charge's progress), the
//! `Spark` on the fill's top edge while filling, and the `Frame` border.

use ui_toolkit::rsx;
use ui_toolkit::screen::SharedContext;
use ui_toolkit::widget_def::Element;

use crate::main_action_bar_component::{BAR_BOTTOM, BUTTON_SIZE};
use crate::ui::anchor::FrameName;
use crate::ui::screens::inworld_unit_frames_component::inworld_unit_frames_art::AtlasArt;

pub const VIGOR_BAR: FrameName = FrameName("UIWidgetPowerBarContainerFrame");

/// What the widget shows: `numTotalFrames`, `numFullFrames` and the filling frame's
/// `fillValue` of `fillMax` as a 0..1 fraction. `None` hides it.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct VigorBarState {
    pub shown: Option<VigorFrames>,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct VigorFrames {
    pub total: u8,
    pub full: u8,
    pub filling: f32,
}

/// UiTextureAtlas 2188 (FDID 4730866, 512×512).
const fn vigor(rect: (f32, f32, f32, f32)) -> AtlasArt {
    AtlasArt {
        fdid: 4_730_866,
        atlas: (512.0, 512.0),
        rect,
    }
}

pub const VIGOR_ART_FDIDS: [u32; 1] = [4_730_866];

/// `dragonriding_vigor_background` (20974).
const BACKGROUND: AtlasArt = vigor((373.0, 434.0, 120.0, 191.0));
/// `dragonriding_vigor_decor` (20975).
const DECOR: AtlasArt = vigor((299.0, 392.0, 1.0, 118.0));
/// `dragonriding_vigor_fill` (20976).
const FILL: AtlasArt = vigor((394.0, 466.0, 1.0, 73.0));
/// `dragonriding_vigor_fillfull` (21407).
const FILL_FULL: AtlasArt = vigor((299.0, 371.0, 120.0, 192.0));
/// `dragonriding_vigor_frame` (20978).
const FRAME: AtlasArt = vigor((123.0, 243.0, 373.0, 503.0));
/// `dragonriding_vigor_spark` (20979).
const SPARK: AtlasArt = vigor((394.0, 487.0, 75.0, 95.0));

/// `fixedSizeByTextureKit.dragonriding_vigor`.
const FRAME_W: f32 = 42.0;
const FRAME_H: f32 = 45.0;
/// `firstAndLastPadding.dragonriding_vigor`.
const END_PADDING: f32 = -20.0;
/// `decorTopPadding.dragonriding_vigor`.
const DECOR_TOP: f32 = 8.0;

struct DynName(String);

/// Width and height of the laid-out widget for `total` frames.
pub fn vigor_bar_size(total: u8) -> (f32, f32) {
    let (decor_w, decor_h) = DECOR.size();
    let frames = FRAME_W * f32::from(total) + 2.0 * END_PADDING;
    (2.0 * decor_w + frames, (DECOR_TOP + decor_h).max(FRAME_H))
}

pub fn vigor_bar_screen(ctx: &SharedContext) -> Element {
    let state = ctx
        .get::<VigorBarState>()
        .expect("VigorBarState must be in SharedContext");
    let total = state.shown.map_or(0, |frames| frames.total);
    let (width, height) = vigor_bar_size(total);
    let first_x = DECOR.size().0 + END_PADDING;
    let fill_frames: Element = state
        .shown
        .into_iter()
        .flat_map(|frames| {
            (0..frames.total).flat_map(move |index| {
                fill_frame(index, first_x + FRAME_W * f32::from(index), frames)
            })
        })
        .collect();
    let decor = decor(width);
    rsx! {
        r#frame {
            name: VIGOR_BAR,
            width,
            height,
            hidden: {state.shown.is_none()},
            pos_type: "absolute",
            left: "50%",
            translate_x: "-50%",
            bottom: {BAR_BOTTOM + BUTTON_SIZE},
            {decor}
            {fill_frames}
        }
    }
}

/// `DecorLeft` (mirrored) and `DecorRight` at the ends of a `width` wide widget.
fn decor(width: f32) -> Element {
    let (decor_w, decor_h) = DECOR.size();
    [
        texture(
            "UIWidgetFillUpFramesDecorLeft".into(),
            &DECOR,
            (0.0, DECOR_TOP, decor_w, decor_h),
            Crop::Mirrored,
        ),
        texture(
            "UIWidgetFillUpFramesDecorRight".into(),
            &DECOR,
            (width - decor_w, DECOR_TOP, decor_w, decor_h),
            Crop::Whole,
        ),
    ]
    .into_iter()
    .flatten()
    .collect()
}

/// The `Bar` value of frame `index`: full frames are full, the first one after them fills
/// with the recovering charge (`isFilling`), the rest are empty.
fn frame_value(index: u8, frames: VigorFrames) -> f32 {
    if index < frames.full {
        1.0
    } else if index == frames.full {
        frames.filling.clamp(0.0, 1.0)
    } else {
        0.0
    }
}

/// One `UIWidgetFillUpFrameTemplate` at `x`: full, filling or empty.
fn fill_frame(index: u8, x: f32, frames: VigorFrames) -> Element {
    let name = format!("UIWidgetFillUpFrame{}", index + 1);
    let full = index < frames.full;
    let value = frame_value(index, frames);
    let fill = if full { &FILL_FULL } else { &FILL };
    let (fill_w, fill_h) = fill.size();
    let (bar_x, bar_y) = centred(fill.size());
    let fill_top = bar_y + fill_h * (1.0 - value);
    let spark_shown = index == frames.full && value > 0.0 && value < 1.0;
    let parts: Element = [
        texture(
            format!("{name}BG"),
            &BACKGROUND,
            placed(&BACKGROUND),
            Crop::Whole,
        ),
        texture(
            format!("{name}Bar"),
            fill,
            (bar_x, fill_top, fill_w, fill_h * value),
            Crop::Bottom(value),
        ),
        if spark_shown {
            spark(&name, fill_top)
        } else {
            Vec::new()
        },
        texture(format!("{name}Frame"), &FRAME, placed(&FRAME), Crop::Whole),
    ]
    .into_iter()
    .flatten()
    .collect();
    rsx! {
        r#frame {
            name: {DynName(name)},
            width: FRAME_W,
            height: FRAME_H,
            pos_type: "absolute",
            pos_x: x,
            pos_y: 0.0,
            {parts}
        }
    }
}

/// `Spark` centred on the fill texture's TOP edge at `fill_top`.
fn spark(name: &str, fill_top: f32) -> Element {
    let (spark_w, spark_h) = SPARK.size();
    texture(
        format!("{name}Spark"),
        &SPARK,
        (
            (FRAME_W - spark_w) / 2.0,
            fill_top - spark_h / 2.0,
            spark_w,
            spark_h,
        ),
        Crop::Whole,
    )
}

/// TOPLEFT of a `(width, height)` region anchored CENTER in a fill frame.
fn centred((width, height): (f32, f32)) -> (f32, f32) {
    ((FRAME_W - width) / 2.0, (FRAME_H - height) / 2.0)
}

/// An atlas at its own size, centred in a fill frame.
fn placed(art: &AtlasArt) -> (f32, f32, f32, f32) {
    let (width, height) = art.size();
    let (x, y) = centred((width, height));
    (x, y, width, height)
}

enum Crop {
    Whole,
    /// `TexCoords left="1" right="0"`.
    Mirrored,
    /// A vertical `StatusBar` revealing the bottom `fraction` of its texture.
    Bottom(f32),
}

fn tex_coords(art: &AtlasArt, crop: Crop) -> String {
    let (left, right, top, bottom) = art.rect;
    let (width, height) = art.atlas;
    let (left, right, top) = match crop {
        Crop::Whole => (left, right, top),
        Crop::Mirrored => (right, left, top),
        Crop::Bottom(fraction) => (left, right, bottom - (bottom - top) * fraction),
    };
    format!(
        "{},{},{},{}",
        left / width,
        right / width,
        top / height,
        bottom / height
    )
}

fn texture(
    name: String,
    art: &AtlasArt,
    (x, y, width, height): (f32, f32, f32, f32),
    crop: Crop,
) -> Element {
    let coords = tex_coords(art, crop);
    rsx! {
        texture {
            name: {DynName(name)},
            width,
            height,
            hidden: {height <= 0.0},
            texture_fdid: {art.fdid},
            tex_coords: {coords.as_str()},
            pos_type: "absolute",
            pos_x: x,
            pos_y: y,
        }
    }
}
