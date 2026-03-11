use game_engine::ui::frame::WidgetData;
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
    if !ch.chars().all(|c| !c.is_control()) {
        return;
    }
    if let Some(WidgetData::EditBox(eb)) = reg.get_mut(id).and_then(|f| f.widget_data.as_mut()) {
        if eb
            .max_letters
            .is_some_and(|max| eb.text.len() >= max as usize)
        {
            return;
        }
        eb.text.insert_str(eb.cursor_position, ch);
        eb.cursor_position += ch.len();
    }
}

#[allow(dead_code)]
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
    })
}

// --- Button visual helpers ---

#[allow(dead_code)]
const LOGIN_BUTTON_GENERATED_REGULAR_UP_ATLAS: &str = "defaultbutton-nineslice-up";
#[allow(dead_code)]
const LOGIN_BUTTON_GENERATED_REGULAR_PRESSED_ATLAS: &str = "defaultbutton-nineslice-pressed";
#[allow(dead_code)]
const LOGIN_BUTTON_GENERATED_REGULAR_HIGHLIGHT_ATLAS: &str = "defaultbutton-nineslice-highlight";
#[allow(dead_code)]
const LOGIN_BUTTON_GENERATED_REGULAR_DISABLED_ATLAS: &str = "defaultbutton-nineslice-disabled";
#[allow(dead_code)]
const LOGIN_BUTTON_GENERATED_REGULAR_RAW: &str = "output/imagegen/button-dark-bronze-regular.ktx2";
#[allow(dead_code)]
const LOGIN_BUTTON_GENERATED_KNOTWORK: &str = "output/imagegen/button-carved-bronze-knotwork.ktx2";
#[allow(dead_code)]
const LOGIN_BUTTON_GENERATED_WALNUT: &str = "output/imagegen/button-walnut-bronze-framed.ktx2";

#[allow(dead_code)]
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

#[allow(dead_code)]
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

#[allow(dead_code)]
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

#[allow(dead_code)]
fn selected_generated_login_button_path() -> Option<&'static str> {
    match std::env::var("LOGIN_BUTTON_VARIANT").ok().as_deref() {
        Some("regular") => Some(LOGIN_BUTTON_GENERATED_REGULAR_RAW),
        Some("knotwork") => Some(LOGIN_BUTTON_GENERATED_KNOTWORK),
        Some("walnut") => Some(LOGIN_BUTTON_GENERATED_WALNUT),
        _ => None,
    }
}

