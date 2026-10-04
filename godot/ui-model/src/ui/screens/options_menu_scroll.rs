//! The Options content area as a scroll list: Retail's `SettingsListTemplate` is a
//! `WowScrollBoxList` with a `MinimalScrollBar` beside it
//! (`Blizzard_Settings_Shared/Blizzard_SettingsList.xml:48-58`), sized to the category list
//! (`Blizzard_SettingsPanel.xml:59-74`); the bar shows only while the list overflows
//! (`Blizzard_SettingsList.lua:71-76`, `ScrollUtil.AddManagedScrollBarVisibilityBehavior`).
//!
//! It is a ui-toolkit scroll list (`ui_toolkit::widgets::scroll_list`): the position lives in
//! `FrameRegistry::scroll_lists` under [`OPTIONS_CONTENT_SCROLL`], the `scroll_list` attribute
//! records its geometry, and a change of position rebuilds the screen. The toolkit's builder
//! takes rows of one height; a page's rows differ, so the page is laid out here, one row per
//! page item, and only the items inside the area exist as frames.

use ui_toolkit::rsx;
use ui_toolkit::widget_def::{Element, WidgetChild, WidgetDef};
use ui_toolkit::widgets::scroll_list::{ScrollGeometry, thumb_name, track_name};

use super::DynName;

/// Scroll list name of the Options content area.
pub const OPTIONS_CONTENT_SCROLL: &str = "OptionsContentScroll";

/// The toolkit scroll list's plain track and thumb colours (`scroll_list.rs`).
const TRACK_COLOR: &str = "0,0,0,0.5";
const THUMB_COLOR: &str = "0.8,0.7,0.5,0.9";
const TRACK_W: f32 = 8.0;
const TRACK_RIGHT: f32 = 3.0;

/// One options page: its rows top to bottom and the gap between them.
pub struct OptionsPage {
    pub items: Element,
    pub gap: f32,
}

/// Where the content area sits and how big it is.
pub(super) struct ScrollArea {
    /// Width of the rows.
    pub row_width: f32,
    /// Width of the area: the rows plus the margin the scroll bar sits in.
    pub width: f32,
    pub height: f32,
}

/// The page's items from `first_item`, as many as fit in `area`, with a scroll bar when the
/// page is taller than the area.
pub(super) fn scroll_area(page: OptionsPage, area: &ScrollArea, first_item: usize) -> Element {
    let items = into_widgets(page.items);
    let extents: Vec<f32> = items.iter().map(widget_extent).collect();
    let max_first = max_first_item(&extents, page.gap, area.height);
    let first = first_item.min(max_first);
    let shown = fitting_items(&extents[first..], page.gap, area.height);
    let geometry = ScrollGeometry {
        row_count: extents.len(),
        visible_rows: extents.len() - max_first,
        row_height: area.height / (extents.len() - max_first).max(1) as f32,
        track_height: area.height,
    };
    let items: Element = items
        .into_iter()
        .skip(first)
        .take(shown)
        .map(WidgetChild::Widget)
        .collect();
    let config = format!(
        "{},{},{},{}",
        geometry.row_count, geometry.visible_rows, geometry.row_height, geometry.track_height
    );
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
                top: 0,
                {items}
            }
            {scroll_track(geometry, first, max_first == 0)}
        }
    }
}

fn scroll_track(geometry: ScrollGeometry, first: usize, hide: bool) -> Element {
    rsx! {
        r#frame {
            name: DynName(track_name(OPTIONS_CONTENT_SCROLL)),
            width: TRACK_W,
            height: {geometry.track_height},
            pos_type: "absolute",
            right: TRACK_RIGHT,
            top: 0,
            hidden: {hide},
            background_color: TRACK_COLOR,
            r#frame {
                name: DynName(thumb_name(OPTIONS_CONTENT_SCROLL)),
                width: TRACK_W,
                height: {geometry.thumb_height()},
                pos_type: "absolute",
                left: 0,
                top: {geometry.thumb_top(first)},
                mouse_enabled: true,
                background_color: THUMB_COLOR,
            }
        }
    }
}

/// How many of `extents`, from the first, fit in `height` with `gap` between them; at least
/// one, so an item taller than the area still shows.
fn fitting_items(extents: &[f32], gap: f32, height: f32) -> usize {
    let mut used = 0.0;
    let mut count = 0;
    for extent in extents {
        let next = if count == 0 {
            *extent
        } else {
            used + gap + extent
        };
        if count > 0 && next > height {
            break;
        }
        used = next;
        count += 1;
    }
    count
}

/// The first item that shows the last one at the bottom: the furthest the page scrolls.
fn max_first_item(extents: &[f32], gap: f32, height: f32) -> usize {
    let reversed: Vec<f32> = extents.iter().rev().copied().collect();
    extents.len() - fitting_items(&reversed, gap, height).min(extents.len())
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
