//! Main-thread consumption of the original synchronous JS action compiler.

use std::{
    collections::{HashMap, VecDeque},
    path::Path,
    sync::LazyLock,
};

use game_engine_network::{
    automation_data::{KeyChord, KeyCode, UiAutomationAction},
    game_state_enum::GameState,
    ipc_wire::Response,
    js_automation::load_js_automation_script,
};
use godot::{
    classes::{
        Control, INode, InputEvent, InputEventKey, InputEventMouseButton, InputEventMouseMotion,
        LineEdit, Node, TextEdit,
    },
    global::{Key, MouseButton},
    prelude::*,
};

use crate::{GameClient, ipc};

/// Owns the compiled queue; all methods run on Godot's main thread.
pub(crate) struct NativeJsAutomation {
    actions: VecDeque<UiAutomationAction>,
    input: VecDeque<Gd<InputEvent>>,
    elapsed: f64,
    wait_started: Option<f64>,
    error: Option<String>,
    last_wait_error: Option<String>,
}

impl NativeJsAutomation {
    /// QuickJS completes synchronously before any native input is dispatched.
    pub(crate) fn load(path: &Path) -> Result<Self, String> {
        let actions = load_js_automation_script(path)?;
        Ok(Self {
            actions: actions.into(),
            input: VecDeque::new(),
            elapsed: 0.0,
            wait_started: None,
            error: None,
            last_wait_error: None,
        })
    }

    /// Returns true when complete. Never call with an active GameClient bind.
    pub(crate) fn poll(
        &mut self,
        client: &Gd<Node>,
        delta: f64,
        state: GameState,
    ) -> Result<bool, String> {
        if let Some(error) = &self.error {
            return Err(error.clone());
        }
        let result = self.advance(client, delta, state);
        if let Err(error) = &result {
            self.actions.clear();
            self.input.clear();
            self.error = Some(error.clone());
        }
        result
    }

    fn advance(&mut self, client: &Gd<Node>, delta: f64, state: GameState) -> Result<bool, String> {
        if !delta.is_finite() || delta < 0.0 {
            return Err("native JS automation: invalid frame delta".into());
        }
        if !client.is_inside_tree() {
            return Err("native JS automation: client is not mounted".into());
        }
        self.elapsed += delta;
        if let Some(event) = self.input.pop_front() {
            client
                .get_tree()
                .get_root()
                .push_input_ex(&event)
                .in_local_coords(true)
                .done();
            return Ok(false);
        }
        let Some(action) = self.actions.front().cloned() else {
            return Ok(true);
        };
        if !self.apply(client, state, action)? {
            return Ok(false);
        }
        self.actions.pop_front();
        self.wait_started = None;
        Ok(self.actions.is_empty() && self.input.is_empty())
    }

    fn apply(
        &mut self,
        client: &Gd<Node>,
        state: GameState,
        action: UiAutomationAction,
    ) -> Result<bool, String> {
        self.input = match action {
            UiAutomationAction::WaitForState(target, timeout) => {
                return self.wait_until(state == target, timeout, &format!("state {target:?}"));
            }
            UiAutomationAction::WaitForFrame(name, timeout) => {
                return self.wait_for_frame(client, &name, timeout);
            }
            UiAutomationAction::Wait(seconds) => return self.wait_delay(seconds),
            UiAutomationAction::ClickFrame(name) => {
                click_events(client, &name, MouseButton::LEFT, false)
            }
            UiAutomationAction::ClickFrameAt(name, across) => {
                click_at_events(client, &name, across)
            }
            UiAutomationAction::WheelFrame(name, notches) => wheel_events(client, &name, notches),
            UiAutomationAction::RightClickFrame(name) => right_click_events(client, state, &name),
            UiAutomationAction::ShiftClickFrame(name) => shift_click_events(client, state, &name),
            UiAutomationAction::TypeText(text) => focused_text_events(client, &text),
            UiAutomationAction::PressKey(chord) => state_chord_events(state, &chord),
            UiAutomationAction::DumpTree => return print_tree(ipc::dump_tree(client, None)),
            UiAutomationAction::DumpUiTree => return print_tree(ipc::dump_ui_tree(client, None)),
        }?;
        Ok(true)
    }

    fn wait_for_frame(
        &mut self,
        client: &Gd<Node>,
        name: &str,
        timeout: f32,
    ) -> Result<bool, String> {
        let ready = visible_control(client, name).is_some();
        self.wait_until(ready, timeout, &format!("frame '{name}'"))
    }

    fn wait_delay(&mut self, seconds: f32) -> Result<bool, String> {
        validate_seconds(seconds)?;
        let started = *self.wait_started.get_or_insert(self.elapsed);
        Ok(self.elapsed - started >= f64::from(seconds))
    }

    fn wait_until(&mut self, ready: bool, timeout: f32, target: &str) -> Result<bool, String> {
        validate_seconds(timeout)?;
        if ready {
            return Ok(true);
        }
        let started = *self.wait_started.get_or_insert(self.elapsed);
        if self.elapsed - started > f64::from(timeout) {
            let error = self.last_wait_error.insert(format!(
                "native JS automation: timed out waiting for {target} after {timeout:.2}s"
            ));
            godot_error!("{error}");
            return Ok(true);
        }
        Ok(false)
    }
}

fn validate_seconds(seconds: f32) -> Result<(), String> {
    if !seconds.is_finite() || seconds < 0.0 {
        return Err("native JS automation: wait requires finite nonnegative seconds".into());
    }
    Ok(())
}

fn require_inworld(state: GameState, action: &str) -> Result<(), String> {
    if state != GameState::InWorld {
        return Err(format!(
            "native JS automation: {action} is only supported in InWorld"
        ));
    }
    Ok(())
}

/// Breadth-first: nearest actual visible named Control, not registry/fixture data.
fn visible_control(client: &Gd<Node>, name: &str) -> Option<Gd<Control>> {
    let mut nodes = VecDeque::from([client.clone()]);
    while let Some(node) = nodes.pop_front() {
        if node.get_name().to_string() == name
            && let Ok(control) = node.clone().try_cast::<Control>()
            && control.is_inside_tree()
            && control.is_visible_in_tree()
        {
            return Some(control);
        }
        nodes.extend(node.get_children().iter_shared());
    }
    None
}

/// A left click at `across` (0..=1) of the frame's width, at mid-height.
fn click_at_events(
    client: &Gd<Node>,
    name: &str,
    across: f32,
) -> Result<VecDeque<Gd<InputEvent>>, String> {
    if !(0.0..=1.0).contains(&across) {
        return Err(format!(
            "native JS automation: clickAt '{name}' needs a fraction in 0..=1, got {across}"
        ));
    }
    let control = visible_control(client, name)
        .ok_or_else(|| format!("native JS automation: no visible mounted frame '{name}'"))?;
    let size = control.get_size();
    let point =
        control.get_global_transform_with_canvas() * Vector2::new(size.x * across, size.y * 0.5);
    let mut events = VecDeque::from([mouse_motion_event(point, false)]);
    for pressed in [true, false] {
        events.push_back(mouse_button_event(point, MouseButton::LEFT, pressed, false));
    }
    Ok(events)
}

/// One press and release of the wheel button per notch over the frame's centre.
fn wheel_events(
    client: &Gd<Node>,
    name: &str,
    notches: i32,
) -> Result<VecDeque<Gd<InputEvent>>, String> {
    let control = visible_control(client, name)
        .ok_or_else(|| format!("native JS automation: no visible mounted frame '{name}'"))?;
    let point = control.get_global_transform_with_canvas() * (control.get_size() * 0.5);
    let button = if notches < 0 {
        MouseButton::WHEEL_UP
    } else {
        MouseButton::WHEEL_DOWN
    };
    let mut events = VecDeque::from([mouse_motion_event(point, false)]);
    for _ in 0..notches.unsigned_abs() {
        for pressed in [true, false] {
            events.push_back(mouse_button_event(point, button, pressed, false));
        }
    }
    Ok(events)
}

fn click_events(
    client: &Gd<Node>,
    name: &str,
    button: MouseButton,
    shift: bool,
) -> Result<VecDeque<Gd<InputEvent>>, String> {
    let control = visible_control(client, name)
        .ok_or_else(|| format!("native JS automation: no visible mounted frame '{name}'"))?;
    let size = control.get_size();
    if size.x <= 0.0 || size.y <= 0.0 {
        return Err(format!(
            "native JS automation: frame '{name}' has no clickable area"
        ));
    }
    let point = control.get_global_transform_with_canvas() * (size * 0.5);
    let mut events = VecDeque::new();
    if shift {
        events.push_back(key_event(Key::SHIFT, true, &[Key::SHIFT]));
    }
    events.push_back(mouse_motion_event(point, shift));
    for pressed in [true, false] {
        events.push_back(mouse_button_event(point, button, pressed, shift));
    }
    if shift {
        events.push_back(key_event(Key::SHIFT, false, &[]));
    }
    Ok(events)
}

fn mouse_motion_event(point: Vector2, shift: bool) -> Gd<InputEvent> {
    let mut event = InputEventMouseMotion::new_gd();
    event.set_position(point);
    event.set_global_position(point);
    event.set_shift_pressed(shift);
    event.upcast()
}

fn mouse_button_event(
    point: Vector2,
    button: MouseButton,
    pressed: bool,
    shift: bool,
) -> Gd<InputEvent> {
    let mut event = InputEventMouseButton::new_gd();
    event.set_position(point);
    event.set_global_position(point);
    event.set_button_index(button);
    event.set_pressed(pressed);
    event.set_shift_pressed(shift);
    event.upcast()
}

fn right_click_events(
    client: &Gd<Node>,
    state: GameState,
    name: &str,
) -> Result<VecDeque<Gd<InputEvent>>, String> {
    require_inworld(state, "rightClick")?;
    click_events(client, name, MouseButton::RIGHT, false)
}

fn shift_click_events(
    client: &Gd<Node>,
    state: GameState,
    name: &str,
) -> Result<VecDeque<Gd<InputEvent>>, String> {
    require_inworld(state, "shiftClick")?;
    click_events(client, name, MouseButton::LEFT, true)
}

fn focused_text_events(client: &Gd<Node>, text: &str) -> Result<VecDeque<Gd<InputEvent>>, String> {
    require_focused_editor(client)?;
    Ok(text_events(text))
}

fn state_chord_events(
    state: GameState,
    chord: &KeyChord,
) -> Result<VecDeque<Gd<InputEvent>>, String> {
    if state != GameState::InWorld {
        chord.unmodified_key()?;
    }
    chord_events(chord)
}

fn require_focused_editor(client: &Gd<Node>) -> Result<(), String> {
    let focused = client
        .get_tree()
        .get_root()
        .gui_get_focus_owner()
        .ok_or("native JS automation: ui.type requires a focused native editor")?;
    if !focused.is_visible_in_tree() || !focused.is_inside_tree() {
        return Err("native JS automation: focused editor is hidden or unmounted".into());
    }
    let editable = if let Ok(editor) = focused.clone().try_cast::<LineEdit>() {
        editor.is_editable()
    } else if let Ok(editor) = focused.try_cast::<TextEdit>() {
        editor.is_editable()
    } else {
        false
    };
    if !editable {
        return Err(
            "native JS automation: ui.type requires an editable focused LineEdit or TextEdit"
                .into(),
        );
    }
    Ok(())
}

fn text_events(text: &str) -> VecDeque<Gd<InputEvent>> {
    let mut events = VecDeque::new();
    for character in text.chars() {
        for pressed in [true, false] {
            let mut event = InputEventKey::new_gd();
            event.set_unicode(u32::from(character));
            event.set_pressed(pressed);
            events.push_back(event.upcast());
        }
    }
    events
}

fn chord_events(chord: &KeyChord) -> Result<VecDeque<Gd<InputEvent>>, String> {
    let key = native_key(chord.key)?;
    let modifiers = chord
        .modifiers
        .iter()
        .map(|modifier| native_key(*modifier))
        .collect::<Result<Vec<_>, _>>()?;
    let mut held = Vec::new();
    let mut events = VecDeque::new();
    for modifier in modifiers {
        held.push(modifier);
        events.push_back(key_event(modifier, true, &held));
    }
    events.push_back(key_event(key, true, &held));
    events.push_back(key_event(key, false, &held));
    while let Some(modifier) = held.pop() {
        events.push_back(key_event(modifier, false, &held));
    }
    Ok(events)
}

fn key_event(key: Key, pressed: bool, held: &[Key]) -> Gd<InputEvent> {
    let mut event = InputEventKey::new_gd();
    event.set_keycode(key);
    event.set_physical_keycode(key);
    event.set_pressed(pressed);
    event.set_shift_pressed(held.contains(&Key::SHIFT));
    event.set_ctrl_pressed(held.contains(&Key::CTRL));
    event.set_alt_pressed(held.contains(&Key::ALT));
    event.upcast()
}

const NATIVE_KEY_PAIRS: [(KeyCode, Key); 71] = [
    (KeyCode::KeyA, Key::A),
    (KeyCode::KeyB, Key::B),
    (KeyCode::KeyC, Key::C),
    (KeyCode::KeyD, Key::D),
    (KeyCode::KeyE, Key::E),
    (KeyCode::KeyF, Key::F),
    (KeyCode::KeyG, Key::G),
    (KeyCode::KeyH, Key::H),
    (KeyCode::KeyI, Key::I),
    (KeyCode::KeyJ, Key::J),
    (KeyCode::KeyK, Key::K),
    (KeyCode::KeyL, Key::L),
    (KeyCode::KeyM, Key::M),
    (KeyCode::KeyN, Key::N),
    (KeyCode::KeyO, Key::O),
    (KeyCode::KeyP, Key::P),
    (KeyCode::KeyQ, Key::Q),
    (KeyCode::KeyR, Key::R),
    (KeyCode::KeyS, Key::S),
    (KeyCode::KeyT, Key::T),
    (KeyCode::KeyU, Key::U),
    (KeyCode::KeyV, Key::V),
    (KeyCode::KeyW, Key::W),
    (KeyCode::KeyX, Key::X),
    (KeyCode::KeyY, Key::Y),
    (KeyCode::KeyZ, Key::Z),
    (KeyCode::Digit0, Key::KEY_0),
    (KeyCode::Digit1, Key::KEY_1),
    (KeyCode::Digit2, Key::KEY_2),
    (KeyCode::Digit3, Key::KEY_3),
    (KeyCode::Digit4, Key::KEY_4),
    (KeyCode::Digit5, Key::KEY_5),
    (KeyCode::Digit6, Key::KEY_6),
    (KeyCode::Digit7, Key::KEY_7),
    (KeyCode::Digit8, Key::KEY_8),
    (KeyCode::Digit9, Key::KEY_9),
    (KeyCode::F1, Key::F1),
    (KeyCode::F2, Key::F2),
    (KeyCode::F3, Key::F3),
    (KeyCode::F4, Key::F4),
    (KeyCode::F5, Key::F5),
    (KeyCode::F6, Key::F6),
    (KeyCode::F7, Key::F7),
    (KeyCode::F8, Key::F8),
    (KeyCode::F9, Key::F9),
    (KeyCode::F10, Key::F10),
    (KeyCode::F11, Key::F11),
    (KeyCode::F12, Key::F12),
    (KeyCode::Space, Key::SPACE),
    (KeyCode::Tab, Key::TAB),
    (KeyCode::Escape, Key::ESCAPE),
    (KeyCode::Minus, Key::MINUS),
    (KeyCode::Equal, Key::EQUAL),
    (KeyCode::BracketLeft, Key::BRACKETLEFT),
    (KeyCode::BracketRight, Key::BRACKETRIGHT),
    (KeyCode::ArrowLeft, Key::LEFT),
    (KeyCode::ArrowRight, Key::RIGHT),
    (KeyCode::ArrowUp, Key::UP),
    (KeyCode::ArrowDown, Key::DOWN),
    (KeyCode::PageUp, Key::PAGEUP),
    (KeyCode::PageDown, Key::PAGEDOWN),
    (KeyCode::NumLock, Key::NUMLOCK),
    (KeyCode::Home, Key::HOME),
    (KeyCode::End, Key::END),
    (KeyCode::Insert, Key::INSERT),
    (KeyCode::Delete, Key::DELETE),
    (KeyCode::Backspace, Key::BACKSPACE),
    (KeyCode::Enter, Key::ENTER),
    (KeyCode::ShiftLeft, Key::SHIFT),
    (KeyCode::ControlLeft, Key::CTRL),
    (KeyCode::AltLeft, Key::ALT),
];

static NATIVE_KEYS: LazyLock<HashMap<KeyCode, Key>> =
    LazyLock::new(|| NATIVE_KEY_PAIRS.into_iter().collect());

fn native_key(key: KeyCode) -> Result<Key, String> {
    NATIVE_KEYS
        .get(&key)
        .copied()
        .ok_or_else(|| format!("native JS automation: unconverted native key {key:?}"))
}

fn print_tree(response: Response) -> Result<bool, String> {
    match response {
        Response::Tree(text) => {
            println!("{text}");
            Ok(true)
        }
        Response::Error(error) => Err(error),
        _ => Err("native JS automation: tree formatter returned a non-tree response".into()),
    }
}

/// Separate process bind prevents viewport input from reentering a bound GameClient.
#[derive(GodotClass)]
#[class(base = Node)]
pub(crate) struct NativeJsAutomationHost {
    base: Base<Node>,
    runtime: Option<NativeJsAutomation>,
}

#[godot_api]
impl INode for NativeJsAutomationHost {
    fn init(base: Base<Node>) -> Self {
        Self {
            base,
            runtime: None,
        }
    }

    fn process(&mut self, delta: f64) {
        if self.runtime.is_none() {
            return;
        }
        let Some(parent) = self.base().get_parent() else {
            godot_error!("native JS automation: host has no client parent");
            self.runtime = None;
            return;
        };
        let Ok(client) = parent.try_cast::<GameClient>() else {
            godot_error!("native JS automation: host parent is not GameClient");
            self.runtime = None;
            return;
        };
        let state = client.bind().automation_game_state();
        let Some(runtime) = self.runtime.as_mut() else {
            return;
        };
        match runtime.poll(&client.upcast(), delta, state) {
            // Retain the last wait failure after draining; it was reported at its deadline.
            Ok(true) => self.base_mut().set_process(false),
            Ok(false) => {}
            Err(error) => {
                godot_error!("{error}");
                self.runtime = None;
            }
        }
    }
}

/// Loads and mounts the queue after startup parsing; input starts in the host's process.
pub(crate) fn attach(client: &mut Gd<GameClient>, path: &Path) -> Result<(), String> {
    let runtime = NativeJsAutomation::load(path)?;
    let mut host = NativeJsAutomationHost::new_alloc();
    host.set_name("NativeJsAutomation");
    host.set_process_priority(2);
    host.bind_mut().runtime = Some(runtime);
    client.add_child(&host);
    Ok(())
}
