# Loot Frame

The Retail `LootFrame`, lootable corpses and the unit cursor, running against the live server loot. The contract is shared-protocol `protocol/loot_messages.rs`; server rules are in game-server `docs/specs/loot.md`.

References:
- LF.xml / LF.lua = `Blizzard_UIPanels_Game/Mainline/LootFrame.xml` / `LootFrame.lua`
- SFP = `Blizzard_UIPanels_Game/Mainline/ScrollingFlatPanel.xml` / `.lua`
- SUPT = `Blizzard_SharedXML/Mainline/SharedUIPanelTemplates.xml`
- all under `~/.cache/wow-ui-sim/blizzard-ui/retail/AddOns/`; strings from GlobalStrings (build 12.1)

## What it must do

- [x] A dead NPC plays `Death` (animation 1) once and holds its last frame; movement and emote animation leave it alone.
- [x] `CorpseLootable` marks the corpse `Lootable`: it sparkles (gold glow like the quest sparkle) and shows the `LootAll` cursor. `lootable: false` removes both.
- [x] Unit cursor by role and reaction: lootable corpse `LootAll`; other corpse the pointer; hostile, or neutral without services, `Attack`; flight master `Taxi`; vendor `Buy`; trainer `Trainer`; any other service `Speak`; none the pointer.
- [x] Right-click on a lootable corpse in range sends `LootUnit { auto }` with `auto = autoLootDefault XOR Shift` (`AUTOLOOTTOGGLE` default Shift, Bindings_Standard.xml:1773). Other corpses are only targeted; living NPCs are interacted with. IPC `quest interact` makes the same choice (auto off).
- [x] The Auto Loot option (`autoLootDefault`, off by default) is in the options menu and saved with the HUD options.
- [x] `LootResponse` opens the frame; `LootSlotRemoved` removes the card and prints `LOOT_ITEM_SELF` "You receive loot: %s" (`LOOT_ITEM_SELF_MULTIPLE` "%sx%d") or `YOU_LOOT_MONEY` "You loot %s"; `LootClosed` closes it; `LootFailed` shows its Retail text (`ERR_LOOT_TOO_FAR`, `ERR_LOOT_DIDNT_KILL`, `ERR_INV_FULL`, ...).
- [x] Frame: 220 wide `ScrollingFlatPanelTemplate` titled `ITEMS` "Items" (LF.xml:113-117): `DefaultPanelFlatTemplate` background tinted `PANEL_BACKGROUND_COLOR` (GlobalColor 191) at 6,20 / -2,2 with the `uiframebackground-nineslice-corner*` bottom corners (SUPT:404-436, 527-536), the `ButtonFrameTemplateNoPortrait` metal border (`metal_frame_no_portrait`), title and close button.
- [x] Placement: Retail `lootUnderMouse` (LF.lua:180-190): TOPLEFT at the cursor x-30, 50 above it and at least 350 above the screen bottom, clamped to the screen.
- [x] Cards 46 tall, 2 apart, inside the 6 px padding of the `ScrollBox` at 4,22 (LF.lua:1-3, SFP): height = cards + 46.
  - Item card: `looting_itemcard_bg` tinted by quality, `looting_itemcard_stroke_normal`, `looting_raritytag_frame` 100×13 at TOPRIGHT with `ITEM_QUALITY%d_DESC` (LF.xml:71-95), the 37×37 item button at 5,4 with UI-Quickslot2 and the count above 1, the name at the button's TOPRIGHT +8,-8 in the quality colour.
  - Money card: the coin icon (`inv_misc_coin_01/03/05` by largest coin), one `GOLD_AMOUNT`/`SILVER_AMOUNT`/`COPPER_AMOUNT` line per coin, 93×38 beside the button (LF.xml:97-111).
- [x] Clicking a card sends `LootSlotRequest` (`LootSlot`, LF.lua:38-57); the close button sends `LootRelease` (`CloseLoot`).
- [x] Looted items and coins reach the bags and money through `InventoryDelta` and `Gold`.
- [ ] No Loot All button (Retail has none).
- [ ] Not built: the scroll bar (more than five cards grow the frame past `panelMaxHeight` 290), card hover/pushed strokes, item tooltips, the show/hide and slide-out animations, quest item overlays, Escape to close, loot sounds, group loot roll frames (`GroupLootFrame`), `LOOT_BIND` confirmation.
- [ ] Right-clicking a hostile living NPC still also sends an interaction the server refuses ("Target is hostile").

## Native Godot conversion

- [x] Receive per-looter `CorpseLootable` and existing loot-window messages through the native transport/account host (bounded owned fixture).
- [x] Right-click the actual corpse with authored Auto Loot and Shift inversion in four cases; empty corpse remains target-only.
- [x] Prove inclusive five-unit corpse reach and friendly living-NPC dispatch in the bounded native fixture ([saved reach evidence](../wiki/systems/godot-conversion.md#native-loot-reach--bounded-runtime-proof)).
- [ ] Prove hostile living-NPC behavior and role/UI responses; corpse-pose parity remains open.
- [x] Mount the shared authored LootFrame, dispatch slot/close actions, and apply matching removals, chat/error text and closure (bounded owned fixture, duplicate chat once).
- [x] Preserve loot cursor/sparkle and authoritative inventory/currency updates without new protocol messages (bounded fixture: bags 11, money 32756).
- [ ] Prove the native UI/network boundary with owned loopback fixtures and inspected rendering; Bevy proof above does not establish native parity.

Native integration reuses the authored frame and shared loot-state, card, placement and click policies. `LootChannel` traffic is relayed in channel order; auto collection remains server-owned. `InventoryDelta` and replicated `Gold` remain the bag/currency authorities. Native completion boxes require runtime proof, not merely compiled handlers. Bounded evidence includes shared exports/tests (8 + 1), ordered relay wire test (1 at `c6ae14bf`) and final source `292a2fb2`, Depot `tt4c247nl1` (`/tmp/claude/native-ui-caption-build.log`). Earlier `/tmp/claude/native-ui-caption-run.log` reaches `LOOT_DONE` after Auto Loot false/true/true/false, InventoryFull rejection/retry, matching removals/closure and authoritative bags 11/money 32756. Its full exit is 101 AFTER DONE: `loot fixture timed out; model_ready=true, opens=4`, not the prior `af03660f` RenderingServer-null failure (retained historical evidence). Shutdown is unresolved and explicitly deferred; clean acceptance stays open. Independent `/tmp/claude/verify-native-ui-overflow-final.md` accepts the bounded functional gate: 42 category/scale records, 6,792 descendant observations with zero enclosure violations, money geometry and four loot cases. Saved current root check and scoped format gates pass. Process termination remains FAIL/deferred; no independent glyph-pixel oracle or full-conversion acceptance.

Main records 42 Options checks (13 categories plus largest ActionBar at scales 1/0.75/1.25), no overflow failures; money Label 93×38, font 12, three visible lines, minimum height 35, paint/shadow inside card 46. Main inspected Items/stack 2/Poor captions in `data/diagnostics/native-ui-overflow-final/captures/`. Initial all-height caps hid caption 2 (`/tmp/claude/native-ui-visible-label-red.log`); `292a2fb2` bounds maximum width on all fixed Label axes and maximum height ONLY on spacing-fitted explicit multilines. No glyph clipping/font-size shrink. These literal boundaries do not establish full native parity or clean process completion. See [native evidence and root causes](../wiki/systems/godot-conversion.md#native-loot--implemented-proof-pending); commit IDs are provenance, not source-pinning gates.

### Native reach probes — bounded runtime proof

Test-only commits `76e5bd5c` / `6b1468e2` add `godot/network/examples/native_input_fixture/loot_range.rs` and `godot/tests/loot_reach_probe.gd` to the existing loot flow; production receiver behavior is unchanged. The policy reference remains `src/rendering/ui/target.rs:432` (`INTERACT_RANGE = 5`) and `interact_with_clicked_npc` at line 518: the `> 5` guard makes five-yard reach inclusive.

Fresh authoritative NPCs exercise actual posed-mesh right-clicks: corpse at 5.1 yards must send no `LootUnit`; corpses at 4.9 and 5.0 must each send one manual `LootUnit`, open the authored frame and send one real close-button `LootRelease`, without slot collection or awards; friendly living NPC at 5.0 must send exactly one `InteractNpc` and no `LootUnit`. Strict phase counts reject unexpected requests. Fresh spawns avoid interpolation failing to land on the exact boundary. Observed player pose only arranges inputs; it does not calibrate expected reach.

The [saved reach gate](../wiki/systems/godot-conversion.md#native-loot-reach--bounded-runtime-proof) now proves these probes alongside the original four Auto Loot/Shift cases and 42 Options checks. Hostile living-NPC and role/UI-response proof remain absent. Full process completion is **FAIL: exit 101 after DONE**, RenderingServer-null; shutdown deferred. No clean full-slice claim or source-pinning gate. Settings-reload `71ae65f6` + `92abd2d2` / `b8aca2ff` remain test preparation only, not fresh-process proof.

## Tests asserting this spec

- `src/game/loot_state.rs` tests: slots taken until close, Shift inversion, right-click choice, money lines and coin icon, loot chat lines.
- `src/ui/screens/loot_frame_component_tests.rs`: frame size, title, card offsets, quality tag only on items, stack count.
- `src/scenes/loot_frame/mod.rs` tests: placement under the cursor, card clicks, card state.
- `tests/unit/target_tests/interactions.rs`: right-click loots with Shift inversion, empty corpse, living NPC.
- `src/rendering/ui/wow_cursor.rs` tests: cursor by role, reaction and corpse.
- `src/rendering/ui/quest_sparkle.rs` tests: the corpse sparkle follows `Lootable`.
- `tests/unit/animation_tests/death_pose.rs`: Death once and held.
- `src/ui/panel_styles_tests.rs`: the no-portrait metal layout.
- Live evidence: `data/diagnostics/npcloot-20260925/` (t02 corpse, t04 loot window, t05 looted with chat lines, t11 backpack).
