//! Main-thread consumption of the original synchronous JS action compiler.

use std::{collections::VecDeque, path::Path};

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
        match action {
            UiAutomationAction::WaitForState(target, timeout) => {
                self.wait_until(state == target, timeout, &format!("state {target:?}"))
            }
            UiAutomationAction::WaitForFrame(name, timeout) => self.wait_until(
                visible_control(client, &name).is_some(),
                timeout,
                &format!("frame '{name}'"),
            ),
            UiAutomationAction::Wait(seconds) => {
                validate_seconds(seconds)?;
                let started = *self.wait_started.get_or_insert(self.elapsed);
                Ok(self.elapsed - started >= f64::from(seconds))
            }
            UiAutomationAction::ClickFrame(name) => {
                self.input = click_events(client, &name, MouseButton::LEFT, false)?;
                Ok(true)
            }
            UiAutomationAction::RightClickFrame(name) => {
                require_inworld(state, "rightClick")?;
                self.input = click_events(client, &name, MouseButton::RIGHT, false)?;
                Ok(true)
            }
            UiAutomationAction::ShiftClickFrame(name) => {
                require_inworld(state, "shiftClick")?;
                self.input = click_events(client, &name, MouseButton::LEFT, true)?;
                Ok(true)
            }
            UiAutomationAction::TypeText(text) => {
                require_focused_editor(client)?;
                self.input = text_events(&text);
                Ok(true)
            }
            UiAutomationAction::PressKey(chord) => {
                if state != GameState::InWorld {
                    chord.unmodified_key()?;
                }
                self.input = chord_events(&chord)?;
                Ok(true)
            }
            UiAutomationAction::DumpTree => print_tree(ipc::dump_tree(client, None)),
            UiAutomationAction::DumpUiTree => print_tree(ipc::dump_ui_tree(client, None)),
        }
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
    let mut motion = InputEventMouseMotion::new_gd();
    motion.set_position(point);
    motion.set_global_position(point);
    motion.set_shift_pressed(shift);
    let mut events = VecDeque::new();
    if shift {
        events.push_back(key_event(Key::SHIFT, true, &[Key::SHIFT]));
    }
    events.push_back(motion.upcast());
    for pressed in [true, false] {
        let mut event = InputEventMouseButton::new_gd();
        event.set_position(point);
        event.set_global_position(point);
        event.set_button_index(button);
        event.set_pressed(pressed);
        event.set_shift_pressed(shift);
        events.push_back(event.upcast());
    }
    if shift {
        events.push_back(key_event(Key::SHIFT, false, &[]));
    }
    Ok(events)
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

fn native_key(key: KeyCode) -> Result<Key, String> {
    use KeyCode as B;
    Ok(match key {
        B::KeyA => Key::A,
        B::KeyB => Key::B,
        B::KeyC => Key::C,
        B::KeyD => Key::D,
        B::KeyE => Key::E,
        B::KeyF => Key::F,
        B::KeyG => Key::G,
        B::KeyH => Key::H,
        B::KeyI => Key::I,
        B::KeyJ => Key::J,
        B::KeyK => Key::K,
        B::KeyL => Key::L,
        B::KeyM => Key::M,
        B::KeyN => Key::N,
        B::KeyO => Key::O,
        B::KeyP => Key::P,
        B::KeyQ => Key::Q,
        B::KeyR => Key::R,
        B::KeyS => Key::S,
        B::KeyT => Key::T,
        B::KeyU => Key::U,
        B::KeyV => Key::V,
        B::KeyW => Key::W,
        B::KeyX => Key::X,
        B::KeyY => Key::Y,
        B::KeyZ => Key::Z,
        B::Digit0 => Key::KEY_0,
        B::Digit1 => Key::KEY_1,
        B::Digit2 => Key::KEY_2,
        B::Digit3 => Key::KEY_3,
        B::Digit4 => Key::KEY_4,
        B::Digit5 => Key::KEY_5,
        B::Digit6 => Key::KEY_6,
        B::Digit7 => Key::KEY_7,
        B::Digit8 => Key::KEY_8,
        B::Digit9 => Key::KEY_9,
        B::F1 => Key::F1,
        B::F2 => Key::F2,
        B::F3 => Key::F3,
        B::F4 => Key::F4,
        B::F5 => Key::F5,
        B::F6 => Key::F6,
        B::F7 => Key::F7,
        B::F8 => Key::F8,
        B::F9 => Key::F9,
        B::F10 => Key::F10,
        B::F11 => Key::F11,
        B::F12 => Key::F12,
        B::Space => Key::SPACE,
        B::Tab => Key::TAB,
        B::Escape => Key::ESCAPE,
        B::Minus => Key::MINUS,
        B::Equal => Key::EQUAL,
        B::BracketLeft => Key::BRACKETLEFT,
        B::BracketRight => Key::BRACKETRIGHT,
        B::ArrowLeft => Key::LEFT,
        B::ArrowRight => Key::RIGHT,
        B::ArrowUp => Key::UP,
        B::ArrowDown => Key::DOWN,
        B::PageUp => Key::PAGEUP,
        B::PageDown => Key::PAGEDOWN,
        B::NumLock => Key::NUMLOCK,
        B::Home => Key::HOME,
        B::End => Key::END,
        B::Insert => Key::INSERT,
        B::Delete => Key::DELETE,
        B::Backspace => Key::BACKSPACE,
        B::Enter => Key::ENTER,
        B::ShiftLeft => Key::SHIFT,
        B::ControlLeft => Key::CTRL,
        B::AltLeft => Key::ALT,
        other => {
            return Err(format!(
                "native JS automation: unconverted native key {other:?}"
            ));
        }
    })
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
