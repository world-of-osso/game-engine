# Player death flow

The native client receives owner-only `DeathStateUpdate` and uses existing StaticPopup rendering/input for Retail death dialogs. [Contract](../../specs/death-flow.md); both skins share the same mechanics.

## State and actions

`NetworkBridge::connect` subscribes to the single death reply type. Account dispatch emits `AccountEvent::Death`. `DeathFlow` keeps the snapshot across error-only replies, presents DEATH/RECOVER_CORPSE/XP_LOSS, and sends at most one request until the next authoritative reply. Only errors go to UIErrorsFrame; routine server status messages aren't treated as errors.

Server range flags are not movement updates: `death_support.rs:40-58` computes them only while building a reply. Client proximity uses the latest replicated local server position, matching map and the shared 30-yard corpse/20-yard healer ranges. Corpse recovery takes precedence over the healer when their ranges overlap. Cancelling the healer suppresses it until leaving healer range; dead/corpse popups remain available. Alive clears death popups.

Local ghost visuals apply per-mesh transparency, not shared material changes. Remote ghost visuals cannot be inferred from zero health; the protocol does not replicate a ghost component. Rendered visual equivalence has not been proven.

## Retail sources

Sources under `~/.cache/wow-ui-sim/blizzard-ui/retail/AddOns/`:

- `Blizzard_StaticPopup_Game/Mainline/GameDialogDefs.lua:183-241`: DEATH; `RepopMe` on button 1, no-release-timer text when no timer exists.
- `Blizzard_StaticPopup_Game/GameDialogDefs.lua:2031-2050`: RECOVER_CORPSE; ACCEPT invokes `RetrieveCorpse`.
- `Blizzard_StaticPopup_Game/Mainline/GameDialogDefs.lua:1050-1078`: XP_LOSS; spirit-healer confirmation.
- `Blizzard_StaticPopup_Game/GameDialogDefs.lua:1138-1172`: RESURRECT; ACCEPT/DECLINE/timeout and caster-name text.
- `Blizzard_StaticPopup/StaticPopup.lua`: shared popup framework; actual Retail game definitions are in the files above, not this framework file.
- `data/GlobalStrings.csv:430,4876,1912,169,171`: DEATH_RELEASE, DEATH_RELEASE_NOTIMER, RECOVER_CORPSE, ACCEPT, CANCEL.
- `data/GlobalStrings.csv:1910,4231`: healer text templates. Client substitutes realm's 25% equipped-durability loss and ten-minute sickness, not Retail's 50%; authoritative server behavior remains unchanged.

## Blocked protocol/server capabilities

Pinned shared protocol `f1d0452`, `src/protocol/gameplay_messages.rs:565-588`, supports QueryDeathStatus, ReleaseSpirit, ResurrectAtCorpse, AcceptSpiritHealerResurrection, UseStuckEscape and DeathStateUpdate only. `src/protocol_snapshots.rs:403-425` has state, corpse/graveyard positions and range booleans, but no resurrection offer/caster/offer expiry/accept/decline. Server DeathPlugin installs no player-offer handler. The pure shared `cast_resurrect` helper is not a message. A truthful RESURRECT popup/request test is therefore blocked without changing the protocol/server.

Tap-denied is also blocked. Server `crates/server/src/creature_tap.rs:27-33,76-104` stores character-ID tappers, subgroup sharing and damage requirements locally. Shared `UnitFlags` (`src/components/unit_frames.rs:209-249`) carries selection/attackability/pet combat flags, not viewer-relative tap-denied. The protocol registration and client codec contain no tap owner/list or equivalent viewer eligibility component. Health, threat membership and faction reaction cannot establish tap eligibility. No guessed grey rendering is added.

## Sources

- [Client subscription](../../../godot/network/src/lib.rs)
- [Account dispatch](../../../godot/rust/src/account.rs)
- [Death model](../../../godot/ui-model/src/death_flow.rs)
- [Native projection](../../../godot/rust/src/death_flow.rs)
- Shared protocol and read-only server sources cited above.

## See Also

- [[networking]] — transport and replica boundaries.
- [[minimap]] — corpse marker presentation.
