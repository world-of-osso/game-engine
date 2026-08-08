# World Builder

World Builder is an opt-in InWorld diagnostic sidebar for inspecting the live Bevy scene forest, isolating render or processing cost by subtree, and editing a bounded set of runtime properties. It is absent from normal runs and does not persist changes.

## Activation and Lifecycle

Pass `--world-builder` when launching `game-engine`. The plugin mounts its UI on entering `GameState::InWorld`, tears it down on exit, and keeps F9 as a fixed sidebar visibility toggle.

The plugin is conditionally installed in `src/main.rs`. Without the flag, its resources, systems, and frames are not registered.

## Scene Inventory

`build_scene_snapshot` collects runtime entities carrying `Transform` or `GlobalTransform`, including entities marked `Disabled`. It records entity IDs, labels, parent/child relationships, component names, transforms, and supported light values.

Rows use stable entity-derived names. Search matches labels and component names while retaining matching ancestors. Pagination bounds the number of rendered rows because the UI toolkit's `ScrollFrame` does not implement scrolling, clipping, or virtualization.

## Diagnostic Overrides

Render suppression recursively replaces subtree `Visibility` with `Hidden` and stores the previous value. Clearing the override restores each stored value rather than forcing `Inherited` or `Visible`.

Processing suppression recursively inserts Bevy `Disabled`. A marker records whether World Builder inserted it, so clearing the override preserves entities that were already disabled. Default Bevy queries then skip the suspended subtree. This stops the engine's M2 animation queries, particle-effect registration, model-particle emitter ticks, and model-particle simulation for those entities.

M2 effect-material UV evaluation needs separate handling because materials can be shared outside the disabled entity. While World Builder is installed, its filtered updater skips a material only when every scene entity using that material is processing-suspended. A shared material with any active user continues updating.

## Property Editing

The inspector lists every component name read-only. Mutable fields are intentionally limited to:

- `Transform` translation, Euler rotation, and scale
- `PointLight` intensity, range, and shadow toggle
- `DirectionalLight` illuminance and shadow toggle

Numeric values are parsed and validated before ECS mutation. Invalid or non-finite values leave the component unchanged and produce sidebar status text.

## Measurement Workflow

1. Launch with `--world-builder` and enter the target world.
2. Refresh the snapshot after the scene workload stabilizes.
3. Record the initial entity count and performance sample.
4. Disable render or processing for one entity or subtree.
5. Wait for timing to stabilize, then compare the same workload.
6. Use **Enable All** before changing scenes or ending the diagnostic.

Do not treat screenshot frames or frames immediately following `dump-scene` as performance baselines. Refresh after entities spawn or despawn so shared-material suppression is recalculated against the current scene.

## Constraints

- Runtime-only: no persistence, export, despawn, collision editing, or server-state mutation.
- Processing suppression covers default-query ECS work. Systems that explicitly include `Disabled` require their own filtering.
- The sidebar is a performance diagnostic, not a general reflected-component editor.

## Sources

- [world-builder spec](../../specs/world-builder.md) — behavioral contract and test inventory
- [`src/main.rs`](../../../src/main.rs) — conditional plugin installation
- [`src/rendering/ui/world_builder.rs`](../../../src/rendering/ui/world_builder.rs) — plugin resources and scheduling
- [`src/rendering/ui/world_builder/core.rs`](../../../src/rendering/ui/world_builder/core.rs) — snapshots, filtering, suppression, and validated edits
- [`src/rendering/ui/world_builder/runtime.rs`](../../../src/rendering/ui/world_builder/runtime.rs) — interaction, ECS mutation, and M2 material filtering
- [`src/rendering/particles/emitters.rs`](../../../src/rendering/particles/emitters.rs) — particle-effect registration query
- [`src/rendering/particles/emitters_model_particles.rs`](../../../src/rendering/particles/emitters_model_particles.rs) — model-particle tick and simulation queries
- [`src/rendering/particles/mod.rs`](../../../src/rendering/particles/mod.rs) — particle system scheduling
- [`src/rendering/model/animation.rs`](../../../src/rendering/model/animation.rs) — M2 animation queries filtered by `Disabled`
- [`src/ui/screens/world_builder_component.rs`](../../../src/ui/screens/world_builder_component.rs) — sidebar view and action contract

## See Also

- [[ui-system]] — Screen, SharedContext, FrameRegistry, and fixed UI inputs
- [[rendering-pipeline]] — M2 rendering and performance-investigation context
- [[animation]] — M2 animation systems affected by processing suppression
- [[keybindings]] — fixed F9 toggle
