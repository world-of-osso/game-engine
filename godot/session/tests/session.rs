use game_engine_session::{
    AuthRequest, ReconnectPhase, Session, SessionEffect, SessionOptions, SessionScreen,
    normalize_auth_token, token_path,
};
use shared::components::{CharacterAppearance, EquipmentAppearance};
use shared::protocol::{
    CharacterListEntry, CharacterListUpdate, CreateCharacterResponse, DeleteCharacterResponse,
    EnterWorldResponse, ForcedDisconnect, LoginResponse, RegisterResponse,
};
use std::path::Path;

fn character(id: u64, name: &str) -> CharacterListEntry {
    CharacterListEntry {
        character_id: id,
        name: name.into(),
        level: 10,
        race: 1,
        class: 2,
        appearance: CharacterAppearance::default(),
        equipment_appearance: EquipmentAppearance::default(),
    }
}

fn success(characters: Vec<CharacterListEntry>) -> LoginResponse {
    LoginResponse {
        success: true,
        token: "new-token".into(),
        characters,
        error: None,
    }
}

#[test]
fn credentials_never_send_cached_token_and_token_only_requires_both_fields_empty() {
    let mut session = Session::default();
    session.token = Some(" cached-token \n".into());
    let AuthRequest::Login(credentials) = session.auth_request("Alice", "secret", false) else {
        panic!("login")
    };
    assert!(credentials.token.is_none());
    assert_eq!(
        (credentials.username.as_str(), credentials.password.as_str()),
        ("Alice", "secret")
    );
    let AuthRequest::Login(whitespace) = session.auth_request(" ", "", false) else {
        panic!("login")
    };
    assert_eq!(whitespace.token.as_deref(), Some("cached-token"));
    let AuthRequest::Login(partial) = session.auth_request("Alice", "", false) else {
        panic!("login")
    };
    assert!(partial.token.is_none());
    let AuthRequest::Login(token) = session.auth_request("", "", false) else {
        panic!("login")
    };
    assert_eq!(token.token.as_deref(), Some("cached-token"));
    assert_eq!((token.username.as_str(), token.password.as_str()), ("", ""));
}

#[test]
fn registration_request_does_not_include_a_token() {
    let mut session = Session::default();
    session.token = Some("cached".into());
    let AuthRequest::Register(request) = session.auth_request("Alice", "secret", true) else {
        panic!("register")
    };
    assert_eq!(
        (request.username.as_str(), request.password.as_str()),
        ("Alice", "secret")
    );
}

#[test]
fn token_rules_keep_original_storage_names_and_ignore_test_sentinels() {
    assert_eq!(normalize_auth_token("  token \n"), Some("token".into()));
    for value in [
        "  ",
        " SAVED-TOKEN ",
        "11111111-1111-1111-1111-111111111111",
    ] {
        assert_eq!(normalize_auth_token(value), None);
    }
    let base = Path::new("/engine/data");
    assert_eq!(
        token_path(base, Some("EU.WorldOfOsso.com:5000")),
        base.join("auth_token")
    );
    assert_eq!(
        token_path(base, Some("127.0.0.1:5000")),
        base.join("auth_token.127.0.0.1_5000")
    );
    assert_eq!(
        token_path(base, Some("test/realm:5000")),
        base.join("auth_token.test_realm_5000")
    );
}

#[test]
fn login_success_selects_case_insensitive_name_and_emits_auto_entry() {
    let mut session = Session::default();
    let effects = session.receive_login(
        success(vec![character(7, "Elara"), character(9, "Borin")]),
        SessionOptions {
            preselected_name: Some("bOrIn"),
            auto_enter_world: true,
            startup_screen: None,
        },
    );
    assert_eq!(session.selected_index, Some(1));
    assert_eq!(session.screen, SessionScreen::Loading);
    assert_eq!(session.characters.len(), 2);
    assert_eq!(session.token.as_deref(), Some("new-token"));
    assert!(effects.iter().any(
        |effect| matches!(effect, SessionEffect::PersistToken(token) if token == "new-token")
    ));
    assert!(effects.iter().any(|effect| matches!(effect, SessionEffect::SelectCharacter(request) if request.character_id == 9)));
    assert!(
        effects
            .iter()
            .any(|effect| matches!(effect, SessionEffect::Transition(SessionScreen::Loading)))
    );
}

#[test]
fn missing_name_selects_first_but_empty_roster_cannot_auto_enter() {
    let mut session = Session::default();
    let effects = session.receive_login(
        success(vec![character(7, "Elara")]),
        SessionOptions {
            preselected_name: Some("missing"),
            auto_enter_world: false,
            startup_screen: Some(SessionScreen::CharacterCreate),
        },
    );
    assert_eq!(session.selected_index, Some(0));
    assert_eq!(session.screen, SessionScreen::CharacterCreate);
    assert!(
        !effects
            .iter()
            .any(|effect| matches!(effect, SessionEffect::SelectCharacter(_)))
    );
    let effects = session.receive_login(
        success(vec![]),
        SessionOptions {
            auto_enter_world: true,
            ..Default::default()
        },
    );
    assert_eq!(session.selected_index, None);
    assert_eq!(session.screen, SessionScreen::CharacterSelect);
    assert!(
        !effects
            .iter()
            .any(|effect| matches!(effect, SessionEffect::SelectCharacter(_)))
    );
}

#[test]
fn login_failure_preserves_roster_and_token_but_requests_reset_and_login() {
    let mut session = Session::default();
    session.receive_login(
        success(vec![character(7, "Elara")]),
        SessionOptions::default(),
    );
    let effects = session.receive_login(
        LoginResponse {
            success: false,
            token: "ignored".into(),
            characters: vec![],
            error: Some("Invalid password".into()),
        },
        SessionOptions::default(),
    );
    assert_eq!(session.token.as_deref(), Some("new-token"));
    assert_eq!(session.characters[0].name, "Elara");
    assert_eq!(
        session.feedback.as_deref(),
        Some("Incorrect username or password")
    );
    assert_eq!(session.screen, SessionScreen::Login);
    assert!(
        effects
            .iter()
            .any(|effect| matches!(effect, SessionEffect::ResetNetworkWorld))
    );
    assert!(
        effects
            .iter()
            .any(|effect| matches!(effect, SessionEffect::Transition(SessionScreen::Login)))
    );
    assert!(
        !effects
            .iter()
            .any(|effect| matches!(effect, SessionEffect::PersistToken(_)))
    );
}

#[test]
fn registration_validation_requires_both_fields() {
    for (username, password) in [
        ("", "fbtest"),
        ("  ", "fbtest"),
        ("fb_regtest1", ""),
        ("fb_regtest1", " \t"),
    ] {
        assert_eq!(
            game_engine_session::validate_registration(username, password),
            Err("Please fill in all fields")
        );
    }
    assert_eq!(
        game_engine_session::validate_registration("fb_regtest1", "fbtest"),
        Ok(())
    );
}

#[test]
fn registration_pending_approval_never_authenticates_or_persists_a_token() {
    for success in [false, true] {
        let mut session = Session::default();
        let effects = session.receive_registration(RegisterResponse {
            success,
            token: "must-not-be-saved".into(),
            pending_approval: true,
            error: None,
        });
        assert_eq!(session.screen, SessionScreen::Login);
        assert_eq!(session.token, None);
        assert_eq!(
            session.feedback.as_deref(),
            Some(
                "Registration submitted. Pending administrator approval. Return to Login after approval."
            )
        );
        assert!(
            !effects
                .iter()
                .any(|effect| matches!(effect, SessionEffect::PersistToken(_)))
        );
    }
}

#[test]
fn capturepolish_registration_feedback_severity_follows_reply_not_text() {
    let mut session = Session::default();
    for success in [false, true] {
        session.receive_registration(RegisterResponse {
            success,
            token: String::new(),
            pending_approval: true,
            error: Some("Pending administrator approval".into()),
        });
        assert!(session.feedback_is_informational);
        assert!(session.token.is_none());
    }
    session.receive_registration(RegisterResponse {
        success: false,
        token: String::new(),
        pending_approval: false,
        error: Some("Pending administrator approval".into()),
    });
    assert!(
        !session.feedback_is_informational,
        "identical text on a refusal is an error"
    );
    session.receive_registration(RegisterResponse {
        success: false,
        token: String::new(),
        pending_approval: true,
        error: None,
    });
    session.receive_login(
        LoginResponse {
            success: false,
            token: String::new(),
            characters: Vec::new(),
            error: Some("Invalid credentials".into()),
        },
        SessionOptions::default(),
    );
    assert!(
        !session.feedback_is_informational,
        "login failure clears pending severity"
    );
}

#[test]
fn registration_success_clears_roster_and_failure_surfaces_raw_error() {
    let mut session = Session::default();
    session.receive_login(
        success(vec![character(7, "Elara")]),
        SessionOptions::default(),
    );
    let effects = session.receive_registration(RegisterResponse {
        success: true,
        token: "registered".into(),
        pending_approval: false,
        error: None,
    });
    assert_eq!(session.token.as_deref(), Some("registered"));
    assert!(session.characters.is_empty());
    assert_eq!(session.screen, SessionScreen::CharacterSelect);
    assert!(effects.iter().any(
        |effect| matches!(effect, SessionEffect::PersistToken(token) if token == "registered")
    ));
    let effects = session.receive_registration(RegisterResponse {
        success: false,
        token: "ignored".into(),
        pending_approval: false,
        error: Some("pending admin approval".into()),
    });
    assert_eq!(session.feedback.as_deref(), Some("pending admin approval"));
    assert_eq!(session.token.as_deref(), Some("registered"));
    assert_eq!(session.screen, SessionScreen::Login);
    assert!(
        !effects
            .iter()
            .any(|effect| matches!(effect, SessionEffect::PersistToken(_)))
    );
}

#[test]
fn registration_refusal_without_error_has_visible_feedback() {
    let mut session = Session::default();
    session.receive_registration(RegisterResponse {
        success: false,
        token: String::new(),
        pending_approval: false,
        error: None,
    });
    assert_eq!(session.screen, SessionScreen::Login);
    assert_eq!(session.feedback.as_deref(), Some("Registration failed."));
    assert!(session.token.is_none());
}

#[test]
fn registration_pending_then_approved_password_login_reaches_character_select() {
    let mut session = Session::default();
    session.receive_registration(RegisterResponse {
        success: false,
        token: String::new(),
        pending_approval: true,
        error: None,
    });
    let AuthRequest::Login(request) = session.auth_request("fb_regtest1", "fbtest", false) else {
        panic!("password login")
    };
    assert_eq!(request.token, None);
    assert_eq!(
        (request.username.as_str(), request.password.as_str()),
        ("fb_regtest1", "fbtest")
    );
    session.receive_login(success(Vec::new()), SessionOptions::default());
    assert_eq!(session.screen, SessionScreen::CharacterSelect);
    assert_eq!(session.feedback, None);
    assert!(session.token.is_some());
}

#[test]
fn roster_responses_and_manual_selection_follow_original_index_behavior() {
    let mut session = Session::default();
    session.receive_login(
        success(vec![character(7, "Elara")]),
        SessionOptions::default(),
    );
    assert_eq!(session.select_character().unwrap().character_id, 7);
    session.receive_character_created(CreateCharacterResponse {
        success: true,
        character: Some(character(9, "Borin")),
        error: None,
    });
    session.receive_character_update(CharacterListUpdate {
        character: character(9, "Borin II"),
    });
    assert_eq!(session.characters.len(), 2);
    assert_eq!(session.characters[1].name, "Borin II");
    session.receive_character_deleted(DeleteCharacterResponse {
        success: false,
        character_id: 7,
        error: Some("denied".into()),
    });
    assert_eq!(session.characters.len(), 2);
    session.selected_index = Some(1);
    session.receive_character_deleted(DeleteCharacterResponse {
        success: true,
        character_id: 7,
        error: None,
    });
    assert_eq!(session.characters.len(), 1);
    assert!(
        session.select_character().is_none(),
        "stale index cannot silently select another character"
    );
    session.selected_index = Some(0);
    assert_eq!(session.select_character().unwrap().character_id, 9);
}

#[test]
fn entry_reply_records_selected_roster_identity_and_respects_failure() {
    let mut session = Session::default();
    session.receive_login(
        success(vec![character(7, "Elara")]),
        SessionOptions::default(),
    );
    let effects = session.receive_enter_world(EnterWorldResponse {
        success: true,
        player_entity: Some(3),
        error: None,
    });
    assert_eq!(session.selected_character_id, Some(7));
    assert_eq!(session.selected_character_name.as_deref(), Some("Elara"));
    assert!(
        effects
            .iter()
            .any(|effect| matches!(effect, SessionEffect::Transition(SessionScreen::Loading)))
    );
    let effects = session.receive_enter_world(EnterWorldResponse {
        success: false,
        player_entity: None,
        error: Some("denied".into()),
    });
    assert_eq!(session.screen, SessionScreen::CharacterSelect);
    assert_eq!(session.selected_character_id, Some(7));
    assert!(effects.iter().any(|effect| matches!(
        effect,
        SessionEffect::Transition(SessionScreen::CharacterSelect)
    )));
}

#[test]
fn forced_notice_disconnects_first_then_surfaces_message_and_resets_world() {
    let mut session = Session::default();
    session.screen = SessionScreen::Loading;
    let effects = session.receive_forced_disconnect(ForcedDisconnect {
        message: "Kicked".into(),
        reconnect_allowed: false,
    });
    assert!(
        effects
            .iter()
            .any(|effect| matches!(effect, SessionEffect::RequestDisconnect))
    );
    assert_eq!(
        session.feedback, None,
        "notice is consumed by disconnect lifecycle"
    );
    let effects = session.receive_disconnected_with_reason(None);
    assert_eq!(session.feedback.as_deref(), Some("Kicked"));
    assert_eq!(session.screen, SessionScreen::Login);
    assert!(
        effects
            .iter()
            .any(|effect| matches!(effect, SessionEffect::ResetNetworkWorld))
    );
    assert!(
        effects
            .iter()
            .any(|effect| matches!(effect, SessionEffect::Transition(SessionScreen::Login)))
    );
}

#[test]
fn protocol_rejection_while_connecting_from_login_shows_reason() {
    let mut session = Session::default();
    assert_eq!(session.screen, SessionScreen::Login);
    let reason = "No protocol fingerprint from the peer within 10 s: client and server builds are incompatible.";
    session.receive_protocol_rejected(reason.into());
    let effects = session.receive_disconnected_with_reason(Some("Client trigger"));
    assert_eq!(session.feedback.as_deref(), Some(reason));
    assert!(
        effects
            .iter()
            .any(|effect| matches!(effect, SessionEffect::ShowFeedback))
    );
    assert!(
        !effects
            .iter()
            .any(|effect| matches!(effect, SessionEffect::Transition(_))),
        "the login screen stays up"
    );
}

#[test]
fn every_connection_loss_on_login_shows_feedback_even_when_unchanged() {
    let mut session = Session::default();
    for _ in 0..2 {
        let effects = session.receive_disconnected_with_reason(Some("timed out"));
        assert_eq!(session.feedback.as_deref(), Some("Connection lost."));
        assert!(matches!(effects.as_slice(), [SessionEffect::ShowFeedback]));
    }
}

#[test]
fn protocol_rejection_during_world_reconnect_returns_to_login_with_reason() {
    let mut session = Session::default();
    session.screen = SessionScreen::InWorld;
    session.token = Some("real-token".into());
    session.receive_disconnected_with_reason(Some("lost"));
    assert_eq!(session.reconnect_phase, ReconnectPhase::PendingConnect);
    let reason = "Client and server protocols differ (component registry). Rebuild both from the same shared-protocol revision.";
    session.receive_protocol_rejected(reason.into());
    let effects = session.receive_disconnected_with_reason(Some("Client trigger"));
    assert_eq!(session.feedback.as_deref(), Some(reason));
    assert_eq!(session.screen, SessionScreen::Login);
    assert_eq!(session.reconnect_phase, ReconnectPhase::Inactive);
    assert!(
        effects
            .iter()
            .any(|effect| matches!(effect, SessionEffect::Transition(SessionScreen::Login)))
    );
}

#[test]
fn charselect_disconnect_with_token_resets_and_reconnects_without_auto_entry() {
    let mut session = Session::default();
    session.screen = SessionScreen::CharacterSelect;
    session.token = Some(" saved-token-real ".into());
    session.selected_character_name = Some("Elara".into());
    let effects = session.receive_disconnected_with_reason(Some("lost"));
    assert_eq!(session.reconnect_phase, ReconnectPhase::PendingConnect);
    assert!(!session.gameplay_input_allowed());
    assert_eq!(session.screen, SessionScreen::CharacterSelect);
    assert!(
        effects
            .iter()
            .any(|effect| matches!(effect, SessionEffect::ResetNetworkWorld))
    );
    assert!(
        !effects
            .iter()
            .any(|effect| matches!(effect, SessionEffect::Transition(_)))
    );
    session.receive_connected();
    assert_eq!(session.reconnect_phase, ReconnectPhase::AwaitingWorld);
    let effects = session.receive_login(
        success(vec![character(9, "Borin"), character(7, "Elara")]),
        SessionOptions::default(),
    );
    assert_eq!(session.selected_index, Some(0));
    assert_eq!(session.screen, SessionScreen::CharacterSelect);
    assert!(
        !effects
            .iter()
            .any(|effect| matches!(effect, SessionEffect::SelectCharacter(_)))
    );
    assert_eq!(session.reconnect_phase, ReconnectPhase::Inactive);
    assert!(session.gameplay_input_allowed());
}

#[test]
fn world_disconnect_preserves_name_auto_entry_and_inworld_screen() {
    for screen in [SessionScreen::InWorld, SessionScreen::Loading] {
        let mut session = Session::default();
        session.screen = screen;
        session.token = Some("real-token".into());
        session.selected_character_name = Some("Elara".into());
        let effects = session.receive_disconnected_with_reason(Some("lost"));
        assert_eq!(session.reconnect_phase, ReconnectPhase::PendingConnect);
        assert_eq!(session.screen, SessionScreen::InWorld);
        assert_eq!(session.selected_character_name.as_deref(), Some("Elara"));
        assert!(
            effects
                .iter()
                .any(|effect| matches!(effect, SessionEffect::ResetNetworkWorld))
        );
        assert!(
            effects
                .iter()
                .any(|effect| matches!(effect, SessionEffect::Transition(SessionScreen::InWorld)))
        );
        session.receive_connected();
        let effects = session.receive_login(
            success(vec![character(9, "Borin"), character(7, "Elara")]),
            SessionOptions::default(),
        );
        assert_eq!(session.selected_index, Some(1));
        assert!(effects.iter().any(|effect| matches!(effect, SessionEffect::SelectCharacter(request) if request.character_id == 7)));
        assert_eq!(session.reconnect_phase, ReconnectPhase::AwaitingWorld);
    }
}

#[test]
fn initial_marker_only_ignored_while_pending_without_reason_or_forced_notice() {
    let mut session = Session::default();
    session.screen = SessionScreen::InWorld;
    session.token = Some("real-token".into());
    session.receive_disconnected_with_reason(Some("lost"));
    session.feedback = Some("unchanged".into());
    assert!(session.receive_disconnected_with_reason(None).is_empty());
    assert_eq!(session.feedback.as_deref(), Some("unchanged"));
    assert_eq!(session.reconnect_phase, ReconnectPhase::PendingConnect);
    assert!(
        !session
            .receive_disconnected_with_reason(Some("failed"))
            .is_empty()
    );
    session.receive_forced_disconnect(ForcedDisconnect {
        message: "Kicked".into(),
        reconnect_allowed: false,
    });
    let effects = session.receive_disconnected_with_reason(None);
    assert_eq!(session.feedback.as_deref(), Some("Kicked"));
    assert_eq!(session.reconnect_phase, ReconnectPhase::Inactive);
    assert!(
        effects
            .iter()
            .any(|effect| matches!(effect, SessionEffect::ResetNetworkWorld))
    );
}

#[test]
fn world_reconnect_finishes_only_after_terrain_refresh_and_local_player() {
    let mut session = Session::default();
    session.screen = SessionScreen::InWorld;
    session.token = Some("real-token".into());
    session.receive_disconnected_with_reason(Some("lost"));
    session.receive_terrain_refresh();
    assert!(session.terrain_refresh_seen);
    assert!(!session.finish_reconnect(true));
    session.receive_connected();
    assert_eq!(session.reconnect_phase, ReconnectPhase::AwaitingWorld);
    assert!(!session.finish_reconnect(false));
    assert!(!session.gameplay_input_allowed());
    assert!(session.finish_reconnect(true));
    assert_eq!(session.reconnect_phase, ReconnectPhase::Inactive);
    assert!(!session.terrain_refresh_seen);
    assert!(session.gameplay_input_allowed());
}

#[test]
fn no_token_and_failed_auth_or_entry_clear_reconnect() {
    let mut session = Session::default();
    session.screen = SessionScreen::CharacterSelect;
    assert!(matches!(
        session
            .receive_disconnected_with_reason(Some("lost"))
            .as_slice(),
        [SessionEffect::ShowFeedback]
    ));
    assert_eq!(
        session.feedback.as_deref(),
        Some("Connection lost. Char select is now offline.")
    );
    assert_eq!(session.reconnect_phase, ReconnectPhase::Inactive);
    session.screen = SessionScreen::InWorld;
    let effects = session.receive_disconnected_with_reason(Some("lost"));
    assert_eq!(session.screen, SessionScreen::Login);
    assert!(
        effects
            .iter()
            .any(|effect| matches!(effect, SessionEffect::Transition(SessionScreen::Login)))
    );
    session.token = Some("real-token".into());
    session.screen = SessionScreen::InWorld;
    session.receive_disconnected_with_reason(Some("lost"));
    session.receive_login(
        LoginResponse {
            success: false,
            token: "".into(),
            characters: vec![],
            error: Some("denied".into()),
        },
        SessionOptions::default(),
    );
    assert_eq!(session.reconnect_phase, ReconnectPhase::Inactive);
    session.screen = SessionScreen::InWorld;
    session.receive_disconnected_with_reason(Some("lost"));
    session.receive_enter_world(EnterWorldResponse {
        success: false,
        player_entity: None,
        error: Some("denied".into()),
    });
    assert_eq!(session.reconnect_phase, ReconnectPhase::Inactive);
}

#[test]
fn a_connect_failure_on_login_shows_its_reason() {
    let mut session = Session::default();
    let reason = "Failed to connect: the server did not answer within 5 seconds.";
    let effects = session.receive_connect_failed(reason);
    assert_eq!(session.feedback.as_deref(), Some(reason));
    assert!(matches!(effects.as_slice(), [SessionEffect::ShowFeedback]));
}

#[test]
fn a_connect_failure_during_world_reconnect_keeps_reconnecting() {
    let mut session = Session::default();
    session.screen = SessionScreen::InWorld;
    session.token = Some("real-token".into());
    session.receive_disconnected_with_reason(Some("lost"));
    assert_eq!(session.reconnect_phase, ReconnectPhase::PendingConnect);
    session
        .receive_connect_failed("Failed to connect: the server did not answer within 5 seconds.");
    assert_eq!(session.reconnect_phase, ReconnectPhase::PendingConnect);
    assert_eq!(session.screen, SessionScreen::InWorld);
    assert_eq!(session.feedback, None);
}
