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
        let button = registry
            .get(
                registry
                    .get_by_name("PerksProgramSlowPurchaseButton")
                    .unwrap(),
            )
            .unwrap();
        let Some(WidgetData::Button(button)) = &button.widget_data else {
            panic!("Return button");
        };
        assert_eq!(
            button.state,
            ui_toolkit::widgets::button::ButtonState::Normal
        );
        assert!(button.enabled);
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

// RED used the existing snapshot clock. GREEN uses the approved independent request clock.
fn assert_request_clock_reaches_slow_notice(checkout: bool) {
    let mut view = accept_purchase(checkout);
    view.update_purchase_timeout(9.999);
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
    view.update_purchase_timeout(10.0);
    assert_slow_notice(&view);
}

#[test]
fn trading_post_slow_purchase_replaces_single_confirmation_at_ten_seconds() {
    assert_request_clock_reaches_slow_notice(false);
}

#[test]
fn trading_post_slow_purchase_replaces_checkout_confirmation_at_ten_seconds() {
    assert_request_clock_reaches_slow_notice(true);
}

fn assert_no_purchase_popup(view: &View) {
    for skin in [ActiveSkin::Modern, ActiveSkin::Forever] {
        let registry = render(skin, view.clone());
        assert!(
            registry
                .get_by_name("PerksProgramSlowPurchaseDialog")
                .is_none()
        );
        assert!(registry.get_by_name("PerksProgramConfirmDialog").is_none());
    }
}

fn purchase_reply(view: &View, checkout: bool) -> TradingPostSnapshot {
    let mut reply = (**view.snapshot.as_ref().unwrap()).clone();
    reply.reply_kind = if checkout {
        TradingPostReplyKind::Checkout
    } else {
        TradingPostReplyKind::Purchase
    };
    reply.tender = 400;
    reply.items[0].purchased = true;
    reply
}

#[test]
fn trading_post_slow_purchase_matching_success_cancels_before_deadline() {
    for checkout in [false, true] {
        let mut view = accept_purchase(checkout);
        view.update_purchase_timeout(9.0);
        view.receive(Arc::new(purchase_reply(&view, checkout)));
        view.update_purchase_timeout(10.0);
        assert!(!view.pending);
        assert_no_purchase_popup(&view);
        assert_eq!(view.snapshot.as_ref().unwrap().tender, 400);
    }
}

#[test]
fn trading_post_slow_purchase_query_does_not_restart_request_deadline() {
    for checkout in [false, true] {
        let mut view = accept_purchase(checkout);
        view.update_purchase_timeout(9.0);
        let mut query = (**view.snapshot.as_ref().unwrap()).clone();
        query.now_unix += 9;
        view.receive(Arc::new(query));
        assert_eq!(view.advance_server_clock(0), None);
        view.update_purchase_timeout(9.999);
        assert!(!view.slow_purchase);
        view.update_purchase_timeout(10.0);
        assert_slow_notice(&view);
    }
}

#[test]
fn trading_post_slow_purchase_coalesced_success_and_refusal_clear_notice() {
    use game_engine_ui_model::trading_post::Reply;
    for checkout in [false, true] {
        for error in [None, Some("Purchase refused".to_string())] {
            let mut view = accept_purchase(checkout);
            view.update_purchase_timeout(10.0);
            let mut final_query = purchase_reply(&view, checkout);
            let kind = final_query.reply_kind;
            final_query.reply_kind = TradingPostReplyKind::Query;
            if error.is_some() {
                final_query.tender = 500;
                final_query.items[0].purchased = false;
            }
            view.receive_replies(
                Arc::new(final_query),
                &[
                    Reply {
                        kind,
                        error: error.clone(),
                    },
                    Reply {
                        kind: TradingPostReplyKind::Query,
                        error: None,
                    },
                ],
            );
            view.update_purchase_timeout(20.0);
            assert_no_purchase_popup(&view);
            assert!(!view.pending);
            assert_eq!(view.error, error.unwrap_or_default());
        }
    }
}

#[test]
fn trading_post_slow_purchase_return_dismisses_notice_not_vendor_or_authority() {
    for checkout in [false, true] {
        let mut view = accept_purchase(checkout);
        let authority = view.snapshot.clone();
        view.update_purchase_timeout(10.0);
        assert_slow_notice(&view);
        assert_eq!(view.click("perks:cancel").unwrap(), None);
        assert!(view.visible);
        assert!(!view.pending);
        assert_eq!(view.snapshot, authority);
        view.update_purchase_timeout(30.0);
        assert_no_purchase_popup(&view);
        // A late actual reply still updates authority after returning to the vendor.
        view.receive(Arc::new(purchase_reply(&view, checkout)));
        assert_no_purchase_popup(&view);
        assert_eq!(view.snapshot.as_ref().unwrap().tender, 400);
    }
}

#[test]
fn trading_post_slow_purchase_escape_dismissal_keeps_vendor_visible() {
    let mut view = accept_purchase(false);
    assert!(
        !view.dismiss_slow_purchase(),
        "Ordinary pending confirmations still own Escape"
    );
    view.update_purchase_timeout(10.0);
    assert!(
        view.dismiss_slow_purchase(),
        "Native Escape delegates to this dismissal"
    );
    assert!(view.visible);
    assert!(!view.pending);
    assert!(!view.dismiss_slow_purchase());
    assert_no_purchase_popup(&view);
}

#[test]
fn trading_post_slow_purchase_vendor_hide_cancels_future_notice() {
    for checkout in [false, true] {
        let mut view = accept_purchase(checkout);
        view.update_purchase_timeout(9.0);
        // Native close hides the view and cancels its confirmation before the next clock tick.
        view.visible = false;
        view.cancel_confirmation();
        view.update_purchase_timeout(20.0);
        view.visible = true;
        view.update_purchase_timeout(30.0);
        assert_no_purchase_popup(&view);
    }
}

#[test]
fn trading_post_slow_purchase_clock_does_not_apply_to_refund() {
    let mut data = snapshot();
    data.items[0].purchased = true;
    data.items[0].refund_seconds = 3600;
    let mut view = View {
        visible: true,
        selected: Some(4),
        snapshot: Some(Arc::new(data)),
        ..Default::default()
    };
    view.click("perks:refund").unwrap();
    assert_eq!(
        view.click("perks:confirm").unwrap(),
        Some(Action::Request(TradingPostRequest::Refund {
            vendor_item_id: 4
        }))
    );
    view.update_purchase_timeout(45.0);
    assert!(view.pending);
    for skin in [ActiveSkin::Modern, ActiveSkin::Forever] {
        let registry = render(skin, view.clone());
        assert!(registry.get_by_name("PerksProgramConfirmSpinner").is_some());
        assert!(
            registry
                .get_by_name("PerksProgramSlowPurchaseDialog")
                .is_none()
        );
    }
}
