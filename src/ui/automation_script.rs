use std::path::{Path, PathBuf};

use bevy::input::keyboard::KeyCode;
use serde::Deserialize;

use crate::game_state_enum::GameState;
use crate::ui::automation::UiAutomationAction;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct UiAutomationScriptPath {
    pub path: PathBuf,
}

#[derive(Debug, Deserialize)]
#[serde(untagged)]
enum RawAutomationAction {
    Click {
        click: String,
    },
    Type {
        #[serde(rename = "type")]
        text: String,
    },
    Key {
        key: String,
    },
    WaitForState {
        wait_for_state: String,
        timeout_secs: f32,
    },
    WaitForFrame {
        wait_for_frame: String,
        timeout_secs: f32,
    },
    DumpTree {
        dump_tree: bool,
    },
    DumpUiTree {
        dump_ui_tree: bool,
    },
}

pub fn parse_automation_script_arg(args: &[String]) -> Option<UiAutomationScriptPath> {
    args.windows(2).find_map(|window| {
        (window[0] == "--run-ui-script").then(|| UiAutomationScriptPath {
            path: PathBuf::from(&window[1]),
        })
    })
}

pub fn load_automation_script(path: &Path) -> Result<Vec<UiAutomationAction>, String> {
    let contents = std::fs::read_to_string(path)
        .map_err(|err| format!("failed to read {}: {err}", path.display()))?;
    parse_automation_script(&contents)
}

pub fn parse_automation_script(script: &str) -> Result<Vec<UiAutomationAction>, String> {
    let raw: Vec<RawAutomationAction> =
        serde_json::from_str(script).map_err(|err| format!("invalid automation script: {err}"))?;
    raw.into_iter().map(raw_action_to_action).collect()
}

fn raw_action_to_action(raw: RawAutomationAction) -> Result<UiAutomationAction, String> {
    match raw {
        RawAutomationAction::Click { click } => Ok(UiAutomationAction::ClickFrame(click)),
        RawAutomationAction::Type { text } => Ok(UiAutomationAction::TypeText(text)),
        RawAutomationAction::Key { key } => Ok(UiAutomationAction::PressKey(parse_key(&key)?)),
        RawAutomationAction::WaitForState {
            wait_for_state,
            timeout_secs,
        } => Ok(UiAutomationAction::WaitForState(
            parse_state(&wait_for_state)?,
            timeout_secs,
        )),
        RawAutomationAction::WaitForFrame {
            wait_for_frame,
            timeout_secs,
        } => Ok(UiAutomationAction::WaitForFrame(
            wait_for_frame,
            timeout_secs,
        )),
        RawAutomationAction::DumpTree { dump_tree } => dump_tree
            .then_some(UiAutomationAction::DumpTree)
            .ok_or_else(|| "dump_tree must be true".to_string()),
        RawAutomationAction::DumpUiTree { dump_ui_tree } => dump_ui_tree
            .then_some(UiAutomationAction::DumpUiTree)
            .ok_or_else(|| "dump_ui_tree must be true".to_string()),
    }
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

fn parse_key(value: &str) -> Result<KeyCode, String> {
    match value {
        "Enter" | "enter" => Ok(KeyCode::Enter),
        "Tab" | "tab" => Ok(KeyCode::Tab),
        "Escape" | "escape" | "esc" => Ok(KeyCode::Escape),
        "Backspace" | "backspace" => Ok(KeyCode::Backspace),
        other => Err(format!("unsupported automation key '{other}'")),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_json_script_into_actions() {
        let script = r#"
        [
          {"click":"UsernameInput"},
          {"type":"alice"},
          {"click":"PasswordInput"},
          {"type":"secret"},
          {"click":"ConnectButton"},
          {"wait_for_state":"Connecting","timeout_secs":1.0},
          {"dump_tree":true}
        ]
        "#;
        let actions = parse_automation_script(script).expect("script should parse");
        assert_eq!(actions.len(), 7);
        assert_eq!(
            actions[0],
            UiAutomationAction::ClickFrame("UsernameInput".to_string())
        );
        assert_eq!(
            actions[5],
            UiAutomationAction::WaitForState(GameState::Connecting, 1.0)
        );
        assert_eq!(actions[6], UiAutomationAction::DumpTree);
    }

    #[test]
    fn startup_flag_loads_script_path_into_queue() {
        let args = vec![
            "--run-ui-script".to_string(),
            "debug/login.json".to_string(),
        ];
        let config = parse_automation_script_arg(&args).expect("expected script path");
        assert_eq!(config.path, PathBuf::from("debug/login.json"));
    }
}
