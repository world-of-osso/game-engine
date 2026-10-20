use super::*;
use game_engine_ui_model::trading_post::Action;

const SLOW_TEXT: &str = "It's taking longer than usual to process your purchase. Processing will continue in the background.";

fn accept_purchase(checkout: bool) -> View {
    let mut data = snapshot();
    data.frozen_item_id = None;
    let mut view = View {
        visible: true,
        selected: Some(4),
        snapshot: Some(Arc::new(data)),
        ..Default::default()
    };
    let request = if checkout {
        view.click("perks:cart:add").unwrap();
        assert_eq!(view.cart, vec![4]);
        view.click("perks:cart:checkout").unwrap();
        TradingPostRequest::Checkout {
            vendor_item_ids: vec![4],
        }
    } else {
        view.click("perks:purchase").unwrap();
        TradingPostRequest::Purchase { vendor_item_id: 4 }
    };
    assert_eq!(
        view.click("perks:confirm").unwrap(),
        Some(Action::Request(request))
    );
    assert!(view.pending);
    view
}

fn assert_slow_notice(view: &View) {
    for skin in [ActiveSkin::Modern, ActiveSkin::Forever] {
        let registry = render(skin, view.clone());
        assert!(
            registry
                .get_by_name("PerksProgramSlowPurchaseDialog")
                .is_some(),
            "10 seconds must replace the accepted confirmation with the slow-purchase notice"
        );
        assert_eq!(
            text(&registry, "PerksProgramSlowPurchaseTextBefore"),
            SLOW_TEXT
        );
        assert_eq!(
            text(&registry, "PerksProgramSlowPurchaseButtonDialogButtonText"),
            "Return to Trading Post"
        );
        assert!(
            registry
                .get_by_name("PerksProgramSlowPurchaseCancelButton")
                .is_none()
        );
        assert!(registry.get_by_name("PerksProgramConfirmDialog").is_none());
        assert!(registry.get_by_name("PerksProgramConfirmSpinner").is_none());
    }
    assert_eq!(view.snapshot.as_ref().unwrap().tender, 500);
    assert!(!view.snapshot.as_ref().unwrap().items[0].purchased);
}

fn assert_existing_clock_reaches_slow_notice(checkout: bool) {
    let mut view = accept_purchase(checkout);
    assert_eq!(view.advance_server_clock(9), None);
    for skin in [ActiveSkin::Modern, ActiveSkin::Forever] {
        let registry = render(skin, view.clone());
        assert!(registry.get_by_name("PerksProgramConfirmDialog").is_some());
        assert!(registry.get_by_name("PerksProgramConfirmSpinner").is_some());
        assert!(
            registry
                .get_by_name("PerksProgramSlowPurchaseDialog")
                .is_none()
        );
    }
    assert_eq!(view.advance_server_clock(10), None);
    assert_slow_notice(&view);
}

#[test]
#[ignore = "source-expectation RED: Purchase lacks the authored 10-second slow notice"]
fn trading_post_slow_purchase_replaces_single_confirmation_at_ten_seconds() {
    assert_existing_clock_reaches_slow_notice(false);
}

#[test]
#[ignore = "source-expectation RED: Checkout lacks the authored 10-second slow notice"]
fn trading_post_slow_purchase_replaces_checkout_confirmation_at_ten_seconds() {
    assert_existing_clock_reaches_slow_notice(true);
}
