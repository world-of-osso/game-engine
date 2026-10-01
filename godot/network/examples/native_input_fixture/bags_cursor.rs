//! Authored standalone slots → decoded InventoryChannel requests → authoritative deltas.
use super::*;
use shared::protocol::{
    BagSlotItem, DestroyItem, EquipItem, InventoryDelta, InventorySlotChange, ItemLocation,
    SortBags, SplitItem, SwapItem, UseItem,
};

const QUIET: Duration = Duration::from_millis(400);
const REQUEST_WAIT: Duration = Duration::from_secs(6);
const SOURCE: ItemLocation = ItemLocation::Bag { bag: 0, slot: 0 };
const SWAP_TARGET: ItemLocation = ItemLocation::Bag { bag: 1, slot: 0 };
const SPLIT_TARGET: ItemLocation = ItemLocation::Bag { bag: 1, slot: 1 };

#[derive(Resource, Default)]
struct Requests {
    swaps: Vec<SwapItem>,
    splits: Vec<SplitItem>,
    forbidden: Vec<String>,
}

fn receive(
    mut swaps: Query<&mut MessageReceiver<SwapItem>>,
    mut splits: Query<&mut MessageReceiver<SplitItem>>,
    mut destroys: Query<&mut MessageReceiver<DestroyItem>>,
    mut equips: Query<&mut MessageReceiver<EquipItem>>,
    mut uses: Query<&mut MessageReceiver<UseItem>>,
    mut sorts: Query<&mut MessageReceiver<SortBags>>,
    mut requests: ResMut<Requests>,
) {
    for mut receiver in &mut swaps {
        requests.swaps.extend(receiver.receive());
    }
    for mut receiver in &mut splits {
        requests.splits.extend(receiver.receive());
    }
    for mut receiver in &mut destroys {
        requests
            .forbidden
            .extend(receiver.receive().map(|r| format!("DestroyItem {r:?}")));
    }
    for mut receiver in &mut equips {
        requests
            .forbidden
            .extend(receiver.receive().map(|r| format!("EquipItem {r:?}")));
    }
    for mut receiver in &mut uses {
        requests
            .forbidden
            .extend(receiver.receive().map(|r| format!("UseItem {r:?}")));
    }
    for mut receiver in &mut sorts {
        requests
            .forbidden
            .extend(receiver.receive().map(|r| format!("SortBags {r:?}")));
    }
}

#[derive(Debug, PartialEq, Eq)]
enum Phase {
    Loading,
    Ready,
    Pickup,
    Return,
    EscapeStart,
    Escape,
    Stale,
    StaleCleared,
    SwapArm,
    SwapRequest,
    SwapDone,
    SplitArm,
    SplitRequest,
    SplitDone,
    Done,
    Complete,
}

struct Session {
    phase: Phase,
    since: Instant,
    swaps: usize,
    splits: usize,
}

impl Session {
    fn advance(&mut self, phase: Phase) {
        self.phase = phase;
        self.since = Instant::now();
    }

    fn require_quiet(&self) -> Result<(), String> {
        if self.since.elapsed() < QUIET {
            return Err(format!("cursor quiet phase too short: {:?}", self.phase));
        }
        println!(
            "BAGS CURSOR QUIET {:?} swaps={} splits={}",
            self.phase, self.swaps, self.splits
        );
        Ok(())
    }

    fn observe(&mut self, app: &mut App, line: &str, selected: bool) -> Result<(), String> {
        bags::reject_runtime_error(line)?;
        match (line, &self.phase) {
            ("FIXTURE BAGS_CURSOR_LOADING", Phase::Loading) if selected => {
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
            ("FIXTURE BAGS_CURSOR_READY", Phase::Ready) => {
                send_snapshot(app);
                self.advance(Phase::Pickup);
            }
            ("FIXTURE BAGS_CURSOR_PICKUP", Phase::Pickup) => self.advance(Phase::Return),
            ("FIXTURE BAGS_CURSOR_RETURN", Phase::Return) => {
                self.require_quiet()?;
                self.advance(Phase::EscapeStart);
            }
            ("FIXTURE BAGS_CURSOR_ESCAPE_START", Phase::EscapeStart) => self.advance(Phase::Escape),
            ("FIXTURE BAGS_CURSOR_ESCAPE", Phase::Escape) => {
                self.require_quiet()?;
                self.advance(Phase::Stale);
            }
            ("FIXTURE BAGS_CURSOR_STALE", Phase::Stale) => {
                send_delta(app, &[(SOURCE, None)]);
                self.advance(Phase::StaleCleared);
            }
            ("FIXTURE BAGS_CURSOR_STALE_CLEARED", Phase::StaleCleared) => {
                send_delta(app, &[(SOURCE, Some(3))]);
                self.advance(Phase::SwapArm);
            }
            ("FIXTURE BAGS_CURSOR_SWAP_ARM", Phase::SwapArm) => self.advance(Phase::SwapRequest),
            ("FIXTURE BAGS_CURSOR_SWAP_DONE", Phase::SwapDone) => {
                send_delta(app, &[(SOURCE, Some(3)), (SWAP_TARGET, None)]);
                self.advance(Phase::SplitArm);
            }
            ("FIXTURE BAGS_CURSOR_SPLIT_ARM", Phase::SplitArm) => self.advance(Phase::SplitRequest),
            ("FIXTURE BAGS_CURSOR_SPLIT_DONE", Phase::SplitDone) => self.advance(Phase::Done),
            ("FIXTURE BAGS_CURSOR_DONE", Phase::Done) => {
                self.require_quiet()?;
                self.advance(Phase::Complete);
            }
            (other, _) if other.starts_with("FIXTURE BAGS_CURSOR_") => {
                return Err(format!(
                    "out-of-order cursor marker in {:?}: {other}",
                    self.phase
                ));
            }
            _ => {}
        }
        Ok(())
    }

    fn respond(&mut self, app: &mut App) -> Result<(), String> {
        let requests = std::mem::take(&mut *app.world_mut().resource_mut::<Requests>());
        if !requests.forbidden.is_empty() {
            return Err(format!(
                "forbidden cursor requests: {:?}",
                requests.forbidden
            ));
        }
        reject_unrelated_requests(app)?;
        for request in requests.swaps {
            self.respond_swap(app, request)?;
        }
        for request in requests.splits {
            self.respond_split(app, request)?;
        }
        let awaiting_request = matches!(self.phase, Phase::SwapRequest | Phase::SplitRequest);
        if awaiting_request && self.since.elapsed() > REQUEST_WAIT {
            return Err(format!("no decoded exact request in {:?}", self.phase));
        }
        Ok(())
    }

    fn respond_swap(&mut self, app: &mut App, request: SwapItem) -> Result<(), String> {
        let expected = SwapItem {
            from: SOURCE,
            to: SWAP_TARGET,
        };
        if self.phase != Phase::SwapRequest || self.swaps != 0 || request != expected {
            return Err(format!(
                "unexpected SwapItem {request:?} in {:?}; count={}",
                self.phase, self.swaps
            ));
        }
        self.swaps += 1;
        println!(
            "BAGS CURSOR DECODED SwapItem {request:?} count={}",
            self.swaps
        );
        send_delta(app, &[(SOURCE, None), (SWAP_TARGET, Some(3))]);
        self.advance(Phase::SwapDone);
        Ok(())
    }

    fn respond_split(&mut self, app: &mut App, request: SplitItem) -> Result<(), String> {
        let expected = SplitItem {
            from: SOURCE,
            to: SPLIT_TARGET,
            count: 2,
        };
        if self.phase != Phase::SplitRequest || self.splits != 0 || request != expected {
            return Err(format!(
                "unexpected SplitItem {request:?} in {:?}; count={}",
                self.phase, self.splits
            ));
        }
        self.splits += 1;
        println!(
            "BAGS CURSOR DECODED SplitItem {request:?} count={}",
            self.splits
        );
        send_delta(app, &[(SOURCE, Some(1)), (SPLIT_TARGET, Some(2))]);
        self.advance(Phase::SplitDone);
        Ok(())
    }
}

fn reject_unrelated_requests(app: &mut App) -> Result<(), String> {
    let incoming = app.world().resource::<Incoming>();
    if !incoming.interactions.is_empty()
        || !incoming.closes.is_empty()
        || !incoming.casts.is_empty()
    {
        return Err("cursor fixture emitted interaction/close/spell request".into());
    }
    Ok(())
}

fn send_snapshot(app: &mut App) {
    send::<_, InventoryChannel>(
        app,
        InventorySnapshot {
            bags: vec![
                BagContents {
                    bag: 0,
                    size: 16,
                    items: vec![BagSlotItem {
                        slot: 0,
                        item: loot::candle_stack(3),
                    }],
                },
                BagContents {
                    bag: 1,
                    size: 8,
                    items: vec![],
                },
            ],
        },
    );
}

fn send_delta(app: &mut App, slots: &[(ItemLocation, Option<u32>)]) {
    let changes = slots
        .iter()
        .map(|&(location, count)| {
            let item = count.map(|count| {
                let mut item = loot::candle_stack(count);
                if location == SPLIT_TARGET {
                    item.item_guid += 1;
                }
                item
            });
            InventorySlotChange { location, item }
        })
        .collect();
    send::<_, InventoryChannel>(app, InventoryDelta { changes });
}

fn run_until_done(
    app: &mut App,
    child: &mut Child,
    lines: &Receiver<String>,
) -> Result<(), String> {
    app.init_resource::<Requests>();
    app.add_systems(Update, receive);
    let mut selected = None;
    let mut remote = None;
    let mut session = Session {
        phase: Phase::Loading,
        since: Instant::now(),
        swaps: 0,
        splits: 0,
    };
    let deadline = Instant::now() + TIMEOUT + Duration::from_secs(180);
    while Instant::now() < deadline {
        app.update();
        respond_to_login(app, StartupScreen::BagsCursor)?;
        respond_to_selection(app, StartupScreen::BagsCursor, &mut selected, &mut remote)?;
        for line in lines.try_iter() {
            session.observe(app, line.trim(), selected.is_some())?;
        }
        session.respond(app)?;
        if let Some(status) = child
            .try_wait()
            .map_err(|error| format!("inspect cursor child: {error}"))?
        {
            return Err(format!(
                "cursor child exited before deliberate cleanup: {status}; phase={:?}",
                session.phase
            ));
        }
        if session.phase == Phase::Complete {
            if session.swaps != 1 || session.splits != 1 {
                return Err(format!(
                    "cursor request totals: swaps={} splits={}",
                    session.swaps, session.splits
                ));
            }
            return Ok(());
        }
        thread::sleep(TICK);
    }
    Err(format!(
        "cursor fixture timed out; phase={:?}",
        session.phase
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
    // Joined readers cannot enqueue later errors; inspect the entire remaining stream.
    for line in lines.try_iter() {
        if let Err(error) = bags::reject_runtime_error(line.trim()) {
            result = Err(match result {
                Ok(()) => error,
                Err(original) => format!("{original}; {error}"),
            });
        }
    }
    match (result, cleanup) {
        (Err(error), Err(cleanup)) => Err(format!("{error}; owned cleanup failed: {cleanup}")),
        (Err(error), _) | (_, Err(error)) => Err(error),
        (Ok(()), Ok(())) => {
            println!(
                "PASS: BAGS_CURSOR actual slots, source-return/Escape/stale quiet, exact SwapItem/SplitItem and authoritative rendering; deliberate kill/reap/readers drained, NOT normal shutdown proof"
            );
            Ok(())
        }
    }
}
