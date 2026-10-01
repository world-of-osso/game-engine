//! Verifier-only observation peer: logs every decoded UI request; never asserts policy.
//! The GDScript side prints case markers; counts are read from the combined log.
use super::*;
use shared::protocol::{
    AuctionChannel, AuctionHouseOpened, BagSlotItem, BuyItem, BuybackItemRequest, DestroyItem,
    EquipItem, InteractionClosed, ItemStack, OpenAuctionHouse, RepairItem, SellAllJunkItems,
    SellItem, SortBags, SplitItem, SwapItem, UseItem,
};

const QUIET: Duration = Duration::from_millis(400);

#[derive(Resource, Default)]
struct Requests(Vec<String>);

#[derive(Resource, Default)]
struct AuctionOpens(usize);

fn receive_auction(
    mut opens: Query<&mut MessageReceiver<OpenAuctionHouse>>,
    mut count: ResMut<AuctionOpens>,
) {
    for mut receiver in &mut opens {
        count.0 += receiver.receive().count();
    }
}

fn collect<M: network::Message + std::fmt::Debug>(
    receivers: &mut Query<&mut MessageReceiver<M>>,
    requests: &mut Requests,
) {
    for mut receiver in receivers.iter_mut() {
        requests.0.extend(receiver.receive().map(|request| {
            let name = std::any::type_name::<M>()
                .rsplit("::")
                .next()
                .unwrap_or("?");
            format!("{name} {request:?}")
        }));
    }
}

#[allow(clippy::too_many_arguments)]
fn receive(
    mut buys: Query<&mut MessageReceiver<BuyItem>>,
    mut sells: Query<&mut MessageReceiver<SellItem>>,
    mut buybacks: Query<&mut MessageReceiver<BuybackItemRequest>>,
    mut junk: Query<&mut MessageReceiver<SellAllJunkItems>>,
    mut repairs: Query<&mut MessageReceiver<RepairItem>>,
    mut swaps: Query<&mut MessageReceiver<SwapItem>>,
    mut splits: Query<&mut MessageReceiver<SplitItem>>,
    mut destroys: Query<&mut MessageReceiver<DestroyItem>>,
    mut equips: Query<&mut MessageReceiver<EquipItem>>,
    mut uses: Query<&mut MessageReceiver<UseItem>>,
    mut sorts: Query<&mut MessageReceiver<SortBags>>,
    mut requests: ResMut<Requests>,
) {
    collect(&mut buys, &mut requests);
    collect(&mut sells, &mut requests);
    collect(&mut buybacks, &mut requests);
    collect(&mut junk, &mut requests);
    collect(&mut repairs, &mut requests);
    collect(&mut swaps, &mut requests);
    collect(&mut splits, &mut requests);
    collect(&mut destroys, &mut requests);
    collect(&mut equips, &mut requests);
    collect(&mut uses, &mut requests);
    collect(&mut sorts, &mut requests);
}

fn linen(guid: u64, count: u32) -> ItemStack {
    ItemStack {
        item_guid: guid,
        item_id: 2589,
        count,
        durability: None,
        soulbound: false,
    }
}

fn send_snapshot(app: &mut App) {
    send::<_, InventoryChannel>(
        app,
        InventorySnapshot {
            bags: vec![
                BagContents {
                    bag: 0,
                    size: 16,
                    items: vec![
                        BagSlotItem {
                            slot: 0,
                            item: linen(7_100_001, 5),
                        },
                        BagSlotItem {
                            slot: 1,
                            item: linen(7_100_002, 1),
                        },
                    ],
                },
                BagContents {
                    bag: 1,
                    size: 8,
                    items: vec![
                        BagSlotItem {
                            slot: 0,
                            item: linen(7_100_003, 3),
                        },
                        BagSlotItem {
                            slot: 3,
                            item: linen(7_100_004, 2),
                        },
                    ],
                },
            ],
        },
    );
    println!("UIOWN PEER SNAPSHOT bag0/0 x5 bag0/1 x1 bag1/0 x3 bag1/3 x2");
}

fn send_vendor(app: &mut App, npc: u64) {
    send::<_, InteractionChannel>(
        app,
        InteractionOpened {
            npc,
            kind: InteractionKind::Role(NpcRole::Vendor),
        },
    );
    send::<_, MerchantChannel>(
        app,
        VendorInventory {
            npc,
            can_repair: false,
            items: vec![VendorItem {
                slot: 0,
                item_id: 2589,
                name: "Linen Cloth".into(),
                quality: 1,
                price: 25,
                stack_count: 1,
                max_stack: 1000,
                num_available: None,
                usable: true,
            }],
        },
    );
    println!("UIOWN PEER VENDOR_OPENED npc={npc}");
}

struct Session {
    selected: Option<Entity>,
    done: Option<Instant>,
    requests: usize,
}

impl Session {
    fn observe(&mut self, app: &mut App, line: &str) -> Result<(), String> {
        let message = line
            .strip_prefix("GODOT_STDERR: ")
            .unwrap_or(line)
            .trim_start();
        if message.starts_with("ERROR:") || message.starts_with("SCRIPT ERROR:") {
            println!("UIOWN PEER GODOT_ERROR {message}");
        }
        let vendor = app
            .world()
            .resource::<Incoming>()
            .vendor
            .map(Entity::to_bits);
        match line {
            "FIXTURE UIOWN_LOADING" if self.selected.is_some() => send::<_, TerrainChannel>(
                app,
                LoadTerrain {
                    map_name: "azeroth".into(),
                    initial_tile_y: 32,
                    initial_tile_x: 48,
                },
            ),
            "FIXTURE UIOWN_READY" | "FIXTURE UIOWN_SNAPSHOT" => send_snapshot(app),
            "FIXTURE UIOWN_SERVER_CLOSE" => {
                let npc = vendor.ok_or("server close without vendor")?;
                send::<_, InteractionChannel>(app, InteractionClosed { npc });
                println!("UIOWN PEER SENT InteractionClosed npc={npc}");
            }
            "FIXTURE UIOWN_AH_OPEN" => {
                let npc = vendor.ok_or("AH open without vendor")?;
                send::<_, InteractionChannel>(
                    app,
                    InteractionOpened {
                        npc,
                        kind: InteractionKind::Role(NpcRole::AuctionHouse),
                    },
                );
                println!("UIOWN PEER SENT InteractionOpened AuctionHouse npc={npc}");
            }
            "FIXTURE UIOWN_DONE" => self.done = Some(Instant::now()),
            _ => {}
        }
        Ok(())
    }

    fn respond(&mut self, app: &mut App) {
        let (interactions, closes, casts) = {
            let mut incoming = app.world_mut().resource_mut::<Incoming>();
            (
                std::mem::take(&mut incoming.interactions),
                std::mem::take(&mut incoming.closes),
                std::mem::take(&mut incoming.casts),
            )
        };
        for request in interactions {
            self.requests += 1;
            println!("UIOWN REQ #{} InteractNpc {request:?}", self.requests);
            send_vendor(app, request.npc);
        }
        for request in closes {
            self.requests += 1;
            println!("UIOWN REQ #{} CloseInteraction {request:?}", self.requests);
        }
        for request in casts {
            self.requests += 1;
            println!("UIOWN REQ #{} SpellCastIntent {request:?}", self.requests);
        }
        let opens = std::mem::take(&mut app.world_mut().resource_mut::<AuctionOpens>().0);
        for _ in 0..opens {
            self.requests += 1;
            println!("UIOWN REQ #{} OpenAuctionHouse", self.requests);
            send::<_, AuctionChannel>(
                app,
                AuctionHouseOpened {
                    success: true,
                    error: None,
                },
            );
        }
        let requests = std::mem::take(&mut app.world_mut().resource_mut::<Requests>().0);
        for request in requests {
            self.requests += 1;
            println!("UIOWN REQ #{} {request}", self.requests);
        }
    }
}

fn run_until_done(
    app: &mut App,
    child: &mut Child,
    lines: &Receiver<String>,
) -> Result<(), String> {
    app.init_resource::<Requests>();
    app.init_resource::<AuctionOpens>();
    app.add_systems(Update, (receive, receive_auction));
    let mut remote = None;
    let mut session = Session {
        selected: None,
        done: None,
        requests: 0,
    };
    let deadline = Instant::now() + TIMEOUT + Duration::from_secs(300);
    while Instant::now() < deadline {
        app.update();
        respond_to_login(app, StartupScreen::UiOwnership)?;
        respond_to_selection(
            app,
            StartupScreen::UiOwnership,
            &mut session.selected,
            &mut remote,
        )?;
        for line in lines.try_iter() {
            session.observe(app, line.trim())?;
        }
        session.respond(app);
        if let Some(status) = child
            .try_wait()
            .map_err(|error| format!("inspect ui-ownership child: {error}"))?
        {
            return Err(format!("ui-ownership child exited early: {status}"));
        }
        if session.done.is_some_and(|done| done.elapsed() >= QUIET) {
            return Ok(());
        }
        thread::sleep(TICK);
    }
    Err("ui-ownership timed out".into())
}

pub(super) fn run(
    app: &mut App,
    child: &mut Child,
    lines: Receiver<String>,
    readers: Vec<thread::JoinHandle<()>>,
) -> Result<(), String> {
    let result = run_until_done(app, child, &lines);
    let cleanup = bags::cleanup_owned_child(child, readers);
    result.and(cleanup)?;
    println!("UIOWN PEER FINISHED (observation only; see case lines)");
    Ok(())
}
