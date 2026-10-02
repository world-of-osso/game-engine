//! Authenticated native BankFrame actions against an owned loopback protocol peer.
//! The peer withholds every mutation until native input proves unchanged pre-ack UI.
use super::*;
use shared::protocol::{
    BANK_TAB_SLOTS, BagSlotItem, BankAutoDeposit, BankChannel, BankContents, BankDeposit,
    BankError, BankFailed, BankMoneyTransfer, BankPurchaseTab, BankTabView, BankType,
    BankUpdateTabSettings, BankWithdraw, InteractionClosed, ItemStack,
};

const GUID: u64 = 9_170_201;
const INITIAL_GOLD: u64 = 20_000_000;
const QUIET: Duration = Duration::from_millis(600);

#[derive(Debug, PartialEq, Eq)]
enum Request {
    Deposit(BankDeposit),
    Withdraw(BankWithdraw),
    Purchase(BankPurchaseTab),
    Money(BankMoneyTransfer),
    Auto(BankAutoDeposit),
    Settings(BankUpdateTabSettings),
}

#[derive(Resource, Default)]
struct Requests(Vec<Request>);

fn receive(
    mut deposits: Query<&mut MessageReceiver<BankDeposit>>,
    mut withdrawals: Query<&mut MessageReceiver<BankWithdraw>>,
    mut purchases: Query<&mut MessageReceiver<BankPurchaseTab>>,
    mut money: Query<&mut MessageReceiver<BankMoneyTransfer>>,
    mut autos: Query<&mut MessageReceiver<BankAutoDeposit>>,
    mut settings: Query<&mut MessageReceiver<BankUpdateTabSettings>>,
    mut requests: ResMut<Requests>,
) {
    for mut receiver in &mut deposits {
        requests.0.extend(receiver.receive().map(Request::Deposit));
    }
    for mut receiver in &mut withdrawals {
        requests.0.extend(receiver.receive().map(Request::Withdraw));
    }
    for mut receiver in &mut purchases {
        requests.0.extend(receiver.receive().map(Request::Purchase));
    }
    for mut receiver in &mut money {
        requests.0.extend(receiver.receive().map(Request::Money));
    }
    for mut receiver in &mut autos {
        requests.0.extend(receiver.receive().map(Request::Auto));
    }
    for mut receiver in &mut settings {
        requests.0.extend(receiver.receive().map(Request::Settings));
    }
}

fn linen() -> ItemStack {
    ItemStack {
        item_guid: GUID,
        item_id: 2589,
        count: 3,
        durability: None,
        soulbound: false,
    }
}

fn tab(name: &str, filled: bool) -> BankTabView {
    let mut slots = vec![None; BANK_TAB_SLOTS];
    if filled {
        slots[0] = Some(linen());
    }
    BankTabView {
        name: name.into(),
        icon: 134400,
        deposit_flags: 0,
        slots,
    }
}

struct Session {
    player: Entity,
    banker: Entity,
    character: BankContents,
    account: BankContents,
    gold: u64,
    bag: bool,
    step: usize,
    armed: bool,
    pending: Option<Instant>,
    opens: usize,
    closes: usize,
    loading: bool,
    complete: bool,
}

impl Session {
    fn new(app: &mut App, player: Entity) -> Self {
        app.world_mut()
            .entity_mut(player)
            .insert((Gold(INITIAL_GOLD), UnitFactionTemplate(1)));
        let banker = app
            .world_mut()
            .spawn((
                Npc {
                    template_id: 2455,
                    name: "Fixture Banker".into(),
                },
                NpcFlags(NpcFlags::BANKER),
                UnitFactionTemplate(35),
                ModelDisplay { display_id: 26 },
                Position {
                    x: FIRST[0] - 2.0,
                    y: FIRST[1] + 0.1,
                    z: FIRST[2] + 3.0,
                },
                Replicate::to_clients(NetworkTarget::All),
            ))
            .id();
        Self {
            player,
            banker,
            character: BankContents {
                bank: BankType::Character,
                tabs: vec![tab("Supplies", false)],
                next_tab_cost: Some(10_000),
                money: None,
            },
            account: BankContents {
                bank: BankType::Account,
                tabs: vec![tab("Shared", false), tab("Reserve", false)],
                next_tab_cost: Some(250_000_000),
                money: Some(30_000),
            },
            gold: INITIAL_GOLD,
            bag: true,
            step: 0,
            armed: false,
            pending: None,
            opens: 0,
            closes: 0,
            loading: false,
            complete: false,
        }
    }

    fn snapshot(&self, app: &mut App) {
        send::<_, BankChannel>(app, self.character.clone());
        send::<_, BankChannel>(app, self.account.clone());
        send::<_, InventoryChannel>(
            app,
            InventorySnapshot {
                bags: vec![BagContents {
                    bag: 0,
                    size: 16,
                    items: if self.bag {
                        vec![BagSlotItem {
                            slot: 0,
                            item: linen(),
                        }]
                    } else {
                        vec![]
                    },
                }],
            },
        );
        app.world_mut()
            .entity_mut(self.player)
            .insert(Gold(self.gold));
    }

    fn expected(&self) -> Result<Request, String> {
        let npc = self.banker.to_bits();
        let character = BankType::Character;
        let account = BankType::Account;
        Ok(match self.step {
            0 => Request::Deposit(BankDeposit {
                npc,
                bank: character,
                tab: 0,
                item_guid: GUID,
            }),
            1 => Request::Withdraw(BankWithdraw {
                npc,
                bank: character,
                tab: 0,
                slot: 0,
            }),
            2 => Request::Auto(BankAutoDeposit {
                npc,
                bank: character,
                include_reagents: false,
            }),
            3 => Request::Purchase(BankPurchaseTab {
                npc,
                bank: character,
            }),
            4 => Request::Settings(BankUpdateTabSettings {
                npc,
                bank: character,
                tab: 1,
                name: "Renamed".into(),
                icon: 134400,
                deposit_flags: 0x9e,
            }),
            5 => Request::Deposit(BankDeposit {
                npc,
                bank: account,
                tab: 1,
                item_guid: GUID,
            }),
            6 => Request::Withdraw(BankWithdraw {
                npc,
                bank: account,
                tab: 1,
                slot: 0,
            }),
            7 => Request::Money(BankMoneyTransfer {
                npc,
                bank: account,
                copper: 10_203,
                deposit: true,
            }),
            8 => Request::Money(BankMoneyTransfer {
                npc,
                bank: account,
                copper: 10_203,
                deposit: false,
            }),
            9 => Request::Auto(BankAutoDeposit {
                npc,
                bank: account,
                include_reagents: true,
            }),
            10 => Request::Money(BankMoneyTransfer {
                npc,
                bank: account,
                copper: 99_000_000,
                deposit: false,
            }),
            _ => return Err(format!("unexpected bank request after step {}", self.step)),
        })
    }

    fn requests(&mut self, app: &mut App) -> Result<(), String> {
        let (interactions, closes) = {
            let mut incoming = app.world_mut().resource_mut::<Incoming>();
            (
                std::mem::take(&mut incoming.interactions),
                std::mem::take(&mut incoming.closes),
            )
        };
        for request in interactions {
            if request.npc != self.banker.to_bits() || self.opens != self.closes || self.opens >= 3
            {
                return Err(format!("unexpected banker InteractNpc {request:?}"));
            }
            self.opens += 1;
            println!(
                "BANK EXACT InteractNpc banker={} open={}",
                request.npc, self.opens
            );
            send::<_, InteractionChannel>(
                app,
                InteractionOpened {
                    npc: request.npc,
                    kind: InteractionKind::Role(NpcRole::Banker),
                },
            );
            self.snapshot(app);
        }
        for request in closes {
            if request.npc != self.banker.to_bits() || self.closes >= self.opens || self.closes >= 2
            {
                return Err(format!("unexpected banker CloseInteraction {request:?}"));
            }
            self.closes += 1;
            println!("BANK EXACT CloseInteraction close={}", self.closes);
        }
        let requests = std::mem::take(&mut app.world_mut().resource_mut::<Requests>().0);
        for request in requests {
            if !self.armed
                || self.pending.is_some()
                || self.opens != 1
                || request != self.expected()?
            {
                return Err(format!(
                    "bank step {} unexpected request {request:?}; armed={} pending={:?}",
                    self.step, self.armed, self.pending
                ));
            }
            println!(
                "BANK EXACT step={} request={request:?}; ACK WITHHELD",
                self.step
            );
            self.pending = Some(Instant::now());
        }
        if self
            .pending
            .is_some_and(|since| since.elapsed() > Duration::from_secs(12))
        {
            return Err(format!(
                "bank step {} missing native preack barrier",
                self.step
            ));
        }
        Ok(())
    }

    fn acknowledge(&mut self, app: &mut App) -> Result<(), String> {
        let since = self
            .pending
            .take()
            .ok_or_else(|| format!("bank step {} PREACK without protocol request", self.step))?;
        if since.elapsed() < QUIET {
            return Err("bank preack observation shorter than 600ms".into());
        }
        match self.step {
            0 => {
                self.bag = false;
                self.character.tabs[0].slots[0] = Some(linen());
            }
            1 => {
                self.bag = true;
                self.character.tabs[0].slots[0] = None;
            }
            2 => {} // No eligible reagent: peer alone decides the no-op.
            3 => {
                self.character.tabs.push(tab("New Tab", false));
                self.character.next_tab_cost = Some(100_000);
                self.gold -= 10_000;
            }
            4 => {
                self.character.tabs[1].name = "Renamed".into();
                self.character.tabs[1].deposit_flags = 0x9e;
            }
            5 => {
                self.bag = false;
                self.account.tabs[1].slots[0] = Some(linen());
            }
            6 => {
                self.bag = true;
                self.account.tabs[1].slots[0] = None;
            }
            7 => {
                self.account.money = Some(40_203);
                self.gold -= 10_203;
            }
            8 => {
                self.account.money = Some(30_000);
                self.gold += 10_203;
            }
            9 => {} // Same no-op authority for Include tradeable reagents.
            10 => {
                send::<_, BankChannel>(
                    app,
                    BankFailed {
                        npc: self.banker.to_bits(),
                        error: BankError::NotEnoughMoney,
                    },
                );
            }
            _ => return Err("bank acknowledgement after final action".into()),
        }
        self.snapshot(app);
        println!(
            "BANK ACK step={} gold={} bag={}",
            self.step, self.gold, self.bag
        );
        self.step += 1;
        self.armed = false;
        Ok(())
    }

    fn observe(&mut self, app: &mut App, line: &str) -> Result<(), String> {
        bags::reject_runtime_error(line)?;
        match line {
            "FIXTURE BANK_LOADING" if !self.loading => {
                self.loading = true;
                send::<_, TerrainChannel>(
                    app,
                    LoadTerrain {
                        map_name: "azeroth".into(),
                        initial_tile_y: 32,
                        initial_tile_x: 48,
                    },
                );
            }
            "FIXTURE BANK_OPEN" if self.loading && self.opens == 1 && self.step == 0 => {}
            "FIXTURE BANK_ARM" if self.opens == 1 && !self.armed && self.step <= 10 => {
                self.armed = true;
            }
            "FIXTURE BANK_PREACK" if self.armed => self.acknowledge(app)?,
            "FIXTURE BANK_PEER_REFRESH" if self.step == 11 && self.opens == 1 => {
                self.account.tabs[1].name = "Peer Shared".into();
                self.account.tabs[1].slots[2] = Some(linen());
                self.account.money = Some(44_444);
                self.snapshot(app);
            }
            "FIXTURE BANK_SERVER_CLOSE" if self.opens == 3 && self.closes == 2 => {
                send::<_, InteractionChannel>(
                    app,
                    InteractionClosed {
                        npc: self.banker.to_bits(),
                    },
                );
            }
            "FIXTURE BANK_DONE" if self.step == 11 && self.opens == 3 && self.closes == 2 => {
                self.complete = true;
            }
            other if other.starts_with("FIXTURE BANK_") => {
                return Err(format!(
                    "out-of-order bank marker step={} opens={} closes={}: {other}",
                    self.step, self.opens, self.closes
                ));
            }
            _ => {}
        }
        Ok(())
    }
}

fn run_until_done(app: &mut App, child: &mut Child, lines: &ClientLines) -> Result<(), String> {
    app.init_resource::<Requests>();
    app.add_systems(Update, receive);
    let mut selected = None;
    let mut remote = None;
    let mut session = None;
    let deadline = Instant::now() + TIMEOUT + Duration::from_secs(180);
    while Instant::now() < deadline {
        app.update();
        respond_to_login(app, StartupScreen::Bank)?;
        respond_to_selection(app, StartupScreen::Bank, &mut selected, &mut remote)?;
        if session.is_none() {
            if let Some(player) = selected {
                session = Some(Session::new(app, player));
            }
        }
        if let Some(session) = &mut session {
            for line in lines.after_selection(true) {
                session.observe(app, line.trim())?;
            }
            session.requests(app)?;
            if session.complete {
                return Ok(());
            }
        }
        if let Some(status) = child.try_wait().map_err(|e| e.to_string())? {
            return Err(format!("bank native child exited early: {status}"));
        }
        thread::sleep(TICK);
    }
    Err("bank native fixture timed out".into())
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
            result = Err(error);
        }
    }
    match (result, cleanup) {
        (Err(error), Err(cleanup)) => Err(format!("{error}; cleanup: {cleanup}")),
        (Err(error), _) | (_, Err(error)) => Err(error),
        (Ok(()), Ok(())) => {
            println!(
                "PASS: BANK physical controls/exact protocol/11 authority barriers/local and server close; deliberate cleanup NOT shutdown proof"
            );
            Ok(())
        }
    }
}
