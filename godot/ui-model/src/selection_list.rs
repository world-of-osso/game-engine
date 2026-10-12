//! Retail list interaction, separate from UIPanelButton action chrome.
//! LFGList.xml's search entry uses `search-highlight`; its group rows use
//! `groupfinder-highlightbar-blue`. Atlas resolution selects Forever art only
//! when that skin has an authored member for the same atlas name.
use crate::bank_art::label;
use crate::quest_art::{DynName, NORMAL_FONT_COLOR};
use ui_toolkit::{
    frame::{Frame, WidgetData},
    rsx,
    widget_def::Element,
    widgets::texture::TextureSource,
};

pub const HOVER: &str = "search-highlight";
pub const SELECTED: &str = "groupfinder-highlightbar-blue";

/// Shared hit target. Specialized rows may retain their source icons/subtext/art
/// in `content`; idle generic rows deliberately have no background texture.
pub fn row(
    name: String,
    action: &str,
    enabled: bool,
    rect: (f32, f32, f32, f32),
    content: Element,
) -> Element {
    let (left, top, width, height) = rect;
    let disabled = !enabled;
    let action = if enabled { action } else { "" };
    rsx! { button { name: {DynName(name)}, left, top, width, height, pos_type: "absolute",
        disabled, onclick: action, button_default_skin: false,
        button_atlas_highlight: HOVER, button_highlight_alpha: 0.35, {content}
    } }
}

/// Keep the atlas identity for Retail's ADD blend, at the same depth as the label.
pub fn selection_background(name: String, selected: bool, rect: (f32, f32, f32, f32)) -> Element {
    let (left, top, width, height) = rect;
    let hidden = !selected;
    rsx! { texture { name: {DynName(name)}, width, height, left, top, hidden,
    pos_type: "absolute", texture_atlas: SELECTED } }
}

pub fn text_row(
    name: String,
    text: &str,
    action: &str,
    enabled: bool,
    selected: bool,
    rect: (f32, f32, f32, f32),
) -> Element {
    let (_, _, width, height) = rect;
    let hidden = !selected;
    let mut content = selection_background(
        format!("{name}Selected"),
        selected,
        (0.0, 0.0, width, height),
    );
    content.extend(
        rsx! { texture { name: {DynName(format!("{name}Check"))}, width: 16.0, height: 16.0,
        left: {width-22.0}, top: {(height-16.0)/2.0}, hidden, pos_type: "absolute",
        texture_atlas: "groupfinder-icon-greencheckmark" } },
    );
    let color = if enabled {
        NORMAL_FONT_COLOR
    } else {
        "0.5,0.5,0.5,1"
    };
    let label_height = if text.contains('\n') {
        height
    } else {
        height.min(20.0)
    };
    content.extend(label(
        format!("{name}Name"),
        text,
        (
            8.0,
            (height - label_height) / 2.0,
            width - 32.0,
            label_height,
        ),
        (12.0, color, "LEFT"),
    ));
    row(name, action, enabled, rect, content)
}

/// These rows opt into native keyboard focus; ordinary push buttons keep their
/// existing behavior. Their hover atlas identifies the shared row representation.
pub fn is_selection_row(frame: &Frame) -> bool {
    let Some(WidgetData::Button(button)) = &frame.widget_data else {
        return false;
    };
    !button.use_default_skin
        && matches!(&button.highlight_texture, Some(TextureSource::Atlas(name)) if name == HOVER)
}
