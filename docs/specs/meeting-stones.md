# Meeting stones (client)

Native Godot port: `godot/rust/src/meeting_stones.rs`, `godot/ui-model/src/summon.rs`, and the default network bridge. Current native tests/live evidence are recorded below; historical Bevy evidence is not native acceptance.

Using a meeting stone and answering a summon. Contract: shared-protocol `protocol/interaction_messages.rs` (`UseGameObject`, `SummonRequest`, `SummonResponse`). Server rules: game-server `docs/specs/meeting-stones.md`.

Retail references (cached `~/.cache/wow-ui-sim/blizzard-ui/retail/AddOns/`): `Blizzard_StaticPopup_Game/GameDialogDefs.lua:2402-2438` (`GetConfirmSummonExpiryText`, `CONFIRM_SUMMON`); `Blizzard_StaticPopup/StaticPopup.lua` (`StaticPopup_OnUpdate`); `Blizzard_StaticPopup_Game/GameDialog.lua:88-90` (summon events).

## What it must do

### Using the stone
- [x] Meeting stones and the Meeting Stone Summoning Portal are replicated game objects; right-clicking one sends `UseGameObject` and keeps the current target (the party member to summon).
- [x] Refusals show in the error frame with Retail text: `SPELL_FAILED_BAD_TARGETS` "Invalid target", `SPELL_FAILED_LEVEL_REQUIREMENT` "You are not high enough level", `SPELL_FAILED_LOWLEVEL` "Target is too low level", `SPELL_FAILED_SUMMON_PENDING` "A summon is already pending".

### CONFIRM_SUMMON
- [x] `SummonRequest` shows StaticPopup `CONFIRM_SUMMON` "%s wants to summon you to %s. The spell will be canceled in %d %s." with Accept / Cancel. A new request replaces the open one. Resolve `zone_id` through the minimap's existing AreaTable catalog and retain that name through countdown updates. While the catalog loads, defer display without pausing the offer timer or treating loading as an unknown area.
- [x] The countdown follows `StaticPopup_OnUpdate`: `ceil` of the time left, shown in seconds below 60 s, otherwise in minutes rounded up ("2 Minutes", "1 Minute", "59 Seconds", "1 Second").
- [x] Accept is disabled while the player is in combat and cannot be accepted by click or Enter.
- [x] Accept sends `SummonResponse { accept: true }`; Cancel and the countdown running out send `accept: false`.
- [x] IPC: `quest interact --npc <name>` right-clicks the nearest NPC or game object of that name, else targets the player of that name.

## Gaps
- Native Retail's return string for an unknown area is not documented in the cached API/FrameXML. Missing AreaTable rows currently use an empty string, not an invented "Unknown" label; exact Retail unknown-area parity is unverified.
- Only replicated players (within the 100 yd interest radius) can be targeted, so a party member farther away cannot be picked for a summon.
- No 0.5 s decline lock (`SetupLockOnDeclineButtonAndEscape`), no `PlayerCanTeleport` check, no `CANCEL_SUMMON` from the server.
- Cross-map summons are accepted through a map transfer ([instances](instances.md)); starting one needs the target in the interest radius (above).

## Summon-zone regression (2026-10-07)

Server city-rest terrain lookup resolves Trade District 5148 to Stormwind City 1519; the protocol is unchanged. Client tests send 1519 over real UDP, resolve it with the native minimap catalog, and assert the exact StaticPopup text/registry in Modern and Forever, including countdown retention. Zero/absent rows exercise the empty substitution. Cached `GameDialogDefs.lua:2402-2409` inserts `GetSummonConfirmAreaName()` directly with no UI placeholder; `SummonInfoDocumentation.lua:19-24` guarantees a non-nil string but does not specify the unknown-area value.

## Native verification (2026-10-07)

Rust implementation `b64a2f33`, fractional-frame correction `a3ec8ada`; live driver `c189dd69`. Evidence: `data/diagnostics/meetingstones-2026-10-07/` (`proof-ledger.txt`, input hashes, argv/PID records, retained RED/GREEN logs and `cleanup.txt`). Server binary `8b3819b`, copied offline pre-8b3819b player backup, owned read-only SQLite world snapshot; protected :5000 unchanged.

| Behavior | Current proof |
| --- | --- |
| Request/countdown/replacement/combat/answers/expiry | Ten targeted tests PASS: four native-host cases, one network case, five UI-model cases. Real UDP → Account → native StaticPopup registry → exact Accept/Cancel/expiry wire replies in Modern and Forever; stale replacement clicks rejected; fractional frame boundaries reproduced RED then fixed. |
| Build/format | Local helper `--cli` build PASS; three changed packages' `cargo fmt --check` PASS. Tests/build cover current Rust code; later commits only change the live driver/docs. |
| Stone use/target/refusals | Two level-60 characters, `fb_stone1`/`fb_stone2`, in a party at Stockade stone 205553. Physical right-click retained target 4294857633; server logged spell 59782 opening ritual 179944. A targetless click delivered "Invalid target"; all four refusal strings have UDP/host tests. |
| Summon popup/IPC/arrival | Helper portal use through native `quest interact` opened the actual popup. Inspected `client2/02-summon-popup.png` (2 Minutes), `03-before-accept-away.png` (54 Seconds), `04-arrival.png`: physical Accept returned the recipient 11.986 yd, from WoW (-8793.041, 796, 99.58086) to (-8805, 796, 98.77057); server logged accepted summon to map 0. `confirm-summon-ipc.txt` exposes StaticPopup1Text/Button1/Button2. Physical Cancel closed a second offer without moving; captures 05/06 inspected. |
| Boundaries/cleanup | Four rendered client launches, maximum two concurrently; two parser-only Godot checks (six total invocations). Exact owned PIDs terminated, own slice inactive/dead, UDP 5310 free. Forced termination is not renderer-resource shutdown proof. |

The center-only portal mouse probe did not find a pickable point with both characters at its position; the ritual completed through native IPC instead. Portal mouse-picking, live combat/replacement/expiry (covered by behavioral tests), cross-map arrival, and full Retail visual parity are not claimed. Existing NPC spell-attachment errors were observed and left out of scope.

## Historical Bevy live evidence (2026-09-26)
Own server :5080, three headless clients (Stonecaller, Ritebearer, Wayfarer, level 30) at Blackrock Mountain stone 179584: Stonecaller targeted Wayfarer (88 yd away) and used the stone, the portal opened after 5 s, Ritebearer clicked it, Wayfarer's `CONFIRM_SUMMON` read "Stonecaller wants to summon you to Unknown. The spell will be canceled in 2 Minutes.", Accept moved Wayfarer from (-7622, -1222, 232) to Stonecaller at (-7588.7, -1139.8, 260.8). Screenshots in `data/diagnostics/summonstone/`.

## Tests asserting this spec
Use `scripts/depot-build.py --build-host local --test -p game-engine-godot -p game-engine-ui-model -p game-engine-network --lib --no-fail-fast meetingstones` through build-lock + `agent-run meetingstones`. Never use retired root-bin selectors.
