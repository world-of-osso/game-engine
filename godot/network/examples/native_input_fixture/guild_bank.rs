//! Owned authenticated guild-vault peer. Registered as one parent fixture mode.
//! Opening-only RED uses GUILD_BANK_OPENING_PROBE=1; it is not vault-pick acceptance.
use super::*;
use shared::protocol::{
    GAMEOBJECT_TYPE_GUILD_BANK, GameObjectInfo, GuildBankChannel, GuildBankContents,
    GuildBankTabView, UseGameObject,
};

#[derive(Resource, Default)]
struct Requests(Vec<UseGameObject>);

fn receive(
    mut receivers: Query<&mut MessageReceiver<UseGameObject>>,
    mut requests: ResMut<Requests>,
) {
    for mut receiver in &mut receivers {
        requests.0.extend(receiver.receive());
    }
}

pub(super) fn install(app: &mut App) {
    app.init_resource::<Requests>().add_systems(Update, receive);
}

fn spawn_vault(app: &mut App) -> u64 {
    app.world_mut()
        .spawn((
            GameObjectInfo {
                entry: 187_235,
                go_type: GAMEOBJECT_TYPE_GUILD_BANK,
                display_id: 7607,
                name: "Fixture Guild Vault".into(),
                scale: 1.0,
            },
            Position {
                x: FIRST[0] - 2.0,
                y: FIRST[1],
                z: FIRST[2] + 3.0,
            },
            shared::components::Rotation {
                x: 0.0,
                y: 0.0,
                z: 0.0,
            },
            Replicate::to_clients(NetworkTarget::All),
        ))
        .id()
        .to_bits()
}

fn contents(object: u64) -> GuildBankContents {
    GuildBankContents {
        object,
        guild_name: "Fixture Guild".into(),
        tabs: vec![GuildBankTabView {
            name: "Supplies".into(),
            icon: 132_074,
            viewable: true,
            can_deposit: true,
            withdrawals_per_day: Some(2),
            remaining_withdrawals: Some(2),
            text: "Raid supplies".into(),
            slots: vec![None; 98],
        }],
        money: 50_000,
        withdraw_money_remaining: Some(10_000),
        next_tab_cost: Some(100_000),
        is_leader: true,
    }
}

fn opened(app: &mut App, object: u64) {
    send::<_, InteractionChannel>(
        app,
        InteractionOpened {
            npc: object,
            kind: InteractionKind::Role(NpcRole::GuildBanker),
        },
    );
    send::<_, GuildBankChannel>(app, contents(object));
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
    println!("GUILD_BANK AUTHORITY_SENT object={object} role=GuildBanker guild=FixtureGuild");
}

pub(super) fn run(
    app: &mut App,
    child: &mut Child,
    lines: Receiver<String>,
    readers: Vec<thread::JoinHandle<()>>,
) -> Result<(), String> {
    let probe = std::env::var("GUILD_BANK_OPENING_PROBE").as_deref() == Ok("1");
    let mut selected = None;
    let mut remote = None;
    let mut vault = None;
    let mut loading = false;
    let mut ready = false;
    let mut opened_once = false;
    let mut done = false;
    let mut readers = Some(readers);
    let deadline = Instant::now() + TIMEOUT + Duration::from_secs(180);
    while Instant::now() < deadline {
        app.update();
        respond_to_login(app, StartupScreen::GuildBank)?;
        respond_to_selection(app, StartupScreen::GuildBank, &mut selected, &mut remote)?;
        if selected.is_some() && vault.is_none() {
            vault = Some(spawn_vault(app));
        }
        for line in lines.try_iter() {
            let line = line.trim();
            bags::reject_runtime_error(line)?;
            match line {
                "FIXTURE GUILD_BANK_LOADING" if selected.is_some() && !loading => {
                    loading = true;
                    send::<_, TerrainChannel>(
                        app,
                        LoadTerrain {
                            map_name: "azeroth".into(),
                            initial_tile_y: 32,
                            initial_tile_x: 48,
                        },
                    );
                }
                "FIXTURE GUILD_BANK_READY" if loading && !ready => {
                    ready = true;
                    if probe {
                        opened(app, vault.ok_or("opening probe without replicated vault")?);
                        opened_once = true;
                    }
                }
                "FIXTURE GUILD_BANK_OPEN_DONE" if ready && opened_once => done = true,
                other if other.starts_with("FIXTURE GUILD_BANK_") => {
                    return Err(format!("out-of-order guild bank marker: {other}"));
                }
                _ => {}
            }
        }
        let requests = std::mem::take(&mut app.world_mut().resource_mut::<Requests>().0);
        for request in requests {
            if probe || !ready || opened_once || Some(request.object) != vault {
                return Err(format!(
                    "unexpected UseGameObject {request:?}; vault={vault:?}"
                ));
            }
            println!("GUILD_BANK PHYSICAL_USE object={}", request.object);
            opened(app, request.object);
            opened_once = true;
        }
        if let Some(status) = child.try_wait().map_err(|e| e.to_string())? {
            for reader in readers.take().expect("join fixture readers once") {
                reader
                    .join()
                    .map_err(|_| "guild bank output reader panicked")?;
            }
            // Drain final stdout after reader join on the next iteration.
            for line in lines.try_iter() {
                bags::reject_runtime_error(line.trim())?;
                if line.trim() == "FIXTURE GUILD_BANK_OPEN_DONE" && opened_once {
                    done = true;
                }
            }
            if status.success() && done && opened_once {
                println!(
                    "PASS: authenticated guild bank opening; opening_probe={probe}; full feature acceptance pending"
                );
                return Ok(());
            }
            return Err(format!(
                "guild bank fixture exited {status}; ready={ready}, authority_sent={opened_once}, done={done}, opening_probe={probe}"
            ));
        }
        thread::sleep(TICK);
    }
    Err(format!(
        "guild bank timed out; ready={ready}, authority_sent={opened_once}, opening_probe={probe}"
    ))
}
