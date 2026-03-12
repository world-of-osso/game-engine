use game_engine::ui::automation::UiAutomationAction;

use crate::game_state::GameState;

/// For `--screen charselect/charcreate/inworld`: auto-login with admin/admin, then navigate.
pub fn apply(
    actions: &mut Vec<UiAutomationAction>,
    server_addr: &mut Option<(std::net::SocketAddr, bool)>,
    initial_state: &mut Option<GameState>,
    auto_enter: &mut bool,
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
    *initial_state = Some(GameState::Login);
    *actions = auto_login_actions();
    if target == Some(GameState::CharCreate) {
        actions.push(UiAutomationAction::ClickFrame("CreateChar".to_string()));
        actions.push(UiAutomationAction::WaitForState(GameState::CharCreate, 5.0));
    } else if target == Some(GameState::InWorld) {
        *auto_enter = true;
    }
}

fn auto_login_actions() -> Vec<UiAutomationAction> {
    vec![
        UiAutomationAction::WaitForFrame("UsernameInput".to_string(), 5.0),
        UiAutomationAction::ClickFrame("UsernameInput".to_string()),
        UiAutomationAction::TypeText("admin".to_string()),
        UiAutomationAction::ClickFrame("PasswordInput".to_string()),
        UiAutomationAction::TypeText("admin".to_string()),
        UiAutomationAction::ClickFrame("ConnectButton".to_string()),
        UiAutomationAction::WaitForState(GameState::CharSelect, 10.0),
    ]
}
