//! Real private5891 membership transitions and owner spell receipts; no injected grants.
use game_engine_network::{Event, NetworkBridge, ProtocolMessage};
use shared::protocol::*;
use std::{
    collections::BTreeSet,
    path::Path,
    thread,
    time::{Duration, Instant},
};

struct Actor {
    bridge: NetworkBridge,
    name: String,
    entered: bool,
    snapshot: bool,
    known: BTreeSet<u32>,
    additions: Vec<u32>,
    removals: Vec<u32>,
}

impl Actor {
    fn connect(suffix: &str, client_id: u64) -> Result<Self, String> {
        let bridge = NetworkBridge::connect("127.0.0.1:5891".parse().unwrap(), client_id)?;
        bridge.send::<_, AuthChannel>(LoginRequest {
            token: None,
            username: format!("fb_guildcompletion_perks{suffix}"),
            password: "fbtest".into(),
        })?;
        Ok(Self {
            bridge,
            name: format!("Gcperks{suffix}"),
            entered: false,
            snapshot: false,
            known: BTreeSet::new(),
            additions: Vec::new(),
            removals: Vec::new(),
        })
    }

    fn poll(&mut self) -> Result<(), String> {
        for event in self.bridge.drain_events()? {
            match event {
                Event::Message(message) => self.receive(message)?,
                Event::Disconnected(reason) => {
                    return Err(format!("{} disconnected: {reason:?}", self.name));
                }
                Event::ProtocolRejected(reason) => return Err(reason),
                _ => {}
            }
        }
        Ok(())
    }

    fn receive(&mut self, message: ProtocolMessage) -> Result<(), String> {
        if message.is::<LoginResponse>() {
            return self.login(message);
        }
        if message.is::<NewWorld>() {
            return self.bridge.send::<_, TransferChannel>(WorldPortAck);
        }
        if message.is::<EnterWorldResponse>() {
            let reply = message
                .downcast::<EnterWorldResponse>()
                .map_err(|_| "entry decode")?;
            if !reply.success {
                return Err(format!("Entry refused: {:?}", reply.error));
            }
            self.entered = true;
        } else if message.is::<GuildInvitation>() {
            return self.accept_invitation(message);
        } else {
            return self.receive_reply(message);
        }
        Ok(())
    }

    fn accept_invitation(&self, message: ProtocolMessage) -> Result<(), String> {
        let invitation = message
            .downcast::<GuildInvitation>()
            .map_err(|_| "invite decode")?;
        let expected_peer = self.name == "Gcperkspeer";
        let expected_inviter = invitation.inviter_name == "Gcperkslead";
        if !expected_peer || !expected_inviter || invitation.guild_name != "Guild Perks" {
            return Err(format!("Unexpected invitation: {invitation:?}"));
        }
        self.bridge.send::<_, AuthChannel>(GuildAcceptInviteRequest)
    }

    fn receive_reply(&mut self, message: ProtocolMessage) -> Result<(), String> {
        if message.is::<GuildInviteResponse>() {
            let reply = message
                .downcast::<GuildInviteResponse>()
                .map_err(|_| "invite reply decode")?;
            if !reply.success {
                return Err(format!("Invite refused: {:?}", reply.error));
            }
        } else if message.is::<GuildAcceptInviteResponse>() {
            let reply = message
                .downcast::<GuildAcceptInviteResponse>()
                .map_err(|_| "accept decode")?;
            if !reply.success {
                return Err(format!("Accept refused: {:?}", reply.error));
            }
        } else {
            self.receive_knowledge(message)?;
        }
        Ok(())
    }

    fn login(&self, message: ProtocolMessage) -> Result<(), String> {
        let reply = message
            .downcast::<LoginResponse>()
            .map_err(|_| "login decode")?;
        if !reply.success {
            return Err(format!("Login refused: {:?}", reply.error));
        }
        let character = reply
            .characters
            .iter()
            .find(|row| row.name == self.name)
            .ok_or_else(|| format!("Missing actual character {}", self.name))?;
        self.bridge.send::<_, AuthChannel>(SelectCharacter {
            character_id: character.character_id,
        })
    }

    fn receive_knowledge(&mut self, message: ProtocolMessage) -> Result<(), String> {
        if message.is::<KnownSpellsSnapshot>() {
            let reply = message
                .downcast::<KnownSpellsSnapshot>()
                .map_err(|_| "snapshot decode")?;
            self.known = reply.spells.into_iter().collect();
            self.snapshot = true;
        } else if message.is::<SpellsLearned>() {
            let reply = message
                .downcast::<SpellsLearned>()
                .map_err(|_| "learn decode")?;
            self.additions.extend(&reply.spells);
            self.known.extend(reply.spells);
        } else if message.is::<SpellsUnlearned>() {
            let reply = message
                .downcast::<SpellsUnlearned>()
                .map_err(|_| "unlearn decode")?;
            self.removals.extend(&reply.spells);
            for id in reply.spells {
                self.known.remove(&id);
            }
        }
        Ok(())
    }

    fn invite(&self) -> Result<(), String> {
        self.bridge.send::<_, AuthChannel>(GuildInviteRequest {
            target_character_name: "Gcperkspeer".into(),
        })
    }
}

fn wait_for(
    actors: &mut [Actor; 2],
    label: &str,
    predicate: impl Fn(&[Actor; 2]) -> bool,
) -> Result<(), String> {
    let deadline = Instant::now() + Duration::from_secs(90);
    loop {
        for actor in &mut *actors {
            actor.poll()?;
        }
        if predicate(actors) {
            return Ok(());
        }
        if Instant::now() >= deadline {
            return Err(format!(
                "Timed out: {label}; lead={:?}, peer={:?}",
                actors[0].known, actors[1].known
            ));
        }
        thread::sleep(Duration::from_millis(20));
    }
}

fn await_login(actors: &mut [Actor; 2], perks: &BTreeSet<u32>) -> Result<BTreeSet<u32>, String> {
    wait_for(actors, "actual member/nonmember login", |a| {
        a.iter().all(|v| v.entered && v.snapshot) && perks.is_subset(&a[0].known)
    })?;
    assert!(
        perks.is_disjoint(&actors[1].known),
        "Nonmember login must not know perks"
    );
    let baseline = actors[1].known.clone();
    actors[1].additions.clear();
    actors[1].removals.clear();
    Ok(baseline)
}

fn transitions(actors: &mut [Actor; 2], perks: &BTreeSet<u32>) -> Result<(), String> {
    let baseline = await_login(actors, perks)?;
    actors[0].invite()?;
    wait_for(actors, "accepted invitation grants", |a| {
        perks.is_subset(&a[1].known)
    })?;
    actors[0]
        .bridge
        .send::<_, GuildChannel>(GuildRankRequest::RemoveMember {
            character_name: "Gcperkspeer".into(),
        })?;
    wait_for(actors, "online forced-removal revocation", |a| {
        a[1].known == baseline
    })?;
    actors[0].invite()?;
    wait_for(actors, "rejoin grants", |a| perks.is_subset(&a[1].known))?;
    actors[1]
        .bridge
        .send::<_, GuildChannel>(GuildRankRequest::Leave)?;
    wait_for(actors, "ordinary quit revocation", |a| {
        a[1].known == baseline
    })?;
    assert_exact_deltas(&actors[1], perks);
    Ok(())
}

fn assert_exact_deltas(peer: &Actor, perks: &BTreeSet<u32>) {
    let mut expected: Vec<_> = perks.iter().copied().chain(perks.iter().copied()).collect();
    expected.sort_unstable();
    let mut additions = peer.additions.clone();
    additions.sort_unstable();
    let mut removals = peer.removals.clone();
    removals.sort_unstable();
    assert_eq!(
        additions, expected,
        "Exactly two real perk grants, no other knowledge mutation"
    );
    assert_eq!(
        removals, expected,
        "Exactly two real revocations, preserving class knowledge"
    );
}

fn run() -> Result<(), String> {
    let data = std::env::var("GUILD_PERKS_DATA").map_err(|e| e.to_string())?;
    let output = std::env::var("GUILD_PERKS_MEMBERSHIP_RECEIPT").map_err(|e| e.to_string())?;
    let perks = shared::guild_perks::load_guild_perks(Path::new(&data))?
        .into_iter()
        .map(|row| row.spell_id)
        .collect();
    let mut actors = [
        Actor::connect("lead", 9589101)?,
        Actor::connect("peer", 9589102)?,
    ];
    let result = transitions(&mut actors, &perks);
    if result.is_ok() {
        let receipt = serde_json::json!({"perk_ids": perks, "peer_final_known": actors[1].known,
            "actual_additions": actors[1].additions, "actual_removals": actors[1].removals});
        std::fs::write(
            &output,
            serde_json::to_vec_pretty(&receipt).map_err(|e| e.to_string())?,
        )
        .map_err(|e| e.to_string())?;
    }
    for mut actor in actors {
        actor.bridge.stop()?;
    }
    result?;
    println!(
        "PASS GUILD_PERKS_MEMBERSHIP actual_invite_accept=true forced_removal=true rejoin=true quit=true class_conserved=true"
    );
    Ok(())
}

fn main() {
    if let Err(error) = run() {
        eprintln!("FAIL GUILD_PERKS_MEMBERSHIP {error}");
        std::process::exit(1);
    }
}
