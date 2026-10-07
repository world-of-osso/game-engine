# Native meeting stones

The Godot client uses replicated stone and ritual models and the existing StaticPopup registry. [Client contract and proof](../../specs/meeting-stones.md) own requirements, verification status and retained gaps.

## Host path

`game_objects.rs` renders/picks `GAMEOBJECT_TYPE_MEETINGSTONE` and `GAMEOBJECT_TYPE_RITUAL` with the Interact cursor. `merchant.rs` tries game-object use before changing the hard target. `meeting_stones.rs` sends `UseGameObject`; the server validates model-box range and ritual rules.

The default network bridge receives `SummonRequest`; Account emits `AccountEvent::Summon`. `SummonPopup` stores the popup ID, summoner and remaining duration. Replacement gets a distinct ID so queued actions cannot answer the new request. Per-frame updates advance offer time and combat gating before popup clicks are resolved. Keyboard acceptance refreshes the combat gate too. Matching popup results send `SummonResponse` once; world reset clears the offer. Rendering, registry names and automation reuse the common `StaticPopup1..3` host.

## Sources

- [Meeting-stone contract](../../specs/meeting-stones.md) — Retail citations, behavior, tests and live proof.
- `godot/rust/src/{game_objects,meeting_stones,merchant,party_frames,account}.rs` — native interaction and dispatch.
- `godot/ui-model/src/summon.rs` — deterministic offer lifecycle.
- `godot/tests/meetingstones_live.gd` — private two-client mouse/registry capture driver.

## See Also

- [[godot-conversion]] — native client conversion.
- [[portrait-party-frames]] — party roster and shared popup host.
