use game_engine::ui::anchor::{Anchor, AnchorPoint};
use game_engine::ui::frame::{Backdrop, Frame, WidgetData, WidgetType};
use game_engine::ui::layout::resolve_frame_layout;
use game_engine::ui::plugin::UiState;
use game_engine::ui::registry::FrameRegistry;
use game_engine::ui::strata::FrameStrata;
use game_engine::ui::widgets::button::{ButtonData, CheckButtonData};
use game_engine::ui::widgets::edit_box::EditBoxData;
use game_engine::ui::widgets::font_string::{FontStringData, JustifyH};
use game_engine::ui::widgets::texture::TextureSource;

// --- EditBox manipulation ---

pub fn editbox_backspace(reg: &mut FrameRegistry, id: u64) {
    if let Some(WidgetData::EditBox(eb)) = reg.get_mut(id).and_then(|f| f.widget_data.as_mut()) {
        if eb.cursor_position > 0 {
            eb.cursor_position -= 1;
            eb.text.remove(eb.cursor_position);
        }
    }
}

pub fn editbox_delete(reg: &mut FrameRegistry, id: u64) {
    if let Some(WidgetData::EditBox(eb)) = reg.get_mut(id).and_then(|f| f.widget_data.as_mut()) {
        if eb.cursor_position < eb.text.len() {
            eb.text.remove(eb.cursor_position);
        }
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

pub fn set_editbox_password(reg: &mut FrameRegistry, id: u64) {
    if let Some(WidgetData::EditBox(eb)) = reg.get_mut(id).and_then(|f| f.widget_data.as_mut()) {
        eb.password = true;
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

pub fn select_all_editbox(reg: &mut FrameRegistry, id: u64) {
    if let Some(WidgetData::EditBox(eb)) = reg.get_mut(id).and_then(|f| f.widget_data.as_mut()) {
        eb.cursor_position = eb.text.len();
    }
}

// --- Frame helpers ---

pub fn hit_frame(ui: &UiState, frame_id: u64, mx: f32, my: f32) -> bool {
    ui.registry.get(frame_id).is_some_and(|f| {
        f.layout_rect
            .as_ref()
            .is_some_and(|r| mx >= r.x && mx <= r.x + r.width && my >= r.y && my <= r.y + r.height)
    })
}

pub fn remove_frame_tree(reg: &mut FrameRegistry, id: u64) {
    let children = reg.get(id).map(|f| f.children.clone()).unwrap_or_default();
    for child in children {
        remove_frame_tree(reg, child);
    }
    reg.remove_frame(id);
}

pub fn hide_frame(reg: &mut FrameRegistry, id: u64) {
    if let Some(frame) = reg.get_mut(id) {
        frame.visible = false;
        frame.shown = false;
    }
}

pub fn create_frame(
    reg: &mut FrameRegistry,
    name: &str,
    parent: Option<u64>,
    wt: WidgetType,
    w: f32,
    h: f32,
) -> u64 {
    let id = reg.next_id();
    let mut frame = Frame::new(id, Some(name.to_string()), wt);
    frame.parent_id = parent;
    frame.width = w;
    frame.height = h;
    frame.mouse_enabled = true;
    reg.insert_frame(frame);
    id
}

pub fn create_editbox(
    reg: &mut FrameRegistry,
    name: &str,
    parent: Option<u64>,
    w: f32,
    h: f32,
) -> u64 {
    let id = create_frame(reg, name, parent, WidgetType::EditBox, w, h);
    if let Some(frame) = reg.get_mut(id) {
        frame.widget_data = Some(WidgetData::EditBox(EditBoxData::default()));
    }
    id
}

pub fn checkbox_text(checked: bool) -> &'static str {
    if checked {
        "[x] Save account name"
    } else {
        "[ ] Save account name"
    }
}

pub fn set_button_text(reg: &mut FrameRegistry, id: u64, text: &str) {
    if let Some(WidgetData::Button(bd)) = reg.get_mut(id).and_then(|f| f.widget_data.as_mut()) {
        bd.text = text.to_string();
    }
}

pub fn create_button(
    reg: &mut FrameRegistry,
    name: &str,
    parent: Option<u64>,
    w: f32,
    h: f32,
    text: &str,
) -> u64 {
    let id = create_frame(reg, name, parent, WidgetType::Button, w, h);
    if let Some(frame) = reg.get_mut(id) {
        frame.widget_data = Some(WidgetData::Button(ButtonData {
            text: text.to_string(),
            ..Default::default()
        }));
    }
    id
}

pub fn create_check_button(
    reg: &mut FrameRegistry,
    name: &str,
    parent: Option<u64>,
    w: f32,
    h: f32,
    checked: bool,
) -> u64 {
    let id = create_frame(reg, name, parent, WidgetType::CheckButton, w, h);
    if let Some(frame) = reg.get_mut(id) {
        let mut data = CheckButtonData::default();
        data.checked = checked;
        data.button.text = checkbox_text(checked).to_string();
        frame.widget_data = Some(WidgetData::Button(data.button));
    }
    id
}

pub fn set_layout(reg: &mut FrameRegistry, id: u64, x: f32, y: f32, w: f32, h: f32) {
    let (relative_to, x_offset, y_offset) = reg
        .get(id)
        .and_then(|frame| frame.parent_id)
        .and_then(|parent_id| {
            reg.get(parent_id)
                .and_then(|parent| parent.layout_rect.as_ref())
                .map(|rect| (Some(parent_id), x - rect.x, y - rect.y))
        })
        .unwrap_or((None, x, y));
    if let Some(frame) = reg.get_mut(id) {
        frame.width = w;
        frame.height = h;
        frame.layout_rect = None;
    }
    reg.clear_all_points(id);
    reg.set_point(
        id,
        Anchor {
            point: AnchorPoint::TopLeft,
            relative_to,
            relative_point: AnchorPoint::TopLeft,
            x_offset,
            y_offset: -y_offset,
        },
    )
    .expect("screen layout helper must create a valid anchor");
    if let Some(layout_rect) = resolve_frame_layout(reg, id)
        && let Some(frame) = reg.get_mut(id)
    {
        frame.layout_rect = Some(layout_rect);
    }
}

pub fn set_bg(reg: &mut FrameRegistry, id: u64, color: [f32; 4]) {
    if let Some(frame) = reg.get_mut(id) {
        frame.background_color = Some(color);
    }
}

pub fn set_strata(reg: &mut FrameRegistry, id: u64, strata: FrameStrata) {
    if let Some(frame) = reg.get_mut(id) {
        frame.strata = strata;
    }
}

pub fn set_font_string(reg: &mut FrameRegistry, id: u64, text: &str, size: f32, color: [f32; 4]) {
    if let Some(frame) = reg.get_mut(id) {
        frame.widget_data = Some(WidgetData::FontString(FontStringData {
            text: text.to_string(),
            font_size: size,
            color,
            justify_h: JustifyH::Center,
            ..Default::default()
        }));
    }
}

pub fn set_font_string_left(
    reg: &mut FrameRegistry,
    id: u64,
    text: &str,
    size: f32,
    color: [f32; 4],
) {
    if let Some(frame) = reg.get_mut(id) {
        frame.widget_data = Some(WidgetData::FontString(FontStringData {
            text: text.to_string(),
            font_size: size,
            color,
            justify_h: JustifyH::Left,
            ..Default::default()
        }));
    }
}

pub fn set_editbox_backdrop(reg: &mut FrameRegistry, id: u64) {
    if let Some(frame) = reg.get_mut(id) {
        frame.backdrop = Some(Backdrop {
            bg_color: Some([0.06, 0.06, 0.10, 0.9]),
            border_color: Some([0.3, 0.25, 0.15, 1.0]),
            edge_size: 1.0,
            insets: [0.0; 4],
        });
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

pub fn set_button_textures(
    reg: &mut FrameRegistry,
    id: u64,
    normal: &str,
    pushed: &str,
    highlight: &str,
) {
    if let Some(WidgetData::Button(bd)) = reg.get_mut(id).and_then(|f| f.widget_data.as_mut()) {
        bd.normal_texture = Some(TextureSource::File(normal.to_string()));
        bd.pushed_texture = Some(TextureSource::File(pushed.to_string()));
        bd.highlight_texture = Some(TextureSource::File(highlight.to_string()));
    }
}
