use super::*;

#[test]
fn filter_default_values() {
    let filter = AuctionSearchFilter::default();
    assert!(filter.text.is_empty());
    assert!(filter.min_level.is_none());
    assert!(filter.quality.is_none());
    assert!(!filter.usable_only);
    assert_eq!(filter.sort, AuctionSortField::Name);
    assert!(!filter.sort_ascending);
    assert_eq!(filter.page, 0);
}

#[test]
fn filter_reset_clears_text_and_levels() {
    let mut filter = AuctionSearchFilter {
        text: "Sword".into(),
        min_level: Some(10),
        max_level: Some(60),
        quality: Some(3),
        usable_only: true,
        sort: AuctionSortField::Buyout,
        sort_ascending: true,
        page: 5,
    };

    filter.reset();

    assert!(filter.text.is_empty());
    assert!(filter.min_level.is_none());
    assert!(filter.quality.is_none());
    assert!(!filter.usable_only);
    assert_eq!(filter.page, 0);
    assert_eq!(filter.sort, AuctionSortField::Buyout);
    assert!(filter.sort_ascending);
}

#[test]
fn filter_set_sort_toggles_direction() {
    let mut filter = AuctionSearchFilter::default();
    filter.set_sort(AuctionSortField::Buyout);
    assert_eq!(filter.sort, AuctionSortField::Buyout);
    assert!(filter.sort_ascending);

    filter.set_sort(AuctionSortField::Buyout);
    assert!(!filter.sort_ascending);
}

#[test]
fn filter_set_sort_new_field_resets_direction() {
    let mut filter = AuctionSearchFilter {
        sort: AuctionSortField::Buyout,
        sort_ascending: false,
        page: 3,
        ..Default::default()
    };

    filter.set_sort(AuctionSortField::Level);

    assert_eq!(filter.sort, AuctionSortField::Level);
    assert!(filter.sort_ascending);
    assert_eq!(filter.page, 0);
}

#[test]
fn state_starts_closed() {
    let state = AuctionHouseState::default();
    assert!(!state.is_open);
}

#[test]
fn state_open_and_close() {
    let mut state = AuctionHouseState::default();
    state.open();
    assert!(state.is_open);
    assert!(state.search_results.is_empty());

    state.close();
    assert!(!state.is_open);
}

#[test]
fn state_update_results() {
    let mut state = AuctionHouseState::default();
    state.open();
    state.update_results(vec![sample_listing()], 42);
    assert_eq!(state.search_results.len(), 1);
    assert_eq!(state.total_results, 42);
}

#[test]
fn state_update_listings() {
    let mut state = AuctionHouseState::default();
    let listings = vec![MyAuctionListing {
        item_name: "Axe".into(),
        time_left: AuctionDuration::Short,
        bid_amount: Money(100),
        buyout_amount: Money(200),
        sold: false,
    }];
    state.update_listings(listings);
    assert_eq!(state.my_listings.len(), 1);
}

#[test]
fn state_page_count() {
    let state = AuctionHouseState {
        total_results: 25,
        ..Default::default()
    };
    assert_eq!(state.page_count(10), 3);
    assert_eq!(state.page_count(25), 1);
    assert_eq!(state.page_count(0), 1);
}

#[test]
fn close_clears_results() {
    let mut state = AuctionHouseState::default();
    state.open();
    state.update_results(vec![sample_listing()], 1);
    state.close();
    assert!(state.search_results.is_empty());
    assert_eq!(state.total_results, 0);
}
