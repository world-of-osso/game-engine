# Showcase client bugs (2026-09-30)

Five bugs seen in the Northshire mage showcase screenshots (`data/diagnostics/showcase-2026-09-30/`) of the Godot client, with root causes. Before/after captures: `data/diagnostics/showbugs-2026-09-30/{before,after}/`.

## Floating combat text stacked

Numbers fanned out over three fixed lanes: `[0, -0.8, 0.8]` yards along **world X**, indexed by `floating.len() % 3`. With the camera looking along X the lanes projected to the same pixel; and when an older number expired, the next one took a lane still used by a live one. In showcase-2 "98" and a crit "2!" were drawn on top of each other.

Retail drives world text from engine CVars whose formulas are unpublished (wowless retail 12.0.7 `cvars.yaml:1613-1626`): `WorldTextStartPosRandomness_v2` 1.0, `WorldTextRandomZMin/Max_v2` 0.8/1.5, `WorldTextRampDuration_v2` 1.0, `WorldTextRampPow_v2` 1.9, `WorldTextRampPowCrit_v2` 8.0, `floatingCombatTextFloatMode_v2` 1 (scroll up). `godot/rust/src/combat_text.rs` reads them by name: each number starts at its own point of an R2 low-discrepancy sequence in the camera plane (±1 yd sideways, 0–0.7 yd up), so any four in a row start ≥ 0.5 yd apart, and pops 1.5× then settles with the (crit) ramp power.

## Target nameplate faded

`plate_alpha` multiplied every plate, the target's included, by the legacy camera-distance fade (full to half of `HudOptions.nameplate_distance` 40, zero at 40). A target 29 yd from the camera read alpha 0.58 while in clear view, and beyond 40 yd its plate vanished. Retail gives the selected plate `nameplateSelectedAlpha` 1.0 (cvars.yaml:990) instead of the distance fade; `nameplateOccludedAlphaMult` still dims it behind geometry.

## Apprentice's Robe drawn as shirt and trousers

Two causes in the shared equipment resolver (`src/game/equipment/equipment_appearance_data.rs`):

- **Geosets.** The chest slot only ever produced `(22, 2)`. A robe's `GeosetGroup[2]` (display 12647: 1/0/1) never selected skirt 1302, and its `GeosetGroup[0]` never selected sleeves 802; the pants' bare legs stayed. Build 12340's `CCharacterComponent` order (solarityclient `character_component/geoset.rs` `apply_equipment_geosets`, `apply_robe`) is now applied after all items: gloves take the arms else chest sleeves 801+n; a chest robe (else a legs robe) hides boots 5xx, kneepads 902-999 and pants 11xx and shows 1301+n.
- **Texture order.** Item textures were pasted in equip order. The server lists the mage's items MainHand, Feet, Chest, Legs, Shirt, so the pants painted over the robe's legs and the shirt over its torso. They are now sorted by the recovered `ITEM_PRIORITIES` table and its sleeve/robe/boot adjustments (solarityclient `character_component/atlas.rs`).

## Staff not visible

The Bent Staff was attached, but always in the right palm: player equipment ignored sheath state, and the server replicates none for players. It now uses the creature virtual-item placement (`npc_gear_data::virtual_item_attachment`, `Item.SheatheType` 2 → back attachment 30) with `SheathState::Unarmed` out of combat (TrinityCore's initial `SheatheState`, UnitDefines.h:82) and `Melee` in combat. The in-combat draw is inferred, not cited.

## NPC animation errors

- **"M2 animation ID 6 has no base variation".** Injured Stormwind Infantry (50047, StandState Dead) request the pose clip Dead 6, which HumanMale HD lacks. The pose path skipped `AnimationData.Fallback`, which maps 6 → 1 (Death) and 136 → 62. The pose now resolves through the chain; Death plays once and holds its last frame.
- **".anim track … read_i16 out of bounds".** Not a parser bug. The cached `data/models/{model}.skel` files for goblin male 119376 (SKID 2184754) and human female HD 1000764 (SKID 2137789) date from 2026-03-12 and differ from current local CASC, while their `.anim` files were extracted 2026-09-27/28. With the stale skeleton the last bone track of sequence 95 ends at 0x8180, past the 0x8150 AFSB chunk; with the current skeleton it ends exactly at 0x8150. The shared asset cache never revalidates a positive hit across game builds.

## Sources

- [spellbook-action-bar spec](../../specs/spellbook-action-bar.md) — floating combat text contract
- [nameplate-style spec](../../specs/nameplate-style.md) — selected alpha
- [npc-appearance spec](../../specs/npc-appearance.md) — pose fallback

## See Also

- [[asset-pipeline]] — the file cache that keeps stale `.skel` files
- [[godot-conversion]] — client conversion status
