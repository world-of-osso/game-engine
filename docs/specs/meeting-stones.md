# Meeting stones (client)

Native Godot port: `godot/rust/src/meeting_stones.rs`, `godot/ui-model/src/summon.rs`, and the default network bridge. Current native tests/live evidence are recorded below; historical Bevy evidence is not native acceptance.

Using a meeting stone and answering a summon. Contract: shared-protocol `protocol/interaction_messages.rs` (`UseGameObject`, `SummonRequest`, `SummonResponse`). Server rules: game-server `docs/specs/meeting-stones.md`.

Retail references (cached `~/.cache/wow-ui-sim/blizzard-ui/retail/AddOns/`): `Blizzard_StaticPopup_Game/GameDialogDefs.lua:2402-2438` (`GetConfirmSummonExpiryText`, `CONFIRM_SUMMON`); `Blizzard_StaticPopup/StaticPopup.lua` (`StaticPopup_OnUpdate`); `Blizzard_StaticPopup_Game/GameDialog.lua:88-90` (summon events).

## What it must do

### Using the stone
- [x] Meeting stones and the Meeting Stone Summoning Portal are replicated game objects; right-clicking one sends `UseGameObject` and keeps the current target (the party member to summon).
- [x] Refusals show in the error frame with Retail text: `SPELL_FAILED_BAD_TARGETS` "Invalid target", `SPELL_FAILED_LEVEL_REQUIREMENT` "You are not high enough level", `SPELL_FAILED_LOWLEVEL` "Target is too low level", `SPELL_FAILED_SUMMON_PENDING` "A summon is already pending".

### CONFIRM_SUMMON
- [x] `SummonRequest` shows StaticPopup `CONFIRM_SUMMON` "%s wants to summon you to %s. The spell will be canceled in %d %s." with Accept / Cancel. A new request replaces the open one.
- [x] The countdown follows `StaticPopup_OnUpdate`: `ceil` of the time left, shown in seconds below 60 s, otherwise in minutes rounded up ("2 Minutes", "1 Minute", "59 Seconds", "1 Second").
- [x] Accept is disabled while the player is in combat and cannot be accepted by click or Enter.
- [x] Accept sends `SummonResponse { accept: true }`; Cancel and the countdown running out send `accept: false`.
- [x] IPC: `quest interact --npc <name>` right-clicks the nearest NPC or game object of that name, else targets the player of that name.

## Gaps
- The area name shows "Unknown": the server tracks no player zone, so `SummonRequest.zone_id` is 0.
- Only replicated players (within the 100 yd interest radius) can be targeted, so a party member farther away cannot be picked for a summon.
- No 0.5 s decline lock (`SetupLockOnDeclineButtonAndEscape`), no `PlayerCanTeleport` check, no `CANCEL_SUMMON` from the server.
- Cross-map summons are accepted through a map transfer ([instances](instances.md)); starting one needs the target in the interest radius (above).

## Native verification (2026-10-07)

Targeted `meetingstones` selectors cover request/countdown/replacement, combat click/Enter gate, Accept/Cancel/expiry responses, cursor selection and Retail refusal strings. Native transport/registry integration and private-server proof pending; no current live acceptance claim.

## Historical Bevy live evidence (2026-09-26)
Own server :5080, three headless clients (Stonecaller, Ritebearer, Wayfarer, level 30) at Blackrock Mountain stone 179584: Stonecaller targeted Wayfarer (88 yd away) and used the stone, the portal opened after 5 s, Ritebearer clicked it, Wayfarer's `CONFIRM_SUMMON` read "Stonecaller wants to summon you to Unknown. The spell will be canceled in 2 Minutes.", Accept moved Wayfarer from (-7622, -1222, 232) to Stonecaller at (-7588.7, -1139.8, 260.8). Screenshots in `data/diagnostics/summonstone/`.

## Tests asserting this spec
Use `scripts/depot-build.py --build-host local --test -p game-engine-godot -p game-engine-ui-model -p game-engine-network --no-fail-fast meetingstones` through build-lock + `agent-run meetingstones`. Never use retired root-bin selectors.
