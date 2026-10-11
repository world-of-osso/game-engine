# Non-visual equipment and Forever pointer ordering

Verified: 2026-10-11. Server storage, migration and item rules live in the [inventory system](../../../../game-server/docs/wiki/systems/inventory.md#non-visual-equipment--verified-2026-10-11). UI contract: [Character Frame](../../specs/character-frame.md).

## Native wiring

Both skins already project the same18 Mainline buttons. `InventoryState` consumes authoritative equipment snapshots/deltas; `paperdoll_slots` supplies owned icons/quality and cursor dimming. `paperdoll_tooltip` resolves each EquipmentSlot through the inventory, never EquipmentAppearance. `cursor_action_target` and `paperdoll_unequip_request` send the same equipment location for jewelry/trinkets as visible gear. Ranged weapons use the main hand; no new ranged/relic button is introduced.

## Pointer root cause

Forever's398px model scene overlaps both slot columns. Model and slot siblings had equal strata/frame_level/raise_order; `Projection::pointer_frame_at` sorted only those keys and selected the earlier model frame. Drawing instead creates same-level siblings in ascending frame ID, so later slot frames appear above the model. Descending frame ID breaks picking ties consistently with their draw order. Modern's smaller model did not overlap those columns.

Production-order regression: the same private actor acquired/equipped Mood Ring7338, all five non-visual slots rendered, but Forever ring hover timed out. After the tie-order fix the persisted actor showed the ring tooltip and a native finger drag sent the swap. `equip_slots_live.gd` exercises this boundary in both skins, plus exact five-slot identities and money change; it does not bypass input with direct equip setters.

## Proof

PrivateUDP5821, fb_equip_mfinal/fb_equip_ffinal. A read-only copied world.db relocates ordinary Alliance vendor Arred17512 near Northshire; original vendor flags/items/prices remain unchanged. Each actor buys Mood Ring7338 for10000copper, equips it with authentic bag-granted neck/ring/trinkets, and displays Stamina895→897/Versatility1%. Both skins pass native tooltip/drag, then fresh client processes after server restart preserve exact GUIDs/slots/stats. Twelve inspected1280×720 PNGs: `/syncthing/AgentShared/2026-10-11/equip-slots/`. Logs/receipts: `data/diagnostics/equip-slots-2026-10-11/`.

Arred's missing authored profile17222 was imported from local Retail DB2, preserving15038 existing profiles; its required textures were published through authenticated no-clobber asset fragments. No model fallback or canonical world.db write. No full engine/server suite. Legacy relic storage has no current Retail catalog fixture; existing untracked proficiencies/on-equip ItemEffect triggers are not new claims.

## Sources

- [Slot and screen model](../../../godot/ui-model/src/character_frame/mod.rs).
- [Tooltip consumer](../../../godot/rust/src/tooltip_sources.rs).
- [Pointer projection](../../../godot/rust/src/ui/projection.rs).
- [Both-skin behavioral test](../../../godot/ui-model/tests/character_frame.rs).
- [Native acquisition/input/relog fixture](../../../godot/tests/equip_slots_live.gd).
