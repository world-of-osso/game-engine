use std::cell::RefCell;
use std::path::{Path, PathBuf};

use quick_js::{Arguments, Context, JsValue};

use crate::game_state_enum::GameState;
use crate::ui::automation::UiAutomationAction;

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
    ctx.add_callback("__type", |text: String| -> bool {
        push_action(UiAutomationAction::TypeText(text));
        true
    })
    .map_err(|err| format!("failed to register type callback: {err}"))?;
    ctx.add_callback("__key", |key: String| -> Result<bool, String> {
        push_action(UiAutomationAction::PressKey(parse_key(&key)?));
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

fn parse_state(value: &str) -> Result<GameState, String> {
    match value {
        "Login" | "login" => Ok(GameState::Login),
        "Connecting" | "connecting" => Ok(GameState::Connecting),
        "CharSelect" | "charselect" => Ok(GameState::CharSelect),
        "Loading" | "loading" => Ok(GameState::Loading),
        "InWorld" | "inworld" => Ok(GameState::InWorld),
        other => Err(format!("unknown game state '{other}'")),
    }
}

fn parse_key(value: &str) -> Result<bevy::input::keyboard::KeyCode, String> {
    match value {
        "Enter" | "enter" => Ok(bevy::input::keyboard::KeyCode::Enter),
        "Tab" | "tab" => Ok(bevy::input::keyboard::KeyCode::Tab),
        "Escape" | "escape" | "esc" => Ok(bevy::input::keyboard::KeyCode::Escape),
        "Backspace" | "backspace" => Ok(bevy::input::keyboard::KeyCode::Backspace),
        other => Err(format!("unsupported automation key '{other}'")),
    }
}

const PRELUDE: &str = r#"
globalThis.ui = {
  click: (name) => __click(name),
  type: (text) => __type(text),
  key: (key) => __key(key),
  waitForState: (state, timeoutSecs) => __waitForState(state, timeoutSecs),
  waitForFrame: (name, timeoutSecs) => __waitForFrame(name, timeoutSecs),
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

    #[test]
    fn js_click_and_type_emit_automation_actions() {
        let script = r#"
            ui.click("UsernameInput");
            ui.type("alice");
            ui.click("PasswordInput");
            ui.type("secret");
        "#;
        let actions = run_js_to_actions(script).expect("JS actions should parse");
        assert_eq!(
            actions,
            vec![
                UiAutomationAction::ClickFrame("UsernameInput".into()),
                UiAutomationAction::TypeText("alice".into()),
                UiAutomationAction::ClickFrame("PasswordInput".into()),
                UiAutomationAction::TypeText("secret".into()),
            ]
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
