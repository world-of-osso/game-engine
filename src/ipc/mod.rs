pub(crate) mod format;
pub mod plugin;

pub use plugin::IpcPlugin;

use std::path::{Path, PathBuf};
use std::sync::{OnceLock, mpsc};

use peercred_ipc::{Connection, Server};

static SOCKET_PATH: OnceLock<PathBuf> = OnceLock::new();

extern "C" fn signal_handler(sig: libc::c_int) {
    if let Some(path) = SOCKET_PATH.get() {
        let _ = std::fs::remove_file(path);
    }
    unsafe {
        libc::signal(sig, libc::SIG_DFL);
        libc::raise(sig);
    }
}
use serde::{Deserialize, Serialize};
use shared::protocol::{
    AuctionSearchQuery, BuyoutAuction, CancelAuction, ClaimAuctionMail, CreateAuction, PlaceBid,
};

use crate::item_info::ItemInfoQuery;
use crate::mail::{ClaimMail, DeleteMail, ListMailQuery, ReadMail, SendMail};

/// IPC request from CLI to engine.
#[derive(Debug, Serialize, Deserialize, PartialEq)]
pub enum Request {
    Ping,
    Screenshot,
    DumpTree {
        filter: Option<String>,
    },
    DumpUiTree {
        filter: Option<String>,
    },
    AuctionOpen,
    AuctionBrowse {
        query: AuctionSearchQuery,
    },
    AuctionOwned,
    AuctionBids,
    AuctionInventory,
    AuctionMailbox,
    AuctionCreate {
        create: CreateAuction,
    },
    AuctionBid {
        bid: PlaceBid,
    },
    AuctionBuyout {
        buyout: BuyoutAuction,
    },
    AuctionCancel {
        cancel: CancelAuction,
    },
    AuctionClaimMail {
        claim: ClaimAuctionMail,
    },
    AuctionStatus,
    NetworkStatus,
    TerrainStatus,
    SoundStatus,
    CurrenciesStatus,
    ReputationsStatus,
    CharacterStatsStatus,
    BagsStatus,
    GuildVaultStatus,
    WarbankStatus,
    EquippedGearStatus,
    ItemInfo {
        query: ItemInfoQuery,
    },
    MailSend {
        mail: SendMail,
    },
    MailList {
        query: ListMailQuery,
    },
    MailRead {
        read: ReadMail,
    },
    MailClaim {
        claim: ClaimMail,
    },
    MailDelete {
        delete: DeleteMail,
    },
    MailStatus,
    InventoryList,
    InventorySearch {
        text: String,
    },
    InventoryWhereis {
        item_id: u32,
    },
    QuestList,
    QuestWatch,
    QuestShow {
        quest_id: u32,
    },
    GroupRoster,
    GroupStatus,
    GroupInvite {
        name: String,
    },
    GroupUninvite {
        name: String,
    },
    SpellCast {
        spell: String,
        target: Option<String>,
    },
    SpellStop,
    CombatLog {
        lines: u16,
    },
    CombatRecap {
        target: Option<String>,
    },
    ReputationList,
    CollectionMounts {
        missing: bool,
    },
    CollectionPets {
        missing: bool,
    },
    ProfessionRecipes {
        text: String,
    },
    EquipmentSet {
        slot: String,
        model_path: String,
    },
    EquipmentClear {
        slot: String,
    },
    ExportCharacter {
        output_path: String,
        character_name: Option<String>,
        character_id: Option<u64>,
    },
    ExportScene {
        output_path: String,
    },
    MapPosition,
    MapTarget,
    MapWaypointAdd {
        x: f32,
        y: f32,
    },
    MapWaypointClear,
    DumpScene {
        filter: Option<String>,
    },
}

/// IPC response from engine to CLI.
#[derive(Debug, Serialize, Deserialize)]
pub enum Response {
    Pong,
    Screenshot(Vec<u8>), // WebP bytes
    Tree(String),
    Text(String),
    Error(String),
}

/// Internal command passed from IPC server thread to Bevy main loop.
pub struct Command {
    pub request: Request,
    pub respond: mpsc::Sender<Response>,
}

/// Socket path: /tmp/game-engine-<pid>.sock
fn socket_path() -> PathBuf {
    let pid = std::process::id();
    PathBuf::from(format!("/tmp/game-engine-{pid}.sock"))
}

/// Pattern for discovering sockets.
pub fn socket_glob() -> String {
    "/tmp/game-engine-*.sock".into()
}

/// Remove stale sockets whose PID no longer exists.
pub fn cleanup_stale_sockets() {
    let pattern = socket_glob();
    let Ok(paths) = glob::glob(&pattern) else {
        return;
    };
    for entry in paths.flatten() {
        if extract_pid_and_check(&entry) == Some(true) {
            let _ = std::fs::remove_file(&entry);
        }
    }
}

fn extract_pid_and_check(path: &Path) -> Option<bool> {
    let stem = path.file_stem()?.to_str()?;
    let pid_str = stem.strip_prefix("game-engine-")?;
    let pid: u32 = pid_str.parse().ok()?;
    // Check if process exists via kill(pid, 0)
    let alive = unsafe { libc::kill(pid as i32, 0) } == 0;
    Some(!alive)
}

/// RAII guard that removes the socket file on drop.
pub struct SocketGuard {
    path: PathBuf,
}

impl Drop for SocketGuard {
    fn drop(&mut self) {
        let _ = std::fs::remove_file(&self.path);
    }
}

fn register_signal_handlers() {
    unsafe {
        libc::signal(
            libc::SIGTERM,
            signal_handler as *const () as libc::sighandler_t,
        );
        libc::signal(
            libc::SIGINT,
            signal_handler as *const () as libc::sighandler_t,
        );
    }
}

/// Spawn the IPC server on a tokio runtime in a background thread.
/// Returns a receiver for commands and a guard that cleans up the socket.
pub fn init() -> (mpsc::Receiver<Command>, SocketGuard) {
    cleanup_stale_sockets();

    let path = socket_path();
    SOCKET_PATH.set(path.clone()).ok();
    register_signal_handlers();

    let (tx, rx) = mpsc::channel();
    let sock = path.clone();
    std::thread::spawn(move || {
        let rt = tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
            .expect("tokio runtime");
        rt.block_on(serve(sock, tx));
    });

    (rx, SocketGuard { path })
}

async fn serve(path: PathBuf, tx: mpsc::Sender<Command>) {
    let server = match Server::bind(&path) {
        Ok(s) => s,
        Err(e) => {
            eprintln!("ipc: bind failed: {e}");
            return;
        }
    };

    loop {
        let (conn, _caller) = match server.accept().await {
            Ok(pair) => pair,
            Err(e) => {
                eprintln!("ipc: accept error: {e}");
                continue;
            }
        };
        let tx = tx.clone();
        tokio::spawn(handle_connection(conn, tx));
    }
}

async fn handle_connection(mut conn: Connection, tx: mpsc::Sender<Command>) {
    let request: Request = match conn.read().await {
        Ok(r) => r,
        Err(e) => {
            eprintln!("ipc: read error: {e}");
            return;
        }
    };

    let (resp_tx, resp_rx) = mpsc::channel();
    let cmd = Command {
        request,
        respond: resp_tx,
    };

    if tx.send(cmd).is_err() {
        return;
    }

    // Wait for Bevy to produce a response (blocking in async context is fine
    // here — this is a dedicated per-connection task).
    let response = match resp_rx.recv() {
        Ok(r) => r,
        Err(_) => Response::Error("internal: channel closed".into()),
    };

    if let Err(e) = conn.write(&response).await {
        eprintln!("ipc: write error: {e}");
    }
}
