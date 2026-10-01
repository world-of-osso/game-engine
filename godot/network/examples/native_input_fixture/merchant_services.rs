//! Owned UDP oracle for direct services; fixture prices are not server-pricing proof.
use super::*;
use shared::protocol::{
    BagSlotItem, BuyItem, BuybackItem, BuybackItemRequest, BuybackList, DestroyItem,
    DurabilityChannel, DurabilitySnapshot, DurabilityStateUpdate, EquipItem, InventoryDelta,
    InventorySlotChange, ItemLocation, ItemStack, RepairItem, SellAllJunkItems, SellItem, SortBags,
    SplitItem, SwapItem, UseItem,
};

const NPC: u64 = 4_294_966_979;
// Observed ItemSparse 12.1.0.69933 row4865: Poor, SellPrice5.
const PELT_PRICE: u64 = 5;
const FINAL_GOLD: u64 = 984 + 2 * PELT_PRICE;
const QUIET: Duration = Duration::from_millis(900);
const REQUEST_WAIT: Duration = Duration::from_secs(6);

#[derive(Resource, Default)]
struct Requests {
    repairs: Vec<RepairItem>,
    junk: Vec<SellAllJunkItems>,
    forbidden: Vec<String>,
}

fn collect_forbidden<M: network::Message + std::fmt::Debug>(
    receivers: &mut Query<&mut MessageReceiver<M>>,
    requests: &mut Requests,
) {
    for mut receiver in receivers.iter_mut() {
        requests
            .forbidden
            .extend(receiver.receive().map(|message| format!("{message:?}")));
    }
}

fn receive_services(
    mut repairs: Query<&mut MessageReceiver<RepairItem>>,
    mut junk: Query<&mut MessageReceiver<SellAllJunkItems>>,
    mut requests: ResMut<Requests>,
) {
    for mut receiver in &mut repairs {
        requests.repairs.extend(receiver.receive());
    }
    for mut receiver in &mut junk {
        requests.junk.extend(receiver.receive());
    }
}

fn receive_merchant_rejections(
    mut buys: Query<&mut MessageReceiver<BuyItem>>,
    mut sells: Query<&mut MessageReceiver<SellItem>>,
    mut buybacks: Query<&mut MessageReceiver<BuybackItemRequest>>,
    mut requests: ResMut<Requests>,
) {
    collect_forbidden(&mut buys, &mut requests);
    collect_forbidden(&mut sells, &mut requests);
    collect_forbidden(&mut buybacks, &mut requests);
}

fn receive_inventory_rejections(
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
    RepairReady,
    RepairRequest,
    RepairDelta,
    RepairQuiet,
    JunkRequest,
    JunkDelta,
    Drain,
}

struct Session {
    phase: Phase,
    since: Instant,
    selected: Option<Entity>,
    opens: usize,
    repairs: usize,
    junk: usize,
    commit: bool,
    configured: bool,
}

impl Session {
    fn advance(&mut self, phase: Phase) {
        println!(
            "MERCHANT SERVICES PHASE {phase:?} repair={} junk={}",
            self.repairs, self.junk
        );
        self.phase = phase;
        self.since = Instant::now();
        self.commit = false;
    }

    fn require_quiet(&self) -> Result<(), String> {
        if self.since.elapsed() < QUIET {
            return Err(format!(
                "services marker before900ms quiet: {:?}",
                self.phase
            ));
        }
        Ok(())
    }

    fn receive_marker(&mut self, app: &mut App, line: &str) -> Result<(), String> {
        bags::reject_runtime_error(line)?;
        if !line.starts_with("FIXTURE MERCHANT_SERVICES_") {
            return Ok(());
        }
        self.selected
            .ok_or("services marker before authenticated selection")?;
        match (&self.phase, line) {
            (Phase::Loading, "FIXTURE MERCHANT_SERVICES_LOADING") => self.send_loading(app),
            (Phase::Ready, "FIXTURE MERCHANT_SERVICES_READY") => self.advance(Phase::Open),
            (Phase::VendorOpen, "FIXTURE MERCHANT_SERVICES_VENDOR_OPEN") => {
                self.advance(Phase::RepairReady)
            }
            (Phase::RepairReady, "FIXTURE MERCHANT_SERVICES_REPAIR_ARM") => {
                self.advance(Phase::RepairRequest)
            }
            (Phase::RepairRequest, "FIXTURE MERCHANT_SERVICES_REPAIR_COMMIT")
            | (Phase::JunkRequest, "FIXTURE MERCHANT_SERVICES_JUNK_COMMIT") => {
                self.require_quiet()?;
                if self.commit {
                    return Err("duplicate services commit marker".into());
                }
                self.commit = true;
            }
            (Phase::RepairDelta, "FIXTURE MERCHANT_SERVICES_REPAIR_DONE") => {
                self.require_quiet()?;
                self.advance(Phase::RepairQuiet);
            }
            (Phase::RepairQuiet, "FIXTURE MERCHANT_SERVICES_JUNK_ARM") => {
                self.require_quiet()?;
                self.advance(Phase::JunkRequest);
            }
            (Phase::JunkDelta, "FIXTURE MERCHANT_SERVICES_DONE") => {
                self.require_quiet()?;
                self.advance(Phase::Drain);
            }
            _ => {
                return Err(format!(
                    "services marker out of order {line} in {:?}",
                    self.phase
                ));
            }
        }
        Ok(())
    }

    fn send_loading(&mut self, app: &mut App) {
        app.world_mut()
            .entity_mut(self.selected.unwrap())
            .insert(Gold(1000));
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

    fn receive_interaction(&mut self, app: &mut App) -> Result<(), String> {
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
                "unexpected services Close/Cast: {closes:?} {casts:?}"
            ));
        }
        for request in interactions {
            let owned = vendor.map(Entity::to_bits) == Some(NPC);
            let first_open = self.phase == Phase::Open && self.opens == 0;
            if !owned || !first_open || request.npc != NPC {
                return Err(format!(
                    "unexpected services interaction {request:?} in {:?}",
                    self.phase
                ));
            }
            self.opens += 1;
            send_vendor(app);
            self.advance(Phase::VendorOpen);
        }
        Ok(())
    }

    fn receive_requests(&mut self, app: &mut App) -> Result<(), String> {
        let requests = std::mem::take(&mut *app.world_mut().resource_mut::<Requests>());
        if !requests.forbidden.is_empty() {
            return Err(format!(
                "forbidden services request: {:?}",
                requests.forbidden
            ));
        }
        for request in requests.repairs {
            let expected = RepairItem {
                npc: NPC,
                item_guid: None,
            };
            let first = self.phase == Phase::RepairRequest && self.repairs == 0;
            if !first || request != expected {
                return Err(format!(
                    "unexpected/duplicate services Repair {request:?} in {:?}",
                    self.phase
                ));
            }
            self.repairs += 1;
            println!("MERCHANT SERVICES DECODED {request:?} count1; authority withheld");
        }
        self.receive_junk(requests.junk)
    }

    fn receive_junk(&mut self, requests: Vec<SellAllJunkItems>) -> Result<(), String> {
        for request in requests {
            let first = self.phase == Phase::JunkRequest && self.junk == 0;
            if !first || request.npc != NPC {
                return Err(format!(
                    "unexpected/duplicate services Junk {request:?} in {:?}",
                    self.phase
                ));
            }
            self.junk += 1;
            println!("MERCHANT SERVICES DECODED {request:?} count1; authority withheld");
        }
        Ok(())
    }

    fn send_committed_authority(&mut self, app: &mut App) {
        if !self.commit {
            return;
        }
        let selected = self.selected.unwrap();
        match self.phase {
            Phase::RepairRequest if self.repairs == 1 => {
                send_durability(app, 0);
                app.world_mut().entity_mut(selected).insert(Gold(984));
                self.advance(Phase::RepairDelta);
            }
            Phase::JunkRequest if self.junk == 1 => {
                send_junk_authority(app, selected);
                self.advance(Phase::JunkDelta);
            }
            _ => {}
        }
    }

    fn require_request_before_timeout(&self) -> Result<(), String> {
        let waiting = match self.phase {
            Phase::RepairRequest => self.repairs == 0,
            Phase::JunkRequest => self.junk == 0,
            _ => false,
        };
        if waiting && self.since.elapsed() >= REQUEST_WAIT {
            return Err(format!(
                "services request timeout phase={:?} repair={} junk={}; authority unchanged",
                self.phase, self.repairs, self.junk
            ));
        }
        Ok(())
    }
}

/// Call once after root selection creates the same owned vendor as merchant-cursor.
pub(super) fn setup(app: &mut App) -> Result<(), String> {
    let vendor = app
        .world()
        .resource::<Incoming>()
        .vendor
        .ok_or("services requires owned vendor")?;
    if vendor.to_bits() != NPC {
        return Err(format!(
            "services vendor allocation changed: expected{NPC}, got{}",
            vendor.to_bits()
        ));
    }
    app.world_mut()
        .entity_mut(vendor)
        .insert(NpcFlags(NpcFlags::VENDOR | NpcFlags::REPAIR));
    Ok(())
}

fn bag_item(slot: u32, item_id: u32, count: u32) -> BagSlotItem {
    BagSlotItem {
        slot,
        item: ItemStack {
            item_guid: 9_180_000 + u64::from(item_id),
            item_id,
            count,
            durability: None,
            soulbound: false,
        },
    }
}

fn send_durability(app: &mut App, cost: u32) {
    send::<_, DurabilityChannel>(
        app,
        DurabilityStateUpdate {
            snapshot: Some(DurabilitySnapshot {
                total_repair_cost: cost,
                slots: vec![],
            }),
            message: None,
            error: None,
        },
    );
}

fn send_vendor(app: &mut App) {
    send::<_, InteractionChannel>(
        app,
        InteractionOpened {
            npc: NPC,
            kind: InteractionKind::Role(NpcRole::Vendor),
        },
    );
    send::<_, InventoryChannel>(
        app,
        InventorySnapshot {
            bags: vec![BagContents {
                bag: 0,
                size: 16,
                items: vec![bag_item(0, 4865, 2), bag_item(1, 2589, 3)],
            }],
        },
    );
    send::<_, MerchantChannel>(
        app,
        VendorInventory {
            npc: NPC,
            can_repair: true,
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
    send_durability(app, 16);
}

fn send_junk_authority(app: &mut App, selected: Entity) {
    send::<_, InventoryChannel>(
        app,
        InventoryDelta {
            changes: vec![InventorySlotChange {
                location: ItemLocation::Bag { bag: 0, slot: 0 },
                item: None,
            }],
        },
    );
    app.world_mut()
        .entity_mut(selected)
        .insert(Gold(FINAL_GOLD));
    send::<_, MerchantChannel>(
        app,
        BuybackList {
            items: vec![BuybackItem {
                slot: 0,
                item_id: 4865,
                name: "Ruined Pelt".into(),
                quality: 0,
                count: 2,
                price: (2 * PELT_PRICE) as u32,
            }],
        },
    );
    println!(
        "MERCHANT SERVICES AUTHORITY Poor0 Linen3 Gold{FINAL_GOLD}; fixture input SellPrice{PELT_PRICE}, not server pricing proof"
    );
}

fn tick_peer(
    app: &mut App,
    session: &mut Session,
    remote: &mut Option<Entity>,
) -> Result<(), String> {
    app.update();
    respond_to_login(app, StartupScreen::MerchantServices)?;
    respond_to_selection(
        app,
        StartupScreen::MerchantServices,
        &mut session.selected,
        remote,
    )?;
    if session.selected.is_some() && !session.configured {
        setup(app)?;
        session.configured = true;
    }
    Ok(())
}

fn run_iteration(
    app: &mut App,
    child: &mut Child,
    lines: &Receiver<String>,
    session: &mut Session,
    remote: &mut Option<Entity>,
) -> Result<(), String> {
    tick_peer(app, session, remote)?;
    for line in lines.try_iter() {
        session.receive_marker(app, line.trim())?;
    }
    session.receive_interaction(app)?;
    session.receive_requests(app)?;
    session.send_committed_authority(app);
    session.require_request_before_timeout()?;
    if let Some(status) = child
        .try_wait()
        .map_err(|e| format!("inspect services child: {e}"))?
    {
        return Err(format!(
            "services child exited before owned forced cleanup: {status}, phase={:?}",
            session.phase
        ));
    }
    Ok(())
}

fn run_until_done(
    app: &mut App,
    child: &mut Child,
    lines: &Receiver<String>,
) -> Result<(), String> {
    app.init_resource::<Requests>();
    app.add_systems(
        Update,
        (
            receive_services,
            receive_merchant_rejections,
            receive_inventory_rejections,
        ),
    );
    let mut session = Session {
        phase: Phase::Loading,
        since: Instant::now(),
        selected: None,
        opens: 0,
        repairs: 0,
        junk: 0,
        commit: false,
        configured: false,
    };
    let mut remote = None;
    let deadline = Instant::now() + TIMEOUT + Duration::from_secs(60);
    while Instant::now() < deadline {
        run_iteration(app, child, lines, &mut session, &mut remote)?;
        if session.phase == Phase::Drain && session.since.elapsed() >= QUIET {
            if session.opens != 1 || session.repairs != 1 || session.junk != 1 {
                return Err("services final request totals differ from Open1 Repair1 Junk1".into());
            }
            return Ok(());
        }
        thread::sleep(TICK);
    }
    Err(format!(
        "services timeout phase={:?} repair={} junk={}",
        session.phase, session.repairs, session.junk
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
            result = Err(error);
        }
        if line.starts_with("FIXTURE MERCHANT_SERVICES_") {
            result = Err(format!("services marker after cleanup: {line}"));
        }
    }
    match (result, cleanup) {
        (Err(error), Err(cleanup)) => Err(format!("{error}; owned cleanup failed: {cleanup}")),
        (Err(error), _) | (_, Err(error)) => Err(error),
        (Ok(()), Ok(())) => {
            println!(
                "MERCHANT SERVICES Open1 Repair1 Junk1 authority barriers passed; forced cleanup, NOT shutdown proof"
            );
            Ok(())
        }
    }
}
