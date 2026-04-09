use super::*;

#[test]
fn intent_search() {
    let mut queue = AuctionIntentQueue::default();
    let filter = AuctionSearchFilter {
        text: "Sword".into(),
        ..Default::default()
    };
    queue.search(filter.clone());
    let drained = queue.drain();
    assert_eq!(drained.len(), 1);
    assert_eq!(drained[0], AuctionIntent::Search { filter });
}

#[test]
fn intent_query_owned() {
    let mut queue = AuctionIntentQueue::default();
    queue.query_owned();
    let drained = queue.drain();
    assert_eq!(drained[0], AuctionIntent::QueryOwned);
}

#[test]
fn intent_query_bids() {
    let mut queue = AuctionIntentQueue::default();
    queue.query_bids();
    let drained = queue.drain();
    assert_eq!(drained[0], AuctionIntent::QueryBids);
}

#[test]
fn intent_open_close() {
    let mut queue = AuctionIntentQueue::default();
    queue.open();
    queue.close();
    let drained = queue.drain();
    assert_eq!(drained.len(), 2);
    assert_eq!(drained[0], AuctionIntent::Open);
    assert_eq!(drained[1], AuctionIntent::Close);
}

#[test]
fn intent_post() {
    let mut queue = AuctionIntentQueue::default();
    queue.post(0, 5, 20, Money(1000), Money(5000), AuctionDuration::Long);
    let drained = queue.drain();
    assert_eq!(drained.len(), 1);
    assert_eq!(
        drained[0],
        AuctionIntent::Post {
            item_bag: 0,
            item_slot: 5,
            stack_count: 20,
            min_bid: Money(1000),
            buyout: Money(5000),
            duration: AuctionDuration::Long,
        }
    );
}

#[test]
fn intent_bid() {
    let mut queue = AuctionIntentQueue::default();
    queue.bid(42, Money(5000));
    let drained = queue.drain();
    assert_eq!(
        drained[0],
        AuctionIntent::Bid {
            auction_id: 42,
            amount: Money(5000)
        }
    );
}

#[test]
fn intent_buyout() {
    let mut queue = AuctionIntentQueue::default();
    queue.buyout(99);
    let drained = queue.drain();
    assert_eq!(drained[0], AuctionIntent::Buyout { auction_id: 99 });
}

#[test]
fn intent_cancel() {
    let mut queue = AuctionIntentQueue::default();
    queue.cancel(7);
    let drained = queue.drain();
    assert_eq!(drained[0], AuctionIntent::Cancel { auction_id: 7 });
}

#[test]
fn intent_drain_clears() {
    let mut queue = AuctionIntentQueue::default();
    queue.search(AuctionSearchFilter::default());
    queue.query_owned();
    assert_eq!(queue.drain().len(), 2);
    assert!(queue.pending.is_empty());
}
