//! Mouse input for the ui-toolkit scroll lists a canvas draws (`FrameRegistry::scroll_lists`):
//! the Godot side of the toolkit's Bevy `scroll_input.rs`. The wheel over a list scrolls it
//! (`ScrollControllerMixin:OnMouseWheel`, `Blizzard_SharedXML/Shared/Scroll/ScrollController.lua:93-99`):
//! a row per notch for a row list, two pan extents of pixels for the Options content, one
//! 30-pixel pan extent for a QuestFrame scroll frame (`ScrollUtil.lua:235-238,260-262`). A press
//! on a Back / Forward stepper scrolls one pan extent (`ScrollBarMixin:OnStepperMouseDown`,
//! `ScrollBar.lua:307-311`); a press on its thumb drags it until release. A changed position
//! rebuilds the Screens that read it.

use game_engine_ui_model::game_menu_component::GameMenuViewModel;
use game_engine_ui_model::options_menu_component::{
    OPTIONS_CONTENT_SCROLL, back_stepper_name, forward_stepper_name, options_pan_extent,
    options_wheel_extent,
};
use game_engine_ui_model::quest_frame_component::{QUEST_SCROLL_FRAMES, SCROLL_PAN_EXTENT};
use godot::classes::{InputEvent, InputEventMouseButton, InputEventMouseMotion};
use godot::global::MouseButton;
use godot::prelude::*;
use ui_toolkit::frame::WidgetData;
use ui_toolkit::registry::FrameRegistry;
use ui_toolkit::widgets::button::ButtonState;
use ui_toolkit::widgets::scroll_list::{thumb_name, track_name};

use super::{RegistryModel, RegistryUi};

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

/// How far `list` scrolls per wheel notch and per stepper press, in its scroll units.
fn scroll_steps(model: &RegistryModel, list: &str) -> (usize, usize) {
    use game_engine_ui_model::character_frame::{
        REPUTATION_DESCRIPTION_SCROLL, REPUTATION_SCROLL, reputation_description_pan_extent,
        reputation_pan_extent,
    };
    if list == REPUTATION_DESCRIPTION_SCROLL {
        let pan = reputation_description_pan_extent();
        return (pan * 2, pan);
    }
    if list == REPUTATION_SCROLL {
        let pan = reputation_pan_extent();
        return (pan * 2, pan);
    }
    if QUEST_SCROLL_FRAMES.contains(&list) {
        return (SCROLL_PAN_EXTENT, SCROLL_PAN_EXTENT);
    }
    let options = (list == OPTIONS_CONTENT_SCROLL)
        .then(|| model.shared.get::<GameMenuViewModel>())
        .flatten();
    match options {
        Some(view) => (
            options_wheel_extent(&view.options),
            options_pan_extent(&view.options),
        ),
        None => (1, 1),
    }
}

/// One wheel notch over `list`; returns whether it moved.
pub(super) fn wheel(model: &mut RegistryModel, list: &str, up: bool) -> bool {
    let (step, _) = scroll_steps(model, list);
    let step = step as isize;
    model
        .registry
        .scroll_lists
        .scroll_by(list, if up { -step } else { step })
}

/// A press on frame `hit` steps its list when `hit` is a Back or Forward stepper; returns
/// whether it was one. A stepper at its end of the list is disabled and does nothing
/// (`ScrollBarMixin:Update`, `ScrollBar.lua:237-239`).
pub(super) fn press_stepper(model: &mut RegistryModel, hit: u64) -> bool {
    let Some(list) = list_containing(&model.registry, hit) else {
        return false;
    };
    let name = model.registry.get(hit).and_then(|frame| frame.name.clone());
    let direction = match name {
        Some(name) if name == back_stepper_name(&list) => -1,
        Some(name) if name == forward_stepper_name(&list) => 1,
        _ => return false,
    };
    let (_, step) = scroll_steps(model, &list);
    model
        .registry
        .scroll_lists
        .scroll_by(&list, direction * step as isize);
    true
}

/// A press on frame `hit` at UI-unit height `y` grabs its list's thumb when `hit` is the thumb.
pub(super) fn press_thumb(registry: &mut FrameRegistry, hit: u64, y: f32) -> bool {
    let Some(list) = list_containing(registry, hit) else {
        return false;
    };
    if registry.get_by_name(&track_name(&list)) == Some(hit) {
        return page_track(registry, &list, y);
    }
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

/// Retail ScrollBar.lua:122-132 pages .95 of the visible extent, not the track height.
fn page_track(registry: &mut FrameRegistry, list: &str, y: f32) -> bool {
    let Some(thumb) = registry
        .get_by_name(&thumb_name(list))
        .and_then(|id| registry.get(id))
        .and_then(|frame| frame.layout_rect.as_ref())
    else {
        return false;
    };
    let direction = if y < thumb.y {
        -1
    } else if y > thumb.y + thumb.height {
        1
    } else {
        return true;
    };
    let visible = registry
        .scroll_lists
        .get(list)
        .unwrap()
        .geometry
        .visible_rows;
    let page = (visible as f32 * 0.95).round() as isize;
    registry.scroll_lists.scroll_by(list, direction * page);
    true
}

/// Retail ScrollBar.lua:307-340, ScrollBar.xml:16-17: one immediate step, then a
/// strict >0.5s delay and >0.1s intervals. Leaving pauses elapsed time; no catch-up steps.
pub(super) struct StepperHold {
    name: String,
    elapsed: f64,
    delay: f64,
    over: bool,
}

impl StepperHold {
    fn press(model: &mut RegistryModel, hit: u64) -> (bool, Option<Self>) {
        if !press_stepper(model, hit) {
            return (false, None);
        }
        let frame = model.registry.get(hit).unwrap();
        let disabled = matches!(&frame.widget_data,
            Some(WidgetData::Button(button)) if button.state == ButtonState::Disabled);
        if disabled {
            return (true, None);
        }
        let held = Self {
            name: frame.name.clone().unwrap(),
            elapsed: 0.0,
            delay: 0.5,
            over: true,
        };
        (true, Some(held))
    }

    fn advance(&mut self, model: &mut RegistryModel, delta: f64) -> bool {
        let Some(hit) = model.registry.get_by_name(&self.name) else {
            return false;
        };
        let frame = model.registry.get(hit).unwrap();
        if !self.over || !frame.visible {
            return false;
        }
        self.elapsed += delta;
        if self.elapsed <= self.delay {
            return false;
        }
        self.elapsed = 0.0;
        self.delay = 0.1;
        let list = list_containing(&model.registry, hit).unwrap();
        let before = model.registry.scroll_lists.get(&list).unwrap().first_row;
        press_stepper(model, hit);
        model.registry.scroll_lists.get(&list).unwrap().first_row != before
    }
}

/// Move grabbed thumbs to UI-unit height `y`; returns whether a thumb is grabbed.
pub(super) fn drag_thumbs(registry: &mut FrameRegistry, y: f32) -> bool {
    let targets: Vec<(String, usize)> = registry
        .scroll_lists
        .dragging()
        .filter_map(|(name, state)| {
            let track = registry.get(registry.get_by_name(&track_name(name))?)?;
            let thumb = registry.get(registry.get_by_name(&thumb_name(name))?)?;
            let track = track.layout_rect.as_ref()?;
            let travel = track.height - thumb.layout_rect.as_ref()?.height;
            let thumb_top = y - state.drag_grab? - track.y;
            let fraction = if travel > 0.0 {
                (thumb_top / travel).clamp(0.0, 1.0)
            } else {
                0.0
            };
            let row = (fraction * state.geometry.max_first_row() as f32).round() as usize;
            Some((name.to_string(), row))
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
    pub(super) fn advance_scroll_stepper(&mut self, delta: f64) -> Result<(), String> {
        let changed = match (self.model.as_mut(), self.scroll_stepper.as_mut()) {
            (Some(model), Some(held)) => held.advance(model, delta),
            _ => false,
        };
        if changed {
            self.sync_model()?;
        }
        Ok(())
    }

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
            let hit = self.pointer_frame_at(motion.get_position());
            if let (Some(model), Some(held)) = (self.model.as_ref(), self.scroll_stepper.as_mut()) {
                held.over = hit == model.registry.get_by_name(&held.name);
            }
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
        let y = at.y / model.registry.ui_scale;
        match button.get_button_index() {
            index @ (MouseButton::WHEEL_UP | MouseButton::WHEEL_DOWN) => {
                let Some(list) = hit.and_then(|hit| list_containing(&model.registry, hit)) else {
                    return false;
                };
                if button.is_pressed() {
                    wheel(model, &list, index == MouseButton::WHEEL_UP);
                }
                true
            }
            MouseButton::LEFT if button.is_pressed() => hit.is_some_and(|hit| {
                let (taken, held) = StepperHold::press(model, hit);
                self.scroll_stepper = held;
                taken || press_thumb(&mut model.registry, hit, y)
            }),
            MouseButton::LEFT => {
                let held = self.scroll_stepper.take().is_some();
                release_thumbs(&mut model.registry) || held
            }
            _ => false,
        }
    }
}

#[cfg(test)]
#[path = "scroll_lists_tests.rs"]
pub(super) mod tests;

#[cfg(test)]
#[path = "quest_scroll_tests.rs"]
mod quest_tests;

#[cfg(test)]
#[path = "character_reputation_scroll_lists_tests.rs"]
mod character_reputation_tests;
