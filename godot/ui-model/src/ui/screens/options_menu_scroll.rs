//! The Options content area as a scroll box: Retail's `SettingsListTemplate` is a
//! `WowScrollBoxList` with a `MinimalScrollBar` beside it
//! (`Blizzard_Settings_Shared/Blizzard_SettingsList.xml:48-58`), sized to the category list
//! (`Blizzard_SettingsPanel.xml:59-74`); the bar shows only while the list overflows
//! (`Blizzard_SettingsList.lua:71-76`, `ScrollUtil.AddManagedScrollBarVisibilityBehavior`).
//!
//! The list scrolls by pixels, not rows: its position is a pixel offset kept in
//! `FrameRegistry::scroll_lists` under [`OPTIONS_CONTENT_SCROLL`] (one scroll-list "row" is
//! one pixel), and a row cut by the top or bottom edge is clipped, as `ScrollBoxBaseTemplate`
//! clips its children (`Blizzard_SharedXML/Shared/Scroll/ScrollBox.xml:20`). Only the items
//! that reach into the area exist as frames.

use ui_toolkit::rsx;
use ui_toolkit::widget_def::{Element, WidgetChild, WidgetDef};
use ui_toolkit::widgets::scroll_list::{ScrollGeometry, thumb_name, track_name};

use super::DynName;

/// Scroll list name of the Options content area.
pub const OPTIONS_CONTENT_SCROLL: &str = "OptionsContentScroll";

/// `MinimalScrollBar` (`Blizzard_SharedXML/Shared/Scroll/MinimalScrollBar.xml`): 8 wide
/// (:16), its track 19 in from either end (:26-27), a thumb at least 23 long (:20), and
/// 17×11 Back / Forward steppers at its top and bottom (:100-137).
const BAR_W: f32 = 8.0;
const TRACK_INSET: f32 = 19.0;
const MIN_THUMB: f32 = 23.0;
const STEPPER_W: f32 = 17.0;
const STEPPER_H: f32 = 11.0;
/// The track and thumb end caps are 8×8 atlas members.
const CAP: f32 = 8.0;
/// `minimal-scrollbar-small-thumb-middle` is 715 tall; the thumb shows its top
/// `height / 715` (`MinimalScrollBar.lua:193-203`).
const THUMB_MIDDLE_ATLAS_H: f32 = 715.0;
/// The bar sits at the list's right edge, 4 below its top and 7 above its bottom
/// (`Blizzard_SettingsList.xml:53-57`).
const BAR_TOP: f32 = 4.0;
const BAR_BOTTOM: f32 = 7.0;
/// `ScrollControllerMixin.Defaults.WheelPanScalar`: a wheel notch pans two pan extents
/// (`Blizzard_SharedXML/Shared/Scroll/ScrollController.lua:77-81,93-99,118-121,152-154`).
const WHEEL_PAN_SCALAR: usize = 2;

/// The Back stepper of scroll list `list`: scrolls one pan extent up.
pub fn back_stepper_name(list: &str) -> String {
    format!("{list}ScrollBack")
}

/// The Forward stepper of scroll list `list`: scrolls one pan extent down.
pub fn forward_stepper_name(list: &str) -> String {
    format!("{list}ScrollForward")
}

/// One options page: its rows top to bottom and the gap between them.
pub struct OptionsPage {
    pub items: Element,
    pub gap: f32,
}

impl OptionsPage {
    /// Pixels a stepper click scrolls: the first element's extent plus the spacing, as
    /// `ScrollBoxListViewMixin:GetPanExtent` caches it (`ScrollBoxListView.lua:637-661`) and
    /// `ScrollBarMixin:ScrollStepInDirection` pans by it (`ScrollBar.lua:117-119`).
    pub fn pan_extent(self) -> usize {
        let items = into_widgets(self.items);
        let first = items.first().map_or(0.0, widget_extent);
        (first + self.gap).round() as usize
    }

    /// Pixels a wheel notch scrolls: two pan extents.
    pub fn wheel_extent(self) -> usize {
        self.pan_extent() * WHEEL_PAN_SCALAR
    }
}

/// Where the content area sits and how big it is.
pub(super) struct ScrollArea {
    /// Width of the rows.
    pub row_width: f32,
    /// Width of the area: the rows plus the margin the scroll bar sits in.
    pub width: f32,
    pub height: f32,
}

/// The page scrolled down by `offset` pixels: the items reaching into `area`, cut at its
/// edges, with a scroll bar when the page is taller than the area.
pub(super) fn scroll_area(page: OptionsPage, area: &ScrollArea, offset: usize) -> Element {
    let items = into_widgets(page.items);
    let tops = item_tops(&items, page.gap);
    let content = tops.last().map_or(0.0, |(_, bottom)| *bottom);
    let max_offset = (content - area.height).max(0.0).ceil() as usize;
    let offset = offset.min(max_offset);
    let visible = area.height.round() as usize;
    let geometry = ScrollGeometry {
        row_count: visible + max_offset,
        visible_rows: visible,
        row_height: 1.0,
        track_height: area.height - BAR_TOP - BAR_BOTTOM - 2.0 * TRACK_INSET,
    };
    let (window_top, window_bottom) = (offset as f32, offset as f32 + area.height);
    let first = tops
        .iter()
        .position(|(_, bottom)| *bottom > window_top)
        .unwrap_or(items.len());
    let inner_top = tops.get(first).map_or(0.0, |(top, _)| top - window_top);
    let items: Element = items
        .into_iter()
        .zip(&tops)
        .skip(first)
        .take_while(|(_, (top, _))| *top < window_bottom)
        .map(|(item, _)| WidgetChild::Widget(item))
        .collect();
    let config = format!(
        "{},{},{},{}",
        geometry.row_count, geometry.visible_rows, geometry.row_height, geometry.track_height
    );
    let bar = ScrollBar {
        left: area.row_width,
        height: area.height - BAR_TOP - BAR_BOTTOM,
        geometry,
        offset,
    };
    rsx! {
        r#frame {
            name: DynName(OPTIONS_CONTENT_SCROLL.to_string()),
            width: {area.width},
            height: {area.height},
            mouse_enabled: true,
            scroll_list: {config},
            r#frame {
                name: "OptionsContentInner",
                width: {area.row_width},
                height: "auto",
                layout: "flex-column",
                gap: {page.gap},
                pos_type: "absolute",
                left: 0,
                top: {inner_top},
                {items}
            }
            {bar.element(max_offset == 0)}
        }
    }
}

/// Top and bottom of each item in the page's column.
fn item_tops(items: &[WidgetDef], gap: f32) -> Vec<(f32, f32)> {
    let mut next = 0.0;
    items
        .iter()
        .map(|item| {
            let top = next;
            let bottom = top + widget_extent(item);
            next = bottom + gap;
            (top, bottom)
        })
        .collect()
}

/// A `MinimalScrollBar` for a list scrolled `offset` pixels.
struct ScrollBar {
    left: f32,
    height: f32,
    geometry: ScrollGeometry,
    offset: usize,
}

impl ScrollBar {
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
            self.offset as f32 / max_offset as f32
        };
        ((track - height) * fraction, height)
    }

    fn element(&self, hide: bool) -> Element {
        let track = self.geometry.track_height;
        let (thumb_top, thumb_height) = self.thumb();
        let middle_v = (thumb_height / THUMB_MIDDLE_ATLAS_H).min(1.0);
        let stepper_left = (BAR_W - STEPPER_W) / 2.0;
        rsx! {
            r#frame {
                name: DynName(format!("{OPTIONS_CONTENT_SCROLL}ScrollBar")),
                width: BAR_W,
                height: {self.height},
                pos_type: "absolute",
                left: {self.left},
                top: BAR_TOP,
                hidden: {hide},
                {stepper(back_stepper_name(OPTIONS_CONTENT_SCROLL), "minimal-scrollbar-arrow-top", stepper_left, 0.0)}
                r#frame {
                    name: DynName(track_name(OPTIONS_CONTENT_SCROLL)),
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
                        name: DynName(thumb_name(OPTIONS_CONTENT_SCROLL)),
                        width: BAR_W,
                        height: {thumb_height},
                        pos_type: "absolute",
                        left: 0,
                        top: {thumb_top},
                        mouse_enabled: true,
                        hit_rect_insets: "-4,-4,-4,-4",
                        {art("minimal-scrollbar-small-thumb-top", 0.0, CAP, None)}
                        {art("minimal-scrollbar-small-thumb-middle", CAP, thumb_height - 2.0 * CAP, Some(middle_v))}
                        {art("minimal-scrollbar-small-thumb-bottom", thumb_height - CAP, CAP, None)}
                    }
                }
                {stepper(forward_stepper_name(OPTIONS_CONTENT_SCROLL), "minimal-scrollbar-arrow-bottom", stepper_left, self.height - STEPPER_H)}
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

/// Height an item takes in the page's column: its `height`, or for an `auto` column or row
/// the height its children stack to.
fn widget_extent(def: &WidgetDef) -> f32 {
    let height = attr(def, "height").unwrap_or_else(|| panic!("{:?} has no height", def.name));
    if let Ok(height) = height.parse() {
        return height;
    }
    assert_eq!(
        height, "auto",
        "{:?}: unsupported height {height}",
        def.name
    );
    let children: Vec<f32> = widgets(&def.children).map(widget_extent).collect();
    match attr(def, "layout") {
        Some("flex-column") => {
            let gap: f32 = attr(def, "gap").map_or(0.0, |gap| gap.parse().unwrap());
            children.iter().sum::<f32>() + gap * children.len().saturating_sub(1) as f32
        }
        Some("flex-row") => children.into_iter().fold(0.0, f32::max),
        layout => panic!(
            "{:?}: auto height needs a flex layout, not {layout:?}",
            def.name
        ),
    }
}

/// The page's rows, fragments opened.
fn into_widgets(children: Element) -> Vec<WidgetDef> {
    children
        .into_iter()
        .flat_map(|child| match child {
            WidgetChild::Widget(def) => vec![def],
            WidgetChild::Fragment(children) => into_widgets(children),
            WidgetChild::Dynamic => panic!("an options row must be a concrete widget"),
        })
        .collect()
}

/// The widgets of `children`, fragments opened.
fn widgets(children: &[WidgetChild]) -> Box<dyn Iterator<Item = &WidgetDef> + '_> {
    Box::new(children.iter().flat_map(|child| match child {
        WidgetChild::Widget(def) => Box::new(std::iter::once(def)),
        WidgetChild::Fragment(children) => widgets(children),
        WidgetChild::Dynamic => panic!("an options row must be a concrete widget"),
    }))
}

fn attr<'a>(def: &'a WidgetDef, name: &str) -> Option<&'a str> {
    def.attrs
        .iter()
        .find(|attr| attr.effective_name() == name)
        .map(|attr| attr.value_str())
}
