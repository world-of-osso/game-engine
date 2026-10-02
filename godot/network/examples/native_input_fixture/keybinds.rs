//! Key binding peer: held bags 0/1/2/4 (no bag 3), a second NPC, the vendor targeting
//! the remote player, a quest-giver interaction on request. The flow asserts behavior.
use super::*;
use shared::components::UnitTarget;
use shared::protocol::{BagSlotItem, ItemStack};

#[derive(Debug, PartialEq, Eq)]
enum Phase {
    Loading,
    Ready,
    Running,
    Complete,
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

fn send_inventory(app: &mut App) {
    let bag = |bag: u8, size: u8, guid: u64| BagContents {
        bag,
        size,
        items: vec![BagSlotItem {
            slot: 0,
            item: linen(guid, 2),
        }],
    };
    send::<_, InventoryChannel>(
        app,
        InventorySnapshot {
            bags: vec![
                bag(0, 16, 7_200_001),
                bag(1, 8, 7_200_002),
                bag(2, 10, 7_200_003),
                bag(4, 12, 7_200_004),
            ],
        },
    );
}

/// A second NPC farther than the vendor, so Tab and Shift-Tab have a cycle to walk.
fn spawn_wolf(app: &mut App) {
    app.world_mut().spawn((
        Npc {
            template_id: 299,
            name: "Fixture Wolf".into(),
        },
        UnitFactionTemplate(35),
        ModelDisplay { display_id: 26 },
        Position {
            x: FIRST[0] + 4.0,
            y: FIRST[1],
            z: FIRST[2] + 6.0,
        },
        Replicate::to_clients(NetworkTarget::All),
    ));
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
            guild_repair_money: None,
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
                max_durability: None,
            }],
        },
    );
}

/// Bags, the wolf, and the vendor's own target (the remote player) for assist.
fn prepare_world(
    app: &mut App,
    vendor: Option<Entity>,
    remote: Option<Entity>,
) -> Result<(), String> {
    send_inventory(app);
    spawn_wolf(app);
    let vendor = vendor.ok_or("keybinds world has no vendor")?;
    let remote = remote.ok_or("keybinds world has no remote player")?;
    app.world_mut()
        .entity_mut(vendor)
        .insert(UnitTarget(Some(remote.to_bits())));
    Ok(())
}

fn observe(
    app: &mut App,
    line: &str,
    phase: &mut Phase,
    remote: Option<Entity>,
) -> Result<(), String> {
    bags::reject_runtime_error(line)?;
    let vendor = app.world().resource::<Incoming>().vendor;
    match line {
        "FIXTURE KEYBINDS_LOADING" if *phase == Phase::Loading => {
            send::<_, TerrainChannel>(
                app,
                LoadTerrain {
                    map_name: "azeroth".into(),
                    initial_tile_y: 32,
                    initial_tile_x: 48,
                },
            );
            *phase = Phase::Ready;
        }
        "FIXTURE KEYBINDS_READY" if *phase == Phase::Ready => {
            prepare_world(app, vendor, remote)?;
            *phase = Phase::Running;
        }
        "FIXTURE KEYBINDS_QUEST_OPEN" if *phase == Phase::Running => {
            let npc = vendor.ok_or("quest open without vendor")?.to_bits();
            send::<_, InteractionChannel>(
                app,
                InteractionOpened {
                    npc,
                    kind: InteractionKind::Role(NpcRole::QuestGiver),
                },
            );
        }
        "FIXTURE KEYBINDS_DONE" if *phase == Phase::Running => *phase = Phase::Complete,
        other if other.starts_with("FIXTURE KEYBINDS_") => {
            return Err(format!(
                "out-of-order keybinds marker in {phase:?}: {other}"
            ));
        }
        _ => {}
    }
    Ok(())
}

fn run_until_done(app: &mut App, child: &mut Child, lines: &ClientLines) -> Result<(), String> {
    let mut selected = None;
    let mut remote = None;
    let mut phase = Phase::Loading;
    // Cold CASC/authentication, world map tile extraction for three zones, then input.
    let deadline = Instant::now() + TIMEOUT + Duration::from_secs(300);
    while Instant::now() < deadline {
        app.update();
        respond_to_login(app, StartupScreen::Keybinds)?;
        respond_to_selection(app, StartupScreen::Keybinds, &mut selected, &mut remote)?;
        for line in lines.after_selection(selected.is_some()) {
            observe(app, line.trim(), &mut phase, remote)?;
        }
        let interactions =
            std::mem::take(&mut app.world_mut().resource_mut::<Incoming>().interactions);
        for request in interactions {
            send_vendor(app, request.npc);
        }
        if let Some(status) = child
            .try_wait()
            .map_err(|error| format!("inspect keybinds child: {error}"))?
        {
            return Err(format!(
                "keybinds child exited before deliberate cleanup: {status}; phase={phase:?}"
            ));
        }
        if phase == Phase::Complete {
            return Ok(());
        }
        thread::sleep(TICK);
    }
    Err(format!("keybinds fixture timed out; phase={phase:?}"))
}

pub(super) fn run(
    app: &mut App,
    child: &mut Child,
    lines: ClientLines,
    readers: Vec<thread::JoinHandle<()>>,
) -> Result<(), String> {
    let result = run_until_done(app, child, &lines);
    let cleanup = bags::cleanup_owned_child(child, readers);
    result?;
    cleanup?;
    println!(
        "PASS: KEYBINDS bag/targeting bindings, three tiled world maps and quest window raise; deliberate cleanup complete"
    );
    Ok(())
}
