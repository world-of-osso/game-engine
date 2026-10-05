//! `MinimalScrollBar` (`Blizzard_SharedXML/Shared/Scroll/MinimalScrollBar.xml:15-139`), the
//! bar beside the Options list and Retail's default `ScrollFrameTemplate` bar
//! (`SCROLL_FRAME_SCROLL_BAR_TEMPLATE`, `Blizzard_SharedXML/Mainline/ScrollDefine.lua:1`).
//! Its list scrolls by pixels: the list's position is a pixel offset kept in
//! `FrameRegistry::scroll_lists` under the list's name, one scroll-list "row" per pixel.

use ui_toolkit::rsx;
use ui_toolkit::widget_def::Element;
use ui_toolkit::widgets::scroll_list::{ScrollGeometry, thumb_name, track_name};

use crate::quest_art::DynName;

/// 8 wide (:16), its track 19 in from either end (:26-27), a thumb at least 23 long (:20),
/// and 17×11 Back / Forward steppers at its top and bottom (:100-137).
pub const BAR_W: f32 = 8.0;
const TRACK_INSET: f32 = 19.0;
const MIN_THUMB: f32 = 23.0;
const STEPPER_W: f32 = 17.0;
const STEPPER_H: f32 = 11.0;
/// The track and thumb end caps are 8×8 atlas members.
const CAP: f32 = 8.0;
/// `minimal-scrollbar-small-thumb-middle` is 715 tall; the thumb shows its top
/// `height / 715` (`MinimalScrollBar.lua:193-203`).
const THUMB_MIDDLE_ATLAS_H: f32 = 715.0;

/// The Back stepper of scroll list `list`: scrolls one pan extent up.
pub fn back_stepper_name(list: &str) -> String {
    format!("{list}ScrollBack")
}

/// The Forward stepper of scroll list `list`: scrolls one pan extent down.
pub fn forward_stepper_name(list: &str) -> String {
    format!("{list}ScrollForward")
}

/// What the bar shows while its list does not overflow.
#[derive(Clone, Copy, PartialEq, Eq)]
pub enum Unscrollable {
    /// `hideIfUnscrollable`: the whole bar hides (`ScrollBar.lua:262-264`).
    HideBar,
    /// The bar stays with its track but no thumb (`ScrollBar.lua:252-255`).
    HideThumb,
}

/// The scroll geometry of a list `visible` pixels tall whose content is `content` tall,
/// beside a bar `bar_height` tall.
pub fn pixel_geometry(visible: f32, content: f32, bar_height: f32) -> ScrollGeometry {
    let max_offset = (content - visible).max(0.0).ceil() as usize;
    let visible = visible.round() as usize;
    ScrollGeometry {
        row_count: visible + max_offset,
        visible_rows: visible,
        row_height: 1.0,
        track_height: bar_height - 2.0 * TRACK_INSET,
    }
}

/// The `scroll_list` attribute of a list with `geometry`.
pub fn scroll_list_attr(geometry: &ScrollGeometry) -> String {
    format!(
        "{},{},{},{}",
        geometry.row_count, geometry.visible_rows, geometry.row_height, geometry.track_height
    )
}

/// A `MinimalScrollBar` for list `list` scrolled `offset` pixels, at (`left`, `top`) in the
/// list frame.
pub struct MinimalScrollBar<'a> {
    pub list: &'a str,
    pub left: f32,
    pub top: f32,
    pub height: f32,
    pub geometry: ScrollGeometry,
    pub offset: usize,
    pub unscrollable: Unscrollable,
}

impl MinimalScrollBar<'_> {
    /// `ScrollBarMixin:Update` (`ScrollBar.lua:219-266`): a proportional thumb at least
    /// `minThumbExtent` long, placed along the track by the scroll fraction.
    fn thumb(&self) -> (f32, f32) {
        let track = self.geometry.track_height;
        let rows = self.geometry.row_count.max(1) as f32;
        let visible = self.geometry.visible_rows as f32 / rows;
        let height = (track * visible).max(MIN_THUMB).min(track);
        let max_offset = self.geometry.max_first_row();
        let fraction = if max_offset == 0 {
            0.0
        } else {
            self.offset.min(max_offset) as f32 / max_offset as f32
        };
        ((track - height) * fraction, height)
    }

    pub fn element(&self) -> Element {
        let list = self.list;
        let scrollable = self.geometry.max_first_row() > 0;
        let hide_bar = !scrollable && self.unscrollable == Unscrollable::HideBar;
        let hide_thumb = !scrollable && self.unscrollable == Unscrollable::HideThumb;
        let track = self.geometry.track_height;
        let (thumb_top, thumb_height) = self.thumb();
        let middle_v = (thumb_height / THUMB_MIDDLE_ATLAS_H).min(1.0);
        let stepper_left = (BAR_W - STEPPER_W) / 2.0;
        rsx! {
            r#frame {
                name: DynName(format!("{list}ScrollBar")),
                width: BAR_W,
                height: {self.height},
                pos_type: "absolute",
                left: {self.left},
                top: {self.top},
                hidden: {hide_bar},
                {stepper(back_stepper_name(list), "minimal-scrollbar-arrow-top", stepper_left, 0.0)}
                r#frame {
                    name: DynName(track_name(list)),
                    width: BAR_W,
                    height: {track},
                    pos_type: "absolute",
                    left: 0,
                    top: TRACK_INSET,
                    mouse_enabled: true,
                    {art("minimal-scrollbar-track-top", 0.0, CAP, None)}
                    {art("!minimal-scrollbar-track-middle", CAP, track - 2.0 * CAP, None)}
                    {art("minimal-scrollbar-track-bottom", track - CAP, CAP, None)}
                    r#frame {
                        name: DynName(thumb_name(list)),
                        width: BAR_W,
                        height: {thumb_height},
                        pos_type: "absolute",
                        left: 0,
                        top: {thumb_top},
                        mouse_enabled: true,
                        hit_rect_insets: "-4,-4,-4,-4",
                        hidden: {hide_thumb},
                        {art("minimal-scrollbar-small-thumb-top", 0.0, CAP, None)}
                        {art("minimal-scrollbar-small-thumb-middle", CAP, thumb_height - 2.0 * CAP, Some(middle_v))}
                        {art("minimal-scrollbar-small-thumb-bottom", thumb_height - CAP, CAP, None)}
                    }
                }
                {stepper(forward_stepper_name(list), "minimal-scrollbar-arrow-bottom", stepper_left, self.height - STEPPER_H)}
            }
        }
    }
}

/// A Back or Forward stepper: its arrow art, pressed by the scroll-list input.
fn stepper(name: String, atlas: &str, left: f32, top: f32) -> Element {
    rsx! {
        r#frame {
            name: DynName(name),
            width: STEPPER_W,
            height: STEPPER_H,
            pos_type: "absolute",
            left,
            top,
            mouse_enabled: true,
            texture {
                width: STEPPER_W,
                height: STEPPER_H,
                texture_atlas: atlas,
                pos_type: "absolute",
                left: 0,
                top: 0,
            }
        }
    }
}

/// One 8-wide piece of track or thumb art `height` tall at `top`; `v` crops a stretched
/// middle to its top part.
fn art(atlas: &str, top: f32, height: f32, v: Option<f32>) -> Element {
    let coords = format!("0,1,0,{}", v.unwrap_or(1.0));
    rsx! {
        texture {
            width: BAR_W,
            height: {height.max(0.0)},
            texture_atlas: atlas,
            tex_coords: {coords},
            pos_type: "absolute",
            left: 0,
            top,
        }
    }
}
