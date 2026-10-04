//! Right-click hit test for the Key Bindings page's binding buttons
//! (docs/specs/key-bindings.md "Binding buttons").

use game_engine_core::input_bindings_data::{BindingSection, InputAction, actions_for_section};
use game_engine_ui_model::options_menu_component::keybinding_button_name;
use ui_toolkit::registry::FrameRegistry;

/// The action of `section` whose shown binding button contains `point` (canvas UI units).
/// Rows scrolled out of the content area have no frames.
pub(crate) fn keybinding_button_at(
    registry: &FrameRegistry,
    section: BindingSection,
    point: [f32; 2],
) -> Option<InputAction> {
    actions_for_section(section).iter().copied().find(|action| {
        let Some(id) = registry.get_by_name(&keybinding_button_name(*action)) else {
            return false;
        };
        let Some(rect) = registry
            .get(id)
            .and_then(|frame| frame.layout_rect.as_ref())
        else {
            return false;
        };
        shown(registry, id)
            && (rect.x..rect.x + rect.width).contains(&point[0])
            && (rect.y..rect.y + rect.height).contains(&point[1])
    })
}

fn shown(registry: &FrameRegistry, id: u64) -> bool {
    let mut id = Some(id);
    while let Some(frame) = id.and_then(|id| registry.get(id)) {
        if frame.hidden {
            return false;
        }
        id = frame.parent_id;
    }
    true
}

#[cfg(test)]
#[path = "options_keybindings_tests.rs"]
mod tests;
