//! Authenticated owned-NPC hover proof; the peer seeds state, never awards items.
use super::*;
use shared::protocol::{
    BagSlotItem, BuyItem, BuybackItem, BuybackItemRequest, BuybackList, DestroyItem,
    DurabilityChannel, DurabilitySnapshot, DurabilityStateUpdate, EquipItem, InteractionClosed,
    ItemStack, RepairItem, SellAllJunkItems, SellItem, SortBags, SplitItem, SwapItem, UseItem,
};

const QUIET: Duration = Duration::from_millis(900);
const MARKERS: &[&str] = &[
    "VENDOR_OPEN",
    "VENDOR",
    "SINGLE",
    "WEAPON",
    "EMPTY_CELL",
    "REFRESH_ARM",
    "REFRESHED",
    "BUYBACK",
    "BUYBACK_SINGLE",
    "BUYBACK_EMPTY",
    "BAG",
    "BAG_EMPTY",
    "AWAY",
    "SERVICE",
    "REPAIR_ITEM",
    "REPAIR_ALL_EMPTY",
    "REPAIR_ALL_SHORT",
    "LAST_BUYBACK",
    "CLOSE_ARM",
    "DONE",
];

#[derive(Resource, Default)]
struct Requests(Vec<String>);

fn collect<M: network::Message + std::fmt::Debug>(
    receivers: &mut Query<&mut MessageReceiver<M>>,
    requests: &mut Requests,
) {
    for mut receiver in receivers.iter_mut() {
        requests.0.extend(
            receiver
                .receive()
                .map(|message| format!("{} {message:?}", std::any::type_name::<M>())),
        );
    }
}

fn receive_merchant(
    mut buys: Query<&mut MessageReceiver<BuyItem>>,
    mut sells: Query<&mut MessageReceiver<SellItem>>,
    mut buybacks: Query<&mut MessageReceiver<BuybackItemRequest>>,
    mut repairs: Query<&mut MessageReceiver<RepairItem>>,
    mut junk: Query<&mut MessageReceiver<SellAllJunkItems>>,
    mut requests: ResMut<Requests>,
) {
    collect(&mut buys, &mut requests);
    collect(&mut sells, &mut requests);
    collect(&mut buybacks, &mut requests);
    collect(&mut repairs, &mut requests);
    collect(&mut junk, &mut requests);
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
    collect(&mut swaps, &mut requests);
    collect(&mut splits, &mut requests);
    collect(&mut destroys, &mut requests);
    collect(&mut equips, &mut requests);
    collect(&mut uses, &mut requests);
    collect(&mut sorts, &mut requests);
}

struct Session {
    selected: Option<Entity>,
    configured: bool,
    loading: bool,
    ready: bool,
    opens: usize,
    marker: usize,
    since: Instant,
}

impl Session {
    fn npc(&self, app: &App) -> Result<u64, String> {
        app.world()
            .resource::<Incoming>()
            .vendor
            .map(Entity::to_bits)
            .ok_or_else(|| "merchant-tooltips requires an owned replicated vendor".into())
    }

    fn receive_marker(&mut self, app: &mut App, line: &str) -> Result<(), String> {
        bags::reject_runtime_error(line)?;
        let Some(marker) = line.strip_prefix("FIXTURE MERCHANT_TOOLTIPS_") else {
            return Ok(());
        };
        self.selected
            .ok_or("tooltip marker before authenticated selection")?;
        match marker {
            "LOADING" if !self.loading => self.send_loading(app),
            "READY" if self.loading && !self.ready => self.ready = true,
            _ => return self.receive_hover_marker(app, marker),
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
        self.loading = true;
    }

    fn receive_hover_marker(&mut self, app: &mut App, marker: &str) -> Result<(), String> {
        let expected = MARKERS.get(self.marker).copied();
        if self.opens != 1 || expected != Some(marker) {
            return Err(format!(
                "tooltip marker {marker}, expected{expected:?}, opens{}",
                self.opens
            ));
        }
        // VENDOR_OPEN acknowledges initial projection. Every later marker follows
        // a full900ms actual display/hidden invariant, including both authority arms.
        if self.marker > 0 && self.since.elapsed() < QUIET {
            return Err(format!("tooltip marker {marker} before900ms quiet"));
        }
        let npc = self.npc(app)?;
        match marker {
            "REFRESH_ARM" => send_catalog(app, npc, 2, 0),
            // Repair All costs more than the seeded Gold1000 (MF.xml:246-248).
            "REPAIR_ALL_EMPTY" => send::<_, DurabilityChannel>(
                app,
                DurabilityStateUpdate {
                    snapshot: Some(DurabilitySnapshot {
                        total_repair_cost: 1016,
                        slots: vec![],
                    }),
                    message: None,
                    error: None,
                },
            ),
            "CLOSE_ARM" => send::<_, InteractionChannel>(app, InteractionClosed { npc }),
            _ => {}
        }
        self.marker += 1;
        self.since = Instant::now();
        println!("MERCHANT TOOLTIPS PHASE {marker}; Buy/Sell/Buyback/Repair/Junk/Inventory=0");
        Ok(())
    }

    fn receive_requests(&mut self, app: &mut App) -> Result<(), String> {
        let (interactions, closes, casts) = {
            let mut incoming = app.world_mut().resource_mut::<Incoming>();
            (
                std::mem::take(&mut incoming.interactions),
                std::mem::take(&mut incoming.closes),
                std::mem::take(&mut incoming.casts),
            )
        };
        if !closes.is_empty() || !casts.is_empty() {
            return Err(format!(
                "tooltip sent unexpected Close/Cast: {closes:?} {casts:?}"
            ));
        }
        for request in interactions {
            let npc = self.npc(app)?;
            if !self.ready || self.opens != 0 || request.npc != npc {
                return Err(format!(
                    "unexpected tooltip InteractNpc {request:?}, opens{}",
                    self.opens
                ));
            }
            self.opens += 1;
            send_vendor(app, npc);
            self.since = Instant::now();
        }
        let forbidden = std::mem::take(&mut app.world_mut().resource_mut::<Requests>().0);
        if !forbidden.is_empty() {
            return Err(format!(
                "tooltip hover must send zero transactions: {forbidden:?}"
            ));
        }
        Ok(())
    }
}

/// MAIN calls after the parent creates its merchant-mode NPC; no new spawn/allocation.
pub(super) fn setup(app: &mut App) -> Result<(), String> {
    let vendor = app
        .world()
        .resource::<Incoming>()
        .vendor
        .ok_or("tooltip vendor missing")?;
    app.world_mut()
        .entity_mut(vendor)
        .insert(NpcFlags(NpcFlags::VENDOR | NpcFlags::REPAIR));
    Ok(())
}

fn send_catalog(app: &mut App, npc: u64, count: u32, stock: u32) {
    send::<_, MerchantChannel>(
        app,
        VendorInventory {
            npc,
            can_repair: true,
            guild_repair_money: None,
            items: vec![
                VendorItem {
                    slot: 0,
                    item_id: 2589,
                    name: "Fixture Linen Bundle".into(),
                    quality: 2,
                    price: 65,
                    stack_count: count,
                    max_stack: 1000,
                    num_available: Some(stock),
                    usable: true,
                    max_durability: None,
                },
                VendorItem {
                    slot: 1,
                    item_id: 4865,
                    name: "Fixture Single Pelt".into(),
                    quality: 0,
                    price: 5,
                    stack_count: 1,
                    max_stack: 20,
                    num_available: None,
                    usable: true,
                    max_durability: None,
                },
                // Worn Shortsword: SetMerchantItem shows a new item's full stats and durability.
                VendorItem {
                    slot: 2,
                    item_id: 25,
                    name: "Fixture Vendor Sword".into(),
                    quality: 1,
                    price: 13,
                    stack_count: 1,
                    max_stack: 1,
                    num_available: None,
                    usable: true,
                    max_durability: Some(20),
                },
            ],
        },
    );
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
                items: vec![BagSlotItem {
                    slot: 0,
                    item: ItemStack {
                        item_guid: 9_190_4865,
                        item_id: 4865,
                        count: 2,
                        durability: None,
                        soulbound: false,
                    },
                }],
            }],
        },
    );
    send_catalog(app, npc, 5, 7);
    send::<_, MerchantChannel>(
        app,
        BuybackList {
            items: vec![
                BuybackItem {
                    slot: 4,
                    item_id: 2589,
                    name: "Fixture Returned Linen".into(),
                    quality: 4,
                    count: 3,
                    price: 39,
                },
                BuybackItem {
                    slot: 8,
                    item_id: 4865,
                    name: "Fixture Returned Pelt".into(),
                    quality: 1,
                    count: 1,
                    price: 5,
                },
            ],
        },
    );
}

fn tick_peer(
    app: &mut App,
    session: &mut Session,
    remote: &mut Option<Entity>,
) -> Result<(), String> {
    app.update();
    respond_to_login(app, StartupScreen::MerchantTooltips)?;
    respond_to_selection(
        app,
        StartupScreen::MerchantTooltips,
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
    lines: &ClientLines,
    session: &mut Session,
    remote: &mut Option<Entity>,
) -> Result<(), String> {
    tick_peer(app, session, remote)?;
    for line in lines.after_selection(session.selected.is_some()) {
        session.receive_marker(app, line.trim())?;
    }
    session.receive_requests(app)?;
    if let Some(status) = child
        .try_wait()
        .map_err(|e| format!("inspect tooltip child: {e}"))?
    {
        return Err(format!(
            "tooltip child exited before owned forced cleanup: {status}, marker{}",
            session.marker
        ));
    }
    Ok(())
}

fn run_until_done(app: &mut App, child: &mut Child, lines: &ClientLines) -> Result<(), String> {
    app.init_resource::<Requests>();
    app.add_systems(Update, (receive_merchant, receive_inventory));
    let mut session = Session {
        selected: None,
        configured: false,
        loading: false,
        ready: false,
        opens: 0,
        marker: 0,
        since: Instant::now(),
    };
    let mut remote = None;
    let deadline = Instant::now() + TIMEOUT + Duration::from_secs(90);
    while Instant::now() < deadline {
        run_iteration(app, child, lines, &mut session, &mut remote)?;
        if session.marker == MARKERS.len() && session.since.elapsed() >= QUIET {
            return Ok(());
        }
        thread::sleep(TICK);
    }
    Err(format!(
        "tooltip timeout marker{} opens{}",
        session.marker, session.opens
    ))
}

pub(super) fn run(
    app: &mut App,
    child: &mut Child,
    lines: ClientLines,
    readers: Vec<thread::JoinHandle<()>>,
) -> Result<(), String> {
    let result = run_until_done(app, child, &lines);
    let cleanup = bags::cleanup_owned_child(child, readers);
    let drain = lines
        .remaining()
        .into_iter()
        .try_for_each(|line| bags::reject_runtime_error(line.trim()));
    result.and(cleanup).and(drain)?;
    println!("MERCHANT TOOLTIPS Open1 and zero transactions; forced cleanup, NOT shutdown proof");
    Ok(())
}
