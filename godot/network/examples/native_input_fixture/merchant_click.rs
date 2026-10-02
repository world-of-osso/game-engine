//! Authenticated merchant pointer effects through an owned UDP server.
use super::*;

pub(super) fn run(
    app: &mut App,
    child: &mut Child,
    lines: ClientLines,
    readers: Vec<thread::JoinHandle<()>>,
) -> Result<(), String> {
    let mut selected = None;
    let mut remote = None;
    let mut loading = false;
    let mut passed = false;
    let mut opens = 0;
    let mut closes = 0;
    let mut readers = Some(readers);
    let deadline = Instant::now() + TIMEOUT;
    while Instant::now() < deadline {
        app.update();
        respond_to_login(app, StartupScreen::MerchantClick)?;
        respond_to_selection(
            app,
            StartupScreen::MerchantClick,
            &mut selected,
            &mut remote,
        )?;
        respond_to_vendor_interaction(app, &mut opens, &mut closes)?;
        let status = child.try_wait().map_err(|error| error.to_string())?;
        if status.is_some() {
            for reader in readers.take().expect("join output once") {
                reader.join().map_err(|_| "Godot output reader panicked")?;
            }
        }
        for line in lines.after_selection(selected.is_some()) {
            observe_line(app, line.trim(), &mut loading, &mut passed)?;
        }
        if let Some(status) = status {
            if status.success() && passed && opens == 3 && closes == 3 {
                println!(
                    "PASS: owned vendor interaction, merchant placement/reset, pointer effects and quiet reopen"
                );
                return Ok(());
            }
            return Err(format!(
                "merchant-click fixture exited {status}; loading={loading}, passed={passed}, opens={opens}, closes={closes}"
            ));
        }
        thread::sleep(TICK);
    }
    Err(format!(
        "timed out awaiting merchant-click fixture; loading={loading}, passed={passed}, opens={opens}, closes={closes}"
    ))
}

fn respond_to_vendor_interaction(
    app: &mut App,
    opens: &mut u32,
    closes: &mut u32,
) -> Result<(), String> {
    let (vendor, interactions, closed) = {
        let mut incoming = app.world_mut().resource_mut::<Incoming>();
        (
            incoming.vendor,
            std::mem::take(&mut incoming.interactions),
            std::mem::take(&mut incoming.closes),
        )
    };
    let vendor = vendor.map(Entity::to_bits);
    for request in interactions {
        if Some(request.npc) != vendor || *opens != *closes || *opens >= 3 {
            return Err(format!(
                "unexpected vendor interaction: {request:?}; vendor={vendor:?}, opens={opens}, closes={closes}"
            ));
        }
        *opens += 1;
        send_vendor_opened(app, request.npc);
        send_vendor_bag_snapshot(app);
        send_vendor_inventory(app, request.npc);
    }
    for request in closed {
        if Some(request.npc) != vendor || *closes >= *opens {
            return Err(format!(
                "unexpected vendor close: {request:?}; vendor={vendor:?}, opens={opens}, closes={closes}"
            ));
        }
        *closes += 1;
    }
    Ok(())
}

fn send_vendor_opened(app: &mut App, npc: u64) {
    send::<_, InteractionChannel>(
        app,
        InteractionOpened {
            npc,
            kind: InteractionKind::Role(NpcRole::Vendor),
        },
    );
}

fn send_vendor_bag_snapshot(app: &mut App) {
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
}

fn send_vendor_inventory(app: &mut App, npc: u64) {
    send::<_, MerchantChannel>(
        app,
        VendorInventory {
            npc,
            can_repair: false,
            guild_repair_money: None,
            items: vec![VendorItem {
                slot: 0,
                item_id: 4540,
                name: "Fixture Bread".into(),
                quality: 1,
                price: 25,
                stack_count: 5,
                max_stack: 20,
                num_available: None,
                usable: true,
                max_durability: None,
            }],
        },
    );
}

fn observe_line(
    app: &mut App,
    line: &str,
    loading: &mut bool,
    passed: &mut bool,
) -> Result<(), String> {
    let missing_scenery = line.starts_with("GODOT_STDERR: ERROR: WorldObjects:")
        && (line.contains("missing textures") || line.contains("particle textures missing"));
    if !missing_scenery
        && (line.starts_with("GODOT_STDERR: ERROR:")
            || line.starts_with("GODOT_STDERR: SCRIPT ERROR:"))
    {
        return Err(format!("Godot merchant-click runtime error: {line}"));
    }
    match line {
        "FIXTURE MERCHANT_CLICK_LOADING" if !*loading => {
            send::<_, TerrainChannel>(
                app,
                LoadTerrain {
                    map_name: "azeroth".into(),
                    initial_tile_y: 32,
                    initial_tile_x: 48,
                },
            );
            *loading = true;
        }
        "FIXTURE MERCHANT_CLICK_PLACED" if *loading => {}
        "FIXTURE MERCHANT_CLICK_DONE" if *loading => *passed = true,
        line if line.starts_with("FIXTURE MERCHANT_CLICK_") => {
            return Err(format!("out-of-order merchant-click marker: {line}"));
        }
        _ => {}
    }
    Ok(())
}
