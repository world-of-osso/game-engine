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

use super::DynName;
use crate::minimal_scroll_bar::{MinimalScrollBar, Unscrollable, pixel_geometry, scroll_list_attr};

/// Scroll list name of the Options content area.
pub const OPTIONS_CONTENT_SCROLL: &str = "OptionsContentScroll";

/// The bar sits at the list's right edge, 4 below its top and 7 above its bottom
/// (`Blizzard_SettingsList.xml:53-57`).
const BAR_TOP: f32 = 4.0;
const BAR_BOTTOM: f32 = 7.0;
/// `ScrollControllerMixin.Defaults.WheelPanScalar`: a wheel notch pans two pan extents
/// (`Blizzard_SharedXML/Shared/Scroll/ScrollController.lua:77-81,93-99,118-121,152-154`).
const WHEEL_PAN_SCALAR: usize = 2;

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
    let bar_height = area.height - BAR_TOP - BAR_BOTTOM;
    let geometry = pixel_geometry(area.height, content, bar_height);
    let offset = geometry.clamp(offset);
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
    let config = scroll_list_attr(&geometry);
    let bar = MinimalScrollBar {
        list: OPTIONS_CONTENT_SCROLL,
        left: area.row_width,
        top: BAR_TOP,
        height: bar_height,
        geometry,
        offset,
        unscrollable: Unscrollable::HideBar,
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
            {bar.element()}
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
