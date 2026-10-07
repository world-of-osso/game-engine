# Player death flow

The native client receives owner-only `DeathStateUpdate` and uses existing StaticPopup rendering/input for Retail death dialogs. [Contract](../../specs/death-flow.md); both skins share the same mechanics.

## State and actions

`NetworkBridge::connect` subscribes to the single death reply type. Account dispatch emits `AccountEvent::Death`. `DeathFlow` keeps the snapshot across error-only replies, presents DEATH/RECOVER_CORPSE/XP_LOSS, and sends at most one request until the next authoritative reply. Only errors go to UIErrorsFrame; routine server status messages aren't treated as errors.

Server range flags are not movement updates: `death_support.rs:40-58` computes them only while building a reply. Client proximity uses the latest replicated local server position, matching map and the shared 30-yard corpse/20-yard healer ranges. Corpse recovery appears automatically in range. Only an explicit right-click/InteractUnit on a spirit healer opens XP_LOSS; that confirmation takes precedence over corpse recovery. Leaving range or cancelling closes healer confirmation; interacting again reopens it. Alive clears death popups.

The owner's corpse is drawn with local-listfile FDID 136445 (`Interface/Minimap/Rotating-MinimapCorpseArrow.blp`); distant corpses clamp to the round Modern or square Forever map edge. A different map or alive state removes it. Existing minimap texture loading extracts the arrow from local CASC when needed.

Local ghost visuals apply per-mesh transparency, not shared material changes. WorldLighting's Environment applies saturation 0.2 while ghost and restores 1.0 alive; UI remains coloured. The world map projects the owner corpse from engine axes into UiMap coordinates and draws Retail CorpsePinTemplate's POIIcons crop. Logout stops transport before releasing world and death state, retaining the saved auth token. Native regression: `godot/tests/deathloop_live_flow.gd`; private-run inputs/results belong in the live proof ledger. Remote ghost visuals cannot be inferred from zero health; the protocol does not replicate a ghost component. Exact Retail visual equivalence remains unproven.

## Retail sources

Sources under `~/.cache/wow-ui-sim/blizzard-ui/retail/AddOns/`:

- `Blizzard_StaticPopup_Game/Mainline/GameDialogDefs.lua:183-241`: DEATH; `RepopMe` on button 1, no-release-timer text when no timer exists.
- `Blizzard_StaticPopup_Game/GameDialogDefs.lua:2031-2050`: RECOVER_CORPSE; ACCEPT invokes `RetrieveCorpse`.
- `Blizzard_StaticPopup_Game/Mainline/GameDialogDefs.lua:1050-1078`: XP_LOSS; spirit-healer confirmation.
- `Blizzard_StaticPopup_Game/GameDialogDefs.lua:1138-1172`: RESURRECT; ACCEPT/DECLINE/timeout and caster-name text.
- `Blizzard_StaticPopup/StaticPopup.lua`: shared popup framework; actual Retail game definitions are in the files above, not this framework file.
- `data/GlobalStrings.csv:430,4876,1912,169,171`: DEATH_RELEASE, DEATH_RELEASE_NOTIMER, RECOVER_CORPSE, ACCEPT, CANCEL.
- `data/GlobalStrings.csv:1910,4231`: healer text templates. Client substitutes realm's 25% equipped-durability loss and ten-minute sickness, not Retail's 50%/inventory-wide loss; authoritative server behavior remains unchanged. The server computes resurrection sickness in shared `accept_spirit_healer` but `death.rs:729-758` does not attach the resulting sickness; Retail's sickness warning is not proof of an implemented server debuff.

## Player resurrection offers and tap eligibility

The resurrection/tap work replaces the former `f1d0452` protocol gaps. Default transport receives `ResurrectionOffer`; Account dispatches it into DeathFlow, which replaces release/corpse/healer dialogs with RESURRECT and its Accept/Decline buttons and server-supplied timeout. Accept sends `ResurrectionResponse` through the normal Account transport once; cancellation/timeout sends decline and restores the ordinary death dialog. An alive snapshot clears the offer. Ordinary player resurrection has no spirit-healer sickness, so the text does not falsely promise sickness (GlobalStrings.csv:3714 is the sickness variant).

Shared `UnitTap` carries stable character IDs from the server's existing multi-tapper list, never damage/threat guesses. The client compares the selected character ID and current roster's new `character_id` fields, including offline/out-of-interest members. NPCs controlled by another unit are exempt. The list is shared across viewers and sent only when tap membership changes/clears: raw 8N ID bytes, or standard bincode's vector-length varint plus N ID varints, before envelopes. `[42,43]` is 3 payload bytes. Existing roster messages add one ID varint per member; no per-viewer creature flag or per-tick tap message is added.

Native nameplates prioritize CompactUnitFrame's tap-denied health RGB `(0.9,0.9,0.9)` over reaction/selection hostility (`Shared/CompactUnitFrame.lua:675-677`, not the name's 0.5 grey at :868-870). The requested target health greying uses `(0.5,0.5,0.5)`, taken from `Mainline/TargetFrame.lua:307-312`'s faction/portrait tint; that source does not itself grey the target health fill. Modern uses a solid grey fill because multiplying baked green art cannot remove its hue. Forever uses the same 0.5 RGB through its existing Flat-texture brightness multiplier. Both preserve fraction and status text.

Sources: `godot/ui-model/src/death_flow.rs`, `godot/rust/src/{account,nameplates,targeting,replicated}.rs`, `godot/network/src/{lib,replica/codec}.rs`; server `crates/server/src/death/resurrection.rs`, `creature_tap.rs` and shared protocol `tests/rezrtap.rs`. Targeted proof/revisions are recorded in `/tmp/claude/rezrtap-proof.md`; these are CPU/UDP proofs, not native GPU equivalence or live realm acceptance.

## Behavioral proof (2026-10-06)

Production code at `0c7e60da`: locked, local, agent-run targeted `deathstate` run compiled all three packages without warnings. Ten tests passed; the marker fixture failed because it omitted required `ActiveSkin` context, not because of a production renderer failure. Test-only `5ba715aa` supplies that context. Its single-test rerun is queued behind the shared build lock, held by an unrelated publishing upload; the upload is outside this task's authority. No further production changes invalidate the ten passing results.

| Test | Result |
|---|---|
| `deathstate_default_bridge_receives_snapshot_over_udp` | PASS |
| `deathstate_account_dispatches_snapshot_and_error` | PASS |
| `deathstate_udp_snapshot_popup_account_request_both_skins` | PASS: release, corpse and healer requests reach real loopback server through production Account sending |
| `deathstate_dead_popup_release_both_skins` | PASS |
| `deathstate_ghost_corpse_range_recovery_both_skins` | PASS |
| `deathstate_spirit_healer_accept_both_skins` | PASS |
| `deathstate_wrong_map_and_leaving_range_hide_stale_corpse` | PASS |
| `deathstate_refusal_preserves_snapshot_and_unlocks_retry` | PASS |
| `deathstate_healer_cancel_reopen_and_range` | PASS |
| `deathstate_healer_requires_interaction` | PASS |
| `deathstate_corpse_marker_and_edge_arrow_both_skins` | PENDING corrected fixture rerun |

Full run: `/tmp/claude/deathstate-0c7e60da.out` (overall exit 101 due to marker fixture). Pending rerun: `/tmp/claude/deathstate-marker.{stdout,stderr}`, PID 2494650. Changed-file Cargo formatting checks pass. Rust readability manually audited; analyzer unavailable. These are CPU registry/UDP proofs, not native GPU/visual equivalence or live realm acceptance. That historical run used the pre-offer/pre-tap schema; the resurrection/tap work above supersedes those wire gaps.

## Sources

- [Client subscription](../../../godot/network/src/lib.rs)
- [Account dispatch](../../../godot/rust/src/account.rs)
- [Death model](../../../godot/ui-model/src/death_flow.rs)
- [Native projection](../../../godot/rust/src/death_flow.rs)
- Shared protocol and read-only server sources cited above.

## See Also

- [[networking]] — transport and replica boundaries.
- [[minimap]] — corpse marker presentation.
