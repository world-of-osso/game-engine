//! A unit frame's Edit Mode size and text settings, applied to the frame as it was built
//! at its authored size (docs/specs/hud-edit-mode.md "Customisable layout settings").

use game_engine_core::ui_layout_data::{
    LayoutFont, SettingRange, UNIT_FRAME_SIZE_RANGE, UNIT_FRAME_TEXT_SIZE_RANGE, UnitFrameSettings,
};
use ui_toolkit::widget_def::{Attr, AttrValue, Element, WidgetChild, WidgetDef};
use ui_toolkit::widgets::font_string::GameFont;

use crate::hud_layout::HudAnchor;

/// How a unit frame is drawn over its authored shape.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct UnitFrameStyle {
    /// `FrameSize` / 100: Retail's `SetScale` on the frame
    /// (`EditModeUnitFrameSystemMixin:UpdateSystemSettingFrameSize`,
    /// EditModeSystemTemplates.lua:1547-1558).
    pub scale: f32,
    /// The face of every text in the frame; `None` keeps each text's authored face.
    pub font: Option<GameFont>,
    /// Text size / 100, on top of `scale`.
    pub text_scale: f32,
}

impl UnitFrameStyle {
    /// The frame as its preset authors it.
    pub const AUTHORED: Self = Self {
        scale: 1.0,
        font: None,
        text_scale: 1.0,
    };

    /// A layout's settings within their Edit Mode ranges; an unset one keeps the preset's.
    pub fn of(settings: UnitFrameSettings) -> Self {
        let percent = |setting: Option<u16>, range: SettingRange| {
            setting.map_or(1.0, |value| f32::from(range.clamp(value)) / 100.0)
        };
        Self {
            scale: percent(settings.frame_size, UNIT_FRAME_SIZE_RANGE),
            font: settings.font.map(|font| match font {
                LayoutFont::FrizQuadrata => GameFont::FrizQuadrata,
                LayoutFont::ArialNarrow => GameFont::ArialNarrow,
            }),
            text_scale: percent(settings.text_size, UNIT_FRAME_TEXT_SIZE_RANGE),
        }
    }
}

const LENGTH_ATTRS: [&str; 10] = [
    "width",
    "height",
    "pos_x",
    "pos_y",
    "left",
    "top",
    "right",
    "bottom",
    "margin_left",
    "margin_top",
];
const PLACEMENT_ATTRS: [&str; 6] = [
    "left",
    "right",
    "top",
    "bottom",
    "margin_left",
    "margin_top",
];

fn set_attr(widget: &mut WidgetDef, name: &'static str, value: String) {
    match widget
        .attrs
        .iter_mut()
        .find(|attr| attr.effective_name() == name)
    {
        Some(attr) => attr.value = AttrValue::Dynamic(value),
        None => widget.attrs.push(Attr::new_dynamic(name, value)),
    }
}

/// Multiply a numeric attribute; "auto", "fill" and percentages keep their meaning.
fn scale_attr(attr: &mut Attr, factor: f32) {
    if let Ok(value) = attr.value_str().parse::<f32>() {
        attr.value = AttrValue::Dynamic((value * factor).to_string());
    }
}

fn restyle_widget(widget: &mut WidgetDef, style: &UnitFrameStyle, root: bool) {
    let is_text = widget
        .attrs
        .iter()
        .any(|attr| attr.effective_name() == "font_size");
    for attr in &mut widget.attrs {
        let name = attr.effective_name();
        if name == "font_size" {
            scale_attr(attr, style.scale * style.text_scale);
        } else if LENGTH_ATTRS.contains(&name) && !(root && PLACEMENT_ATTRS.contains(&name)) {
            scale_attr(attr, style.scale);
        }
    }
    if let (true, Some(font)) = (is_text, style.font) {
        set_attr(widget, "font", font.to_string());
    }
    restyle_children(&mut widget.children, style);
}

fn restyle_children(children: &mut [WidgetChild], style: &UnitFrameStyle) {
    for child in children {
        match child {
            WidgetChild::Widget(widget) => restyle_widget(widget, style, false),
            WidgetChild::Fragment(children) => restyle_children(children, style),
            WidgetChild::Dynamic => {}
        }
    }
}

fn attr_length(widget: &WidgetDef, name: &str) -> f32 {
    widget
        .attrs
        .iter()
        .find(|attr| attr.effective_name() == name)
        .and_then(|attr| attr.value_str().parse().ok())
        .unwrap_or_else(|| panic!("a styled frame has a numeric {name}"))
}

/// One frame built at its authored size, drawn under `style`: Retail's `SetScale` on the
/// frame (every length inside it, and its texts, take the scale) with its `anchor` point
/// kept in place (`EditModeSystemMixin:SetScaleOverride`, EditModeSystemTemplates.lua:117-129),
/// and its texts in the style's face and size.
pub fn styled_frame(mut frame: Element, style: &UnitFrameStyle, anchor: &HudAnchor) -> Element {
    if *style == UnitFrameStyle::AUTHORED {
        return frame;
    }
    for child in &mut frame {
        let WidgetChild::Widget(root) = child else {
            panic!("a styled frame is one root widget");
        };
        restyle_widget(root, style, true);
        let at = anchor.place((attr_length(root, "width"), attr_length(root, "height")));
        set_attr(root, "left", at.left);
        set_attr(root, "right", at.right);
        set_attr(root, "top", at.top);
        set_attr(root, "bottom", at.bottom);
        set_attr(root, "margin_left", at.margin_left.to_string());
        set_attr(root, "margin_top", at.margin_top.to_string());
    }
    frame
}
