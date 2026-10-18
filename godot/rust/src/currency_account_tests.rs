//! Owner-only CurrencyStateUpdate must reach native account/window state.
use super::*;
use shared::protocol::{CurrencyEntrySnapshot, CurrencySnapshot, CurrencyStateUpdate};

#[test]
fn native_currency_receipt_is_not_an_unhandled_account_message() {
    let mut account = Account::new(PathBuf::new());
    account.session.screen = SessionScreen::InWorld;
    let update = CurrencyStateUpdate {
        snapshot: Some(CurrencySnapshot {
            entries: vec![CurrencyEntrySnapshot {
                id: 1792,
                name: "Honor".into(),
                amount: 0,
            }],
        }),
        message: None,
        error: None,
    };
    let mut events = Vec::new();
    account
        .dispatch_message(ProtocolMessage::for_tests(update.clone()), &mut events)
        .expect("authoritative discovered Honor0 must be handled");
    assert_eq!(account.currencies, update.snapshot);
    assert!(matches!(&events[..], [AccountEvent::Currency(received)] if received == &update));
    let refused = CurrencyStateUpdate {
        snapshot: None,
        message: None,
        error: Some("Server-owned currency award required".into()),
    };
    account
        .dispatch_message(ProtocolMessage::for_tests(refused.clone()), &mut events)
        .unwrap();
    assert_eq!(
        account.currencies, update.snapshot,
        "refusal is not an empty wallet"
    );
    assert!(
        matches!(events.last(), Some(AccountEvent::Currency(received)) if received == &refused)
    );
    account
        .dispatch_message(
            ProtocolMessage::for_tests(CurrencyStateUpdate {
                snapshot: Some(CurrencySnapshot { entries: vec![] }),
                message: None,
                error: None,
            }),
            &mut events,
        )
        .unwrap();
    assert_eq!(
        account.currencies,
        Some(CurrencySnapshot { entries: vec![] })
    );
    account
        .apply_effects(vec![SessionEffect::ResetNetworkWorld], &mut events)
        .unwrap();
    assert!(
        account.currencies.is_none(),
        "another connection cannot inherit a cached wallet"
    );
}
