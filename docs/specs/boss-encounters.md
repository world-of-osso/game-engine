# Boss encounters (native client)

Native Godot encounter presentation. Server contract: game-server `docs/specs/boss-scripts.md`; wire contract: shared-protocol `protocol/encounter_messages.rs` and `ChatType::RaidBossEmote`. [Implementation and Retail references](../wiki/systems/boss-encounters.md).

## What it must do
- [x] Creature chat keeps Retail wording and colours; `%s` emotes substitute the speaker. Existing native chat tests cover this independently of encounter HUD.
- [x] Receive Start, Engage, Disengage and End in their reliable EncounterChannel order, including multiple lifecycle cycles received together.
- [x] Engaged units fill at most five frames in ascending priority, equal priorities retain engagement order, duplicate engages are ignored. Late replication fills pending slots; health, power, name, level and classification update from live replication.
- [x] Disengage removes its unit; End (kill or wipe), a new Start, loading, disconnect and world reset clear stale encounter state.
- [x] Left-click on a visible Boss1TargetFrame..Boss5TargetFrame targets that boss through normal SetTarget, never starts auto-attack.
- [x] RaidBossEmote appears in chat and center-screen RaidWarningFrame (800 wide, TOP 182), with 0.2-second fade-in, 10-second hold and 3-second fade-out. Four slots; a fifth evicts the oldest. End/start/world reset clear encounter warnings.

## How it works
- [Native encounter state, HUD and references](../wiki/systems/boss-encounters.md).
- [Unit frame rendering](../wiki/systems/ui-system.md).

## Implementation inventory
- `godot/network/src/lib.rs` — ordered four-type encounter relay.
- `godot/rust/src/account.rs`, `encounter.rs` — protocol dispatch, lifecycle and warning host.
- `godot/rust/src/targeting.rs`, `unit_portraits.rs` — replicated boss frames, click binding and native portrait lifetime.
- `godot/rust/src/chat.rs`, `godot/ui-model/src/raid_warning.rs` — boss-emote routing, timed warning model and center frame.
- `godot/ui-model/src/ui/screens/inworld_unit_frames_component.rs` — five named frame roots and classification art.

## Tests asserting this spec
- `godot/network/src/wire_tests.rs::bossframes_bridge_preserves_encounter_lifecycle_channel_order` — actual UDP, held worker, two concrete lifecycle cycles.
- `godot/rust/src/account.rs::bossframes_account_dispatches_each_encounter_message`.
- `godot/rust/src/encounter_tests.rs` — priority, duplicate, delayed replication, live health/mana, five-slot cap, click identity, disengage, wipe/death/reset.
- `godot/ui-model/tests/bossframes_warnings.rs` — substitution, fade timing, eviction/clear, displayed text/colour/alpha, non-intercepting center frame.
- `godot/ui-model/tests/unit_frame_atlas.rs` — actual classification atlas and shared frame art.

## Known gaps (current cycle)
- [ ] Exact Retail small portraitless boss art and right-managed/edit-mode sizing remain unconverted. Requested classification portraits use target portrait art; the fixed right-side slot can overlap objective tracker (observed in native Hogger captures).
- [ ] Native live proof covers one boss, physical targeting/server echo, enrage and kill/reset. Five simultaneous native bosses, every power/classification and mounted loading/disconnect reset permutations remain unverified; pure projection/lifecycle tests cover five slots and explicit reset.

## Out of scope
- Server/protocol changes. The pinned ChatType has RaidBossEmote but no player RaidWarning message; the warning view supports both colour kinds, but cannot receive nonexistent raid-warning traffic.
- Boss cast/alternate-power bars, encounter journal/timers, RAID_BOSS_WHISPER, creature sounds and chat bubbles.
- Historical Bevy Stockade proof (2026-09-27) is not native Godot proof.

## Native proof — 2026-10-07

Production `cf829040`, test follow-up `cd04108e`: 11 targeted cases PASS across native, network and UI-model; three-crate cargo fmt check and locked extension/CLI build pass. No broad suite or clean-runtime claim.

Private copied server `8b3819b`, UDP5306, Stockade Hogger1144; only fb_bossframes/Fbbossframes. Two launches. Inspected canonical `data/diagnostics/bossframes-20261007/attempt2/`: `01-engaged.png`, `02-click-target.png` plus JSON (target=sent=server_target4294857553, auto_attack null), `03-emote-manual.webp` plus UI dump (yellow lowercase "Hogger enrages!", health fill reduced), `04-killed-cleared.webp` plus UI dump (all five roots hidden; End success=true). Earlier End success=false reset retained. Script's uppercase emote predicate missed actual BroadcastText; manual captures, not its completion banner, establish emote proof.

Proof ledger and exact argv remain in canonical diagnostics. Owned PIDs exited, agents-bossframes.slice inactive, UDP5306 free; protected UDP5000 unchanged. Source/test proof does not establish exact Retail visual/layout parity.
