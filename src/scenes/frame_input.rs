//! Clicks and edit boxes of the trade and mail frames: the frame under a click,
//! which registry frames are the frame's edit boxes (Retail `letters` limits,
//! digit-only money boxes), their text and focus, and typing into the focused one.

use bevy::ecs::system::SystemParam;
use bevy::input::ButtonState;
use bevy::input::keyboard::KeyboardInput;
use bevy::prelude::*;
use bevy::window::PrimaryWindow;
use game_engine::ui::input::{find_frame_at, ui_cursor_position};
use game_engine::ui::plugin::UiState;
use game_engine::ui::screens::bank_art::MoneyBoxNames;
use ui_toolkit::frame::WidgetData;
use ui_toolkit::registry::FrameRegistry;

use crate::ui_input::mutate_editbox_from_key;
use crate::ui_input_mode::focused_editbox;

/// One edit box of a frame.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct EditBoxSpec {
    pub name: &'static str,
    pub letters: usize,
    pub digits_only: bool,
}

/// `MoneyInputFrameTemplate` boxes (MoneyInputFrame.xml:79-111: gold 7, silver /
/// copper 2 digits).
pub fn money_specs(boxes: MoneyBoxNames) -> [EditBoxSpec; 3] {
    [(boxes.gold, 7), (boxes.silver, 2), (boxes.copper, 2)].map(|(name, letters)| EditBoxSpec {
        name,
        letters,
        digits_only: true,
    })
}

pub fn spec_of(registry: &FrameRegistry, specs: &[EditBoxSpec], id: u64) -> Option<EditBoxSpec> {
    specs
        .iter()
        .copied()
        .find(|spec| registry.get_by_name(spec.name) == Some(id))
}

pub fn text(registry: &FrameRegistry, name: &str) -> String {
    registry
        .get_by_name(name)
        .and_then(|id| match registry.get(id)?.widget_data.as_ref()? {
            WidgetData::EditBox(data) => Some(data.text.clone()),
            _ => None,
        })
        .unwrap_or_default()
}

pub fn set_text(registry: &mut FrameRegistry, name: &str, value: &str) {
    let Some(id) = registry.get_by_name(name) else {
        return;
    };
    if let Some(WidgetData::EditBox(data)) = registry
        .get_mut(id)
        .and_then(|frame| frame.widget_data.as_mut())
    {
        data.replace_range(0, data.text.len(), value);
        data.cursor_end();
    }
}

/// Copper typed into a money entry; empty boxes count as zero.
pub fn money(registry: &FrameRegistry, boxes: MoneyBoxNames) -> u64 {
    let part = |name| text(registry, name).trim().parse::<u64>().unwrap_or(0);
    part(boxes.gold) * 10_000 + part(boxes.silver) * 100 + part(boxes.copper)
}

/// `MoneyInputFrame_SetCopper`: gold blank when zero, silver and copper blank when
/// the whole amount is zero.
pub fn set_money(registry: &mut FrameRegistry, boxes: MoneyBoxNames, copper: u64) {
    let blank = |value: u64, shown: bool| {
        if shown {
            value.to_string()
        } else {
            String::new()
        }
    };
    let gold = copper / 10_000;
    set_text(registry, boxes.gold, &blank(gold, gold > 0));
    set_text(
        registry,
        boxes.silver,
        &blank(copper / 100 % 100, copper > 0),
    );
    set_text(registry, boxes.copper, &blank(copper % 100, copper > 0));
}

pub fn set_focus(ui: &mut UiState, id: Option<u64>) {
    ui.focused_frame = id;
    ui.registry.focused_frame = id;
}

/// The focused edit box when it is one of `specs`.
pub fn focused_spec(ui: &UiState, specs: &[EditBoxSpec]) -> Option<EditBoxSpec> {
    focused_editbox(ui).and_then(|id| spec_of(&ui.registry, specs, id))
}

/// Keys edit the focused box of `specs`; Enter, Tab and Escape leave it and return
/// its name.
pub fn type_into(
    key_events: &mut MessageReader<KeyboardInput>,
    ui: &mut UiState,
    specs: &[EditBoxSpec],
) -> Option<&'static str> {
    let Some(focused) = focused_editbox(ui) else {
        key_events.read().for_each(drop);
        return None;
    };
    let spec = spec_of(&ui.registry, specs, focused)?;
    for event in key_events.read() {
        if event.state != ButtonState::Pressed {
            continue;
        }
        match event.key_code {
            KeyCode::Enter | KeyCode::NumpadEnter | KeyCode::Tab | KeyCode::Escape => {
                set_focus(ui, None);
                return Some(spec.name);
            }
            KeyCode::Backspace
            | KeyCode::Delete
            | KeyCode::ArrowLeft
            | KeyCode::ArrowRight
            | KeyCode::Home
            | KeyCode::End => {
                mutate_editbox_from_key(&mut ui.registry, focused, event);
            }
            _ => {
                let typed = event.text.as_deref().unwrap_or("");
                let length = text(&ui.registry, spec.name).chars().count();
                let fits = length + typed.chars().count() <= spec.letters;
                if fits && (!spec.digits_only || typed.chars().all(|ch| ch.is_ascii_digit())) {
                    mutate_editbox_from_key(&mut ui.registry, focused, event);
                }
            }
        }
    }
    None
}

#[derive(SystemParam)]
pub struct Pointer<'w, 's> {
    windows: Query<'w, 's, &'static Window, With<PrimaryWindow>>,
    mouse: Option<Res<'w, ButtonInput<MouseButton>>>,
    reconnect: Option<Res<'w, crate::networking::ReconnectState>>,
    modal_open: Option<Res<'w, crate::scenes::game_menu::UiModalOpen>>,
}

impl Pointer<'_, '_> {
    /// The button pressed this frame and the frame under the cursor.
    pub fn click(self, ui: &UiState) -> Option<(MouseButton, u64)> {
        let mouse = self.mouse.as_ref()?;
        let button = [MouseButton::Left, MouseButton::Right]
            .into_iter()
            .find(|button| mouse.just_pressed(*button))?;
        if self.modal_open.is_some() || !crate::networking::gameplay_input_allowed(self.reconnect) {
            return None;
        }
        let window = self.windows.single().ok()?;
        let cursor = ui_cursor_position(&ui.registry, window)?;
        Some((button, find_frame_at(&ui.registry, cursor.x, cursor.y)?))
    }
}
