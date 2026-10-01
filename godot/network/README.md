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
        Event::ReplicationStarted(schema) => replica = Replica::new(schema),
        Event::Replication(batch) => {
            replica.apply(batch)?;
            for change in replica.drain_changes() { /* read replica.unit(id)?.get::<Position>() */ }
        }
        Event::Connected | Event::ProtocolRejected(_) | Event::Disconnected(_) | Event::Message(_) => {}
    }
}
bridge.disconnect()?; // request transport disconnect, then continue polling
bridge.stop()?;       // join before reconnect; drops old connection's pending events
```

`NetworkBridge::connect` subscribes to LoginResponse, RegisterResponse, ForcedDisconnect, CharacterListUpdate, CreateCharacterResponse, DeleteCharacterResponse, EnterWorldResponse, terrain/transfer, quest log, instance messages, and `MirrorTimerStart`/`Pause`/`Stop` (`receive_mirror_timers`: one relay sorted by `MirrorTimerChannel` message id, because separate per-type receivers hand one frame's messages over in system order). Sending supports any registered `shared` message and channel via `send::<M, C>`. To receive other registered server messages, build `BridgeConfig::new().receive::<M>()...connect(server_addr, client_id)`; register account replies explicitly if using a custom configuration. The receiver is type-erased at the host boundary but keeps each owned authoritative Rust payload; `ProtocolMessage::is/downcast` restores its original type. The caller owns credential/token storage, UI state transitions and reconnect policy. `Disconnected` does not mean reauthentication succeeded; `ForcedDisconnect` must be interpreted by the host before reconnect.

Replication has no ECS copy. The worker disables lightyear's replicon client backend (`LightyearRepliconClientBackend`) and lightyear prediction, reads replicon's update and mutation channels from the Lightyear `Transport`, strips Lightyear's 7-byte checkpoint header, acknowledges every mutate message in the frame it arrives (replicon's `MutationAcks`), and forwards the raw payloads as `Event::Replication`. The host owns `replica::Replica`, which applies them like `bevy_replicon::client::receive_replication` (mappings, despawns, removals, changes, mutations buffered until their update tick, stale per-entity ticks skipped) into per-type columns keyed by the server `Entity::to_bits()`. `Event::ReplicationStarted` carries the `Schema`: the worker resolves every replicon `FnsId` from its `ReplicationRules` to a codec in `replica/codec.rs`; a registered type without a codec fails the connection with its name. `drain_changes` reports each changed entity with its changed components, and each despawn (the server's despawn or lost visibility). Clear the replica on `Disconnected`. The protocol fingerprint equals the stock lightyear client's (`replacing_replicon_client_keeps_the_protocol_fingerprint`, `full_wire_fingerprint_matches_stock_client_including_layout_hashes`). No movement prediction, gameplay requests/state application, world streaming, chat, combat, UI, or visual parity is implemented by this package. See [Godot replication](../../docs/wiki/systems/godot-replication.md).
