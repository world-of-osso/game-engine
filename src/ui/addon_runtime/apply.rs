use bevy::log::warn;
use ui_toolkit::anchor::Anchor;
use ui_toolkit::frame::{Dimension, WidgetData, WidgetType};
use ui_toolkit::widgets::font_string::FontStringData;

use super::{AddonOperation, LoadedAddon};

struct AnchorUpdate<'a> {
    name: &'a str,
    point: ui_toolkit::anchor::AnchorPoint,
    relative_to: Option<&'a str>,
    relative_point: ui_toolkit::anchor::AnchorPoint,
    x: f32,
    y: f32,
}

pub(super) fn remove_owned_frames(
    registry: &mut ui_toolkit::registry::FrameRegistry,
    owned_frames: &std::collections::HashSet<String>,
) {
    let mut names = owned_frames.iter().collect::<Vec<_>>();
    names.sort();
    for name in names {
        if let Some(id) = registry.get_by_name(name) {
            registry.remove_frame_tree(id);
        }
    }
}

pub(super) fn apply_addon(addon: &LoadedAddon, registry: &mut ui_toolkit::registry::FrameRegistry) {
    for operation in &addon.operations {
        apply_operation(addon, operation, registry);
    }
}

fn apply_operation(
    addon: &LoadedAddon,
    operation: &AddonOperation,
    registry: &mut ui_toolkit::registry::FrameRegistry,
) {
    if apply_create_operation(addon, operation, registry) {
        return;
    }
    apply_update_operation(addon, operation, registry);
}

fn apply_create_operation(
    addon: &LoadedAddon,
    operation: &AddonOperation,
    registry: &mut ui_toolkit::registry::FrameRegistry,
) -> bool {
    match operation {
        AddonOperation::CreateFrame { name, parent } => {
            let _ = ensure_owned_frame(addon, registry, name, parent.as_deref(), WidgetType::Frame);
            true
        }
        AddonOperation::CreateFontString { name, parent, text } => {
            create_owned_fontstring(addon, registry, name, parent.as_deref(), text);
            true
        }
        _ => false,
    }
}

fn apply_update_operation(
    addon: &LoadedAddon,
    operation: &AddonOperation,
    registry: &mut ui_toolkit::registry::FrameRegistry,
) {
    if apply_layout_operation(addon, operation, registry) {
        return;
    }
    apply_visual_operation(addon, operation, registry);
}

fn apply_layout_operation(
    addon: &LoadedAddon,
    operation: &AddonOperation,
    registry: &mut ui_toolkit::registry::FrameRegistry,
) -> bool {
    match operation {
        AddonOperation::SetSize {
            name,
            width,
            height,
        } => {
            resize_owned_frame(addon, registry, name, *width, *height);
            true
        }
        AddonOperation::SetPoint {
            name,
            point,
            relative_to,
            relative_point,
            x,
            y,
        } => {
            anchor_owned_frame(
                addon,
                registry,
                AnchorUpdate {
                    name,
                    point: *point,
                    relative_to: relative_to.as_deref(),
                    relative_point: *relative_point,
                    x: *x,
                    y: *y,
                },
            );
            true
        }
        _ => false,
    }
}

fn apply_visual_operation(
    addon: &LoadedAddon,
    operation: &AddonOperation,
    registry: &mut ui_toolkit::registry::FrameRegistry,
) {
    match operation {
        AddonOperation::SetText { name, text } => update_owned_text(addon, registry, name, text),
        AddonOperation::Show { name } => set_owned_visibility(addon, registry, name, true),
        AddonOperation::Hide { name } => set_owned_visibility(addon, registry, name, false),
        AddonOperation::SetAlpha { name, alpha } => set_owned_alpha(addon, registry, name, *alpha),
        AddonOperation::SetBackgroundColor { name, color } => {
            set_owned_background(addon, registry, name, *color);
        }
        AddonOperation::SetFontColor { name, color } => {
            set_owned_font_color(addon, registry, name, *color);
        }
        AddonOperation::CreateFrame { .. }
        | AddonOperation::CreateFontString { .. }
        | AddonOperation::SetSize { .. }
        | AddonOperation::SetPoint { .. } => {}
    }
}

fn create_owned_fontstring(
    addon: &LoadedAddon,
    registry: &mut ui_toolkit::registry::FrameRegistry,
    name: &str,
    parent: Option<&str>,
    text: &str,
) {
    let Some(frame_id) = ensure_owned_frame(addon, registry, name, parent, WidgetType::FontString)
    else {
        return;
    };
    set_fontstring_text(registry, frame_id, text);
}

fn resize_owned_frame(
    addon: &LoadedAddon,
    registry: &mut ui_toolkit::registry::FrameRegistry,
    name: &str,
    width: f32,
    height: f32,
) {
    let Some(frame_id) = owned_frame_id(addon, registry, name) else {
        return;
    };
    if let Some(frame) = registry.get_mut(frame_id) {
        frame.width = Dimension::Fixed(width);
        frame.height = Dimension::Fixed(height);
    }
}

fn anchor_owned_frame(
    addon: &LoadedAddon,
    registry: &mut ui_toolkit::registry::FrameRegistry,
    update: AnchorUpdate<'_>,
) {
    let Some(frame_id) = owned_frame_id(addon, registry, update.name) else {
        return;
    };
    let relative_to = update
        .relative_to
        .and_then(|frame_name| registry.get_by_name(frame_name));
    let _ = registry.set_point(
        frame_id,
        Anchor {
            point: update.point,
            relative_to,
            relative_point: update.relative_point,
            x_offset: update.x,
            y_offset: update.y,
        },
    );
}

fn update_owned_text(
    addon: &LoadedAddon,
    registry: &mut ui_toolkit::registry::FrameRegistry,
    name: &str,
    text: &str,
) {
    let Some(frame_id) = owned_frame_id(addon, registry, name) else {
        return;
    };
    set_fontstring_text(registry, frame_id, text);
}

fn set_owned_visibility(
    addon: &LoadedAddon,
    registry: &mut ui_toolkit::registry::FrameRegistry,
    name: &str,
    visible: bool,
) {
    let Some(frame_id) = owned_frame_id(addon, registry, name) else {
        return;
    };
    if let Some(frame) = registry.get_mut(frame_id) {
        frame.hidden = !visible;
        frame.visible = visible;
    }
}

fn set_owned_alpha(
    addon: &LoadedAddon,
    registry: &mut ui_toolkit::registry::FrameRegistry,
    name: &str,
    alpha: f32,
) {
    let Some(frame_id) = owned_frame_id(addon, registry, name) else {
        return;
    };
    if let Some(frame) = registry.get_mut(frame_id) {
        frame.alpha = alpha.clamp(0.0, 1.0);
        frame.effective_alpha = frame.alpha;
    }
}

fn set_owned_background(
    addon: &LoadedAddon,
    registry: &mut ui_toolkit::registry::FrameRegistry,
    name: &str,
    color: [f32; 4],
) {
    let Some(frame_id) = owned_frame_id(addon, registry, name) else {
        return;
    };
    if let Some(frame) = registry.get_mut(frame_id) {
        frame.background_color = Some(color);
    }
}

fn set_owned_font_color(
    addon: &LoadedAddon,
    registry: &mut ui_toolkit::registry::FrameRegistry,
    name: &str,
    color: [f32; 4],
) {
    let Some(frame_id) = owned_frame_id(addon, registry, name) else {
        return;
    };
    let Some(frame) = registry.get_mut(frame_id) else {
        return;
    };
    let Some(WidgetData::FontString(data)) = frame.widget_data.as_mut() else {
        return;
    };
    data.color = color;
}

fn ensure_owned_frame(
    addon: &LoadedAddon,
    registry: &mut ui_toolkit::registry::FrameRegistry,
    name: &str,
    parent: Option<&str>,
    widget_type: WidgetType,
) -> Option<u64> {
    if !addon.owned_frames.contains(name) {
        return None;
    }
    if let Some(id) = registry.get_by_name(name) {
        let frame = registry.get(id)?;
        if frame.widget_type == widget_type {
            return Some(id);
        }
        warn!(
            "addon {} cannot reuse frame {name} with mismatched widget type",
            addon.name
        );
        return None;
    }
    let parent_id = match parent {
        Some(parent_name) => registry.get_by_name(parent_name)?,
        None => None?,
    };
    let id = registry.create_frame(name, Some(parent_id));
    initialize_widget(registry, id, widget_type)?;
    Some(id)
}

fn initialize_widget(
    registry: &mut ui_toolkit::registry::FrameRegistry,
    frame_id: u64,
    widget_type: WidgetType,
) -> Option<()> {
    let frame = registry.get_mut(frame_id)?;
    frame.widget_type = widget_type;
    if widget_type == WidgetType::FontString {
        frame.widget_data = Some(WidgetData::FontString(FontStringData::default()));
    }
    Some(())
}

fn owned_frame_id(
    addon: &LoadedAddon,
    registry: &mut ui_toolkit::registry::FrameRegistry,
    name: &str,
) -> Option<u64> {
    addon
        .owned_frames
        .contains(name)
        .then(|| registry.get_by_name(name))
        .flatten()
}

fn set_fontstring_text(
    registry: &mut ui_toolkit::registry::FrameRegistry,
    frame_id: u64,
    text: &str,
) {
    let Some(frame) = registry.get_mut(frame_id) else {
        return;
    };
    let Some(WidgetData::FontString(data)) = frame.widget_data.as_mut() else {
        return;
    };
    data.text = text.to_string();
}
