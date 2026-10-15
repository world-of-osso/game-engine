//! Private owner-side movement for stationary-pointer Cleanup replication proof.
//! Uses real login/house admission and restores the paid instance through owner commands.
use game_engine_network::{BridgeConfig, Event, NetworkBridge, ProtocolMessage};
use serde::Deserialize;
use shared::protocol::*;
use std::{
    fs,
    path::{Path, PathBuf},
    thread,
    time::{Duration, Instant, SystemTime, UNIX_EPOCH},
};

const MOVE_REQUEST: u64 = 10;
const RESTORE_REQUEST: u64 = 11;
const ENTER_REQUEST: u64 = 12;
// Admission may queue the native observer before it starts; movement assertions keep their own bounds.
const COORDINATION_TIMEOUT: Duration = Duration::from_secs(3600);

#[derive(Clone, Copy, PartialEq)]
enum Phase {
    Connecting,
    Admission,
    Ready,
    Moving,
    Moved,
    Restoring,
    Done,
}

#[derive(Deserialize)]
struct MoveRequest {
    position: [f32; 3],
}

struct Peer {
    bridge: NetworkBridge,
    coord: PathBuf,
    house: u64,
    phase: Phase,
    original: Option<HousingState>,
    query_id: u64,
    query_pending: bool,
    next_query: Instant,
    enter_requested: bool,
}

fn write_json(path: &Path, value: &impl serde::Serialize) -> Result<(), String> {
    let bytes = serde_json::to_vec_pretty(value).map_err(|e| e.to_string())?;
    let temporary = path.with_extension("tmp");
    fs::write(&temporary, bytes).map_err(|e| format!("Write {}: {e}", temporary.display()))?;
    fs::rename(&temporary, path).map_err(|e| format!("Publish {}: {e}", path.display()))
}

fn owned_layout(state: &HousingState, house: u64) -> Result<&HouseLayout, String> {
    let layout = state
        .layout
        .as_ref()
        .ok_or("Owner has no admitted layout")?;
    if layout.house_id != house || !layout.can_edit {
        return Err("Actual owner house/edit authority required".into());
    }
    Ok(layout)
}

impl Peer {
    fn send_request(&self, request_id: u64, command: HousingCommand) -> Result<(), String> {
        self.bridge.send::<_, HousingChannel>(HousingRequest {
            request_id,
            command,
        })
    }

    fn receive(&mut self, message: ProtocolMessage) -> Result<(), String> {
        if message.is::<LoginResponse>() {
            let reply = message
                .downcast::<LoginResponse>()
                .map_err(|_| "login type")?;
            if !reply.success {
                return Err(format!("Owner login refused: {:?}", reply.error));
            }
            return self.bridge.send::<_, AuthChannel>(SelectCharacter {
                character_id: reply
                    .characters
                    .first()
                    .ok_or("Owner roster empty")?
                    .character_id,
            });
        }
        if message.is::<EnterWorldResponse>() {
            let reply = message
                .downcast::<EnterWorldResponse>()
                .map_err(|_| "entry type")?;
            if !reply.success {
                return Err(format!("Owner world entry refused: {:?}", reply.error));
            }
            self.phase = Phase::Admission;
        } else if message.is::<NewWorld>() {
            self.bridge.send::<_, TransferChannel>(WorldPortAck)?;
        } else if message.is::<TransferAborted>() {
            return Err(format!(
                "Owner transfer refused: {:?}",
                message
                    .downcast::<TransferAborted>()
                    .map_err(|_| "transfer type")?
            ));
        } else if message.is::<HousingReply>() {
            self.receive_housing(
                message
                    .downcast::<HousingReply>()
                    .map_err(|_| "housing type")?,
            )?;
        }
        Ok(())
    }

    fn receive_housing(&mut self, reply: HousingReply) -> Result<(), String> {
        if let Some(error) = reply.error {
            return Err(format!(
                "Owner request{} refused: {error}",
                reply.request_id
            ));
        }
        if self.phase == Phase::Admission && self.query_pending && reply.request_id == self.query_id
        {
            self.query_pending = false;
            return self.receive_admission(reply.state);
        }
        if self.phase == Phase::Moving && reply.request_id == MOVE_REQUEST {
            let layout = owned_layout(&reply.state, self.house)?;
            let original = self.original_decor()?;
            let moved = layout
                .decor
                .iter()
                .find(|decor| decor.instance_id == original.instance_id)
                .ok_or("Moved instance disappeared")?;
            if moved.transform.position == original.transform.position {
                return Err("Owner move did not change the actual instance".into());
            }
            write_json(&self.coord.join("moved.json"), &reply.state)?;
            self.phase = Phase::Moved;
        } else if self.phase == Phase::Restoring && reply.request_id == RESTORE_REQUEST {
            self.receive_restore(reply.state)?;
        }
        Ok(())
    }

    fn receive_admission(&mut self, state: HousingState) -> Result<(), String> {
        let admitted = state
            .layout
            .as_ref()
            .is_some_and(|layout| layout.house_id == self.house && layout.can_edit);
        if admitted {
            let layout = owned_layout(&state, self.house)?;
            if layout.decor.len() != 1 || layout.decor[0].decor_id != 1693 {
                return Err(
                    "Require the existing single paid bookcase, never a fixture grant".into(),
                );
            }
            write_json(&self.coord.join("ready.json"), &state)?;
            self.original = Some(state);
            self.phase = Phase::Ready;
            println!("HOUSING_HOVER_OWNER_READY house={}", self.house);
        } else if !self.enter_requested {
            if !state
                .owned_houses
                .iter()
                .any(|house| house.house_id == self.house)
            {
                return Err("Requested house is not account-owned".into());
            }
            self.send_request(
                ENTER_REQUEST,
                HousingCommand::EnterHouse {
                    house_id: self.house,
                },
            )?;
            self.enter_requested = true;
        }
        self.next_query = Instant::now() + Duration::from_millis(500);
        Ok(())
    }

    fn original_decor(&self) -> Result<&PlacedDecor, String> {
        self.original
            .as_ref()
            .and_then(|state| state.layout.as_ref())
            .and_then(|layout| layout.decor.first())
            .ok_or_else(|| "Original owner instance missing".into())
    }

    fn begin_restore(&mut self) -> Result<(), String> {
        let original = self.original_decor()?;
        self.send_request(
            RESTORE_REQUEST,
            HousingCommand::MoveDecor {
                house_id: self.house,
                instance_id: original.instance_id,
                position: original.transform.position,
            },
        )?;
        self.phase = Phase::Restoring;
        Ok(())
    }

    fn receive_restore(&mut self, state: HousingState) -> Result<(), String> {
        let original = self
            .original
            .as_ref()
            .ok_or("Original owner state missing")?;
        if owned_layout(&state, self.house)?.decor != owned_layout(original, self.house)?.decor
            || state.stored != original.stored
            || state.placed != original.placed
        {
            return Err("Owner restoration failed exact layout/quantity conservation".into());
        }
        write_json(&self.coord.join("restored.json"), &state)?;
        self.phase = Phase::Done;
        println!("HOUSING_HOVER_OWNER_RESTORED house={}", self.house);
        Ok(())
    }

    fn advance(&mut self) -> Result<(), String> {
        if self.phase == Phase::Admission
            && !self.query_pending
            && Instant::now() >= self.next_query
        {
            self.query_id += 1;
            self.send_request(
                self.query_id,
                HousingCommand::ListPlots { neighborhood_id: 1 },
            )?;
            self.query_pending = true;
        }
        let has_original = self.original.is_some();
        let can_abort = matches!(self.phase, Phase::Ready | Phase::Moving | Phase::Moved);
        if has_original && can_abort && self.coord.join("abort.request").exists() {
            return self.begin_restore();
        }
        if self.phase == Phase::Ready && self.coord.join("move.json").exists() {
            let path = self.coord.join("move.json");
            let bytes = fs::read(&path).map_err(|e| format!("Read {}: {e}", path.display()))?;
            let request: MoveRequest =
                serde_json::from_slice(&bytes).map_err(|e| format!("Move request: {e}"))?;
            if !request.position.iter().all(|value| value.is_finite()) {
                return Err("Move position must be finite".into());
            }
            self.send_request(
                MOVE_REQUEST,
                HousingCommand::MoveDecor {
                    house_id: self.house,
                    instance_id: self.original_decor()?.instance_id,
                    position: request.position,
                },
            )?;
            self.phase = Phase::Moving;
        } else if self.phase == Phase::Moved && self.coord.join("restore.request").exists() {
            self.begin_restore()?;
        }
        Ok(())
    }

    fn drive(&mut self, duration: Duration) -> Result<(), String> {
        let deadline = Instant::now() + duration;
        while Instant::now() < deadline {
            for event in self.bridge.drain_events()? {
                match event {
                    Event::Message(message) => self.receive(message)?,
                    Event::Disconnected(reason) => {
                        return Err(format!("Owner disconnected: {reason:?}"));
                    }
                    Event::ProtocolRejected(reason) => return Err(reason),
                    _ => {}
                }
            }
            self.advance()?;
            if self.phase == Phase::Done {
                return Ok(());
            }
            thread::sleep(Duration::from_millis(25));
        }
        Err("Owner/observer coordination deadline".into())
    }
}

fn connect_peer() -> Result<Peer, String> {
    let address = std::env::var("GODOT_TEST_SERVER").map_err(|e| e.to_string())?;
    let account = std::env::var("HOUSING_ACCOUNT").map_err(|e| e.to_string())?;
    let house = std::env::var("HOUSING_HOUSE_ID")
        .map_err(|e| e.to_string())?
        .parse::<u64>()
        .map_err(|e| e.to_string())?;
    let coord = PathBuf::from(std::env::var("HOUSING_HOVER_COORD").map_err(|e| e.to_string())?)
        .canonicalize()
        .map_err(|e| e.to_string())?;
    if address != "127.0.0.1:5870"
        || !account.starts_with("fb_housing_")
        || house == 0
        || !coord.starts_with("/home/osso/.worktrees/housing-core14-evidence")
    {
        return Err(
            "Explicit private5870 owner/house/private coordination directory required".into(),
        );
    }
    if coord.join("ready.json").exists() {
        return Err("Fresh owner coordination directory required".into());
    }
    let client_id = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_err(|e| e.to_string())?
        .as_nanos() as u64;
    let bridge = BridgeConfig::new()
        .receive::<LoginResponse>()
        .receive::<EnterWorldResponse>()
        .receive::<NewWorld>()
        .receive::<TransferAborted>()
        .receive::<HousingReply>()
        .connect(
            address
                .parse()
                .map_err(|e: std::net::AddrParseError| e.to_string())?,
            client_id,
        )?;
    bridge.send::<_, AuthChannel>(LoginRequest {
        token: None,
        username: account,
        password: "fbtest".into(),
    })?;
    Ok(Peer {
        bridge,
        coord,
        house,
        phase: Phase::Connecting,
        original: None,
        query_id: 100,
        query_pending: false,
        next_query: Instant::now(),
        enter_requested: false,
    })
}

fn run() -> Result<(), String> {
    let mut peer = connect_peer()?;
    let result = peer.drive(COORDINATION_TIMEOUT);
    if let Err(error) = result {
        if peer.original.is_some() && peer.phase != Phase::Done {
            peer.begin_restore()
                .map_err(|restore| format!("{error}; restore send failed: {restore}"))?;
            peer.drive(Duration::from_secs(120))
                .map_err(|restore| format!("{error}; restore not acknowledged: {restore}"))?;
        }
        return Err(error);
    }
    println!("PASS: private owner moved and exactly restored paid decor");
    Ok(())
}

fn main() {
    if let Err(error) = run() {
        eprintln!("HOUSING_STATIONARY_OWNER_ERROR {error}");
        std::process::exit(1);
    }
}
