//! Mouse input for the ui-toolkit scroll lists a canvas draws (`FrameRegistry::scroll_lists`):
//! the Godot side of the toolkit's Bevy `scroll_input.rs`. The wheel over a list scrolls it a
//! row per notch, up to earlier rows (`ScrollControllerMixin:OnMouseWheel`,
//! `Blizzard_SharedXML/Shared/Scroll/ScrollController.lua:93-99`); a press on its thumb drags
//! it until release. A changed position rebuilds the Screens that read it.

use godot::classes::{InputEvent, InputEventMouseButton, InputEventMouseMotion};
use godot::global::MouseButton;
use godot::prelude::*;
use ui_toolkit::registry::FrameRegistry;
use ui_toolkit::widgets::scroll_list::{thumb_name, track_name};

use super::RegistryUi;

/// The scroll list frame `id` is, or is inside.
pub(super) fn list_containing(registry: &FrameRegistry, mut id: u64) -> Option<String> {
    loop {
        let frame = registry.get(id)?;
        if let Some(name) = &frame.name
            && registry.scroll_lists.get(name).is_some()
        {
            return Some(name.clone());
        }
        id = frame.parent_id?;
    }
}

/// One wheel notch over `list`; returns whether it moved.
pub(super) fn wheel(registry: &mut FrameRegistry, list: &str, up: bool) -> bool {
    registry
        .scroll_lists
        .scroll_by(list, if up { -1 } else { 1 })
}

/// A press on frame `hit` at UI-unit height `y` grabs its list's thumb when `hit` is the thumb.
pub(super) fn press_thumb(registry: &mut FrameRegistry, hit: u64, y: f32) -> bool {
    let Some(list) = list_containing(registry, hit) else {
        return false;
    };
    if registry.get_by_name(&thumb_name(&list)) != Some(hit) {
        return false;
    }
    let Some(top) = registry
        .get(hit)
        .and_then(|frame| frame.layout_rect.as_ref())
        .map(|rect| rect.y)
    else {
        return false;
    };
    registry.scroll_lists.set_drag(&list, Some(y - top));
    true
}

/// Move grabbed thumbs to UI-unit height `y`; returns whether a thumb is grabbed.
pub(super) fn drag_thumbs(registry: &mut FrameRegistry, y: f32) -> bool {
    let targets: Vec<(String, usize)> = registry
        .scroll_lists
        .dragging()
        .filter_map(|(name, state)| {
            let track = registry.get(registry.get_by_name(&track_name(name))?)?;
            let thumb_top = y - state.drag_grab? - track.layout_rect.as_ref()?.y;
            Some((name.to_string(), state.geometry.row_at_thumb_top(thumb_top)))
        })
        .collect();
    let grabbed = !targets.is_empty();
    for (name, row) in targets {
        registry.scroll_lists.scroll_to(&name, row);
    }
    grabbed
}

/// Let go of grabbed thumbs; returns whether one was grabbed.
pub(super) fn release_thumbs(registry: &mut FrameRegistry) -> bool {
    let names: Vec<String> = registry
        .scroll_lists
        .dragging()
        .map(|(name, _)| name.to_string())
        .collect();
    for name in &names {
        registry.scroll_lists.set_drag(name, None);
    }
    !names.is_empty()
}

impl RegistryUi {
    /// Scroll-list wheel and thumb input; returns whether the event was taken.
    pub(crate) fn scroll_list_input(&mut self, event: &Gd<InputEvent>) -> Result<bool, String> {
        let taken = self.apply_scroll_list_input(event);
        if taken {
            self.sync_model()?;
        }
        Ok(taken)
    }

    fn apply_scroll_list_input(&mut self, event: &Gd<InputEvent>) -> bool {
        if let Ok(motion) = event.clone().try_cast::<InputEventMouseMotion>() {
            let Some(model) = self.model.as_mut() else {
                return false;
            };
            let y = motion.get_position().y / model.registry.ui_scale;
            return drag_thumbs(&mut model.registry, y);
        }
        let Ok(button) = event.clone().try_cast::<InputEventMouseButton>() else {
            return false;
        };
        let at = button.get_position();
        let hit = self.pointer_frame_at(at);
        let Some(model) = self.model.as_mut() else {
            return false;
        };
        let registry = &mut model.registry;
        let y = at.y / registry.ui_scale;
        match button.get_button_index() {
            index @ (MouseButton::WHEEL_UP | MouseButton::WHEEL_DOWN) => {
                let Some(list) = hit.and_then(|hit| list_containing(registry, hit)) else {
                    return false;
                };
                if button.is_pressed() {
                    wheel(registry, &list, index == MouseButton::WHEEL_UP);
                }
                true
            }
            MouseButton::LEFT if button.is_pressed() => {
                hit.is_some_and(|hit| press_thumb(registry, hit, y))
            }
            MouseButton::LEFT => release_thumbs(registry),
            _ => false,
        }
    }
}

#[cfg(test)]
#[path = "scroll_lists_tests.rs"]
pub(super) mod tests;
