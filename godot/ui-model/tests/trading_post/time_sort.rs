use super::*;
use game_engine_ui_model::trading_post::{FilterSubmenu, Sort};
use ui_toolkit::widgets::button::ButtonState;

fn time_view(remaining: i64) -> View {
    let mut view = purchase_price_view();
    let mut data = (**view.snapshot.as_ref().unwrap()).clone();
    data.month_end_unix = data.now_unix + remaining;
    view.receive(Arc::new(data));
    view
}

fn row_ids(view: &View) -> Vec<u32> {
    view.rows().iter().map(|row| row.id).collect()
}

fn assert_button_enabled(registry: &FrameRegistry, name: &str, enabled: bool) {
    let frame = registry.get(registry.get_by_name(name).unwrap()).unwrap();
    let Some(WidgetData::Button(button)) = &frame.widget_data else {
        panic!("{name} is not a button");
    };
    assert_eq!(button.state != ButtonState::Disabled, enabled, "{name}");
}

#[test]
fn trading_post_time_sort_header_and_menu_enable_from_monthly_clock() {
    for skin in [ActiveSkin::Modern, ActiveSkin::Forever] {
        let mut view = time_view(3600);
        view.dropdown = true;
        view.submenu = Some(FilterSubmenu::Sort);
        let registry = render(skin, view.clone());
        assert_button_enabled(&registry, "PerksProgramTimeSortButton", true);
        assert_button_enabled(&registry, "PerksProgramFilterSortTime", true);
        assert_eq!(view.click("perks:sort:time").unwrap(), None);
        assert_eq!(view.sort, Sort::Time);
        assert!(view.ascending);
        let sorted = render(skin, view);
        assert!(sorted.get_by_name("PerksProgramTimeSortArrow").is_some());
    }
}

#[test]
fn trading_post_time_sort_rounds_owned_sentinels_before_name_ties() {
    for (remaining, ascending, descending) in [
        (
            20,
            vec![16, 14, 15, 13, 12, 11, 10],
            vec![16, 11, 10, 13, 12, 14, 15],
        ),
        (
            21,
            vec![16, 14, 15, 11, 13, 10, 12],
            vec![16, 11, 13, 10, 12, 14, 15],
        ),
        (
            86420,
            vec![16, 14, 15, 13, 12, 11, 10],
            vec![16, 11, 10, 13, 12, 14, 15],
        ),
    ] {
        let mut view = time_view(remaining);
        let authority = view.snapshot.clone();
        assert_eq!(view.click("perks:sort:time").unwrap(), None);
        assert!(view.ascending);
        assert_eq!(row_ids(&view), ascending);
        assert_eq!(view.click("perks:sort:time").unwrap(), None);
        assert!(!view.ascending);
        assert_eq!(row_ids(&view), descending);
        assert_eq!(view.selected, Some(4));
        assert_eq!(view.cart, [14]);
        assert_eq!(view.snapshot, authority);
        assert_eq!(view.take_selection_sounds(), 0);
    }
}

#[test]
fn trading_post_time_sort_refund_expiry_uses_each_row_not_selection() {
    // Remaining20 mod60 keeps purchased and refundable in separate displayed minutes.
    let mut view = time_view(80);
    view.sort = Sort::Time;
    view.ascending = true;
    for (elapsed, expected) in [
        (59, vec![16, 14, 15, 11, 13, 10, 12]),
        (60, vec![16, 14, 15, 11, 13, 10, 12]),
        (61, vec![16, 14, 15, 11, 13, 10, 12]),
    ] {
        view.refund_elapsed = elapsed;
        assert_eq!(row_ids(&view), expected);
    }
    // At elapsed59, remaining20 makes ownership buckets distinguishable.
    let mut view = time_view(79);
    view.sort = Sort::Time;
    view.ascending = false;
    view.refund_elapsed = 59;
    assert_eq!(row_ids(&view), [16, 11, 10, 13, 12, 14, 15]);
    view.refund_elapsed = 60;
    assert_eq!(row_ids(&view), [16, 11, 13, 10, 12, 14, 15]);
}

#[test]
fn trading_post_time_sort_disables_without_live_ordinary_rows() {
    for skin in [ActiveSkin::Modern, ActiveSkin::Forever] {
        for remaining in [0, -1, 60] {
            let mut view = time_view(remaining);
            if remaining == 60 {
                let mut data = (**view.snapshot.as_ref().unwrap()).clone();
                data.items.retain(|row| row.id == 4);
                view.receive(Arc::new(data));
            }
            view.dropdown = true;
            view.submenu = Some(FilterSubmenu::Sort);
            let registry = render(skin, view.clone());
            assert_button_enabled(&registry, "PerksProgramTimeSortButton", false);
            assert_button_enabled(&registry, "PerksProgramFilterSortTime", false);
            let before = view.clone();
            assert_eq!(view.click("perks:sort:time").unwrap(), None);
            assert_eq!(view, before);
        }
    }
}

#[test]
fn trading_post_time_sort_provider_rebuild_resets_price_without_cart_transfer() {
    let mut view = time_view(3600);
    view.click("perks:sort:time").unwrap();
    view.click("perks:cart:show").unwrap();
    let authority = view.snapshot.clone().unwrap();
    let cart = view.cart.clone();
    view.receive(authority.clone());
    assert_eq!(view.sort, Sort::Price);
    assert!(!view.ascending);
    assert!(view.cart_visible);
    assert_eq!(view.selected, None);
    assert_eq!(view.cart, cart);
    assert_eq!(view.snapshot, Some(authority));
    view.click("perks:cart:close").unwrap();
    view.click("perks:sort:time").unwrap();
    view.click("perks:filter:collected").unwrap();
    assert_eq!(view.sort, Sort::Price);
    assert!(!view.ascending);
    assert_eq!(row_ids(&view), [15, 14]);
}

#[test]
fn trading_post_time_sort_full_month_replacement_removes_expired_rows() {
    let mut view = time_view(1);
    view.click("perks:sort:time").unwrap();
    assert_eq!(
        view.advance_server_clock(1),
        Some(TradingPostRequest::Query)
    );
    let mut next = (**view.snapshot.as_ref().unwrap()).clone();
    next.now_unix = next.month_end_unix;
    next.month_end_unix += 60;
    next.items.retain(|row| row.id == 4);
    view.receive(Arc::new(next));
    assert!(view.rows().is_empty());
    assert_eq!(view.sort, Sort::Price);
    assert!(!view.ascending);
    assert_eq!(view.selected, Some(4));
    assert_eq!(view.snapshot.as_ref().unwrap().frozen_item_id, Some(4));
}
