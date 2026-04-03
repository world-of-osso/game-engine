use game_engine::ui::frame::WidgetData;
use game_engine::ui::input::find_frame_at;
use game_engine::ui::plugin::UiState;
use game_engine::ui::registry::FrameRegistry;
use game_engine::ui::widgets::texture::TextureSource;

// --- EditBox manipulation ---

pub fn editbox_backspace(reg: &mut FrameRegistry, id: u64) {
    if let Some(WidgetData::EditBox(eb)) = reg.get_mut(id).and_then(|f| f.widget_data.as_mut())
        && eb.cursor_position > 0
    {
        eb.cursor_position -= 1;
        eb.text.remove(eb.cursor_position);
    }
}

pub fn editbox_delete(reg: &mut FrameRegistry, id: u64) {
    if let Some(WidgetData::EditBox(eb)) = reg.get_mut(id).and_then(|f| f.widget_data.as_mut())
        && eb.cursor_position < eb.text.len()
    {
        eb.text.remove(eb.cursor_position);
    }
}

pub fn editbox_move_cursor(reg: &mut FrameRegistry, id: u64, delta: i32) {
    if let Some(WidgetData::EditBox(eb)) = reg.get_mut(id).and_then(|f| f.widget_data.as_mut()) {
        if delta < 0 {
            eb.cursor_position = eb.cursor_position.saturating_sub((-delta) as usize);
        } else {
            eb.cursor_position = (eb.cursor_position + delta as usize).min(eb.text.len());
        }
    }
}

pub fn editbox_cursor_home(reg: &mut FrameRegistry, id: u64) {
    if let Some(WidgetData::EditBox(eb)) = reg.get_mut(id).and_then(|f| f.widget_data.as_mut()) {
        eb.cursor_position = 0;
    }
}

pub fn editbox_cursor_end(reg: &mut FrameRegistry, id: u64) {
    if let Some(WidgetData::EditBox(eb)) = reg.get_mut(id).and_then(|f| f.widget_data.as_mut()) {
        eb.cursor_position = eb.text.len();
    }
}

pub fn insert_char_into_editbox(reg: &mut FrameRegistry, id: u64, ch: &str) {
    insert_text_into_editbox(reg, id, ch);
}

pub fn insert_text_into_editbox(reg: &mut FrameRegistry, id: u64, text: &str) {
    let filtered: String = text.chars().filter(|c| !c.is_control()).collect();
    if filtered.is_empty() {
        return;
    }
    if let Some(WidgetData::EditBox(eb)) = reg.get_mut(id).and_then(|f| f.widget_data.as_mut()) {
        let mut insert = filtered;

        if let Some(max_letters) = eb.max_letters {
            let remaining = (max_letters as usize).saturating_sub(eb.text.chars().count());
            if remaining == 0 {
                return;
            }
            insert = insert.chars().take(remaining).collect();
        }

        if let Some(max_bytes) = eb.max_bytes {
            let remaining = (max_bytes as usize).saturating_sub(eb.text.len());
            if remaining == 0 {
                return;
            }
            let mut truncated = String::new();
            for ch in insert.chars() {
                if truncated.len() + ch.len_utf8() > remaining {
                    break;
                }
                truncated.push(ch);
            }
            insert = truncated;
        }

        if insert.is_empty() {
            return;
        }

        eb.text.insert_str(eb.cursor_position, &insert);
        eb.cursor_position += insert.len();
    }
}

pub fn set_editbox_text(reg: &mut FrameRegistry, id: u64, text: &str) {
    if let Some(WidgetData::EditBox(eb)) = reg.get_mut(id).and_then(|f| f.widget_data.as_mut()) {
        eb.text = text.to_string();
        eb.cursor_position = text.len();
    }
}

pub fn get_editbox_text(reg: &FrameRegistry, id: u64) -> String {
    reg.get(id)
        .and_then(|f| match &f.widget_data {
            Some(WidgetData::EditBox(eb)) => Some(eb.text.clone()),
            _ => None,
        })
        .unwrap_or_default()
}

// --- Frame helpers ---

pub fn hit_frame(ui: &UiState, frame_id: u64, mx: f32, my: f32) -> bool {
    ui.registry.get(frame_id).is_some_and(|f| {
        f.layout_rect
            .as_ref()
            .is_some_and(|r| mx >= r.x && mx <= r.x + r.width && my >= r.y && my <= r.y + r.height)
            && topmost_frame_at(ui, mx, my)
                .is_some_and(|hit_id| frame_or_ancestor_matches(&ui.registry, hit_id, frame_id))
    })
}

pub fn topmost_frame_at(ui: &UiState, mx: f32, my: f32) -> Option<u64> {
    find_frame_at(&ui.registry, mx, my)
}

pub fn frame_or_ancestor_matches(reg: &FrameRegistry, mut frame_id: u64, target_id: u64) -> bool {
    loop {
        if frame_id == target_id {
            return true;
        }
        let Some(frame) = reg.get(frame_id) else {
            return false;
        };
        let Some(parent_id) = frame.parent_id else {
            return false;
        };
        frame_id = parent_id;
    }
}

// --- Button visual helpers ---

const LOGIN_BUTTON_GENERATED_REGULAR_UP_ATLAS: &str = "defaultbutton-nineslice-up";
const LOGIN_BUTTON_GENERATED_REGULAR_PRESSED_ATLAS: &str = "defaultbutton-nineslice-pressed";
const LOGIN_BUTTON_GENERATED_REGULAR_HIGHLIGHT_ATLAS: &str = "defaultbutton-nineslice-highlight";
const LOGIN_BUTTON_GENERATED_REGULAR_DISABLED_ATLAS: &str = "defaultbutton-nineslice-disabled";
const LOGIN_BUTTON_GENERATED_REGULAR_RAW: &str = "output/imagegen/button-dark-bronze-regular.ktx2";
const LOGIN_BUTTON_GENERATED_KNOTWORK: &str = "output/imagegen/button-carved-bronze-knotwork.ktx2";
const LOGIN_BUTTON_GENERATED_WALNUT: &str = "output/imagegen/button-walnut-bronze-framed.ktx2";

pub fn set_button_atlases(
    reg: &mut FrameRegistry,
    id: u64,
    normal: &str,
    pushed: &str,
    highlight: &str,
    disabled: &str,
) {
    if let Some(WidgetData::Button(bd)) = reg.get_mut(id).and_then(|f| f.widget_data.as_mut()) {
        bd.normal_texture = Some(TextureSource::Atlas(normal.to_string()));
        bd.pushed_texture = Some(TextureSource::Atlas(pushed.to_string()));
        bd.highlight_texture = Some(TextureSource::Atlas(highlight.to_string()));
        bd.disabled_texture = Some(TextureSource::Atlas(disabled.to_string()));
    }
}

pub fn set_button_files(
    reg: &mut FrameRegistry,
    id: u64,
    normal: &str,
    pushed: &str,
    highlight: &str,
    disabled: &str,
) {
    if let Some(WidgetData::Button(bd)) = reg.get_mut(id).and_then(|f| f.widget_data.as_mut()) {
        bd.normal_texture = Some(TextureSource::File(normal.to_string()));
        bd.pushed_texture = Some(TextureSource::File(pushed.to_string()));
        bd.highlight_texture = Some(TextureSource::File(highlight.to_string()));
        bd.disabled_texture = Some(TextureSource::File(disabled.to_string()));
    }
}

pub fn set_button_hovered(reg: &mut FrameRegistry, id: u64, hovered: bool) {
    if let Some(WidgetData::Button(bd)) = reg.get_mut(id).and_then(|f| f.widget_data.as_mut()) {
        bd.hovered = hovered;
    }
}

pub fn set_login_primary_button_textures(reg: &mut FrameRegistry, id: u64) {
    match selected_generated_login_button_path() {
        Some(path) => set_button_files(reg, id, path, path, path, path),
        None => set_button_atlases(
            reg,
            id,
            LOGIN_BUTTON_GENERATED_REGULAR_UP_ATLAS,
            LOGIN_BUTTON_GENERATED_REGULAR_PRESSED_ATLAS,
            LOGIN_BUTTON_GENERATED_REGULAR_HIGHLIGHT_ATLAS,
            LOGIN_BUTTON_GENERATED_REGULAR_DISABLED_ATLAS,
        ),
    }
}

fn selected_generated_login_button_path() -> Option<&'static str> {
    match std::env::var("LOGIN_BUTTON_VARIANT").ok().as_deref() {
        Some("regular") => Some(LOGIN_BUTTON_GENERATED_REGULAR_RAW),
        Some("knotwork") => Some(LOGIN_BUTTON_GENERATED_KNOTWORK),
        Some("walnut") => Some(LOGIN_BUTTON_GENERATED_WALNUT),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use game_engine::ui::frame::Dimension;
    use game_engine::ui::layout::LayoutRect;
    use game_engine::ui::strata::FrameStrata;

    fn set_rect(reg: &mut FrameRegistry, id: u64, x: f32, y: f32, w: f32, h: f32) {
        let frame = reg.get_mut(id).expect("frame should exist");
        frame.width = Dimension::Fixed(w);
        frame.height = Dimension::Fixed(h);
        frame.layout_rect = Some(LayoutRect {
            x,
            y,
            width: w,
            height: h,
        });
    }

    fn make_ui_state(registry: FrameRegistry) -> UiState {
        UiState {
            registry,
            event_bus: game_engine::ui::event::EventBus::new(),
            focused_frame: None,
        }
    }

    #[test]
    fn hit_frame_rejects_controls_occluded_by_overlay() {
        let mut registry = FrameRegistry::new(1920.0, 1080.0);
        let button = registry.create_frame("Button", None);
        set_rect(&mut registry, button, 800.0, 520.0, 250.0, 66.0);
        registry.get_mut(button).expect("button").mouse_enabled = true;

        let overlay = registry.create_frame("Overlay", None);
        set_rect(&mut registry, overlay, 0.0, 0.0, 1920.0, 1080.0);
        let overlay_frame = registry.get_mut(overlay).expect("overlay");
        overlay_frame.mouse_enabled = true;
        overlay_frame.strata = FrameStrata::Dialog;
        overlay_frame.frame_level = 100;

        let ui = make_ui_state(registry);

        assert_eq!(topmost_frame_at(&ui, 810.0, 530.0), Some(overlay));
        assert!(!hit_frame(&ui, button, 810.0, 530.0));
    }
}
