//! FontString display runs; the registry keeps the original authored source text.
use godot::classes::{Control, RichTextLabel, StyleBoxEmpty, text_server::AutowrapMode};
use godot::prelude::*;
use ui_toolkit::{
    frame::{Dimension, Frame, WidgetData},
    layout::LayoutRect,
    registry::FrameRegistry,
    text_markup::{TextRun, display_text, parse_text_markup},
    widgets::font_string::FontStringData,
};

use super::{UiProjection, apply_text_effects, color, horizontal, vertical};

/// Decode once per operation: escaped pipes in display text never re-enter the lexer.
pub(super) fn decode_font_string(data: &FontStringData) -> (FontStringData, Vec<TextRun>) {
    let runs = parse_text_markup(&data.text);
    let mut display = data.clone();
    display.text = display_text(&runs);
    (display, runs)
}

fn runs_have_color(runs: &[TextRun]) -> bool {
    runs.iter().any(|run| run.color.is_some())
}

pub(super) fn uses_colored_text(frame: &Frame) -> bool {
    match &frame.widget_data {
        Some(WidgetData::FontString(data)) => runs_have_color(&parse_text_markup(&data.text)),
        _ => false,
    }
}

fn move_text_children(old: &Gd<Control>, new: &Gd<Control>) {
    for mut child in old.get_children().iter_shared() {
        if child.get_name().to_string() != super::PARTS_NODE {
            child.reparent_ex(new).keep_global_transform(false).done();
        }
    }
}

impl UiProjection {
    pub(super) fn replace_font_string_node(
        &mut self,
        frame: &Frame,
        registry: &FrameRegistry,
        parent: &Gd<Control>,
    ) -> Result<bool, String> {
        let Some(WidgetData::FontString(_)) = &frame.widget_data else {
            return Ok(false);
        };
        let Some(mut old) = self.nodes.get(&frame.id).cloned() else {
            return Ok(false);
        };
        if old.is_class("RichTextLabel") == uses_colored_text(frame) {
            return Ok(false);
        }
        let new = self.create_node(frame, registry)?;
        let mut previous_parent = old
            .get_parent()
            .ok_or_else(|| format!("FontString {} has no native parent", frame.id))?;
        previous_parent.remove_child(&old);
        parent.clone().add_child(&new);
        move_text_children(&old, &new);
        old.queue_free();
        self.nodes.insert(frame.id, new);
        self.visuals.remove(&frame.id);
        Ok(true)
    }

    pub(super) fn update_font_string(
        &mut self,
        node: Gd<Control>,
        data: &FontStringData,
        frame: &Frame,
        rect: &LayoutRect,
    ) -> Result<(), String> {
        let (display, runs) = decode_font_string(data);
        if runs_have_color(&runs) {
            self.update_rich_label(node.cast(), &display, &runs, frame, rect)
        } else {
            self.update_label(node.cast(), &display, frame, rect)
        }
    }

    fn update_rich_label(
        &mut self,
        mut node: Gd<RichTextLabel>,
        data: &FontStringData,
        runs: &[TextRun],
        frame: &Frame,
        rect: &LayoutRect,
    ) -> Result<(), String> {
        self.style_rich_label(&mut node, data, frame, rect)?;
        paint_runs(&mut node, runs);
        node.set_size(Vector2::new(rect.width, rect.height));
        Ok(())
    }

    fn style_rich_label(
        &mut self,
        node: &mut Gd<RichTextLabel>,
        data: &FontStringData,
        frame: &Frame,
        rect: &LayoutRect,
    ) -> Result<(), String> {
        let (font, metrics) = self.resolve_font_string(data, frame, 0)?;
        node.add_theme_font_override("normal_font", &font);
        node.add_theme_font_size_override("normal_font_size", data.font_size as i32);
        node.add_theme_color_override("default_color", color(data.color));
        node.add_theme_constant_override("line_separation", metrics.spacing_ui as i32);
        node.add_theme_constant_override("paragraph_separation", 0);
        configure_rich_layout(node, data, frame, rect);
        let shadow = data.shadow_color.map(|color| (color, data.shadow_offset));
        let mut control = node.clone().upcast::<Control>();
        apply_text_effects(&mut control, data.outline, shadow);
        Ok(())
    }
}

fn configure_rich_layout(
    node: &mut Gd<RichTextLabel>,
    data: &FontStringData,
    frame: &Frame,
    rect: &LayoutRect,
) {
    node.set_horizontal_alignment(horizontal(data.justify_h));
    node.set_vertical_alignment(vertical(data.justify_v));
    node.set_scroll_active(false);
    node.set_fit_content(true);
    let wrap = if data.word_wrap && frame.width != Dimension::Auto {
        AutowrapMode::WORD
    } else {
        AutowrapMode::OFF
    };
    node.set_autowrap_mode(wrap);
    let width = if frame.width == Dimension::Auto {
        -1.0
    } else {
        rect.width
    };
    let height = if frame.height == Dimension::Auto {
        -1.0
    } else {
        rect.height
    };
    node.set_custom_maximum_size(Vector2::new(width, height));
    let empty = StyleBoxEmpty::new_gd();
    node.add_theme_stylebox_override("normal", &empty);
    node.add_theme_stylebox_override("focus", &empty);
}

fn paint_runs(node: &mut Gd<RichTextLabel>, runs: &[TextRun]) {
    node.clear();
    for run in runs {
        if let Some(bytes) = run.color {
            let rgba = bytes.map(|channel| f32::from(channel) / 255.0);
            node.push_color(color(rgba));
        }
        node.add_text(&run.text);
        if run.color.is_some() {
            node.pop();
        }
    }
}
