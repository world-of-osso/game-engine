//! Authored pointer presses/motion/releases; exact UDP swaps and peer-only authority.
use super::*;
use shared::protocol::{
    BagSlotItem, DestroyItem, EquipItem, InventoryDelta, InventorySlotChange, ItemLocation,
    SortBags, SplitItem, SwapItem, UseItem,
};

const QUIET: Duration = Duration::from_millis(400);
const REQUEST_WAIT: Duration = Duration::from_secs(10);
const SOURCE: ItemLocation = ItemLocation::Bag { bag: 0, slot: 0 };
const TARGET: ItemLocation = ItemLocation::Bag { bag: 1, slot: 0 };
const QUIET_MARKERS: [&str; 4] = [
    "SAME_SOURCE",
    "HELD_RELEASE",
    "FRAME_RELEASE",
    "FOREIGN_CHAT",
];
const CASES: [&str; 3] = ["SEPARATED", "RAPID", "CLICK"];

#[derive(Resource, Default)]
struct Requests {
    swaps: Vec<SwapItem>,
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
        requests
            .forbidden
            .extend(receiver.receive().map(|r| format!("SplitItem {r:?}")));
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
    Quiet(usize),
    Arm(usize),
    Request(usize),
    Delta(usize),
    Drain,
}

struct Session {
    phase: Phase,
    since: Instant,
    swaps: [usize; 3],
    commit_delta: bool,
}

impl Session {
    fn advance(&mut self, phase: Phase) {
        println!("BAGS DRAG PHASE {phase:?} swaps={:?}", self.swaps);
        self.phase = phase;
        self.since = Instant::now();
    }

    fn require_quiet(&self) -> Result<(), String> {
        if self.since.elapsed() < QUIET {
            return Err(format!("bags-drag quiet phase too short: {:?}", self.phase));
        }
        Ok(())
    }

    fn observe(&mut self, app: &mut App, line: &str, selected: bool) -> Result<(), String> {
        bags::reject_runtime_error(line)?;
        if !line.starts_with("FIXTURE BAGS_DRAG_") {
            return Ok(());
        }
        match self.phase {
            Phase::Loading if line == "FIXTURE BAGS_DRAG_LOADING" && selected => {
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
            Phase::Ready if line == "FIXTURE BAGS_DRAG_READY" => {
                send_snapshot(app);
                self.advance(Phase::Quiet(0));
            }
            Phase::Quiet(index)
                if line == format!("FIXTURE BAGS_DRAG_{}", QUIET_MARKERS[index]) =>
            {
                self.require_quiet()?;
                self.advance(if index == QUIET_MARKERS.len() - 1 {
                    Phase::Arm(0)
                } else {
                    Phase::Quiet(index + 1)
                });
            }
            Phase::Arm(index) if line == format!("FIXTURE BAGS_DRAG_{}_ARM", CASES[index]) => {
                self.commit_delta = false;
                self.advance(Phase::Request(index));
            }
            Phase::Request(index)
                if line == format!("FIXTURE BAGS_DRAG_{}_COMMIT", CASES[index])
                    && !self.commit_delta =>
            {
                self.require_quiet()?;
                self.commit_delta = true;
            }
            Phase::Delta(index) if line == format!("FIXTURE BAGS_DRAG_{}_DONE", CASES[index]) => {
                self.require_quiet()?;
                if index < 2 {
                    // Peer-owned reseed isolates each source0/0 -> target1/0 case.
                    send_snapshot(app);
                    self.advance(Phase::Arm(index + 1));
                } else {
                    self.advance(Phase::Drain);
                }
            }
            _ => {
                return Err(format!(
                    "out-of-order bags-drag marker in {:?}: {line}",
                    self.phase
                ));
            }
        }
        Ok(())
    }

    fn respond(&mut self, app: &mut App) -> Result<(), String> {
        let incoming = app.world().resource::<Incoming>();
        if !incoming.interactions.is_empty()
            || !incoming.closes.is_empty()
            || !incoming.casts.is_empty()
        {
            return Err("bags-drag emitted interaction/close/spell request".into());
        }
        let requests = std::mem::take(&mut *app.world_mut().resource_mut::<Requests>());
        if !requests.forbidden.is_empty() {
            return Err(format!(
                "forbidden bags-drag requests: {:?}",
                requests.forbidden
            ));
        }
        for request in requests.swaps {
            let Phase::Request(index) = self.phase else {
                return Err(format!(
                    "unexpected SwapItem {request:?} in {:?}",
                    self.phase
                ));
            };
            let expected = SwapItem {
                from: SOURCE,
                to: TARGET,
            };
            if request != expected || self.swaps[index] != 0 {
                return Err(format!(
                    "unexpected/duplicate SwapItem {request:?}; case={} count={}",
                    CASES[index], self.swaps[index]
                ));
            }
            self.swaps[index] += 1;
            println!(
                "BAGS DRAG DECODED {} SwapItem {request:?} count=1; delta withheld",
                CASES[index]
            );
        }
        if let Phase::Request(index) = self.phase {
            if self.commit_delta && self.swaps[index] == 1 {
                send_delta(app);
                println!(
                    "BAGS DRAG AUTHORITATIVE {} guid755001 bag0/slot0 empty -> bag1/slot0 count3; bag1/slot7 guid755002 count2 retained",
                    CASES[index]
                );
                self.advance(Phase::Delta(index));
            } else if self.since.elapsed() > REQUEST_WAIT {
                return Err(format!(
                    "no exact request/pre-delta commit in {:?}; swaps={:?}",
                    self.phase, self.swaps
                ));
            }
        }
        Ok(())
    }
}

fn send_snapshot(app: &mut App) {
    let mut sentinel = loot::candle_stack(2);
    sentinel.item_guid += 1;
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
                    items: vec![BagSlotItem {
                        slot: 7,
                        item: sentinel,
                    }],
                },
            ],
        },
    );
}

fn send_delta(app: &mut App) {
    send::<_, InventoryChannel>(
        app,
        InventoryDelta {
            changes: vec![
                InventorySlotChange {
                    location: SOURCE,
                    item: None,
                },
                InventorySlotChange {
                    location: TARGET,
                    item: Some(loot::candle_stack(3)),
                },
            ],
        },
    );
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
        swaps: [0; 3],
        commit_delta: false,
    };
    let deadline = Instant::now() + TIMEOUT + Duration::from_secs(180);
    while Instant::now() < deadline {
        app.update();
        respond_to_login(app, StartupScreen::BagsDrag)?;
        respond_to_selection(app, StartupScreen::BagsDrag, &mut selected, &mut remote)?;
        for line in lines.try_iter() {
            session.observe(app, line.trim(), selected.is_some())?;
        }
        session.respond(app)?;
        if let Some(status) = child
            .try_wait()
            .map_err(|error| format!("inspect bags-drag child: {error}"))?
        {
            return Err(format!(
                "bags-drag child exited before deliberate cleanup: {status}; phase={:?}",
                session.phase
            ));
        }
        if session.phase == Phase::Drain && session.since.elapsed() >= QUIET {
            if session.swaps != [1; 3] {
                return Err(format!(
                    "bags-drag exact request totals failed: {:?}",
                    session.swaps
                ));
            }
            return Ok(());
        }
        thread::sleep(TICK);
    }
    Err(format!(
        "bags-drag timed out; phase={:?} swaps={:?}",
        session.phase, session.swaps
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
        }
    }
    match (result, cleanup) {
        (Err(error), Err(cleanup)) => Err(format!("{error}; owned cleanup failed: {cleanup}")),
        (Err(error), _) | (_, Err(error)) => Err(error),
        (Ok(()), Ok(())) => {
            println!(
                "PASS: BAGS_DRAG same-source/held/nonactionable quiet; separated/rapid/click each exact once SwapItem, peer delta and final counts3/2; deliberate kill/reap/readers drained, NOT normal shutdown proof"
            );
            Ok(())
        }
    }
}
