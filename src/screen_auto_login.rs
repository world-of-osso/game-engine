use game_engine::ui::automation::UiAutomationAction;

use crate::game_state::GameState;

/// For startup screen shortcuts, rewrite the initial auth flow as needed.
pub fn apply(
    actions: &mut Vec<UiAutomationAction>,
    server_addr: &mut Option<(std::net::SocketAddr, bool)>,
    initial_state: &mut Option<GameState>,
    auto_enter: &mut bool,
    startup_login: &mut Option<(String, String)>,
    has_saved_auth_token: bool,
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
    if server_addr.is_none() {
        *server_addr = Some(("127.0.0.1:5000".parse().unwrap(), false));
    }
    if matches!(target, Some(GameState::CharSelect | GameState::InWorld)) {
        *initial_state = Some(GameState::Connecting);
        if !has_saved_auth_token {
            *startup_login = Some(("admin".to_string(), "admin".to_string()));
        }
    } else {
        if has_saved_auth_token {
            *initial_state = Some(GameState::Connecting);
            *actions = saved_token_actions(target);
        } else {
            *initial_state = Some(GameState::Login);
            *actions = auto_login_actions(target);
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

fn auto_login_actions(target: Option<GameState>) -> Vec<UiAutomationAction> {
    let wait_state = match target {
        Some(GameState::CharCreate) => GameState::CharCreate,
        _ => GameState::CharSelect,
    };
    vec![
        UiAutomationAction::WaitForFrame("UsernameInput".to_string(), 5.0),
        UiAutomationAction::ClickFrame("UsernameInput".to_string()),
        UiAutomationAction::TypeText("admin".to_string()),
        UiAutomationAction::ClickFrame("PasswordInput".to_string()),
        UiAutomationAction::TypeText("admin".to_string()),
        UiAutomationAction::ClickFrame("ConnectButton".to_string()),
        UiAutomationAction::WaitForState(wait_state, 10.0),
    ]
}

#[cfg(test)]
mod tests {
    use super::*;

    fn socket() -> std::net::SocketAddr {
        "127.0.0.1:5000".parse().expect("test socket should parse")
    }

    #[test]
    fn saved_token_uses_connecting_state_for_charselect_screen() {
        let mut actions = Vec::new();
        let mut server_addr = Some((socket(), false));
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
        );

        assert_eq!(initial_state, Some(GameState::Connecting));
        assert!(actions.is_empty());
        assert_eq!(startup_login, None);
        assert!(!auto_enter);
    }

    #[test]
    fn saved_token_charcreate_still_waits_for_charselect_before_clicking_create() {
        let mut actions = Vec::new();
        let mut server_addr = Some((socket(), false));
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
        );

        assert_eq!(initial_state, Some(GameState::Connecting));
        assert!(actions.is_empty());
        assert_eq!(startup_login, None);
    }

    #[test]
    fn charcreate_without_token_waits_for_direct_charcreate_transition_after_login() {
        let mut actions = Vec::new();
        let mut server_addr = Some((socket(), false));
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
        let mut server_addr = Some((socket(), false));
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
        let mut server_addr = Some((socket(), false));
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
        );

        assert_eq!(initial_state, Some(GameState::Connecting));
        assert!(actions.is_empty());
        assert_eq!(startup_login, None);
        assert!(auto_enter);
    }

    #[test]
    fn inworld_without_token_connects_directly_with_startup_credentials() {
        let mut actions = Vec::new();
        let mut server_addr = Some((socket(), false));
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
        );

        assert_eq!(initial_state, Some(GameState::Connecting));
        assert!(actions.is_empty());
        assert_eq!(
            startup_login,
            Some(("admin".to_string(), "admin".to_string()))
        );
        assert!(auto_enter);
    }
}
