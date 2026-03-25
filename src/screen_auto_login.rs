use game_engine::ui::automation::UiAutomationAction;

use crate::cli_args::ServerArg;
use crate::game_state::GameState;

/// For startup screen shortcuts, rewrite the initial auth flow as needed.
pub fn apply(
    actions: &mut Vec<UiAutomationAction>,
    server_addr: &mut Option<ServerArg>,
    initial_state: &mut Option<GameState>,
    auto_enter: &mut bool,
    startup_login: &mut Option<(String, String)>,
    has_saved_auth_token: bool,
    default_login: Option<(String, String)>,
) {
    let target = match *initial_state {
        Some(GameState::CharSelect | GameState::CharCreate | GameState::InWorld) => *initial_state,
        _ => return,
    };
    if !actions.is_empty() {
        return;
    }
    // CharCreate can run standalone without a server — skip auto-login if no server is set.
    if server_addr.is_none() && target == Some(GameState::CharCreate) {
        return;
    }
    if matches!(target, Some(GameState::CharSelect | GameState::InWorld)) {
        *initial_state = Some(GameState::Connecting);
        if !has_saved_auth_token {
            *startup_login = Some(default_login_credentials(default_login.clone()));
        }
    } else {
        if has_saved_auth_token {
            *initial_state = Some(GameState::Connecting);
            *actions = saved_token_actions(target);
        } else {
            *initial_state = Some(GameState::Login);
            *actions = auto_login_actions(target, default_login);
        }
    }
    if target == Some(GameState::InWorld) {
        *auto_enter = true;
    }
}

fn saved_token_actions(target: Option<GameState>) -> Vec<UiAutomationAction> {
    match target {
        Some(GameState::CharCreate) => Vec::new(),
        _ => vec![UiAutomationAction::WaitForState(
            GameState::CharSelect,
            10.0,
        )],
    }
}

fn default_login_credentials(default_login: Option<(String, String)>) -> (String, String) {
    default_login.unwrap_or_else(|| ("admin".to_string(), "admin".to_string()))
}

fn auto_login_actions(
    target: Option<GameState>,
    default_login: Option<(String, String)>,
) -> Vec<UiAutomationAction> {
    let wait_state = match target {
        Some(GameState::CharCreate) => GameState::CharCreate,
        _ => GameState::CharSelect,
    };
    let (username, password) = default_login_credentials(default_login);
    vec![
        UiAutomationAction::WaitForFrame("UsernameInput".to_string(), 5.0),
        UiAutomationAction::ClickFrame("UsernameInput".to_string()),
        UiAutomationAction::TypeText(username),
        UiAutomationAction::ClickFrame("PasswordInput".to_string()),
        UiAutomationAction::TypeText(password),
        UiAutomationAction::ClickFrame("ConnectButton".to_string()),
        UiAutomationAction::WaitForState(wait_state, 10.0),
    ]
}

#[cfg(test)]
mod tests {
    use super::*;

    fn test_server() -> ServerArg {
        ServerArg {
            addr: "127.0.0.1:5000".parse().unwrap(),
            hostname: "127.0.0.1:5000".to_string(),
            dev: false,
        }
    }

    #[test]
    fn saved_token_uses_connecting_state_for_charselect_screen() {
        let mut actions = Vec::new();
        let mut server_addr = Some(test_server());
        let mut initial_state = Some(GameState::CharSelect);
        let mut auto_enter = false;
        let mut startup_login = None;

        apply(
            &mut actions,
            &mut server_addr,
            &mut initial_state,
            &mut auto_enter,
            &mut startup_login,
            true,
            None,
        );

        assert_eq!(initial_state, Some(GameState::Connecting));
        assert!(actions.is_empty());
        assert_eq!(startup_login, None);
        assert!(!auto_enter);
    }

    #[test]
    fn saved_token_charcreate_still_waits_for_charselect_before_clicking_create() {
        let mut actions = Vec::new();
        let mut server_addr = Some(test_server());
        let mut initial_state = Some(GameState::CharCreate);
        let mut auto_enter = false;
        let mut startup_login = None;

        apply(
            &mut actions,
            &mut server_addr,
            &mut initial_state,
            &mut auto_enter,
            &mut startup_login,
            true,
            None,
        );

        assert_eq!(initial_state, Some(GameState::Connecting));
        assert!(actions.is_empty());
        assert_eq!(startup_login, None);
    }

    #[test]
    fn charcreate_without_token_waits_for_direct_charcreate_transition_after_login() {
        let mut actions = Vec::new();
        let mut server_addr = Some(test_server());
        let mut initial_state = Some(GameState::CharCreate);
        let mut auto_enter = false;
        let mut startup_login = None;

        apply(
            &mut actions,
            &mut server_addr,
            &mut initial_state,
            &mut auto_enter,
            &mut startup_login,
            false,
            None,
        );

        assert_eq!(initial_state, Some(GameState::Login));
        assert_eq!(
            actions,
            vec![
                UiAutomationAction::WaitForFrame("UsernameInput".to_string(), 5.0),
                UiAutomationAction::ClickFrame("UsernameInput".to_string()),
                UiAutomationAction::TypeText("admin".to_string()),
                UiAutomationAction::ClickFrame("PasswordInput".to_string()),
                UiAutomationAction::TypeText("admin".to_string()),
                UiAutomationAction::ClickFrame("ConnectButton".to_string()),
                UiAutomationAction::WaitForState(GameState::CharCreate, 10.0),
            ]
        );
        assert_eq!(startup_login, None);
        assert!(!auto_enter);
    }

    #[test]
    fn charselect_without_token_connects_directly_with_startup_credentials() {
        let mut actions = Vec::new();
        let mut server_addr = Some(test_server());
        let mut initial_state = Some(GameState::CharSelect);
        let mut auto_enter = false;
        let mut startup_login = None;

        apply(
            &mut actions,
            &mut server_addr,
            &mut initial_state,
            &mut auto_enter,
            &mut startup_login,
            false,
            None,
        );

        assert_eq!(initial_state, Some(GameState::Connecting));
        assert!(actions.is_empty());
        assert_eq!(
            startup_login,
            Some(("admin".to_string(), "admin".to_string()))
        );
        assert!(!auto_enter);
    }

    #[test]
    fn inworld_uses_connecting_without_ui_automation_when_saved_token_exists() {
        let mut actions = Vec::new();
        let mut server_addr = Some(test_server());
        let mut initial_state = Some(GameState::InWorld);
        let mut auto_enter = false;
        let mut startup_login = None;

        apply(
            &mut actions,
            &mut server_addr,
            &mut initial_state,
            &mut auto_enter,
            &mut startup_login,
            true,
            None,
        );

        assert_eq!(initial_state, Some(GameState::Connecting));
        assert!(actions.is_empty());
        assert_eq!(startup_login, None);
        assert!(auto_enter);
    }

    #[test]
    fn inworld_without_token_connects_directly_with_startup_credentials() {
        let mut actions = Vec::new();
        let mut server_addr = Some(test_server());
        let mut initial_state = Some(GameState::InWorld);
        let mut auto_enter = false;
        let mut startup_login = None;

        apply(
            &mut actions,
            &mut server_addr,
            &mut initial_state,
            &mut auto_enter,
            &mut startup_login,
            false,
            None,
        );

        assert_eq!(initial_state, Some(GameState::Connecting));
        assert!(actions.is_empty());
        assert_eq!(
            startup_login,
            Some(("admin".to_string(), "admin".to_string()))
        );
        assert!(auto_enter);
    }

    #[test]
    fn charselect_without_token_uses_config_credentials_when_provided() {
        let mut actions = Vec::new();
        let mut server_addr = Some(test_server());
        let mut initial_state = Some(GameState::CharSelect);
        let mut auto_enter = false;
        let mut startup_login = None;

        apply(
            &mut actions,
            &mut server_addr,
            &mut initial_state,
            &mut auto_enter,
            &mut startup_login,
            false,
            Some(("prod-user".to_string(), "prod-pass".to_string())),
        );

        assert_eq!(initial_state, Some(GameState::Connecting));
        assert!(actions.is_empty());
        assert_eq!(
            startup_login,
            Some(("prod-user".to_string(), "prod-pass".to_string()))
        );
        assert!(!auto_enter);
    }
}
