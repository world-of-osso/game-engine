# Authored NPC rendering

Replicated NPCs render the appearance selected by their creature display data. Runtime integration lives in `src/rendering/character/npc_appearance.rs`; [character rendering](../wiki/systems/character-rendering.md#authored-npc-appearance) describes the pipeline. Cache production has a separate [importer contract](npc-appearance-importer.md).

## What it must do

- [x] Retain full customization choice IDs and apply related materials/geosets only when their required choice is selected; unresolved choices produce an explicit error.
- [x] Resolve race/sex to ChrModel from `ChrRaceXChrModel.csv` (Kul Tiran 32, allied races). Apply choices of the displayed model; choices of the race's unaltered form (`ChrRaces.UnalteredVisualRaceID`, Worgen 22 → Human 1) are accepted without being applied.
- [x] Material target 10 declares the type-6 hair texture only where the layout composes target 10 into texture type 6. Dracthyr layout 155 uses target 10 for type 9 and has no type-6 hair.
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
- `godot/rust/src/assets/appearance.rs` — native lazy read-only `npc_appearance.sqlite`/customization/compositor cache handles; prepares body, type-6, and type-19 textures plus selected/authored geosets.
- `godot/rust/src/assets/mod.rs` — passes an optional prepared native appearance into M2 batch construction; `assets/creature.rs` prepares it after creature asset caching and before model allocation.
- `godot/rust/src/assets/material.rs` — substitutes a prepared non-effect batch base texture; `assets/mod.rs` applies shared selected-then-authored geoset visibility per batch.
- `godot/rust/src/world_models.rs` — owns `NpcAppearances`, prepares by display ID before `load_creature_model`, and retains ordinary displays on the no-appearance path.

## Tests asserting this spec

- `src/rendering/character/npc_appearance.rs::tests` — body pixels/error semantics, full-ID related selections, two-NPC material/geoset isolation and once-only updates; real-data Kul Tiran 140376, Worgen 31054 mixed forms, Dracthyr 110154 without hair; ignored `sweep_all_spawned_profiles` (`SWEEP_CACHE`, `SWEEP_IDS`) prepares every listed profile and reports failures.
- `tests/unit/character_customization_tests.rs` — shared group-zero and exact geoset override semantics.

## Known gaps (current cycle)

- [ ] NPC composition currently binds texture types 1, 6 and 19 only. These are compositor bindings, not the three creature skin-replacement slots (M2 types 2/11, 12 and 13). Other layout texture types (Dracthyr 7–26, types 7/8/20 of other layouts) keep the M2 defaults or are blitted into the body atlas; not visually validated.

- [ ] Parent integration must import current display data and visually validate the actual replicated Northshire NPCs; synthetic material tests are not visual acceptance. The native path reads imported caches only: it neither checks importer freshness nor rebuilds them.

- [ ] `85fa5d8a` adds a native missing-required-type-6 fixture. Against unguarded DLL `1952d1cd`, it reaches `BAKED_READY` and `COMPOSED_READY`, then exits 101 at phase 22 because the ordinary type-6 batch silently retains its original base texture (`/tmp/claude/native-npc-type6-red-85fa5d8a.md`). `6f00e74c` rejects an absent type-1 or type-6 replacement for an ordinary prepared-NPC batch; effect routing is unchanged. At `7526c855`, warning-free native build exits 0 and the actual 24-phase UDP fixture exits 0 (`/tmp/claude/native-npc-type6-{build,green}-7526c855.log`): display `910014` reports its missing type-6 texture for batch 0 without creating a visual, then reset passes. At test commit `3996c869`, the same real-UDP fixture runs 25 phases against native DLL `7526c855` and exits 0 (`/tmp/claude/native-npc-hair-3996c869.log`): ordinary display `910016` reaches `TYPE6_HAIR_READY` with its declared target-10 type-6 `512×512` hair crop bound. The fixture samples the actual native texture-image pixel and proves it differs from base/body/head; it does not sample rendered pixels. Its dummy-material-null diagnostic is captured after `BAKED_READY` and before `COMPOSED_READY` during baked-model replacement, not proven shutdown-only. Prior root library fmt/check and core `npcassets` 4/4 proof remain unchanged at `25d59471` (`/tmp/claude/verify-native-npc-appearance-*`). Verifier376 passes at `fcac4099`: `cargo fmt --check` and `cargo check -p game-engine-godot` exit 0 in `godot/` (`/tmp/claude/final-native-npc-appearance-{native-fmt,native-check}-fcac4099.log`); native sources remain `7526c855`/`0475ef39`. Defer only pre-existing `build_model` assembly length (63 body lines): new batch selection is extracted; cognitive 9/cyclomatic 14; no behavioral or complexity failure authorizes broader refactoring. Effect routing and optional type 19 are source-inspection-only; runtime proof covers missing required type 6 only. This remains resource/fixture state, not rendered-pixel, successful-type-6-texture-fixture, or full-parity proof.

## Out of scope

- Player equipment replication and creature model redesign; authored NPC data is applied through the existing M2 and character paths.
