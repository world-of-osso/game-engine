//! Standalone authored equip/destroy actions; owned UDP deltas, no UI/state setters.
use super::*;
use shared::protocol::{
    BagSlotItem, DestroyItem, EquipItem, EquipmentSlot, EquipmentSnapshot, EquippedItem,
    InventoryDelta, InventorySlotChange, ItemLocation, ItemStack, SortBags, SplitItem, SwapItem,
    UseItem,
};

const QUIET: Duration = Duration::from_millis(400);
const REQUEST_WAIT: Duration = Duration::from_secs(6);
const SWORD_SLOT: u8 = 5;
const STARTUP_SWORD_GUID: u64 = 9_170_105;
const PELT_SLOT: u8 = 2;
const PLANS_SLOT: u8 = 3;

fn bag(slot: u8) -> ItemLocation {
    ItemLocation::Bag { bag: 0, slot }
}

fn stack(slot: u8) -> ItemStack {
    let item_id = match slot {
        SWORD_SLOT => 25,
        PELT_SLOT => 4865,
        PLANS_SLOT => 3871,
        _ => panic!("unknown bags-actions fixture slot {slot}"),
    };
    ItemStack {
        item_guid: 9_170_000 + u64::from(slot),
        item_id,
        count: 1,
        durability: None,
        soulbound: false,
    }
}

#[derive(Resource, Default)]
struct Requests {
    equips: Vec<EquipItem>,
    uses: Vec<UseItem>,
    destroys: Vec<DestroyItem>,
    forbidden: Vec<String>,
}

fn receive(
    mut equips: Query<&mut MessageReceiver<EquipItem>>,
    mut destroys: Query<&mut MessageReceiver<DestroyItem>>,
    mut uses: Query<&mut MessageReceiver<UseItem>>,
    mut swaps: Query<&mut MessageReceiver<SwapItem>>,
    mut splits: Query<&mut MessageReceiver<SplitItem>>,
    mut sorts: Query<&mut MessageReceiver<SortBags>>,
    mut requests: ResMut<Requests>,
) {
    for mut receiver in &mut equips {
        requests.equips.extend(receiver.receive());
    }
    for mut receiver in &mut destroys {
        requests.destroys.extend(receiver.receive());
    }
    for mut receiver in &mut uses {
        requests.uses.extend(receiver.receive());
    }
    for mut receiver in &mut swaps {
        requests
            .forbidden
            .extend(receiver.receive().map(|r| format!("SwapItem {r:?}")));
    }
    for mut receiver in &mut splits {
        requests
            .forbidden
            .extend(receiver.receive().map(|r| format!("SplitItem {r:?}")));
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
    EquipArm,
    EquipRequest,
    EquipDone,
    UseArm,
    UseRequest,
    UseDone,
    PoorNo,
    PoorArm,
    PoorRequest,
    PoorDone,
    RareInert,
    RareArm,
    RareRequest,
    RareDone,
    Done,
    Complete,
}

struct Session {
    phase: Phase,
    since: Instant,
    equips: usize,
    uses: usize,
    destroys: usize,
}

impl Session {
    fn advance(&mut self, phase: Phase) {
        println!(
            "BAGS ACTIONS PHASE {phase:?} equips={} destroys={}",
            self.equips, self.destroys
        );
        self.phase = phase;
        self.since = Instant::now();
    }

    fn require_quiet(&self) -> Result<(), String> {
        if self.since.elapsed() < QUIET {
            return Err(format!(
                "bags-actions quiet phase too short: {:?}",
                self.phase
            ));
        }
        Ok(())
    }

    fn observe(&mut self, app: &mut App, line: &str) -> Result<(), String> {
        bags::reject_runtime_error(line)?;
        let next = match (line, &self.phase) {
            ("FIXTURE BAGS_ACTIONS_LOADING", Phase::Loading) => {
                send::<_, TerrainChannel>(
                    app,
                    LoadTerrain {
                        map_name: "azeroth".into(),
                        initial_tile_y: 32,
                        initial_tile_x: 48,
                    },
                );
                Phase::Ready
            }
            ("FIXTURE BAGS_ACTIONS_READY", Phase::Ready) => {
                send_snapshot(app);
                Phase::EquipArm
            }
            ("FIXTURE BAGS_ACTIONS_EQUIP_ARM", Phase::EquipArm) => Phase::EquipRequest,
            ("FIXTURE BAGS_ACTIONS_EQUIP_DONE", Phase::EquipDone) => Phase::UseArm,
            ("FIXTURE BAGS_ACTIONS_USE_ARM", Phase::UseArm) => Phase::UseRequest,
            ("FIXTURE BAGS_ACTIONS_USE_DONE", Phase::UseDone) => Phase::PoorNo,
            ("FIXTURE BAGS_ACTIONS_POOR_NO", Phase::PoorNo) => {
                self.require_quiet()?;
                Phase::PoorArm
            }
            ("FIXTURE BAGS_ACTIONS_POOR_ARM", Phase::PoorArm) => Phase::PoorRequest,
            ("FIXTURE BAGS_ACTIONS_POOR_DONE", Phase::PoorDone) => Phase::RareInert,
            ("FIXTURE BAGS_ACTIONS_RARE_INERT", Phase::RareInert) => {
                self.require_quiet()?;
                Phase::RareArm
            }
            ("FIXTURE BAGS_ACTIONS_RARE_ARM", Phase::RareArm) => Phase::RareRequest,
            ("FIXTURE BAGS_ACTIONS_RARE_DONE", Phase::RareDone) => Phase::Done,
            ("FIXTURE BAGS_ACTIONS_DONE", Phase::Done) => {
                self.require_quiet()?;
                Phase::Complete
            }
            (other, _) if other.starts_with("FIXTURE BAGS_ACTIONS_") => {
                return Err(format!(
                    "out-of-order bags-actions marker in {:?}: {other}",
                    self.phase
                ));
            }
            _ => return Ok(()),
        };
        self.advance(next);
        Ok(())
    }

    fn respond(&mut self, app: &mut App) -> Result<(), String> {
        reject_unrelated(app)?;
        let requests = std::mem::take(&mut *app.world_mut().resource_mut::<Requests>());
        if !requests.forbidden.is_empty() {
            return Err(format!(
                "forbidden bags-actions requests: {:?}",
                requests.forbidden
            ));
        }
        for request in requests.equips {
            self.respond_equip(app, request)?;
        }
        for request in requests.uses {
            self.respond_use(request)?;
        }
        for request in requests.destroys {
            self.respond_destroy(app, request)?;
        }
        let waiting = matches!(
            self.phase,
            Phase::EquipRequest | Phase::UseRequest | Phase::PoorRequest | Phase::RareRequest
        );
        if waiting && self.since.elapsed() > REQUEST_WAIT {
            return Err(format!(
                "no decoded exact bags-actions request in {:?}",
                self.phase
            ));
        }
        Ok(())
    }

    fn respond_equip(&mut self, app: &mut App, request: EquipItem) -> Result<(), String> {
        let expected = EquipItem {
            from: bag(SWORD_SLOT),
        };
        if self.phase != Phase::EquipRequest || self.equips != 0 || request != expected {
            return Err(format!(
                "unexpected EquipItem {request:?} in {:?}; count={}",
                self.phase, self.equips
            ));
        }
        self.equips += 1;
        println!(
            "BAGS ACTIONS DECODED EquipItem {request:?} count={}",
            self.equips
        );
        send::<_, InventoryChannel>(
            app,
            InventoryDelta {
                changes: vec![
                    InventorySlotChange {
                        location: bag(SWORD_SLOT),
                        item: None,
                    },
                    InventorySlotChange {
                        location: ItemLocation::Equipment(EquipmentSlot::MainHand),
                        item: Some(stack(SWORD_SLOT)),
                    },
                ],
            },
        );
        self.advance(Phase::EquipDone);
        Ok(())
    }

    /// Right-clicking the non-equippable pelt uses it (`UseContainerItem`); the item stays.
    fn respond_use(&mut self, request: UseItem) -> Result<(), String> {
        let expected = UseItem {
            location: bag(PELT_SLOT),
            target: None,
        };
        if self.phase != Phase::UseRequest || self.uses != 0 || request != expected {
            return Err(format!(
                "unexpected UseItem {request:?} in {:?}; count={}",
                self.phase, self.uses
            ));
        }
        self.uses += 1;
        println!(
            "BAGS ACTIONS DECODED UseItem {request:?} count={}",
            self.uses
        );
        self.advance(Phase::UseDone);
        Ok(())
    }

    fn respond_destroy(&mut self, app: &mut App, request: DestroyItem) -> Result<(), String> {
        let (slot, count, next) = match self.phase {
            Phase::PoorRequest => (PELT_SLOT, 0, Phase::PoorDone),
            Phase::RareRequest => (PLANS_SLOT, 1, Phase::RareDone),
            _ => {
                return Err(format!(
                    "DestroyItem {request:?} forbidden in {:?}",
                    self.phase
                ));
            }
        };
        let expected = DestroyItem {
            location: bag(slot),
            count: 0,
        };
        if self.destroys != count || request != expected {
            return Err(format!(
                "unexpected DestroyItem {request:?}; previous count={}",
                self.destroys
            ));
        }
        self.destroys += 1;
        println!(
            "BAGS ACTIONS DECODED DestroyItem {request:?} count={}",
            self.destroys
        );
        send::<_, InventoryChannel>(
            app,
            InventoryDelta {
                changes: vec![InventorySlotChange {
                    location: bag(slot),
                    item: None,
                }],
            },
        );
        self.advance(next);
        Ok(())
    }
}

fn reject_unrelated(app: &App) -> Result<(), String> {
    let incoming = app.world().resource::<Incoming>();
    if !incoming.interactions.is_empty()
        || !incoming.closes.is_empty()
        || !incoming.casts.is_empty()
    {
        return Err("bags-actions emitted interaction/close/spell request".into());
    }
    Ok(())
}

fn send_snapshot(app: &mut App) {
    send::<_, InventoryChannel>(
        app,
        EquipmentSnapshot {
            items: vec![EquippedItem {
                slot: EquipmentSlot::MainHand,
                item: ItemStack {
                    item_guid: STARTUP_SWORD_GUID,
                    ..stack(SWORD_SLOT)
                },
            }],
        },
    );
    let items = [SWORD_SLOT, PELT_SLOT, PLANS_SLOT]
        .into_iter()
        .map(|slot| BagSlotItem {
            slot,
            item: stack(slot),
        })
        .collect();
    send::<_, InventoryChannel>(
        app,
        InventorySnapshot {
            bags: vec![BagContents {
                bag: 0,
                size: 16,
                items,
            }],
        },
    );
}

fn run_until_done(app: &mut App, child: &mut Child, lines: &ClientLines) -> Result<(), String> {
    app.init_resource::<Requests>();
    app.add_systems(Update, receive);
    let mut selected = None;
    let mut remote = None;
    let mut session = Session {
        phase: Phase::Loading,
        since: Instant::now(),
        equips: 0,
        uses: 0,
        destroys: 0,
    };
    let deadline = Instant::now() + TIMEOUT + Duration::from_secs(180);
    while Instant::now() < deadline {
        app.update();
        respond_to_login(app, StartupScreen::BagsActions)?;
        respond_to_selection(app, StartupScreen::BagsActions, &mut selected, &mut remote)?;
        for line in lines.after_selection(selected.is_some()) {
            session.observe(app, line.trim())?;
        }
        session.respond(app)?;
        if let Some(status) = child
            .try_wait()
            .map_err(|e| format!("inspect bags-actions child: {e}"))?
        {
            return Err(format!(
                "bags-actions child exited before deliberate cleanup: {status}; phase={:?}",
                session.phase
            ));
        }
        if session.phase == Phase::Complete {
            if session.equips != 1 || session.uses != 1 || session.destroys != 2 {
                return Err(format!(
                    "bags-actions totals equips={} uses={} destroys={}",
                    session.equips, session.uses, session.destroys
                ));
            }
            return Ok(());
        }
        thread::sleep(TICK);
    }
    Err(format!("bags-actions timed out; phase={:?}", session.phase))
}

pub(super) fn run(
    app: &mut App,
    child: &mut Child,
    lines: ClientLines,
    readers: Vec<thread::JoinHandle<()>>,
) -> Result<(), String> {
    let mut result = run_until_done(app, child, &lines);
    let cleanup = bags::cleanup_owned_child(child, readers);
    for line in lines.remaining() {
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
                "PASS: BAGS_ACTIONS authored equip, right-click use, poor No/Yes, rare DELETE typing/inert/accept, exact 1 Equip/1 Use/2 Destroy and authoritative inventory/equipment; deliberate kill/reap/readers drained, NOT shutdown or mesh parity"
            );
            Ok(())
        }
    }
}
