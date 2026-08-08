# World Builder

World Builder is an opt-in in-world diagnostic sidebar implemented under `src/rendering/ui/world_builder*` and `src/ui/screens/world_builder_component*`. It exposes the live Bevy scene forest for performance isolation. See [world-builder](../wiki/systems/world-builder.md) for implementation details.

## What it must do

### Lifecycle

- [x] Activate only when `--world-builder` is present.
- [x] Mount in `GameState::InWorld` and remove its UI when leaving that state.
- [x] Toggle the mounted sidebar with F9 without enabling the tool in normal runs.

### Scene inventory

- [x] Enumerate transformed runtime scene entities as a stable hierarchy/forest, including entities carrying Bevy `Disabled`.
- [x] Search names and component names while retaining matching ancestors.
- [x] Render stable entity-derived row names, selection actions, expansion controls, and bounded pagination.
- [x] Show every selected entity component name read-only.

### Diagnostic controls

- [x] Recursively hide and restore an entity subtree without despawning it.
- [x] Recursively suspend and resume default-query ECS processing with Bevy `Disabled`.
- [x] Stop M2 UV evaluation only for materials whose scene users are fully processing-suspended.
- [x] Stop M2 animation evaluation, particle-effect registration, and model-particle ticking for processing-suspended entities.
- [x] Disable render or processing for all scene roots and clear all overrides.

### Property editing

- [x] Display and validate selected-entity translation, rotation, and scale edits.
- [x] Display and validate `PointLight` intensity/range and `DirectionalLight` illuminance edits.
- [x] Expose point/directional shadow toggles.
- [x] Prove search and property editing through scheduled pointer and keyboard interaction tests.

## How it works

- [world-builder](../wiki/systems/world-builder.md)
- [ui-system](../wiki/systems/ui-system.md)
- [rendering-pipeline](../wiki/systems/rendering-pipeline.md)

## Implementation inventory

- `src/main.rs` — parses activation and conditionally installs the plugin.
- `src/cli_args.rs` — documents the CLI activation flag.
- `src/rendering/ui/world_builder.rs` — plugin resources and shared runtime types.
- `src/rendering/ui/world_builder/core.rs` — scene snapshots, hierarchy filtering, suppression, and validated edits.
- `src/rendering/ui/world_builder/runtime.rs` — input, action dispatch, UI synchronization, and filtered M2 UV updates.
- `src/ui/screens/world_builder_component.rs` — public view/action contract.
- `src/ui/screens/world_builder_component/` — declarative sidebar sections and component tests.
- `src/rendering/model/m2_effect_material.rs` — reusable single-material UV evaluation helper.
- `tests/unit/animation_tests/core.rs` — disabled-root M2 animation boundary.
- `tests/unit/particle_tests/runtime_tests.rs` — disabled particle registration and model-particle ticking boundaries.

## Tests asserting this spec

- `tests/unit/main_tests.rs`
- `src/rendering/ui/world_builder/tests.rs`
- `src/ui/screens/world_builder_component/tests.rs`
- `src/rendering/model/m2_effect_material.rs`
- `tests/unit/animation_tests/core.rs`
- `tests/unit/particle_tests/runtime_tests.rs`

## Known gaps (current cycle)

- [ ] Run the real InWorld sidebar against the game server and inspect its UI tree.
- [ ] Record the first controlled render-vs-processing FPS isolation sample.

## Out of scope

- Arbitrary reflected-component editing: unsafe and too broad for the FPS diagnostic goal.
- Scene persistence/export: this tool changes only live runtime state.
- Despawning, collision editing, or server-state mutation: temporary isolation must remain reversible.
- Production-game UI: the plugin is absent unless explicitly activated.
