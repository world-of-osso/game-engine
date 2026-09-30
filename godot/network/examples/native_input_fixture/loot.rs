//! Test-only authoritative loot peer. Auto-loot belongs to the server, not the UI.
use super::*;
use shared::{
    components::Health,
    protocol::{
        BagSlotItem, CorpseLootable, InventoryDelta, InventorySlotChange, ItemLocation, ItemStack,
        LootChannel, LootClosed, LootContent, LootError, LootFailed, LootRelease, LootResponse,
        LootSlot, LootSlotRemoved, LootSlotRequest, LootUnit,
    },
};

const CORPSE_NAME: &str = "Fixture Corpse";
const REQUEST_WAIT: Duration = Duration::from_secs(6);
const EMPTY_CLICK_WAIT: Duration = Duration::from_secs(2);
const INITIAL_COUNT: u32 = 3;
const INITIAL_MONEY: u64 = 1_250;
const LOOT_MONEY: u64 = 10_502;

fn candle_stack(count: u32) -> ItemStack {
    ItemStack {
        item_guid: 755_001,
        item_id: 755,
        count,
        durability: None,
        soulbound: false,
    }
}

#[derive(Resource, Default)]
struct Requests {
    units: Vec<LootUnit>,
    slots: Vec<LootSlotRequest>,
    releases: Vec<LootRelease>,
}

fn receive(
    mut units: Query<&mut MessageReceiver<LootUnit>>,
    mut slots: Query<&mut MessageReceiver<LootSlotRequest>>,
    mut releases: Query<&mut MessageReceiver<LootRelease>>,
    mut requests: ResMut<Requests>,
) {
    for mut receiver in &mut units {
        requests.units.extend(receiver.receive());
    }
    for mut receiver in &mut slots {
        requests.slots.extend(receiver.receive());
    }
    for mut receiver in &mut releases {
        requests.releases.extend(receiver.receive());
    }
}

fn spawn_corpse(app: &mut App) -> u64 {
    app.world_mut()
        .spawn((
            Npc {
                template_id: 1213,
                name: CORPSE_NAME.into(),
            },
            ModelDisplay { display_id: 26 },
            UnitFactionTemplate(35),
            Health {
                current: 0.0,
                max: 100.0,
            },
            Position {
                x: FIRST[0] - 2.0,
                y: FIRST[1] + 0.1,
                z: FIRST[2] + 3.0,
            },
            Replicate::to_clients(NetworkTarget::All),
        ))
        .id()
        .to_bits()
}

fn slots() -> Vec<LootSlot> {
    vec![
        LootSlot {
            slot: 0,
            content: LootContent::Item {
                item_id: 755,
                name: "Melted Candle".into(),
                quality: 0,
                count: 2,
            },
        },
        LootSlot {
            slot: 1,
            content: LootContent::Money { copper: 10_502 },
        },
    ]
}

#[derive(Default)]
struct Session {
    corpse: Option<u64>,
    player: Option<Entity>,
    collected_items: u32,
    collected_money: u64,
    rejected: usize,
    empty_click_deadline: Option<Instant>,
    opens: usize,
    clicks: usize,
    active: bool,
    remaining: Vec<u8>,
    awaiting_request: Option<Instant>,
    loading: bool,
    ready: bool,
    passed: bool,
}

impl Session {
    fn corpse(&self) -> Result<u64, String> {
        self.corpse
            .ok_or_else(|| "loot marker before corpse replication".into())
    }

    fn mark_lootable(&self, app: &mut App) -> Result<(), String> {
        send::<_, LootChannel>(
            app,
            CorpseLootable {
                corpse: self.corpse()?,
                lootable: true,
            },
        );
        Ok(())
    }

    fn send_inventory_snapshot(&self, app: &mut App) {
        send::<_, InventoryChannel>(
            app,
            InventorySnapshot {
                bags: vec![BagContents {
                    bag: 0,
                    size: 16,
                    items: vec![BagSlotItem {
                        slot: 0,
                        item: candle_stack(INITIAL_COUNT),
                    }],
                }],
            },
        );
    }

    fn remove_slot(&mut self, app: &mut App, slot: u8) -> Result<(), String> {
        let corpse = self.corpse()?;
        match slot {
            0 => {
                self.collected_items += 2;
                send::<_, InventoryChannel>(
                    app,
                    InventoryDelta {
                        changes: vec![InventorySlotChange {
                            location: ItemLocation::Bag { bag: 0, slot: 0 },
                            item: Some(candle_stack(INITIAL_COUNT + self.collected_items)),
                        }],
                    },
                );
            }
            1 => {
                let player = self.player.ok_or("loot money before selected player")?;
                self.collected_money += LOOT_MONEY;
                app.world_mut()
                    .entity_mut(player)
                    .insert(Gold(INITIAL_MONEY + self.collected_money));
            }
            _ => return Err(format!("unknown fixture loot slot: {slot}")),
        }
        self.remaining.retain(|remaining| *remaining != slot);
        send::<_, LootChannel>(app, LootSlotRemoved { corpse, slot });
        Ok(())
    }

    fn observe(&mut self, app: &mut App, line: &str) -> Result<(), String> {
        // Same unrelated missing-scenery boundary as merchant-click; script errors fail.
        let missing_scenery = line.starts_with("GODOT_STDERR: ERROR: WorldObjects:")
            && (line.contains("missing textures") || line.contains("particle textures missing"));
        if !missing_scenery
            && (line.starts_with("GODOT_STDERR: ERROR:")
                || line.starts_with("GODOT_STDERR: SCRIPT ERROR:"))
        {
            return Err(format!("Godot loot runtime error: {line}"));
        }
        match line {
            "FIXTURE LOOT_LOADING" if self.corpse.is_some() && !self.loading => {
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
            "FIXTURE LOOT_READY" if self.loading && !self.ready => {
                self.ready = true;
                self.send_inventory_snapshot(app);
                self.mark_lootable(app)?;
            }
            "FIXTURE LOOT_CLICKED" if self.ready && self.clicks < 4 => {
                self.clicks += 1;
                // UDP reception can precede stdout observation of the actual click.
                if self.opens < self.clicks {
                    self.awaiting_request = Some(Instant::now() + REQUEST_WAIT);
                }
            }
            "FIXTURE LOOT_REARM" if self.ready && !self.active && self.opens < 4 => {
                self.mark_lootable(app)?
            }
            "FIXTURE LOOT_EMPTY" if self.opens == 4 && self.active && self.remaining.is_empty() => {
                send::<_, LootChannel>(
                    app,
                    LootClosed {
                        corpse: self.corpse()?,
                    },
                );
                self.active = false;
            }
            "FIXTURE LOOT_NOT_LOOTABLE_CLICKED"
                if self.opens == 4 && self.clicks == 4 && !self.active =>
            {
                self.empty_click_deadline = Some(Instant::now() + EMPTY_CLICK_WAIT);
            }
            "FIXTURE LOOT_DONE"
                if self.opens == 4
                    && self.clicks == 4
                    && !self.active
                    && self.rejected == 1
                    && self
                        .empty_click_deadline
                        .is_some_and(|end| Instant::now() >= end) =>
            {
                self.passed = true
            }
            other if other.starts_with("FIXTURE LOOT_") => {
                return Err(format!("out-of-order loot marker: {other}"));
            }
            _ => {}
        }
        Ok(())
    }

    fn respond(&mut self, app: &mut App) -> Result<(), String> {
        let (units, taken, releases) = {
            let mut requests = app.world_mut().resource_mut::<Requests>();
            (
                std::mem::take(&mut requests.units),
                std::mem::take(&mut requests.slots),
                std::mem::take(&mut requests.releases),
            )
        };
        for request in units {
            let expected = [false, true, true, false].get(self.opens).copied();
            if !self.ready
                || self.active
                || Some(request.auto) != expected
                || request.corpse != self.corpse()?
            {
                return Err(format!(
                    "unexpected LootUnit {request:?}; case={}, expected auto={expected:?}",
                    self.opens
                ));
            }
            self.awaiting_request = None;
            self.opens += 1;
            self.active = true;
            self.remaining = vec![0, 1];
            println!("LOOT REQUEST case={} auto={}", self.opens, request.auto);
            send::<_, LootChannel>(
                app,
                LootResponse {
                    corpse: request.corpse,
                    auto: request.auto,
                    slots: slots(),
                },
            );
            if request.auto {
                for slot in [0, 1] {
                    self.remove_slot(app, slot)?;
                    // A duplicate removal cannot create inventory or chat twice.
                    send::<_, LootChannel>(
                        app,
                        LootSlotRemoved {
                            corpse: request.corpse,
                            slot,
                        },
                    );
                }
                send::<_, LootChannel>(
                    app,
                    CorpseLootable {
                        corpse: request.corpse,
                        lootable: false,
                    },
                );
                send::<_, LootChannel>(
                    app,
                    LootClosed {
                        corpse: request.corpse,
                    },
                );
                self.remaining.clear();
                self.active = false;
            } else {
                // Unrelated corpse messages must not alter this open window.
                send::<_, LootChannel>(
                    app,
                    CorpseLootable {
                        corpse: request.corpse.wrapping_add(1),
                        lootable: false,
                    },
                );
                send::<_, LootChannel>(
                    app,
                    LootSlotRemoved {
                        corpse: request.corpse.wrapping_add(1),
                        slot: 0,
                    },
                );
                send::<_, LootChannel>(
                    app,
                    LootClosed {
                        corpse: request.corpse.wrapping_add(1),
                    },
                );
            }
        }
        for request in taken {
            if !self.active
                || request.corpse != self.corpse()?
                || !self.remaining.contains(&request.slot)
            {
                return Err(format!(
                    "unexpected LootSlotRequest {request:?}; remaining={:?}",
                    self.remaining
                ));
            }
            if self.opens == 1 && request.slot == 0 && self.rejected == 0 {
                self.rejected += 1;
                send::<_, LootChannel>(
                    app,
                    LootFailed {
                        corpse: request.corpse,
                        error: LootError::InventoryFull,
                    },
                );
                println!("LOOT REJECTED InventoryFull count={}", self.rejected);
                continue;
            }
            self.remove_slot(app, request.slot)?;
            // Reliable duplicate delivery must not print the content twice.
            send::<_, LootChannel>(
                app,
                LootSlotRemoved {
                    corpse: request.corpse,
                    slot: request.slot,
                },
            );
            if self.remaining.is_empty() {
                send::<_, LootChannel>(
                    app,
                    CorpseLootable {
                        corpse: request.corpse,
                        lootable: false,
                    },
                );
                send::<_, LootChannel>(
                    app,
                    LootClosed {
                        corpse: request.corpse.wrapping_add(1),
                    },
                );
                // Matching close is withheld until the script observes the empty frame.
            }
        }
        for request in releases {
            if self.opens != 1
                || !self.active
                || request.corpse != self.corpse()?
                || self.remaining != [1]
            {
                return Err(format!("unexpected LootRelease {request:?}"));
            }
            send::<_, LootChannel>(
                app,
                LootClosed {
                    corpse: request.corpse,
                },
            );
            self.active = false;
        }
        Ok(())
    }
}

pub(super) fn run(
    app: &mut App,
    child: &mut Child,
    lines: Receiver<String>,
    readers: Vec<thread::JoinHandle<()>>,
) -> Result<(), String> {
    app.init_resource::<Requests>();
    app.add_systems(Update, receive);
    let mut selected = None;
    let mut remote = None;
    let mut session = Session::default();
    let mut readers = Some(readers);
    // Include cold CASC bootstrap before the owned world/input sequence.
    let deadline = Instant::now() + TIMEOUT + Duration::from_secs(180);
    while Instant::now() < deadline {
        app.update();
        respond_to_login(app, StartupScreen::Loot)?;
        respond_to_selection(app, StartupScreen::Loot, &mut selected, &mut remote)?;
        if let Some(player) = selected
            && session.corpse.is_none()
        {
            app.world_mut()
                .entity_mut(player)
                .insert(Gold(INITIAL_MONEY));
            session.player = Some(player);
            session.corpse = Some(spawn_corpse(app));
        }
        let status = child.try_wait().map_err(|error| error.to_string())?;
        if status.is_some() {
            for reader in readers.take().expect("join output once") {
                reader.join().map_err(|_| "Godot output reader panicked")?;
            }
        }
        for line in lines.try_iter() {
            session.observe(app, line.trim())?;
        }
        session.respond(app)?;
        if session
            .awaiting_request
            .is_some_and(|end| Instant::now() >= end)
        {
            return Err(format!(
                "RED: actual corpse right-click produced no LootUnit on LootChannel; case={}",
                session.opens + 1
            ));
        }
        if let Some(status) = status {
            return if status.success() && session.passed {
                Ok(())
            } else {
                Err(format!(
                    "loot fixture exited {status}; opens={}, rejected={}, done={}",
                    session.opens, session.rejected, session.passed
                ))
            };
        }
        thread::sleep(TICK);
    }
    Err(format!(
        "loot fixture timed out; model_ready={}, opens={}",
        session.ready, session.opens
    ))
}
