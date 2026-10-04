use std::cell::RefCell;
use std::path::{Path, PathBuf};

use quick_js::{Arguments, Context, JsValue};

use crate::automation_data::{UiAutomationAction, parse_key_chord, parse_state};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct JsAutomationScriptPath {
    pub path: PathBuf,
}

pub fn parse_js_automation_arg(args: &[String]) -> Option<JsAutomationScriptPath> {
    args.windows(2).find_map(|window| {
        (window[0] == "--run-js-ui-script").then(|| JsAutomationScriptPath {
            path: PathBuf::from(&window[1]),
        })
    })
}

pub fn load_js_automation_script(path: &Path) -> Result<Vec<UiAutomationAction>, String> {
    let contents = std::fs::read_to_string(path)
        .map_err(|err| format!("failed to read {}: {err}", path.display()))?;
    run_js_to_actions(&contents)
}

pub fn run_js_to_actions(script: &str) -> Result<Vec<UiAutomationAction>, String> {
    let ctx = Context::new().map_err(|err| format!("failed to create JS context: {err}"))?;

    JS_ACTIONS.with(|actions| actions.borrow_mut().clear());
    register_callbacks(&ctx)?;
    ctx.eval(PRELUDE)
        .map_err(|err| format!("failed to initialize JS helpers: {err}"))?;
    ctx.eval(script)
        .map_err(|err| format!("failed to execute JS automation script: {err}"))?;

    Ok(JS_ACTIONS.with(|actions| actions.borrow().clone()))
}

fn register_callbacks(ctx: &Context) -> Result<(), String> {
    register_action_callbacks(ctx)?;
    register_wait_callbacks(ctx)?;
    register_debug_callbacks(ctx)?;
    register_env_callback(ctx)?;
    Ok(())
}

fn register_action_callbacks(ctx: &Context) -> Result<(), String> {
    ctx.add_callback("__click", |name: String| -> bool {
        push_action(UiAutomationAction::ClickFrame(name));
        true
    })
    .map_err(|err| format!("failed to register click callback: {err}"))?;
    ctx.add_callback("__clickAt", |name: String, across: f64| -> bool {
        push_action(UiAutomationAction::ClickFrameAt(name, across as f32));
        true
    })
    .map_err(|err| format!("failed to register clickAt callback: {err}"))?;
    ctx.add_callback("__rightClick", |name: String| -> bool {
        push_action(UiAutomationAction::RightClickFrame(name));
        true
    })
    .map_err(|err| format!("failed to register rightClick callback: {err}"))?;
    ctx.add_callback("__shiftClick", |name: String| -> bool {
        push_action(UiAutomationAction::ShiftClickFrame(name));
        true
    })
    .map_err(|err| format!("failed to register shiftClick callback: {err}"))?;
    ctx.add_callback("__type", |text: String| -> bool {
        push_action(UiAutomationAction::TypeText(text));
        true
    })
    .map_err(|err| format!("failed to register type callback: {err}"))?;
    ctx.add_callback("__key", |key: String| -> Result<bool, String> {
        push_action(UiAutomationAction::PressKey(parse_key_chord(&key)?));
        Ok(true)
    })
    .map_err(|err| format!("failed to register key callback: {err}"))
}

fn register_wait_callbacks(ctx: &Context) -> Result<(), String> {
    ctx.add_callback(
        "__waitForState",
        |args: Arguments| -> Result<bool, String> {
            let (state, timeout_secs) = parse_wait_args(args)?;
            push_action(UiAutomationAction::WaitForState(
                parse_state(&state)?,
                timeout_secs,
            ));
            Ok(true)
        },
    )
    .map_err(|err| format!("failed to register waitForState callback: {err}"))?;
    ctx.add_callback(
        "__waitForFrame",
        |args: Arguments| -> Result<bool, String> {
            let (name, timeout_secs) = parse_wait_args(args)?;
            push_action(UiAutomationAction::WaitForFrame(name, timeout_secs));
            Ok(true)
        },
    )
    .map_err(|err| format!("failed to register waitForFrame callback: {err}"))?;
    ctx.add_callback("__wait", |args: Arguments| -> Result<bool, String> {
        let secs = match args.into_vec().into_iter().next() {
            Some(JsValue::Int(value)) => value as f32,
            Some(JsValue::Float(value)) => value as f32,
            _ => return Err("ui.wait requires a number of seconds".to_string()),
        };
        push_action(UiAutomationAction::Wait(secs));
        Ok(true)
    })
    .map_err(|err| format!("failed to register wait callback: {err}"))?;
    Ok(())
}

fn register_debug_callbacks(ctx: &Context) -> Result<(), String> {
    ctx.add_callback("__dumpTree", || -> bool {
        push_action(UiAutomationAction::DumpTree);
        true
    })
    .map_err(|err| format!("failed to register dumpTree callback: {err}"))?;
    ctx.add_callback("__dumpUiTree", || -> bool {
        push_action(UiAutomationAction::DumpUiTree);
        true
    })
    .map_err(|err| format!("failed to register dumpUiTree callback: {err}"))?;
    Ok(())
}

fn register_env_callback(ctx: &Context) -> Result<(), String> {
    ctx.add_callback("__env", move |name: String| -> String {
        std::env::var(name).unwrap_or_default()
    })
    .map_err(|err| format!("failed to register env callback: {err}"))
}

thread_local! {
    static JS_ACTIONS: RefCell<Vec<UiAutomationAction>> = const { RefCell::new(Vec::new()) };
}

fn push_action(action: UiAutomationAction) {
    JS_ACTIONS.with(|actions| actions.borrow_mut().push(action));
}

fn parse_wait_args(args: Arguments) -> Result<(String, f32), String> {
    let values = args.into_vec();
    if values.len() != 2 {
        return Err(format!(
            "expected 2 arguments for wait action, got {}",
            values.len()
        ));
    }
    let mut iter = values.into_iter();
    let state = match iter.next() {
        Some(JsValue::String(value)) => value,
        _ => return Err("wait action requires a string target".to_string()),
    };
    let timeout_secs = match iter.next() {
        Some(JsValue::Int(value)) => value as f32,
        Some(JsValue::Float(value)) => value as f32,
        _ => return Err("wait action requires a numeric timeout".to_string()),
    };
    Ok((state, timeout_secs))
}

const PRELUDE: &str = r#"
globalThis.ui = {
  click: (name) => __click(name),
  clickAt: (name, across) => __clickAt(name, Number(across)),
  rightClick: (name) => __rightClick(name),
  shiftClick: (name) => __shiftClick(name),
  type: (text) => __type(text),
  key: (key) => __key(key),
  waitForState: (state, timeoutSecs) => __waitForState(state, timeoutSecs),
  waitForFrame: (name, timeoutSecs) => __waitForFrame(name, timeoutSecs),
  wait: (secs) => __wait(secs),
  dumpTree: () => __dumpTree(),
  dumpUiTree: () => __dumpUiTree(),
};
globalThis.env = new Proxy({}, {
  get: (_, prop) => __env(String(prop)),
});
"#;

#[cfg(test)]
mod tests {
    use super::*;
    use crate::game_state_enum::GameState;

    #[test]
    fn js_missing_env_lookup_emits_empty_text() {
        let name = format!(
            "WORLD_OF_OSSO_JS_MISSING_ENV_{}_{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        );
        assert!(std::env::var_os(&name).is_none());
        let script = format!("ui.type(env[{name:?}]);");
        assert_eq!(
            run_js_to_actions(&script).unwrap(),
            vec![UiAutomationAction::TypeText(String::new())]
        );
    }

    #[test]
    fn js_frame_wait_and_ui_dump_emit_actions() {
        assert_eq!(
            run_js_to_actions("ui.waitForFrame('LoginRoot', 3); ui.dumpUiTree();").unwrap(),
            vec![
                UiAutomationAction::WaitForFrame("LoginRoot".into(), 3.0),
                UiAutomationAction::DumpUiTree,
            ]
        );
    }

    #[test]
    fn js_state_aliases_preserve_original_states() {
        for (upper, lower, state) in [
            ("Login", "login", GameState::Login),
            ("Connecting", "connecting", GameState::Connecting),
            ("CharSelect", "charselect", GameState::CharSelect),
            ("Loading", "loading", GameState::Loading),
            ("InWorld", "inworld", GameState::InWorld),
        ] {
            let script = format!("ui.waitForState({upper:?}, 1); ui.waitForState({lower:?}, 2);");
            assert_eq!(
                run_js_to_actions(&script).unwrap(),
                vec![
                    UiAutomationAction::WaitForState(state, 1.0),
                    UiAutomationAction::WaitForState(state, 2.0),
                ]
            );
        }
        assert!(run_js_to_actions("ui.waitForState('LOGIN', 1);").is_err());
        assert!(run_js_to_actions("ui.waitForState('CharCreate', 1);").is_err());
    }

    #[test]
    fn js_compilation_does_not_leak_actions_between_scripts() {
        assert!(run_js_to_actions("ui.click('LoginRoot'); throw new Error('stop');").is_err());
        assert_eq!(run_js_to_actions("").unwrap(), vec![]);
        assert_eq!(
            run_js_to_actions("ui.type('next');").unwrap(),
            vec![UiAutomationAction::TypeText("next".into())]
        );
    }

    #[test]
    fn js_wait_rejects_invalid_argument_types_and_counts() {
        for script in [
            "ui.wait('1');",
            "ui.waitForFrame('LoginRoot');",
            "ui.waitForFrame(1, 2);",
            "ui.waitForState('Login', '2');",
        ] {
            assert!(run_js_to_actions(script).is_err(), "accepted {script}");
        }
    }

    #[test]
    fn js_click_and_type_emit_automation_actions() {
        let script = r#"
            ui.click("UsernameInput");
            ui.type("alice");
            ui.click("PasswordInput");
            ui.type("secret");
            ui.rightClick("BuffButton0");
            ui.shiftClick("ContainerFrame0Slot3");
            ui.clickAt("Sliderlayout_text_size", 0.8);
        "#;
        let actions = run_js_to_actions(script).expect("JS actions should parse");
        assert_eq!(
            actions,
            vec![
                UiAutomationAction::ClickFrame("UsernameInput".into()),
                UiAutomationAction::TypeText("alice".into()),
                UiAutomationAction::ClickFrame("PasswordInput".into()),
                UiAutomationAction::TypeText("secret".into()),
                UiAutomationAction::RightClickFrame("BuffButton0".into()),
                UiAutomationAction::ShiftClickFrame("ContainerFrame0Slot3".into()),
                UiAutomationAction::ClickFrameAt("Sliderlayout_text_size".into(), 0.8),
            ]
        );
    }

    #[test]
    fn js_key_parses_letters_digits_function_keys_and_modifier_chords() {
        use bevy::input::keyboard::KeyCode;
        let script = r#"
            ui.key("b");
            ui.key("1");
            ui.key("F12");
            ui.key("Shift+M");
            ui.key("Ctrl+Alt+KeyP");
            ui.key("esc");
        "#;
        let actions = run_js_to_actions(script).expect("JS actions should parse");
        let chord = |modifiers: Vec<KeyCode>, key| {
            UiAutomationAction::PressKey(crate::automation_data::KeyChord { modifiers, key })
        };
        assert_eq!(
            actions,
            vec![
                chord(vec![], KeyCode::KeyB),
                chord(vec![], KeyCode::Digit1),
                chord(vec![], KeyCode::F12),
                chord(vec![KeyCode::ShiftLeft], KeyCode::KeyM),
                chord(vec![KeyCode::ControlLeft, KeyCode::AltLeft], KeyCode::KeyP),
                chord(vec![], KeyCode::Escape),
            ]
        );
    }

    #[test]
    fn js_key_rejects_unknown_key_and_modifier() {
        assert!(run_js_to_actions(r#"ui.key("Hyper+M");"#).is_err());
        assert!(run_js_to_actions(r#"ui.key("NotAKey");"#).is_err());
    }

    #[test]
    fn js_wait_emits_a_delay() {
        assert_eq!(
            run_js_to_actions("ui.wait(2.5); ui.wait(3);").unwrap(),
            vec![UiAutomationAction::Wait(2.5), UiAutomationAction::Wait(3.0)]
        );
    }

    #[test]
    fn js_wait_for_state_and_dump_emit_actions() {
        let script = r#"
            ui.waitForState("CharSelect", 5.0);
            ui.dumpTree();
        "#;
        let actions = run_js_to_actions(script).expect("JS actions should parse");
        assert_eq!(
            actions,
            vec![
                UiAutomationAction::WaitForState(GameState::CharSelect, 5.0),
                UiAutomationAction::DumpTree,
            ]
        );
    }
}
