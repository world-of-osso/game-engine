use bevy::input::ButtonState;
use bevy::input::keyboard::{Key, KeyboardInput};
use bevy::prelude::*;
use lightyear::prelude::*;

use game_engine::ui::frame::{Frame, WidgetData, WidgetType};
use game_engine::ui::layout::LayoutRect;
use game_engine::ui::plugin::UiState;
use game_engine::ui::registry::FrameRegistry;
use game_engine::ui::strata::FrameStrata;
use game_engine::ui::widgets::button::ButtonData;
use game_engine::ui::widgets::edit_box::EditBoxData;
use game_engine::ui::widgets::font_string::{FontStringData, JustifyH};
use shared::protocol::{AuthChannel, CreateCharacter, DeleteCharacter, SelectCharacter};

use crate::game_state::GameState;
use crate::networking::CharacterList;

/// Resource holding frame IDs for the character select UI.
#[derive(Resource)]
struct CharSelectUi {
    root: u64,
    char_buttons: Vec<u64>,
    enter_button: u64,
    create_button: u64,
    delete_button: u64,
    back_button: u64,
    create_panel: u64,
    create_name_input: u64,
    create_confirm_button: u64,
    status_text: u64,
}

/// Which character in the list is highlighted.
#[derive(Resource, Default)]
struct SelectedCharIndex(Option<usize>);

/// Whether the create panel is shown.
#[derive(Resource, Default)]
struct CreatePanelVisible(bool);

/// Focus state for editboxes in char select.
#[derive(Resource, Default)]
struct CharSelectFocus(Option<u64>);

pub struct CharSelectPlugin;

impl Plugin for CharSelectPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<SelectedCharIndex>();
        app.init_resource::<CreatePanelVisible>();
        app.init_resource::<CharSelectFocus>();
        app.add_systems(OnEnter(GameState::CharSelect), build_char_select_ui);
        app.add_systems(OnExit(GameState::CharSelect), teardown_char_select_ui);
        app.add_systems(
            Update,
            (
                char_select_mouse_input,
                char_select_keyboard_input,
                char_select_update_visuals,
            )
                .into_configs()
                .run_if(in_state(GameState::CharSelect)),
        );
    }
}

// --- UI Building ---

fn build_char_select_ui(
    mut ui: ResMut<UiState>,
    mut commands: Commands,
    char_list: Res<CharacterList>,
) {
    let reg = &mut ui.registry;
    let sw = reg.screen_width;
    let sh = reg.screen_height;

    let root = build_cs_background(reg, sw, sh);
    build_cs_title(reg, root, sw, sh);
    let char_buttons = build_character_list(reg, root, sw, sh, &char_list);
    let (enter_button, create_button, delete_button, back_button) =
        build_cs_action_buttons(reg, root, sw, sh);
    let (create_panel, create_name_input, create_confirm_button) =
        build_create_panel(reg, root, sw, sh);
    let status_text = build_cs_status(reg, root, sw, sh);

    commands.insert_resource(CharSelectUi {
        root,
        char_buttons,
        enter_button,
        create_button,
        delete_button,
        back_button,
        create_panel,
        create_name_input,
        create_confirm_button,
        status_text,
    });
}

fn build_cs_background(reg: &mut FrameRegistry, sw: f32, sh: f32) -> u64 {
    let root = create_frame(reg, "CharSelectRoot", None, WidgetType::Frame, sw, sh);
    set_layout(reg, root, 0.0, 0.0, sw, sh);
    set_bg(reg, root, [0.05, 0.05, 0.12, 1.0]);
    set_strata(reg, root, FrameStrata::Fullscreen);
    root
}

fn build_cs_title(reg: &mut FrameRegistry, root: u64, sw: f32, sh: f32) {
    let title = create_frame(
        reg,
        "CSTitle",
        Some(root),
        WidgetType::FontString,
        400.0,
        40.0,
    );
    set_layout(reg, title, (sw - 400.0) / 2.0, sh * 0.08, 400.0, 40.0);
    set_font_string(
        reg,
        title,
        "Character Selection",
        26.0,
        [1.0, 0.82, 0.0, 1.0],
    );
}

fn build_character_list(
    reg: &mut FrameRegistry,
    root: u64,
    sw: f32,
    sh: f32,
    char_list: &CharacterList,
) -> Vec<u64> {
    let panel_w = 380.0;
    let panel_x = (sw - panel_w) / 2.0;
    let mut y = sh * 0.18;
    let mut buttons = Vec::new();

    for ch in &char_list.0 {
        let text = format!("{} - Lv{} R{} C{}", ch.name, ch.level, ch.race, ch.class);
        let btn = create_button(
            reg,
            &format!("Char_{}", ch.character_id),
            Some(root),
            panel_w,
            32.0,
            &text,
        );
        set_layout(reg, btn, panel_x, y, panel_w, 32.0);
        set_bg(reg, btn, [0.12, 0.12, 0.22, 1.0]);
        buttons.push(btn);
        y += 38.0;
    }
    buttons
}

fn build_cs_action_buttons(
    reg: &mut FrameRegistry,
    root: u64,
    sw: f32,
    sh: f32,
) -> (u64, u64, u64, u64) {
    let btn_w = 160.0;
    let gap = 12.0;
    let total_w = btn_w * 4.0 + gap * 3.0;
    let start_x = (sw - total_w) / 2.0;
    let y = sh * 0.78;

    let enter = create_action_button(reg, root, "EnterWorld", "Enter World", start_x, y, btn_w);
    let create = create_action_button(
        reg,
        root,
        "CreateChar",
        "Create",
        start_x + btn_w + gap,
        y,
        btn_w,
    );
    let delete = create_action_button(
        reg,
        root,
        "DeleteChar",
        "Delete",
        start_x + (btn_w + gap) * 2.0,
        y,
        btn_w,
    );
    let back = create_action_button(
        reg,
        root,
        "BackToLogin",
        "Back",
        start_x + (btn_w + gap) * 3.0,
        y,
        btn_w,
    );
    (enter, create, delete, back)
}

fn create_action_button(
    reg: &mut FrameRegistry,
    root: u64,
    name: &str,
    text: &str,
    x: f32,
    y: f32,
    w: f32,
) -> u64 {
    let btn = create_button(reg, name, Some(root), w, 36.0, text);
    set_layout(reg, btn, x, y, w, 36.0);
    set_bg(reg, btn, [0.15, 0.35, 0.6, 1.0]);
    btn
}

fn build_create_panel(reg: &mut FrameRegistry, root: u64, sw: f32, sh: f32) -> (u64, u64, u64) {
    let panel_w = 300.0;
    let panel_x = (sw - panel_w) / 2.0;
    let panel_y = sh * 0.55;

    let panel = create_frame(
        reg,
        "CreatePanel",
        Some(root),
        WidgetType::Frame,
        panel_w,
        120.0,
    );
    set_layout(reg, panel, panel_x, panel_y, panel_w, 120.0);
    set_bg(reg, panel, [0.08, 0.08, 0.18, 0.95]);

    let label = create_frame(
        reg,
        "CreateNameLabel",
        Some(panel),
        WidgetType::FontString,
        panel_w,
        20.0,
    );
    set_layout(reg, label, panel_x, panel_y + 8.0, panel_w, 20.0);
    set_font_string_left(reg, label, "Character Name", 13.0, [0.8, 0.8, 0.9, 1.0]);

    let name_input = create_editbox(reg, "CreateNameInput", Some(panel), panel_w - 20.0, 30.0);
    set_layout(
        reg,
        name_input,
        panel_x + 10.0,
        panel_y + 32.0,
        panel_w - 20.0,
        30.0,
    );
    set_bg(reg, name_input, [0.12, 0.12, 0.2, 1.0]);

    let confirm = create_button(reg, "CreateConfirm", Some(panel), 120.0, 30.0, "Create");
    set_layout(
        reg,
        confirm,
        panel_x + (panel_w - 120.0) / 2.0,
        panel_y + 76.0,
        120.0,
        30.0,
    );
    set_bg(reg, confirm, [0.2, 0.5, 0.3, 1.0]);

    (panel, name_input, confirm)
}

fn build_cs_status(reg: &mut FrameRegistry, root: u64, sw: f32, sh: f32) -> u64 {
    let status = create_frame(
        reg,
        "CSStatus",
        Some(root),
        WidgetType::FontString,
        400.0,
        24.0,
    );
    set_layout(reg, status, (sw - 400.0) / 2.0, sh * 0.88, 400.0, 24.0);
    set_font_string(reg, status, "", 13.0, [0.9, 0.5, 0.5, 1.0]);
    status
}

fn teardown_char_select_ui(
    mut ui: ResMut<UiState>,
    cs_ui: Option<Res<CharSelectUi>>,
    mut commands: Commands,
) {
    if let Some(cs) = cs_ui {
        remove_frame_tree(&mut ui.registry, cs.root);
        commands.remove_resource::<CharSelectUi>();
    }
    ui.focused_frame = None;
}

// --- Input Handling ---

fn char_select_mouse_input(
    buttons: Res<ButtonInput<MouseButton>>,
    windows: Query<&Window>,
    ui: Res<UiState>,
    cs_ui: Option<Res<CharSelectUi>>,
    mut selected: ResMut<SelectedCharIndex>,
    mut focus: ResMut<CharSelectFocus>,
    mut create_visible: ResMut<CreatePanelVisible>,
    mut senders: Query<&mut MessageSender<SelectCharacter>>,
    mut del_senders: Query<&mut MessageSender<DeleteCharacter>>,
    char_list: Res<CharacterList>,
    mut next_state: ResMut<NextState<GameState>>,
) {
    let Some(cs) = cs_ui.as_ref() else { return };
    if !buttons.just_pressed(MouseButton::Left) {
        return;
    }
    let Some(cursor) = cursor_pos(&windows) else {
        return;
    };
    handle_cs_click(
        cs,
        &ui,
        cursor,
        &mut selected,
        &mut focus,
        &mut create_visible,
        &mut senders,
        &mut del_senders,
        &char_list,
        &mut next_state,
    );
}

fn cursor_pos(windows: &Query<&Window>) -> Option<Vec2> {
    windows.iter().next().and_then(|w| w.cursor_position())
}

fn handle_cs_click(
    cs: &CharSelectUi,
    ui: &UiState,
    cursor: Vec2,
    selected: &mut SelectedCharIndex,
    focus: &mut CharSelectFocus,
    create_visible: &mut CreatePanelVisible,
    senders: &mut Query<&mut MessageSender<SelectCharacter>>,
    del_senders: &mut Query<&mut MessageSender<DeleteCharacter>>,
    char_list: &CharacterList,
    next_state: &mut NextState<GameState>,
) {
    let (mx, my) = (cursor.x, cursor.y);
    if let Some(idx) = cs
        .char_buttons
        .iter()
        .position(|&id| hit_frame(ui, id, mx, my))
    {
        selected.0 = Some(idx);
        focus.0 = None;
    } else if hit_frame(ui, cs.enter_button, mx, my) {
        try_enter_world(selected, char_list, senders);
    } else if hit_frame(ui, cs.create_button, mx, my) {
        create_visible.0 = !create_visible.0;
    } else if hit_frame(ui, cs.delete_button, mx, my) {
        try_delete_character(selected, char_list, del_senders);
    } else if hit_frame(ui, cs.back_button, mx, my) {
        next_state.set(GameState::Login);
    } else if hit_frame(ui, cs.create_name_input, mx, my) {
        focus.0 = Some(cs.create_name_input);
    } else if hit_frame(ui, cs.create_confirm_button, mx, my) {
        // Handled by keyboard_input Enter or dedicated click
        focus.0 = None;
    } else {
        focus.0 = None;
    }
}

fn try_enter_world(
    selected: &SelectedCharIndex,
    char_list: &CharacterList,
    senders: &mut Query<&mut MessageSender<SelectCharacter>>,
) {
    let Some(idx) = selected.0 else { return };
    let Some(ch) = char_list.0.get(idx) else {
        return;
    };
    let msg = SelectCharacter {
        character_id: ch.character_id,
    };
    for mut sender in senders.iter_mut() {
        sender.send::<AuthChannel>(msg.clone());
    }
    info!("Requested enter world for '{}'", ch.name);
}

fn try_delete_character(
    selected: &SelectedCharIndex,
    char_list: &CharacterList,
    senders: &mut Query<&mut MessageSender<DeleteCharacter>>,
) {
    let Some(idx) = selected.0 else { return };
    let Some(ch) = char_list.0.get(idx) else {
        return;
    };
    let msg = DeleteCharacter {
        character_id: ch.character_id,
    };
    for mut sender in senders.iter_mut() {
        sender.send::<AuthChannel>(msg.clone());
    }
    info!("Requested delete character '{}'", ch.name);
}

fn char_select_keyboard_input(
    mut key_events: MessageReader<KeyboardInput>,
    mut ui: ResMut<UiState>,
    focus: Res<CharSelectFocus>,
    cs_ui: Option<Res<CharSelectUi>>,
    mut create_senders: Query<&mut MessageSender<CreateCharacter>>,
) {
    let Some(cs) = cs_ui.as_ref() else { return };
    let Some(focused_id) = focus.0 else { return };

    for event in key_events.read() {
        if event.state != ButtonState::Pressed {
            continue;
        }
        if let Key::Character(ch) = &event.logical_key {
            insert_char_into_editbox(&mut ui.registry, focused_id, ch.as_str());
        } else {
            handle_cs_key(event.key_code, focused_id, &mut ui, cs, &mut create_senders);
        }
    }
}

fn handle_cs_key(
    key: KeyCode,
    focused_id: u64,
    ui: &mut UiState,
    cs: &CharSelectUi,
    create_senders: &mut Query<&mut MessageSender<CreateCharacter>>,
) {
    match key {
        KeyCode::Backspace => editbox_backspace(&mut ui.registry, focused_id),
        KeyCode::Delete => editbox_delete(&mut ui.registry, focused_id),
        KeyCode::ArrowLeft => editbox_move_cursor(&mut ui.registry, focused_id, -1),
        KeyCode::ArrowRight => editbox_move_cursor(&mut ui.registry, focused_id, 1),
        KeyCode::Home => editbox_cursor_home(&mut ui.registry, focused_id),
        KeyCode::End => editbox_cursor_end(&mut ui.registry, focused_id),
        KeyCode::Enter => try_create_character(&ui.registry, cs, create_senders),
        _ => {}
    }
}

fn try_create_character(
    reg: &FrameRegistry,
    cs: &CharSelectUi,
    senders: &mut Query<&mut MessageSender<CreateCharacter>>,
) {
    let name = get_editbox_text(reg, cs.create_name_input);
    if name.is_empty() {
        return;
    }
    let msg = CreateCharacter {
        name: name.clone(),
        race: 1,
        class: 1,
    };
    for mut sender in senders.iter_mut() {
        sender.send::<AuthChannel>(msg.clone());
    }
    info!("Requested create character '{name}'");
}

// --- Visual Updates ---

fn char_select_update_visuals(
    mut ui: ResMut<UiState>,
    cs_ui: Option<Res<CharSelectUi>>,
    selected: Res<SelectedCharIndex>,
    create_visible: Res<CreatePanelVisible>,
    char_list: Res<CharacterList>,
) {
    let Some(cs) = cs_ui.as_ref() else { return };
    update_char_button_highlights(&mut ui.registry, cs, &selected);
    update_create_panel_visibility(&mut ui.registry, cs, create_visible.0);
    rebuild_char_buttons_if_changed(&mut ui.registry, cs, &char_list);
}

fn update_char_button_highlights(
    reg: &mut FrameRegistry,
    cs: &CharSelectUi,
    selected: &SelectedCharIndex,
) {
    for (i, &btn_id) in cs.char_buttons.iter().enumerate() {
        let is_selected = selected.0 == Some(i);
        let color = if is_selected {
            [0.2, 0.2, 0.4, 1.0]
        } else {
            [0.12, 0.12, 0.22, 1.0]
        };
        set_bg(reg, btn_id, color);
    }
}

fn update_create_panel_visibility(reg: &mut FrameRegistry, cs: &CharSelectUi, visible: bool) {
    if let Some(frame) = reg.get_mut(cs.create_panel) {
        frame.visible = visible;
    }
}

fn rebuild_char_buttons_if_changed(
    reg: &mut FrameRegistry,
    cs: &CharSelectUi,
    char_list: &CharacterList,
) {
    // Update button text to match current character list.
    for (i, &btn_id) in cs.char_buttons.iter().enumerate() {
        if let Some(ch) = char_list.0.get(i) {
            let text = format!("{} - Lv{} R{} C{}", ch.name, ch.level, ch.race, ch.class);
            if let Some(WidgetData::Button(bd)) =
                reg.get_mut(btn_id).and_then(|f| f.widget_data.as_mut())
            {
                bd.text = text;
            }
        }
    }
}

// --- EditBox manipulation (duplicated from login_screen, consider extracting) ---

fn editbox_backspace(reg: &mut FrameRegistry, id: u64) {
    if let Some(WidgetData::EditBox(eb)) = reg.get_mut(id).and_then(|f| f.widget_data.as_mut()) {
        if eb.cursor_position > 0 {
            eb.cursor_position -= 1;
            eb.text.remove(eb.cursor_position);
        }
    }
}

fn editbox_delete(reg: &mut FrameRegistry, id: u64) {
    if let Some(WidgetData::EditBox(eb)) = reg.get_mut(id).and_then(|f| f.widget_data.as_mut()) {
        if eb.cursor_position < eb.text.len() {
            eb.text.remove(eb.cursor_position);
        }
    }
}

fn editbox_move_cursor(reg: &mut FrameRegistry, id: u64, delta: i32) {
    if let Some(WidgetData::EditBox(eb)) = reg.get_mut(id).and_then(|f| f.widget_data.as_mut()) {
        if delta < 0 {
            eb.cursor_position = eb.cursor_position.saturating_sub((-delta) as usize);
        } else {
            eb.cursor_position = (eb.cursor_position + delta as usize).min(eb.text.len());
        }
    }
}

fn editbox_cursor_home(reg: &mut FrameRegistry, id: u64) {
    if let Some(WidgetData::EditBox(eb)) = reg.get_mut(id).and_then(|f| f.widget_data.as_mut()) {
        eb.cursor_position = 0;
    }
}

fn editbox_cursor_end(reg: &mut FrameRegistry, id: u64) {
    if let Some(WidgetData::EditBox(eb)) = reg.get_mut(id).and_then(|f| f.widget_data.as_mut()) {
        eb.cursor_position = eb.text.len();
    }
}

fn insert_char_into_editbox(reg: &mut FrameRegistry, id: u64, ch: &str) {
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

fn get_editbox_text(reg: &FrameRegistry, id: u64) -> String {
    reg.get(id)
        .and_then(|f| match &f.widget_data {
            Some(WidgetData::EditBox(eb)) => Some(eb.text.clone()),
            _ => None,
        })
        .unwrap_or_default()
}

// --- Frame creation helpers (duplicated from login_screen) ---

fn create_frame(
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

fn create_editbox(reg: &mut FrameRegistry, name: &str, parent: Option<u64>, w: f32, h: f32) -> u64 {
    let id = create_frame(reg, name, parent, WidgetType::EditBox, w, h);
    if let Some(frame) = reg.get_mut(id) {
        frame.widget_data = Some(WidgetData::EditBox(EditBoxData::default()));
    }
    id
}

fn create_button(
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

fn set_layout(reg: &mut FrameRegistry, id: u64, x: f32, y: f32, w: f32, h: f32) {
    if let Some(frame) = reg.get_mut(id) {
        frame.layout_rect = Some(LayoutRect {
            x,
            y,
            width: w,
            height: h,
        });
        frame.width = w;
        frame.height = h;
    }
}

fn set_bg(reg: &mut FrameRegistry, id: u64, color: [f32; 4]) {
    if let Some(frame) = reg.get_mut(id) {
        frame.background_color = Some(color);
    }
}

fn set_strata(reg: &mut FrameRegistry, id: u64, strata: FrameStrata) {
    if let Some(frame) = reg.get_mut(id) {
        frame.strata = strata;
    }
}

fn set_font_string(reg: &mut FrameRegistry, id: u64, text: &str, size: f32, color: [f32; 4]) {
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

fn set_font_string_left(reg: &mut FrameRegistry, id: u64, text: &str, size: f32, color: [f32; 4]) {
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

fn hit_frame(ui: &UiState, frame_id: u64, mx: f32, my: f32) -> bool {
    ui.registry.get(frame_id).is_some_and(|f| {
        f.layout_rect
            .as_ref()
            .is_some_and(|r| mx >= r.x && mx <= r.x + r.width && my >= r.y && my <= r.y + r.height)
    })
}

fn remove_frame_tree(reg: &mut FrameRegistry, id: u64) {
    let children = reg.get(id).map(|f| f.children.clone()).unwrap_or_default();
    for child in children {
        remove_frame_tree(reg, child);
    }
    reg.remove_frame(id);
}
