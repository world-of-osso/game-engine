//! Independent standalone bags peer; no merchant, loot or production UI setters.
use super::*;
use shared::protocol::{BagSlotItem, ItemStack};

#[derive(Debug, PartialEq, Eq)]
enum Phase {
    Loading,
    Ready,
    Done,
    Complete,
}

fn send_inventory(app: &mut App) {
    let mut second = loot::candle_stack(2);
    second.item_guid += 1;
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
                            item: loot::candle_stack(3),
                        },
                        BagSlotItem {
                            slot: 1,
                            item: ItemStack {
                                item_guid: 9_180_001,
                                item_id: 2589,
                                count: 3,
                                durability: None,
                                soulbound: false,
                            },
                        },
                        BagSlotItem {
                            slot: 2,
                            item: ItemStack {
                                item_guid: 9_180_002,
                                item_id: 4865,
                                count: 1,
                                durability: None,
                                soulbound: false,
                            },
                        },
                    ],
                },
                BagContents {
                    bag: 1,
                    size: 8,
                    items: vec![BagSlotItem {
                        slot: 7,
                        item: second,
                    }],
                },
            ],
        },
    );
}

pub(super) fn reject_runtime_error(line: &str) -> Result<(), String> {
    let message = line
        .strip_prefix("GODOT_STDERR: ")
        .unwrap_or(line)
        .trim_start();
    if message.starts_with("ERROR:") || message.starts_with("SCRIPT ERROR:") {
        return Err(format!("Godot bags runtime error: {message}"));
    }
    Ok(())
}

fn observe(app: &mut App, line: &str, phase: &mut Phase) -> Result<(), String> {
    reject_runtime_error(line)?;
    match line {
        "FIXTURE BAGS_LOADING" if *phase == Phase::Loading => {
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
        "FIXTURE BAGS_READY" if *phase == Phase::Ready => {
            send_inventory(app);
            *phase = Phase::Done;
        }
        "FIXTURE BAGS_DONE" if *phase == Phase::Done => *phase = Phase::Complete,
        other if other.starts_with("FIXTURE BAGS_") => {
            return Err(format!("out-of-order bags marker in {phase:?}: {other}"));
        }
        _ => {}
    }
    Ok(())
}

fn run_until_done(app: &mut App, child: &mut Child, lines: &ClientLines) -> Result<(), String> {
    let mut selected = None;
    let mut remote = None;
    let mut phase = Phase::Loading;
    // Include cold CASC/authentication plus bounded terrain and authored HUD waits.
    let deadline = Instant::now() + TIMEOUT + Duration::from_secs(180);
    while Instant::now() < deadline {
        app.update();
        respond_to_login(app, StartupScreen::Bags)?;
        respond_to_selection(app, StartupScreen::Bags, &mut selected, &mut remote)?;
        for line in lines.after_selection(selected.is_some()) {
            observe(app, line.trim(), &mut phase)?;
        }
        if let Some(status) = child
            .try_wait()
            .map_err(|error| format!("inspect bags child: {error}"))?
        {
            return Err(format!(
                "bags child exited before deliberate cleanup: {status}; phase={phase:?}"
            ));
        }
        if phase == Phase::Complete {
            return Ok(());
        }
        thread::sleep(TICK);
    }
    Err(format!("bags fixture timed out; phase={phase:?}"))
}

pub(super) fn cleanup_owned_child(
    child: &mut Child,
    readers: Vec<thread::JoinHandle<()>>,
) -> Result<(), String> {
    let pid = child.id();
    let kill = child.kill();
    let wait = child.wait();
    let mut reader_errors = 0;
    for reader in readers {
        if reader.join().is_err() {
            reader_errors += 1;
        }
    }
    println!(
        "BAGS OWNED CLEANUP pid={pid} kill={kill:?} wait={wait:?} reader_errors={reader_errors}; intentional termination, not normal shutdown proof"
    );
    kill.map_err(|error| format!("kill owned bags child {pid}: {error}"))?;
    wait.map_err(|error| format!("reap owned bags child {pid}: {error}"))?;
    if reader_errors != 0 {
        return Err(format!("bags output readers panicked: {reader_errors}"));
    }
    Ok(())
}

pub(super) fn run(
    app: &mut App,
    child: &mut Child,
    lines: ClientLines,
    readers: Vec<thread::JoinHandle<()>>,
) -> Result<(), String> {
    let mut result = run_until_done(app, child, &lines);
    // Cleanup is unconditional, including runtime errors; readers must be joined.
    let cleanup = cleanup_owned_child(child, readers);
    for line in lines.remaining() {
        if let Err(error) = reject_runtime_error(line.trim()) {
            result = Err(match result {
                Ok(()) => error,
                Err(original) => format!("{original}; {error}"),
            });
            break;
        }
    }
    match (result, cleanup) {
        (Err(error), Err(cleanup)) => Err(format!("{error}; owned cleanup failed: {cleanup}")),
        (Err(error), _) | (_, Err(error)) => Err(error),
        (Ok(()), Ok(())) => {
            println!(
                "PASS: BAGS authored backpack/equipped container toggles, inventory, positions, item hover tooltips and Escape; deliberate cleanup complete"
            );
            Ok(())
        }
    }
}
