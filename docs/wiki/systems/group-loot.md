# Group loot

Retail Mainline prompts, server-owned rolls and loot history share one native path in Modern and Forever. The [client contract](../../specs/group-loot.md) owns requirements; [server contract](../../../../game-server/docs/specs/group-loot.md) owns entitlement, eligibility, RNG and inventory delivery. Master loot and legacy distribution modes are not offered.

## Integration

`LootChannel` carries `GroupLootRoll` snapshots, `GroupLootChoice` and `GroupLootHistoryRequest`. The native transport preserves channel order; `account.rs` decodes snapshots and `loot.rs` applies them to `GroupLootState`. Clients submit identity and choice, never a roll number or inventory grant.

Four 277×67 prompts queue additional rolls. Server confirmation removes answered prompts; the local timer disables expired submissions. Need uses the server's eligibility/reason. Uncollected eligible appearances replace Greed with Transmog. Within Transmog, appropriate classes outrank other collectors before comparing RNG (Blizzard Warbands Preview, article24061008). Preference remains in the server's roll window; no new network field or UI element. `/loot` or `/lootrolls` opens the shared history panel; world entry requests retained server history. Chat announces selection changes, authoritative numeric results on completion, and the winner exactly once; replay does not repeat announcements.

The self member's party/raid context menu exposes Group Loot, Personal Loot and quality thresholds. Frame keys are `UnitFrameContextMenuLootGroup`, `LootPersonal` and `LootThresholdUncommon/Rare/Epic`, not enum variant names. Server authority rejects nonleader changes and unsupported methods.

## Bounded proof — verified 2026-10-11

- `modern8`: two `fb_group_loot2_*` accounts, fresh Glneed/Glgreed characters, private UDP54300. Need90 beats Greed53; winner's Monogrammed Sash count0→1, other count0→0. Both clients show numeric chat, winner and completed history. An additional authentic Rageclaw Boots roll times out/all-passes.
- `forever9`: same accounts, observed Forever skin, party→raid conversion and leader method menu. Greed46 loses to Need43; Glgreed's item count0→1, Glneed unchanged. The existing unique item correctly disables Glneed's Need button. Both clients show numeric results, winner and history.
- Deterministic server test uses authentic Rageclaw Boots15379/appearance2478: collected appearance cannot choose Transmog; uncollected appearance can. Transmog1 beats Greed99, delivers once, and completed history replays identically to a replacement authenticated connection. An additional authentic leather case reproduces and fixes missing class preference: DruidTransmog1 beats WarriorTransmog99 (RED→GREEN). Current protocol6/6 and server group-loot10/10 pass; protocol/server cargo checks, all three format checks and native consumer build pass on the c6e063c resolver. The wire and Need/Greed UI paths are unchanged, so earlier live evidence remains scoped to those paths.
- UI numeric-chat regression RED→GREEN; real loopback UDP snapshot/choice test passes. These are scoped proofs, not whole-project CI or complete Mainline Need-policy certification. Independent verifier authentication was unavailable.

Durable logs, trees, baselines and scripts: `data/diagnostics/group-loot-2026-10-11/`. Eleven inspected PNGs (maximum1280px): `/syncthing/AgentShared/2026-10-11/group-loot/`, prefixes `modern8` and `forever9`. Earlier attempts are diagnostic evidence, not successful award proofs; compare inventory baselines, not final bag contents alone.

## Fixture boundaries

Use private endpoint token files and wait for complete authentication before launching the next account: the token file is realm-wide, while HUD ownership mapping is account-local. Do not touch the5000 token. Start invite automation after initial death UI is cleared; `StaticPopup1Button1` alone does not identify the invite dialog. Arrange both players on the NPC's authentic coordinates before death; below-ground setup can exclude a member from rewards. A missing Mok'rash125830 model receipt did not block authoritative loot or UI proof; no asset was fabricated.

## Sources

- [Group-roll model](../../../godot/ui-model/src/group_loot.rs), [native UI](../../../godot/ui-model/src/group_loot_component.rs), [host](../../../godot/rust/src/loot.rs), [wire test](../../../godot/network/src/group_loot_wire_tests.rs).
- [Server roll lifecycle](../../../../game-server/crates/server/src/loot/group.rs), [behavioral tests](../../../../game-server/crates/server/src/loot/group_tests.rs).
- Local Retail `GroupLootFrame.lua/.xml`, `LootHistory.lua/.xml`, `UnitPopupButtons.lua`, `LootConstantsDocumentation.lua`, `GlobalStrings.csv` (`LOOT_ROLL_ROLLED_NEED/GREED`); TrinityCore a352b1fa `Loot.cpp`; Blizzard Entertainment's April17,2024 Warbands Preview (article24061008), verified October11,2026.

## See Also

- [Loot Frame](../../specs/loot-frame.md) — corpse interaction and inventory/currency authorities.
- [[godot-conversion]] — native account/transport/UI host.
