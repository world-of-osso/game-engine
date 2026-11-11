use super::*;

#[test]
fn auth_stale_token_reject_clears_disk_and_returns_login_with_message() {
    let root = std::env::temp_dir().join(format!("auth-stale-token-{}", std::process::id()));
    fs::create_dir_all(&root).unwrap();
    let mut account = Account::new(root.clone());
    account.hostname = "127.0.0.1:5497".into();
    let path = token_path(&root, Some(&account.hostname));
    fs::write(&path, "token-from-recreated-db").unwrap();
    account.session.token = account.read_token().unwrap();
    let mut events = Vec::new();
    account
        .dispatch_message(
            ProtocolMessage::for_tests(LoginResponse {
                success: false,
                token: String::new(),
                characters: vec![],
                error: Some("Saved session expired. Please log in again.".into()),
            }),
            &mut events,
        )
        .unwrap();
    assert_eq!(account.session.screen, SessionScreen::Login);
    assert_eq!(
        account.session.feedback.as_deref(),
        Some("Saved session expired. Please log in again.")
    );
    assert!(
        events
            .iter()
            .any(|event| matches!(event, AccountEvent::Screen(SessionScreen::Login)))
    );
    assert!(
        account.session.token.is_none(),
        "rejected token retained in memory"
    );
    assert!(!path.exists(), "rejected token retained on disk");
    let reopened = Account::new(root.clone());
    assert!(reopened.read_token().unwrap().is_none());
    fs::remove_dir_all(root).unwrap();
}
