//! Owned authenticated TradeChannel peer. UI input must cause exact decoded requests.
use super::*;
use shared::protocol::{
    AcceptTrade, CancelTrade, CancelTradeAccept, ClearTradeItem, ConfirmTrade, DeclineTrade,
    InitiateTrade, SetTradeItem, SetTradeMoney, TradeChannel, TradeItemSnapshot,
    TradePartySnapshot, TradePhase, TradeSnapshot, TradeStateUpdate,
};

#[derive(Debug, PartialEq, Eq)]
enum Request {
    Initiate(InitiateTrade),
    Accept,
    Decline,
    Cancel,
    Item(SetTradeItem),
    Clear(ClearTradeItem),
    Money(SetTradeMoney),
    Confirm,
    Unaccept,
}

#[derive(Resource, Default)]
struct Requests(Vec<Request>);

fn receive(
    mut initiate: Query<&mut MessageReceiver<InitiateTrade>>,
    mut accept: Query<&mut MessageReceiver<AcceptTrade>>,
    mut decline: Query<&mut MessageReceiver<DeclineTrade>>,
    mut cancel: Query<&mut MessageReceiver<CancelTrade>>,
    mut item: Query<&mut MessageReceiver<SetTradeItem>>,
    mut clear: Query<&mut MessageReceiver<ClearTradeItem>>,
    mut money: Query<&mut MessageReceiver<SetTradeMoney>>,
    mut confirm: Query<&mut MessageReceiver<ConfirmTrade>>,
    mut unaccept: Query<&mut MessageReceiver<CancelTradeAccept>>,
    mut requests: ResMut<Requests>,
) {
    for mut receiver in &mut initiate {
        requests.0.extend(receiver.receive().map(Request::Initiate));
    }
    for mut receiver in &mut accept {
        requests
            .0
            .extend(receiver.receive().map(|_| Request::Accept));
    }
    for mut receiver in &mut decline {
        requests
            .0
            .extend(receiver.receive().map(|_| Request::Decline));
    }
    for mut receiver in &mut cancel {
        requests
            .0
            .extend(receiver.receive().map(|_| Request::Cancel));
    }
    for mut receiver in &mut item {
        requests.0.extend(receiver.receive().map(Request::Item));
    }
    for mut receiver in &mut clear {
        requests.0.extend(receiver.receive().map(Request::Clear));
    }
    for mut receiver in &mut money {
        requests.0.extend(receiver.receive().map(Request::Money));
    }
    for mut receiver in &mut confirm {
        requests
            .0
            .extend(receiver.receive().map(|_| Request::Confirm));
    }
    for mut receiver in &mut unaccept {
        requests
            .0
            .extend(receiver.receive().map(|_| Request::Unaccept));
    }
}

pub(super) fn install(app: &mut App) {
    app.init_resource::<Requests>();
    app.add_systems(Update, receive);
}

fn snapshot(phase: TradePhase, accepted: bool) -> TradeSnapshot {
    let mut player = TradePartySnapshot {
        name: NAME.into(),
        accepted,
        gold: 12345,
        slots: vec![None; 7],
    };
    player.slots[0] = Some(TradeItemSnapshot {
        definition_source: shared::item_data::ItemDefinitionSource::Retail,
        item_guid: 9170001,
        item_id: 2589,
        name: "Linen Cloth".into(),
        quality: 1,
        stack_count: 3,
    });
    TradeSnapshot {
        phase,
        player,
        other: TradePartySnapshot {
            name: REMOTE_NAME.into(),
            accepted: false,
            gold: 67890,
            slots: vec![None; 7],
        },
    }
}

fn update(app: &mut App, trade: Option<TradeSnapshot>, message: Option<&str>) {
    send::<_, TradeChannel>(
        app,
        TradeStateUpdate {
            trade,
            message: message.map(str::to_owned),
            error: None,
        },
    );
}

fn run_until_done(app: &mut App, child: &mut Child, lines: &ClientLines) -> Result<(), String> {
    let mut selected = None;
    let mut remote = None;
    let mut stage = 0;
    let mut arm = None;
    let mut done = false;
    let deadline = Instant::now() + TIMEOUT;
    while Instant::now() < deadline {
        app.update();
        respond_to_login(app, StartupScreen::Trade)?;
        respond_to_selection(app, StartupScreen::Trade, &mut selected, &mut remote)?;
        for line in lines.after_selection(selected.is_some()) {
            bags::reject_runtime_error(line.trim())?;
            match line.trim() {
                "FIXTURE TRADE_LOADING" if stage == 0 => {
                    send::<_, TerrainChannel>(
                        app,
                        LoadTerrain {
                            map_name: "azeroth".into(),
                            initial_tile_y: 32,
                            initial_tile_x: 48,
                        },
                    );
                    stage = 1;
                }
                "FIXTURE TRADE_READY" if stage == 1 => {
                    update(
                        app,
                        Some(snapshot(TradePhase::PendingIncoming, false)),
                        None,
                    );
                    println!("TRADE PEER SENT authenticated PendingIncoming");
                    stage = 2;
                }
                "FIXTURE TRADE_ACCEPT_ARM" if stage == 2 => arm = Some(Request::Accept),
                "FIXTURE TRADE_CONFIRM_ARM" if stage == 3 => arm = Some(Request::Confirm),
                "FIXTURE TRADE_UNACCEPT_ARM" if stage == 4 => arm = Some(Request::Unaccept),
                "FIXTURE TRADE_COMPLETE_ARM" if stage == 5 => arm = Some(Request::Confirm),
                "FIXTURE TRADE_DONE" if stage == 6 => done = true,
                other if other.starts_with("FIXTURE TRADE_") => {
                    return Err(format!("trade marker out of order stage={stage}: {other}"));
                }
                _ => {}
            }
        }
        let requests = std::mem::take(&mut app.world_mut().resource_mut::<Requests>().0);
        for request in requests {
            if arm.take().as_ref() != Some(&request) {
                return Err(format!(
                    "unexpected/duplicate trade request {request:?} at stage {stage}"
                ));
            }
            println!("TRADE PEER DECODED {request:?} stage={stage}");
            match stage {
                2 => update(app, Some(snapshot(TradePhase::Open, false)), None),
                3 => update(app, Some(snapshot(TradePhase::Open, true)), None),
                4 => update(app, Some(snapshot(TradePhase::Open, false)), None),
                5 => update(app, None, Some("Trade complete.")),
                _ => return Err(format!("trade request at invalid stage {stage}")),
            }
            stage += 1;
        }
        if done {
            return Ok(());
        }
        if let Some(status) = child.try_wait().map_err(|e| e.to_string())? {
            return Err(format!("trade native child exited {status}; stage={stage}"));
        }
        thread::sleep(TICK);
    }
    Err(format!(
        "trade fixture timeout stage={stage}; expected authenticated invitation and physical input"
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
    match (result, cleanup) {
        (Err(error), Err(cleanup)) => Err(format!("{error}; cleanup: {cleanup}")),
        (Err(error), _) | (_, Err(error)) => Err(error),
        (Ok(()), Ok(())) => {
            println!(
                "PASS: TRADE authenticated invitation/Yes, seven-slot authored frame, Confirm/CancelTradeAccept/Confirm, peer-only completion; deliberate reap, not shutdown/two-player-server proof"
            );
            Ok(())
        }
    }
}
