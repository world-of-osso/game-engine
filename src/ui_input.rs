use bevy::prelude::KeyCode;
use game_engine::ui::frame::WidgetData;
use game_engine::ui::registry::FrameRegistry;

pub fn walk_up_for_onclick(reg: &FrameRegistry, mut id: u64) -> Option<String> {
    loop {
        if let Some(frame) = reg.get(id) {
            if let Some(ref action) = frame.onclick {
                return Some(action.clone());
            }
            if let Some(parent) = frame.parent_id {
                id = parent;
                continue;
            }
        }
        return None;
    }
}

/// Applies one pressed key to an editbox; returns whether the editbox handled it.
pub fn mutate_editbox_from_key(
    registry: &mut FrameRegistry,
    focused_id: u64,
    event: &bevy::input::keyboard::KeyboardInput,
) -> bool {
    let Some(WidgetData::EditBox(editbox)) = registry
        .get_mut(focused_id)
        .and_then(|frame| frame.widget_data.as_mut())
    else {
        return false;
    };
    match event.key_code {
        KeyCode::Backspace => editbox.backspace(),
        KeyCode::Delete => editbox.delete_forward(),
        KeyCode::ArrowLeft => editbox.cursor_left(),
        KeyCode::ArrowRight => editbox.cursor_right(),
        KeyCode::Home => editbox.cursor_home(),
        KeyCode::End => editbox.cursor_end(),
        _ => {
            let Some(text) = event.text.as_ref() else {
                return false;
            };
            editbox.insert_at_cursor(text.as_str());
        }
    }
    true
}
