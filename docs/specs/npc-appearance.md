# Authored NPC rendering

Replicated NPCs render the appearance selected by their creature display data. Runtime integration lives in `src/rendering/character/npc_appearance.rs`; [character rendering](../wiki/systems/character-rendering.md#authored-npc-appearance) describes the pipeline. Cache production has a separate [importer contract](npc-appearance-importer.md).

## What it must do

- [x] Retain full customization choice IDs and apply related materials/geosets only when their required choice is selected; unresolved choices produce an explicit error.
- [x] Use an authored baked body texture without overwriting its clothing with the composited body. An authored absence of a bake uses composition; an unavailable declared bake is an error.
- [x] Bind distinct body/hair textures to individual NPCs without mutating shared materials or ordinary entities outside the affected visual subtree. Material target10 declares a separate hair texture for M2 type6; failed declared hair composition is an error, not a head-texture substitute. Without target10, type6 uses the composed head atlas.
- [x] Apply selected geosets followed by authored overrides, preserving character group-zero body rules; apply each added request once rather than reallocating materials every update.
- [ ] The real replicated-NPC spawn path must produce visibly correct clothing and distinct authored hairstyles/colors for the affected Northshire scene.

## How it works

- [Authored NPC appearance](../wiki/systems/character-rendering.md#authored-npc-appearance)
- [Appearance importer](npc-appearance-importer.md)

## Implementation inventory

- `src/rendering/character/npc_appearance.rs` — request processing, full-ID selection, compositing and isolated per-mesh application.
- `src/game/networking/npc.rs` — request creation after M2 spawning and update-system registration.
- `src/rendering/character/character_customization.rs` — shared geoset visibility and override rules.
- `src/game/creatures/npc_appearance.rs` — authored display cache reader, owned by the importer/data integration.

## Tests asserting this spec

- `src/rendering/character/npc_appearance.rs::tests` — body pixels/error semantics, full-ID related selections, two-NPC material/geoset isolation and once-only updates.
- `tests/unit/character_customization_tests.rs` — shared group-zero and exact geoset override semantics.

## Known gaps (current cycle)

- [ ] Parent integration must import current display data and visually validate the actual replicated Northshire NPCs; synthetic material tests are not visual acceptance.

## Out of scope

- Player equipment replication and creature model redesign; authored NPC data is applied through the existing M2 and character paths.
