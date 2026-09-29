# game-engine-network

Headless Rust transport boundary for a native Godot host. Requires Bevy **ECS/state/scheduling only**, Lightyear 0.28 UDP/Netcode and Replicon; it does not initialize Bevy rendering, input, UI, assets or game simulation. This is an independent network worker, not a replacement server. `shared::ProtocolPlugin` owns all wire types, directions, channels and replicated component registration. The dependency pins `shared` at the existing `e25c79d` rev; the workspace path patch uses the same local shared-protocol source as the Bevy client. Keep that patch aligned with the authoritative server deployment.

```rust
use game_engine_network::{NetworkBridge, Event};
use shared::protocol::{AuthChannel, LoginRequest, LoginResponse};

let mut bridge = NetworkBridge::connect(server_addr, client_id)?;
bridge.send::<LoginRequest, AuthChannel>(LoginRequest {
    token: None,
    username,
    password,
})?;
for event in bridge.drain_events()? {
    match event {
        Event::Message(message) if message.is::<LoginResponse>() => {
            let reply = message.downcast::<LoginResponse>().ok().unwrap();
            // Apply reply.success and reply.characters in Godot; never infer auth success.
        }
        Event::UnitUpdated(snapshot) => { /* upsert by snapshot.server_id */ }
        Event::UnitRemoved(server_id) => { /* remove by server_id */ }
        Event::Connected | Event::Disconnected(_) | Event::Message(_) => {}
    }
}
bridge.disconnect()?; // request transport disconnect, then continue polling
bridge.stop()?;       // join before reconnect; drops old connection's pending events
```

`NetworkBridge::connect` subscribes to LoginResponse, RegisterResponse, ForcedDisconnect, CharacterListUpdate, CreateCharacterResponse, DeleteCharacterResponse, EnterWorldResponse, terrain/transfer, quest log, instance messages, and `MirrorTimerStart`/`Pause`/`Stop` (`receive_mirror_timers`: one relay sorted by `MirrorTimerChannel` message id, because separate per-type receivers hand one frame's messages over in system order). Sending supports any registered `shared` message and channel via `send::<M, C>`. To receive other registered server messages, build `BridgeConfig::new().receive::<M>()...connect(server_addr, client_id)`; register account replies explicitly if using a custom configuration. The receiver is type-erased at the host boundary but keeps each owned authoritative Rust payload; `ProtocolMessage::is/downcast` restores its original type. The caller owns credential/token storage, UI state transitions and reconnect policy. `Disconnected` does not mean reauthentication succeeded; `ForcedDisconnect` must be interpreted by the host before reconnect.

`UnitUpdated` is a **full sampled** subset of the current replicated player/NPC components (identity, position, rotation, health, mana, model, level, equipment); absent components use `None`. `UnitRemoved` means that server identity despawned or lost its Player/Npc tag. `server_id` contains the generation-aware server entity bits for one connection, never the worker's ECS entity or a Godot node. Drop old Godot mirrors on stop/reconnect. Other replicated component types and game objects are registered on the worker but not projected here. No movement prediction, gameplay requests/state application, world streaming, chat, combat, UI, or visual parity is implemented by this package. They require host subscriptions and application logic. Tests cover UDP handshake/restart, worker command FIFO, absence of synthesized auth replies and owned snapshot/message data; a real server auth/replication end-to-end test remains pending integration.
