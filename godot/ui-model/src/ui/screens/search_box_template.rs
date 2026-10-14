//! Shared/InputBox/InputBoxTemplates.xml: InputBoxVisual + SearchBoxTemplate anchors.
use crate::quest_art::{DynName, named_atlas_texture};
use ui_toolkit::{rsx, widget_def::Element};

pub fn search_box(
    prefix: &str,
    query: &str,
    clear_action: &str,
    (left, top, width, height): (f32, f32, f32, f32),
) -> Element {
    let border_top = top + (height - 20.0) / 2.0;
    let art: Element = [
        (
            "Left",
            "common-search-border-left",
            (left - 5.0, border_top, 8.0, 20.0),
        ),
        (
            "Middle",
            "common-search-border-middle",
            (left + 3.0, border_top, width - 11.0, 20.0),
        ),
        (
            "Right",
            "common-search-border-right",
            (left + width - 8.0, border_top, 8.0, 20.0),
        ),
        (
            "Icon",
            "common-search-magnifyingglass",
            (left + 1.0, top + (height - 10.0) / 2.0 + 1.0, 10.0, 10.0),
        ),
    ]
    .into_iter()
    .flat_map(|(suffix, atlas, rect)| {
        named_atlas_texture(format!("{prefix}Search{suffix}"), atlas, rect)
    })
    .collect();
    let clear = rsx! { texture { name: {DynName(format!("{prefix}SearchClearIcon"))},
    texture_atlas: "common-search-clearbutton", width: 10.0, height: 10.0,
    left: 3.0, top: 3.0, pos_type: "absolute", alpha: 0.5 } };
    let empty = query.is_empty();
    rsx! { {art}
        editbox { name: {DynName(format!("{prefix}SearchBox"))}, text: query, width, height,
            left, top, pos_type: "absolute", font_size: 12.0,
            font_color: "1,1,1,1", text_insets: "16,20,0,0", max_letters: 40 }
        fontstring { name: {DynName(format!("{prefix}SearchHint"))}, text: "Search", hidden: {!empty},
            mouse_enabled: false, width: {width - 36.0}, height, left: {left + 16.0}, top,
            pos_type: "absolute", font_size: 12.0, font_color: "0.35,0.35,0.35,1", justify_h: "LEFT" }
        button { name: {DynName(format!("{prefix}SearchClear"))}, width: 17.0, height: 17.0,
            left: {left + width - 20.0}, top: {top + (height - 17.0) / 2.0},
            pos_type: "absolute", hidden: empty, button_default_skin: false,
            onclick: clear_action, {clear} }
    }
}
