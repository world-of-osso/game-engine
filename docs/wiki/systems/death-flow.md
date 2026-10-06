# Player death flow

The native client receives owner-only `DeathStateUpdate` and uses existing StaticPopup rendering/input for Retail death dialogs. [Contract](../../specs/death-flow.md); both skins share the same mechanics.

## State and actions

`NetworkBridge::connect` subscribes to the single death reply type. Account dispatch emits `AccountEvent::Death`. `DeathFlow` keeps the snapshot across error-only replies, presents DEATH/RECOVER_CORPSE/XP_LOSS, and sends at most one request until the next authoritative reply. Only errors go to UIErrorsFrame; routine server status messages aren't treated as errors.

Server range flags are not movement updates: `death_support.rs:40-58` computes them only while building a reply. Client proximity uses the latest replicated local server position, matching map and the shared 30-yard corpse/20-yard healer ranges. Corpse recovery appears automatically in range. Only an explicit right-click/InteractUnit on a spirit healer opens XP_LOSS; that confirmation takes precedence over corpse recovery. Leaving range or cancelling closes healer confirmation; interacting again reopens it. Alive clears death popups.

The owner's corpse is drawn with local-listfile FDID 136445 (`Interface/Minimap/Rotating-MinimapCorpseArrow.blp`); distant corpses clamp to the round Modern or square Forever map edge. A different map or alive state removes it. Existing minimap texture loading extracts the arrow from local CASC when needed.

Local ghost visuals apply per-mesh transparency, not shared material changes. Remote ghost visuals cannot be inferred from zero health; the protocol does not replicate a ghost component. Rendered visual equivalence has not been proven.

## Retail sources

Sources under `~/.cache/wow-ui-sim/blizzard-ui/retail/AddOns/`:

- `Blizzard_StaticPopup_Game/Mainline/GameDialogDefs.lua:183-241`: DEATH; `RepopMe` on button 1, no-release-timer text when no timer exists.
- `Blizzard_StaticPopup_Game/GameDialogDefs.lua:2031-2050`: RECOVER_CORPSE; ACCEPT invokes `RetrieveCorpse`.
- `Blizzard_StaticPopup_Game/Mainline/GameDialogDefs.lua:1050-1078`: XP_LOSS; spirit-healer confirmation.
- `Blizzard_StaticPopup_Game/GameDialogDefs.lua:1138-1172`: RESURRECT; ACCEPT/DECLINE/timeout and caster-name text.
- `Blizzard_StaticPopup/StaticPopup.lua`: shared popup framework; actual Retail game definitions are in the files above, not this framework file.
- `data/GlobalStrings.csv:430,4876,1912,169,171`: DEATH_RELEASE, DEATH_RELEASE_NOTIMER, RECOVER_CORPSE, ACCEPT, CANCEL.
- `data/GlobalStrings.csv:1910,4231`: healer text templates. Client substitutes realm's 25% equipped-durability loss and ten-minute sickness, not Retail's 50%/inventory-wide loss; authoritative server behavior remains unchanged. The server computes resurrection sickness in shared `accept_spirit_healer` but `death.rs:729-758` does not attach the resulting sickness; Retail's sickness warning is not proof of an implemented server debuff.

## Blocked protocol/server capabilities

Pinned shared protocol `f1d0452`, `src/protocol/gameplay_messages.rs:565-588`, supports QueryDeathStatus, ReleaseSpirit, ResurrectAtCorpse, AcceptSpiritHealerResurrection, UseStuckEscape and DeathStateUpdate only. `src/protocol_snapshots.rs:403-425` has state, corpse/graveyard positions and range booleans, but no resurrection offer/caster/offer expiry/accept/decline. Server DeathPlugin installs no player-offer handler. The pure shared `cast_resurrect` helper is not a message. A truthful RESURRECT popup/request test is therefore blocked without changing the protocol/server.

Tap-denied is also blocked. Server `crates/server/src/creature_tap.rs:27-33,76-104` stores character-ID tappers, subgroup sharing and damage requirements locally. Shared `UnitFlags` (`src/components/unit_frames.rs:209-249`) carries selection/attackability/pet combat flags, not viewer-relative tap-denied. The protocol registration and client codec contain no tap owner/list or equivalent viewer eligibility component. Health, threat membership and faction reaction cannot establish tap eligibility. No guessed grey rendering is added.

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

Full run: `/tmp/claude/deathstate-0c7e60da.out` (overall exit 101 due to marker fixture). Pending rerun: `/tmp/claude/deathstate-marker.{stdout,stderr}`, PID 2494650. Changed-file Cargo formatting checks pass. Rust readability manually audited; analyzer unavailable. These are CPU registry/UDP proofs, not native GPU/visual equivalence or live realm acceptance. Resurrection-offer and tap-denied tests cannot be instantiated with the pinned wire schema.

## Sources

- [Client subscription](../../../godot/network/src/lib.rs)
- [Account dispatch](../../../godot/rust/src/account.rs)
- [Death model](../../../godot/ui-model/src/death_flow.rs)
- [Native projection](../../../godot/rust/src/death_flow.rs)
- Shared protocol and read-only server sources cited above.

## See Also

- [[networking]] — transport and replica boundaries.
- [[minimap]] — corpse marker presentation.
