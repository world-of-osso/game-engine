//! Private5891 real Lillehoff purchases/sales and authenticated news controls.
use game_engine_network::{BridgeConfig, Event, NetworkBridge, ProtocolMessage, WireMessage};
use shared::protocol::*;
use std::{
    fs,
    path::PathBuf,
    thread,
    time::{Duration, Instant, SystemTime, UNIX_EPOCH},
};

struct Peer {
    bridge: NetworkBridge,
    directory: PathBuf,
    inbox: std::collections::VecDeque<ProtocolMessage>,
}
impl Peer {
    fn wait<T: WireMessage>(&mut self) -> Result<T, String> {
        let deadline = Instant::now() + Duration::from_secs(180);
        while Instant::now() < deadline {
            for event in self.bridge.drain_events()? {
                match event {
                    Event::Message(message) => {
                        if message.is::<NewWorld>() {
                            self.bridge.send::<_, TransferChannel>(WorldPortAck)?;
                        } else {
                            self.inbox.push_back(message);
                        }
                    }
                    Event::Disconnected(reason) => return Err(format!("Disconnected: {reason:?}")),
                    Event::ProtocolRejected(reason) => return Err(reason),
                    _ => {}
                }
            }
            while let Some(message) = self.inbox.pop_front() {
                if message.is::<MerchantFailed>() {
                    let refused = message
                        .downcast::<MerchantFailed>()
                        .map_err(|_| "decode merchant refusal")?;
                    return Err(format!("Real merchant refused: {refused:?}"));
                }
                if message.is::<T>() {
                    return message.downcast::<T>().map_err(|_| "decode reply".into());
                }
            }
            thread::sleep(Duration::from_millis(20));
        }
        Err(format!("Timed out: {}", std::any::type_name::<T>()))
    }
    fn write<T: serde::Serialize>(&self, name: &str, value: &T) -> Result<(), String> {
        let path = self.directory.join(name);
        let tmp = path.with_extension("new");
        fs::write(
            &tmp,
            serde_json::to_vec_pretty(value).map_err(|e| e.to_string())?,
        )
        .map_err(|e| e.to_string())?;
        fs::rename(tmp, path).map_err(|e| e.to_string())
    }
    fn marker(&self, name: &str) -> Result<(), String> {
        let deadline = Instant::now() + Duration::from_secs(1200);
        while Instant::now() < deadline {
            if self.directory.join(name).exists() {
                return Ok(());
            }
            thread::sleep(Duration::from_millis(100));
        }
        Err(format!("Missing native command {name}"))
    }
    fn query(&mut self) -> Result<GuildNewsState, String> {
        self.bridge
            .send::<_, GuildChannel>(GuildNewsRequest::Query)?;
        self.wait()
    }
    fn buy_and_sell(&mut self, vendor: &VendorInventory) -> Result<(), String> {
        let item = vendor
            .items
            .iter()
            .find(|i| i.item_id == 50335)
            .ok_or("Actual50335 offer missing")?;
        self.bridge.send::<_, MerchantChannel>(BuyItem {
            npc: vendor.npc,
            slot: item.slot,
            item_id: item.item_id,
            count: 1,
            destination: None,
        })?;
        let deadline = Instant::now() + Duration::from_secs(180);
        let guid = loop {
            let delta: InventoryDelta = self.wait()?;
            if let Some(item) = delta
                .changes
                .iter()
                .filter_map(|c| c.item.as_ref())
                .find(|i| i.item_id == 50335)
            {
                break item.item_guid;
            }
            if Instant::now() >= deadline {
                return Err("No actual purchased bag item".into());
            }
        };
        self.bridge.send::<_, MerchantChannel>(SellItem {
            npc: vendor.npc,
            item_guid: guid,
            count: 0,
        })?;
        loop {
            let delta: InventoryDelta = self.wait()?;
            if delta.changes.iter().any(|c| c.item.is_none()) {
                break;
            }
        }
        Ok(())
    }
    fn open_vendor(&mut self) -> Result<VendorInventory, String> {
        self.write("peer-ready.json", &serde_json::json!({"entered":true}))?;
        self.marker("npc.json")?;
        let npc: u64 = serde_json::from_slice(
            &fs::read(self.directory.join("npc.json")).map_err(|e| e.to_string())?,
        )
        .map_err(|e| e.to_string())?;
        self.bridge
            .send::<_, InteractionChannel>(InteractNpc { npc })?;
        let opened: InteractionOpened = self.wait()?;
        let InteractionKind::Gossip(menu) = opened.kind else {
            return Err("Expected actual Lillehoff gossip".into());
        };
        let option = menu
            .options
            .iter()
            .find(|o| o.icon == 1)
            .ok_or("Vendor gossip missing")?;
        self.bridge
            .send::<_, InteractionChannel>(SelectGossipOption {
                npc,
                option_id: option.option_id,
            })?;
        self.wait()
    }
    fn initial_acquisitions(
        &mut self,
        vendor: &VendorInventory,
        resume: bool,
    ) -> Result<GuildNewsState, String> {
        let initial = if resume {
            let bytes = fs::read(self.directory.join("initial.json")).map_err(|e| e.to_string())?;
            serde_json::from_slice::<GuildNewsState>(&bytes).map_err(|e| e.to_string())?
        } else {
            for _ in 0..2 {
                self.buy_and_sell(vendor)?;
            }
            let initial = self.query()?;
            self.write("initial.json", &initial)?;
            initial
        };
        Ok(initial)
    }
    fn workload(&mut self) -> Result<(), String> {
        let vendor = self.open_vendor()?;
        let resume = std::env::var("GUILD_NEWS_RESUME").as_deref() == Ok("1");
        let initial = self.initial_acquisitions(&vendor, resume)?;
        self.marker("overflow.go")?;
        let initial_id = initial
            .entries
            .last()
            .ok_or("Initial real acquisition missing")?
            .id;
        let target = initial_id + 251;
        let current = self
            .query()?
            .entries
            .last()
            .ok_or("Current real history missing")?
            .id;
        let remaining = target
            .checked_sub(current)
            .ok_or("Unrelated events exceeded private workload target")?;
        println!(
            "REAL_NEWS_TARGET initial_id={initial_id} target_id={target} current_id={current} remaining={remaining} resume={resume}"
        );
        for index in 0..remaining {
            self.buy_and_sell(&vendor)?;
            if index % 25 == 0 {
                println!("REAL_NEWS_COMMITTED_ID {}", current + index + 1);
            }
        }
        let reply = self.verify_retention(target)?;
        self.write("overflow.json", &reply)?;
        self.check_sticky_refusal(reply.entries.last().ok_or("No news")?.id)?;
        self.leave_and_check_privacy()?;
        println!(
            "PASS GUILD_NEWS_PEER cumulative_purchases=253 cumulative_sold=253 retained=250 permission_refused=true former_member_private=true resumed={resume}"
        );
        Ok(())
    }
    fn verify_retention(&mut self, target: u64) -> Result<GuildNewsState, String> {
        let mut reply = self.query()?;
        while reply.entries.last().is_some_and(|entry| entry.id < target) {
            reply = self.query()?;
        }
        if reply.entries.last().map(|entry| entry.id) != Some(target) {
            return Err(format!("Wrong real acquisition target: expected{target}"));
        }
        if reply.entries.len() != 250 {
            return Err(format!("Real retention {}", reply.entries.len()));
        }
        Ok(reply)
    }
    fn check_sticky_refusal(&mut self, id: u64) -> Result<(), String> {
        self.bridge
            .send::<_, GuildChannel>(GuildNewsRequest::SetSticky { id, sticky: true })?;
        loop {
            let denied: GuildNewsState = self.wait()?;
            if denied.error == Some(GuildNewsError::Permissions) {
                self.write("permission-refusal.json", &denied)?;
                break;
            }
        }
        Ok(())
    }
    fn leave_and_check_privacy(&mut self) -> Result<(), String> {
        self.marker("privacy.go")?;
        self.bridge
            .send::<_, GuildChannel>(GuildRankRequest::Leave)?;
        loop {
            let state: GuildRanksState = self.wait()?;
            if state.ranks.is_empty() {
                break;
            }
        }
        let private = self.query()?;
        if private.error != Some(GuildNewsError::NotInGuild) || !private.entries.is_empty() {
            return Err("Former member news exposed".into());
        }
        self.write("privacy.json", &private)?;
        Ok(())
    }
}
fn run() -> Result<(), String> {
    let endpoint = std::env::var("GODOT_TEST_SERVER").map_err(|e| e.to_string())?;
    if endpoint != "127.0.0.1:5891" {
        return Err("Private5891 only".into());
    }
    let directory =
        PathBuf::from(std::env::var("GUILD_NEWS_DIAGNOSTICS").map_err(|e| e.to_string())?);
    let id = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_err(|e| e.to_string())?
        .as_nanos() as u64;
    let bridge = BridgeConfig::new()
        .receive::<LoginResponse>()
        .receive::<EnterWorldResponse>()
        .receive::<NewWorld>()
        .receive::<InteractionOpened>()
        .receive::<VendorInventory>()
        .receive::<InventoryDelta>()
        .receive::<MerchantFailed>()
        .receive::<GuildNewsState>()
        .receive::<GuildRanksState>()
        .connect(
            endpoint
                .parse()
                .map_err(|e: std::net::AddrParseError| e.to_string())?,
            id,
        )?;
    let mut peer = Peer {
        bridge,
        directory,
        inbox: Default::default(),
    };
    peer.bridge.send::<_, AuthChannel>(LoginRequest {
        token: None,
        username: "fb_guildcompletion_newsbuyer".into(),
        password: "fbtest".into(),
    })?;
    let login: LoginResponse = peer.wait()?;
    if !login.success {
        return Err(format!("Login refused {:?}", login.error));
    }
    let character = login
        .characters
        .iter()
        .find(|c| c.name == "Gcnewsbuyer")
        .ok_or("Private buyer missing")?;
    peer.bridge.send::<_, AuthChannel>(SelectCharacter {
        character_id: character.character_id,
    })?;
    let entered: EnterWorldResponse = peer.wait()?;
    if !entered.success {
        return Err(format!("Enter refused {:?}", entered.error));
    }
    let result = peer.workload();
    let stopped = peer.bridge.stop();
    result.and(stopped)
}
fn main() {
    if let Err(error) = run() {
        eprintln!("GUILD_NEWS_PEER_ERROR {error}");
        std::process::exit(1);
    }
}
