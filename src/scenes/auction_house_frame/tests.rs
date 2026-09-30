use std::sync::mpsc::Receiver;

use bevy::ecs::system::SystemId;
use game_engine::auction_house::AuctionHousePlugin;
use game_engine::item_catalog::ItemCatalogEntry;
use game_engine::network_runtime::messages::{ConnectionSender, Inbox};
use game_engine::network_runtime::worker::NetworkCommand;
use game_engine::ui::screens::auction_house_frame_component::{
    BID_BOXES, QUANTITY_BOX, SELL_BUYOUT_BOXES,
};
use lightyear::prelude::Message as NetworkMessage;
use shared::protocol::{
    AuctionChannel, AuctionHouseOpened, AuctionInventoryItem, AuctionInventorySnapshot,
    AuctionListingSummary, AuctionTimeLeft, BuyoutAuction, CancelAuction, CreateAuction,
    OpenAuctionHouse, PlaceBid, QueryAuctionInventory, QueryAuctions, QueryBidAuctions,
    QueryOwnedAuctions,
};

use super::*;

/// Server entity bits of the Stormwind auctioneer in these fixtures.
const AUCTIONEER: u64 = 0x0000_0002_0000_0101;
const LINEN: u32 = 2589;
const COPPER_ORE: u32 = 2770;
const SWORD: u32 = 25;

struct Fixture {
    app: App,
    commands: Receiver<NetworkCommand>,
    window_sync: SystemId,
}

fn fixture() -> Fixture {
    let mut app = App::new();
    app.add_plugins(AuctionHousePlugin)
        .init_resource::<AuctionHouseUi>()
        .init_resource::<WindowManager>()
        .add_message::<NpcFrameEvent>()
        .add_message::<NpcInteractionRequest>();
    let (sender, commands) = std::sync::mpsc::channel();
    app.insert_resource(ConnectionSender::new(Some(sender)));
    let window_sync = app.world_mut().register_system(sync_auction_window);
    Fixture {
        app,
        commands,
        window_sync,
    }
}

impl Fixture {
    fn net(&mut self) -> Mut<'_, AuctionHouseState> {
        self.app.world_mut().resource_mut::<AuctionHouseState>()
    }

    fn deliver<M: NetworkMessage>(&mut self, messages: Vec<M>) {
        self.app.insert_resource(Inbox::new(messages));
        game_engine::network_events::dispatch_incoming(self.app.world_mut());
    }

    fn sync_window(&mut self) {
        self.app.world_mut().run_system(self.window_sync).unwrap();
    }

    fn window_open(&self) -> bool {
        self.app
            .world()
            .resource::<WindowManager>()
            .is_open(WindowId::AuctionHouse)
    }

    /// Opens the house for the auctioneer the way the live client does.
    fn open(&mut self) {
        self.app.world_mut().write_message(NpcFrameEvent::Opened {
            npc: AUCTIONEER,
            role: NpcRole::AuctionHouse,
        });
        self.app
            .world_mut()
            .run_system_cached(open_on_auctioneer)
            .unwrap();
        self.deliver(vec![AuctionHouseOpened {
            success: true,
            error: None,
        }]);
        self.sync_window();
    }

    /// Runs one frame action against the fixture state.
    fn click(&mut self, action: &str, texts: &InputTexts) -> Vec<actions::InputEdit> {
        let world = self.app.world_mut();
        world.resource_scope(|world, mut net: Mut<AuctionHouseState>| {
            let mut ui = world.resource_mut::<AuctionHouseUi>();
            actions::dispatch(action, &mut net, &mut ui, texts)
        })
    }

    fn view(&self, texts: &InputTexts) -> AuctionHouseFrameState {
        view::build_view(&ViewInputs {
            net: self.app.world().resource::<AuctionHouseState>(),
            ui: self.app.world().resource::<AuctionHouseUi>(),
            texts,
            catalog: &catalog,
            visible: true,
        })
    }

    fn sent(&mut self) -> Sent {
        game_engine::network_events::dispatch_outgoing(self.app.world_mut());
        loopback(self.commands.try_iter().collect())
    }

    fn interaction_requests(&mut self) -> Vec<NpcInteractionRequest> {
        self.app
            .world_mut()
            .resource_mut::<Messages<NpcInteractionRequest>>()
            .drain()
            .collect()
    }
}

fn catalog(item_id: u32) -> Option<&'static ItemCatalogEntry> {
    let (class_id, icon_fdid) = match item_id {
        LINEN => (7, 132_889),
        COPPER_ORE => (7, 134_566),
        SWORD => (2, 135_274),
        _ => return None,
    };
    Some(Box::leak(Box::new(ItemCatalogEntry {
        class_id,
        icon_fdid,
        ..Default::default()
    })))
}

fn item(item_guid: u64, item_id: u32, name: &str, stack_count: u32) -> AuctionInventoryItem {
    AuctionInventoryItem {
        item_guid,
        item_id,
        name: name.into(),
        quality: 1,
        required_level: 1,
        stack_count,
        vendor_sell_price: 13,
    }
}

fn listing(
    auction_id: u64,
    item: AuctionInventoryItem,
    bid: u64,
    buyout: Option<u64>,
) -> AuctionListingSummary {
    AuctionListingSummary {
        auction_id,
        stack_count: item.stack_count,
        item,
        owner_name: "Seller".into(),
        min_bid: bid,
        current_bid: None,
        min_next_bid: bid,
        buyout_price: buyout,
        time_left: AuctionTimeLeft::VeryLong,
    }
}

fn texts(pairs: &[(&'static str, &str)]) -> InputTexts {
    pairs
        .iter()
        .map(|(name, value)| (*name, value.to_string()))
        .collect()
}

/// Auction requests the loopback server peer received, by message type.
#[derive(Default)]
struct Sent {
    open: Vec<OpenAuctionHouse>,
    browse: Vec<QueryAuctions>,
    owned: Vec<QueryOwnedAuctions>,
    bids: Vec<QueryBidAuctions>,
    inventory: Vec<QueryAuctionInventory>,
    create: Vec<CreateAuction>,
    bid: Vec<PlaceBid>,
    buyout: Vec<BuyoutAuction>,
    cancel: Vec<CancelAuction>,
}

/// Applies queued worker commands to a loopback transport and reads what arrived.
fn loopback(commands: Vec<NetworkCommand>) -> Sent {
    use lightyear::prelude::client::ClientPlugins;
    use lightyear::prelude::{
        ChannelRegistry, Connected, Link, Linked, MessageReceiver, MessageSender, PeerId, RemoteId,
        Transport,
    };
    let mut worker = App::new();
    worker.add_plugins(bevy::state::app::StatesPlugin);
    worker.add_plugins(ClientPlugins::default());
    worker.add_plugins(shared::ProtocolPlugin);
    worker.finish();
    worker.cleanup();
    let registry = worker.world().resource::<ChannelRegistry>();
    let mut transport = Transport::default();
    transport.add_sender_from_registry::<AuctionChannel>(registry);
    transport.add_receiver_from_registry::<AuctionChannel>(registry);
    let peer = worker
        .world_mut()
        .spawn((
            Link::default(),
            transport,
            Linked,
            Connected,
            RemoteId(PeerId::Local(0)),
        ))
        .id();
    macro_rules! endpoints {
        ($($ty:ty),*) => {
            $(worker.world_mut().entity_mut(peer).insert((
                MessageSender::<$ty>::default(),
                MessageReceiver::<$ty>::default(),
            ));)*
        };
    }
    endpoints!(
        OpenAuctionHouse,
        QueryAuctions,
        QueryOwnedAuctions,
        QueryBidAuctions,
        QueryAuctionInventory,
        CreateAuction,
        PlaceBid,
        BuyoutAuction,
        CancelAuction
    );
    for command in commands {
        let NetworkCommand::Apply(apply) = command else {
            panic!("expected a message, not worker shutdown");
        };
        apply(worker.world_mut());
    }
    worker.world_mut().run_schedule(PostUpdate);
    {
        let mut entity = worker.world_mut().entity_mut(peer);
        let mut link = entity.get_mut::<Link>().unwrap();
        let packets: Vec<_> = link.send.drain().collect();
        for packet in packets {
            link.recv.push_raw(packet);
        }
    }
    worker.world_mut().run_schedule(PreUpdate);
    let mut entity = worker.world_mut().entity_mut(peer);
    macro_rules! received {
        ($ty:ty) => {
            entity
                .get_mut::<MessageReceiver<$ty>>()
                .unwrap()
                .receive()
                .collect()
        };
    }
    Sent {
        open: received!(OpenAuctionHouse),
        browse: received!(QueryAuctions),
        owned: received!(QueryOwnedAuctions),
        bids: received!(QueryBidAuctions),
        inventory: received!(QueryAuctionInventory),
        create: received!(CreateAuction),
        bid: received!(PlaceBid),
        buyout: received!(BuyoutAuction),
        cancel: received!(CancelAuction),
    }
}

#[test]
fn auctioneer_interaction_opens_the_house_window_and_loads_money_auctions_and_bids() {
    let mut f = fixture();
    f.app.world_mut().write_message(NpcFrameEvent::Opened {
        npc: AUCTIONEER,
        role: NpcRole::AuctionHouse,
    });
    f.app
        .world_mut()
        .run_system_cached(open_on_auctioneer)
        .unwrap();
    assert_eq!(f.sent().open, [OpenAuctionHouse]);
    assert!(!f.window_open(), "the server has not opened the house yet");

    f.deliver(vec![AuctionHouseOpened {
        success: true,
        error: None,
    }]);
    f.sync_window();

    assert!(f.window_open());
    let sent = f.sent();
    assert_eq!(sent.inventory, [QueryAuctionInventory]);
    assert_eq!(sent.owned, [QueryOwnedAuctions]);
    assert_eq!(sent.bids, [QueryBidAuctions]);
    assert!(sent.browse.is_empty(), "Retail opens without a search");
}

#[test]
fn closing_the_window_ends_the_auctioneer_interaction() {
    let mut f = fixture();
    f.open();

    f.click(
        game_engine::ui::screens::auction_house_frame_component::ACTION_CLOSE,
        &InputTexts::new(),
    );
    f.sync_window();

    assert!(!f.window_open());
    assert!(!f.app.world().resource::<AuctionHouseState>().is_open);
    assert_eq!(
        f.interaction_requests(),
        vec![NpcInteractionRequest::Close { npc: AUCTIONEER }]
    );
}

#[test]
fn server_ending_the_interaction_closes_the_window_without_a_close_request() {
    let mut f = fixture();
    f.open();

    f.app
        .world_mut()
        .write_message(NpcFrameEvent::Closed { npc: AUCTIONEER });
    f.app
        .world_mut()
        .run_system_cached(open_on_auctioneer)
        .unwrap();
    f.sync_window();

    assert!(!f.window_open());
    assert!(f.interaction_requests().is_empty());
}

#[test]
fn tabs_switch_the_frame_mode_and_title() {
    let mut f = fixture();
    f.open();
    let none = InputTexts::new();
    assert_eq!(f.view(&none).tab, AuctionHouseTab::Buy);

    f.click("auction_tab:sell", &none);
    assert_eq!(f.view(&none).tab, AuctionHouseTab::Sell);
    f.click("auction_tab:auctions", &none);
    assert_eq!(f.view(&none).tab, AuctionHouseTab::Auctions);
    assert_eq!(f.view(&none).tab.title(), "Auctions");
}

#[test]
fn search_sends_the_typed_text_and_results_group_per_item_with_the_lowest_unit_price() {
    let mut f = fixture();
    f.open();
    f.sent();
    let typed = texts(&[(SEARCH_BOX, " linen ")]);
    f.click("auction_search", &typed);
    let sent = f.sent();
    assert_eq!(sent.browse.len(), 1);
    assert_eq!(sent.browse[0].query.text, "linen");

    f.net().search_results = vec![
        listing(1, item(10, LINEN, "Linen Cloth", 20), 100, Some(2_000)),
        listing(2, item(11, LINEN, "Linen Cloth", 5), 50, Some(750)),
        listing(3, item(12, SWORD, "Worn Shortsword", 1), 900, None),
    ];
    let view = f.view(&typed);

    assert_eq!(view.browse.len(), 2);
    assert_eq!(view.browse[0].item.name, "Linen Cloth");
    // 2000 / 20 = 100 per item beats 750 / 5 = 150.
    assert_eq!(view.browse[0].price, 100);
    assert_eq!(view.browse[0].available, 25);
    assert_eq!(view.browse[0].item.icon_fdid, 132_889);
    // No buyout: the bid is the price.
    assert_eq!(view.browse[1].price, 900);
    assert!(!view.search_empty);

    f.click("auction_category:0", &typed);
    let weapons = f.view(&typed);
    assert!(weapons.categories[0].selected);
    assert_eq!(weapons.browse.len(), 1);
    assert_eq!(weapons.browse[0].item_id, SWORD);
}

#[test]
fn bid_prefills_the_minimum_and_sends_place_bid() {
    let mut f = fixture();
    f.open();
    f.net().inventory = Some(AuctionInventorySnapshot {
        gold: 100_000,
        items: vec![],
    });
    f.net().search_results = vec![listing(
        7,
        item(10, LINEN, "Linen Cloth", 20),
        1_250,
        Some(9_000),
    )];
    f.click("auction_browse_item:2589", &InputTexts::new());
    let edits = f.click("auction_select:7", &InputTexts::new());
    assert_eq!(
        edits,
        vec![
            (BID_BOXES.gold, "0".to_string()),
            (BID_BOXES.silver, "12".to_string()),
            (BID_BOXES.copper, "50".to_string()),
        ]
    );
    let bid = texts(&[(BID_BOXES.silver, "13"), (BID_BOXES.copper, "0")]);
    let view = f.view(&bid);
    let item_buy = view.item_buy.expect("item buy frame");
    assert!(item_buy.can_bid);
    assert!(item_buy.can_buyout);
    assert!(item_buy.rows[0].selected);

    f.sent();
    f.click("auction_bid", &bid);

    assert_eq!(
        f.sent().bid,
        [PlaceBid {
            auction_id: 7,
            amount: 1_300,
        }]
    );
}

#[test]
fn buyout_asks_for_confirmation_then_sends_buyout_auction() {
    let mut f = fixture();
    f.open();
    f.net().inventory = Some(AuctionInventorySnapshot {
        gold: 100_000,
        items: vec![],
    });
    f.net().search_results = vec![listing(
        7,
        item(10, LINEN, "Linen Cloth", 20),
        1_250,
        Some(9_000),
    )];
    let none = InputTexts::new();
    f.click("auction_browse_item:2589", &none);
    f.click("auction_select:7", &none);
    f.sent();

    f.click("auction_buyout", &none);
    let dialog = f.view(&none).dialog.expect("buy dialog");
    assert_eq!(dialog.item_text, "Linen Cloth  x20");
    assert_eq!(dialog.price, 9_000);
    assert!(
        f.sent().buyout.is_empty(),
        "nothing is bought before Buy Now"
    );

    f.click("auction_dialog_buy", &none);
    assert_eq!(f.sent().buyout, [BuyoutAuction { auction_id: 7 }]);
    assert!(f.view(&none).dialog.is_none());
}

#[test]
fn posting_sends_the_stack_with_per_item_prices_times_quantity() {
    let mut f = fixture();
    f.open();
    f.net().inventory = Some(AuctionInventorySnapshot {
        gold: 100_000,
        items: vec![item(55, LINEN, "Linen Cloth", 20)],
    });
    f.click("auction_tab:sell", &InputTexts::new());
    f.sent();

    let edits = f.click("auction_sell_item:55", &InputTexts::new());
    assert!(edits.contains(&(QUANTITY_BOX, "1".to_string())));
    let sent = f.sent();
    assert_eq!(
        sent.browse[0].query.text, "Linen Cloth",
        "the sell list shows its auctions"
    );

    let typed = texts(&[(QUANTITY_BOX, "5"), (SELL_BUYOUT_BOXES.silver, "2")]);
    let sell = f.view(&typed).sell;
    assert_eq!(sell.item.expect("sell item").count, 20);
    // 13 vendor copper × 5 × 2 for 24 hours.
    assert_eq!(sell.deposit, 130);
    assert_eq!(sell.total, 1_000);
    assert!(sell.can_post);

    let edits = f.click("auction_post", &typed);

    assert_eq!(
        f.sent().create,
        [CreateAuction {
            item_guid: 55,
            stack_count: 5,
            min_bid: 1_000,
            buyout_price: Some(1_000),
            duration: AuctionDuration::Medium,
        }]
    );
    assert!(edits.contains(&(QUANTITY_BOX, String::new())));
    assert!(f.view(&InputTexts::new()).sell.item.is_none());
}

#[test]
fn posting_is_refused_without_a_price_or_above_the_stack() {
    let mut f = fixture();
    f.open();
    f.net().inventory = Some(AuctionInventorySnapshot {
        gold: 100_000,
        items: vec![item(55, LINEN, "Linen Cloth", 20)],
    });
    f.click("auction_sell_item:55", &InputTexts::new());
    f.sent();

    assert!(!f.view(&texts(&[(QUANTITY_BOX, "5")])).sell.can_post);
    let too_many = texts(&[(QUANTITY_BOX, "21"), (SELL_BUYOUT_BOXES.gold, "1")]);
    assert!(!f.view(&too_many).sell.can_post);
    f.click("auction_post", &too_many);
    assert!(f.sent().create.is_empty());
}

#[test]
fn cancel_sends_cancel_auction_for_the_selected_owned_auction() {
    let mut f = fixture();
    f.open();
    f.net().owned_results = vec![listing(
        31,
        item(56, COPPER_ORE, "Copper Ore", 10),
        300,
        Some(600),
    )];
    let none = InputTexts::new();
    f.click("auction_tab:auctions", &none);
    assert!(!f.view(&none).auctions.can_cancel);
    f.click("auction_select:31", &none);
    let auctions = f.view(&none).auctions;
    assert!(auctions.can_cancel);
    assert_eq!(auctions.rows[0].item.name, "Copper Ore");
    f.sent();

    f.click("auction_cancel", &none);

    assert_eq!(f.sent().cancel, [CancelAuction { auction_id: 31 }]);
}

#[test]
fn a_successful_operation_refreshes_money_auctions_bids_and_the_last_search() {
    let mut f = fixture();
    f.open();
    f.click("auction_search", &texts(&[(SEARCH_BOX, "linen")]));
    f.deliver(vec![shared::protocol::AuctionSearchResults {
        query: shared::protocol::AuctionSearchQuery {
            item_id: None,
            class_id: None,
            text: "linen".into(),
            page: 0,
            page_size: 50,
            min_level: None,
            max_level: None,
            quality: None,
            usable_only: false,
            sort_field: shared::protocol::AuctionSortField::Name,
            sort_dir: shared::protocol::AuctionSortDir::Asc,
            faction: 1,
        },
        total_results: 0,
        results: vec![],
    }]);
    f.sent();

    f.deliver(vec![shared::protocol::AuctionOperationResponse {
        success: true,
        message: "auction cancelled".into(),
    }]);

    let sent = f.sent();
    assert_eq!(sent.inventory.len(), 1);
    assert_eq!(sent.owned.len(), 1);
    assert_eq!(sent.bids.len(), 1);
    assert_eq!(sent.browse.len(), 1);
    assert_eq!(sent.browse[0].query.text, "linen");
}
