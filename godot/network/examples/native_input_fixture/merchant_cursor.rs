//! Physical vendor pickup -> embedded backpack release; owned authoritative BuyItem peer.
use super::*;
use shared::protocol::{
    BuyItem, BuybackItemRequest, DestroyItem, EquipItem, InventoryDelta, InventorySlotChange,
    ItemLocation, ItemStack, RepairItem, SellAllJunkItems, SellItem, SortBags, SplitItem, SwapItem,
    UseItem,
};

const QUIET: Duration = Duration::from_millis(400);
const REQUEST_WAIT: Duration = Duration::from_secs(6);
const DESTINATION: ItemLocation = ItemLocation::Bag { bag: 0, slot: 0 };

#[derive(Resource, Default)]
struct Requests {
    buys: Vec<BuyItem>,
    forbidden: Vec<String>,
}

fn collect_forbidden<M: network::Message + std::fmt::Debug>(
    receivers: &mut Query<&mut MessageReceiver<M>>,
    requests: &mut Requests,
) {
    for mut receiver in receivers.iter_mut() {
        requests.forbidden.extend(
            receiver
                .receive()
                .map(|request| format!("{} {request:?}", std::any::type_name::<M>())),
        );
    }
}

fn receive_merchant(
    mut buys: Query<&mut MessageReceiver<BuyItem>>,
    mut sells: Query<&mut MessageReceiver<SellItem>>,
    mut junk: Query<&mut MessageReceiver<SellAllJunkItems>>,
    mut buybacks: Query<&mut MessageReceiver<BuybackItemRequest>>,
    mut repairs: Query<&mut MessageReceiver<RepairItem>>,
    mut requests: ResMut<Requests>,
) {
    for mut receiver in &mut buys {
        requests.buys.extend(receiver.receive());
    }
    collect_forbidden(&mut sells, &mut requests);
    collect_forbidden(&mut junk, &mut requests);
    collect_forbidden(&mut buybacks, &mut requests);
    collect_forbidden(&mut repairs, &mut requests);
}

fn receive_inventory(
    mut swaps: Query<&mut MessageReceiver<SwapItem>>,
    mut splits: Query<&mut MessageReceiver<SplitItem>>,
    mut destroys: Query<&mut MessageReceiver<DestroyItem>>,
    mut equips: Query<&mut MessageReceiver<EquipItem>>,
    mut uses: Query<&mut MessageReceiver<UseItem>>,
    mut sorts: Query<&mut MessageReceiver<SortBags>>,
    mut requests: ResMut<Requests>,
) {
    collect_forbidden(&mut swaps, &mut requests);
    collect_forbidden(&mut splits, &mut requests);
    collect_forbidden(&mut destroys, &mut requests);
    collect_forbidden(&mut equips, &mut requests);
    collect_forbidden(&mut uses, &mut requests);
    collect_forbidden(&mut sorts, &mut requests);
}

#[derive(Debug, PartialEq, Eq)]
enum Phase {
    Loading,
    Ready,
    Open,
    VendorOpen,
    PickupArm,
    Pickup,
    Held,
    Request,
    Delta,
    Drain,
}

struct Session {
    phase: Phase,
    since: Instant,
    selected: Option<Entity>,
    opens: usize,
    buys: usize,
    commit: bool,
}

impl Session {
    fn advance(&mut self, phase: Phase) {
        println!(
            "MERCHANT CURSOR PHASE {phase:?} opens={} buys={}",
            self.opens, self.buys
        );
        self.phase = phase;
        self.since = Instant::now();
    }

    fn require_quiet(&self) -> Result<(), String> {
        if self.since.elapsed() < QUIET {
            return Err(format!(
                "merchant-cursor quiet phase too short: {:?}",
                self.phase
            ));
        }
        Ok(())
    }

    fn observe(&mut self, app: &mut App, line: &str) -> Result<(), String> {
        bags::reject_runtime_error(line)?;
        if !line.starts_with("FIXTURE MERCHANT_CURSOR_") {
            return Ok(());
        }
        if self.selected.is_none() {
            return Err(format!(
                "merchant-cursor marker before authenticated selection: {line}"
            ));
        }
        match (&self.phase, line) {
            (Phase::Loading, "FIXTURE MERCHANT_CURSOR_LOADING") => {
                let selected = self.selected.ok_or("Loading requires selected player")?;
                app.world_mut().entity_mut(selected).insert(Gold(1000));
                println!(
                    "MERCHANT CURSOR INITIAL selected={} Gold1000",
                    selected.to_bits()
                );
                send::<_, TerrainChannel>(
                    app,
                    LoadTerrain {
                        map_name: "azeroth".into(),
                        initial_tile_y: 32,
                        initial_tile_x: 48,
                    },
                );
                self.advance(Phase::Ready);
            }
            (Phase::Ready, "FIXTURE MERCHANT_CURSOR_READY") => self.advance(Phase::Open),
            (Phase::VendorOpen, "FIXTURE MERCHANT_CURSOR_VENDOR_OPEN") => {
                self.advance(Phase::PickupArm)
            }
            (Phase::PickupArm, "FIXTURE MERCHANT_CURSOR_PICKUP_ARM") => self.advance(Phase::Pickup),
            (Phase::Pickup, "FIXTURE MERCHANT_CURSOR_PICKED_UP") => {
                self.require_quiet()?;
                self.advance(Phase::Held);
            }
            (Phase::Held, "FIXTURE MERCHANT_CURSOR_DROP_ARM") => self.advance(Phase::Request),
            (Phase::Request, "FIXTURE MERCHANT_CURSOR_COMMIT") if !self.commit => {
                self.require_quiet()?;
                self.commit = true;
            }
            (Phase::Delta, "FIXTURE MERCHANT_CURSOR_DONE") => {
                self.require_quiet()?;
                self.advance(Phase::Drain);
            }
            _ => {
                return Err(format!(
                    "out-of-order merchant-cursor marker in {:?}: {line}",
                    self.phase
                ));
            }
        }
        Ok(())
    }

    fn respond(&mut self, app: &mut App) -> Result<(), String> {
        self.respond_to_interaction(app)?;
        self.receive_buy(app)?;
        if self.phase == Phase::Request {
            if self.commit && self.buys == 1 {
                send_purchase(
                    app,
                    self.selected.ok_or("purchase requires selected player")?,
                );
                self.advance(Phase::Delta);
            } else if self.since.elapsed() > REQUEST_WAIT {
                return Err(format!(
                    "merchant-cursor missing exact BuyItem/COMMIT; buys={} commit={}",
                    self.buys, self.commit
                ));
            }
        }
        Ok(())
    }

    fn respond_to_interaction(&mut self, app: &mut App) -> Result<(), String> {
        let (vendor, interactions, closes, casts) = {
            let mut incoming = app.world_mut().resource_mut::<Incoming>();
            (
                incoming.vendor,
                std::mem::take(&mut incoming.interactions),
                std::mem::take(&mut incoming.closes),
                std::mem::take(&mut incoming.casts),
            )
        };
        if !closes.is_empty() || !casts.is_empty() {
            return Err(format!(
                "merchant-cursor unrelated close/spell requests: {closes:?} {casts:?}"
            ));
        }
        for request in interactions {
            if self.phase != Phase::Open
                || self.opens != 0
                || Some(request.npc) != vendor.map(Entity::to_bits)
            {
                return Err(format!(
                    "unexpected merchant-cursor interaction {request:?} in {:?}",
                    self.phase
                ));
            }
            self.opens += 1;
            send_vendor(app, request.npc);
            self.advance(Phase::VendorOpen);
        }
        Ok(())
    }

    fn receive_buy(&mut self, app: &mut App) -> Result<(), String> {
        let requests = std::mem::take(&mut *app.world_mut().resource_mut::<Requests>());
        if !requests.forbidden.is_empty() {
            return Err(format!(
                "forbidden merchant-cursor requests: {:?}",
                requests.forbidden
            ));
        }
        let vendor = app
            .world()
            .resource::<Incoming>()
            .vendor
            .map(Entity::to_bits);
        for request in requests.buys {
            let expected = BuyItem {
                npc: vendor.ok_or("BuyItem requires owned vendor")?,
                slot: 0,
                item_id: 2589,
                count: 1,
                destination: Some(DESTINATION),
            };
            if self.phase != Phase::Request || self.buys != 0 || request != expected {
                return Err(format!(
                    "unexpected/duplicate BuyItem {request:?} in {:?}; count={}",
                    self.phase, self.buys
                ));
            }
            self.buys += 1;
            println!(
                "MERCHANT CURSOR DECODED {request:?} count=1; delta and Gold withheld until COMMIT"
            );
        }
        Ok(())
    }
}

fn send_vendor(app: &mut App, npc: u64) {
    send::<_, InteractionChannel>(
        app,
        InteractionOpened {
            npc,
            kind: InteractionKind::Role(NpcRole::Vendor),
        },
    );
    send::<_, InventoryChannel>(
        app,
        InventorySnapshot {
            bags: vec![BagContents {
                bag: 0,
                size: 16,
                items: vec![],
            }],
        },
    );
    // Local ItemSparse row2589: Stackable1000, VendorStackCount1, quality1.
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
}

fn send_purchase(app: &mut App, selected: Entity) {
    send::<_, InventoryChannel>(
        app,
        InventoryDelta {
            changes: vec![InventorySlotChange {
                location: DESTINATION,
                item: Some(ItemStack {
                    item_guid: 9_182_589,
                    item_id: 2589,
                    count: 1,
                    durability: None,
                    soulbound: false,
                }),
            }],
        },
    );
    app.world_mut().entity_mut(selected).insert(Gold(975));
    println!("MERCHANT CURSOR AUTHORITATIVE bag0/slot0 guid9182589 Linen2589 count1 Gold975");
}

fn tick_peer(
    app: &mut App,
    session: &mut Session,
    remote: &mut Option<Entity>,
) -> Result<(), String> {
    app.update();
    respond_to_login(app, StartupScreen::MerchantCursor)?;
    respond_to_selection(
        app,
        StartupScreen::MerchantCursor,
        &mut session.selected,
        remote,
    )?;
    Ok(())
}

fn run_until_done(
    app: &mut App,
    child: &mut Child,
    lines: &Receiver<String>,
) -> Result<(), String> {
    app.init_resource::<Requests>();
    app.add_systems(Update, (receive_merchant, receive_inventory));
    let mut remote = None;
    let mut session = Session {
        phase: Phase::Loading,
        since: Instant::now(),
        selected: None,
        opens: 0,
        buys: 0,
        commit: false,
    };
    let deadline = Instant::now() + TIMEOUT + Duration::from_secs(180);
    while Instant::now() < deadline {
        tick_peer(app, &mut session, &mut remote)?;
        for line in lines.try_iter() {
            session.observe(app, line.trim())?;
        }
        session.respond(app)?;
        if let Some(status) = child
            .try_wait()
            .map_err(|error| format!("inspect merchant-cursor child: {error}"))?
        {
            return Err(format!(
                "merchant-cursor exited before deliberate cleanup: {status}; phase={:?}",
                session.phase
            ));
        }
        if session.phase == Phase::Drain && session.since.elapsed() >= QUIET {
            if session.opens != 1 || session.buys != 1 || !session.commit {
                return Err(
                    "merchant-cursor final interaction/request/barrier totals failed".into(),
                );
            }
            return Ok(());
        }
        thread::sleep(TICK);
    }
    Err(format!(
        "merchant-cursor timed out; phase={:?} opens={} buys={}",
        session.phase, session.opens, session.buys
    ))
}

pub(super) fn run(
    app: &mut App,
    child: &mut Child,
    lines: Receiver<String>,
    readers: Vec<thread::JoinHandle<()>>,
) -> Result<(), String> {
    let mut result = run_until_done(app, child, &lines);
    let cleanup = bags::cleanup_owned_child(child, readers);
    for line in lines.try_iter() {
        if let Err(error) = bags::reject_runtime_error(line.trim()) {
            result = Err(match result {
                Ok(()) => error,
                Err(original) => format!("{original}; {error}"),
            });
        } else if line.trim().starts_with("FIXTURE MERCHANT_CURSOR_") {
            result = Err(format!(
                "unexpected merchant-cursor marker after cleanup: {}",
                line.trim()
            ));
        }
    }
    match (result, cleanup) {
        (Err(error), Err(cleanup)) => Err(format!("{error}; owned cleanup failed: {cleanup}")),
        (Err(error), _) | (_, Err(error)) => Err(error),
        (Ok(()), Ok(())) => {
            println!(
                "PASS: MERCHANT_CURSOR physical vendor pickup, exact once BuyItem destination bag0/slot0, pre-delta COMMIT empty/1000/cursor cleared, authoritative Linen1/975; deliberate kill/reap/readers drained, NOT normal shutdown proof"
            );
            Ok(())
        }
    }
}
