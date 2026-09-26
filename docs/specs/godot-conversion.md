# Godot client replacement

Replace the Bevy client engine with Godot while retaining reusable Rust and preserving existing feature behavior and UI appearance. The conversion is an isolated experiment; a partial renderer or launchable scene does not satisfy completion.

## What it must do

### Client engine and assets

- [ ] Godot owns client rendering, scenes, input, UI, audio and lifecycle; replaced Bevy engine paths are removed, not kept as runtime fallbacks.
- [ ] Retain existing local-CASC asset resolution and supported M2/BLP/ADT/WMO parsing semantics, materials, characters, animation, equipment and world streaming.
- [ ] Preserve existing camera, collision, light/sky/shadow, particle and sound behavior.

### Application and UI parity

- [ ] Preserve login, registration, character selection/creation/deletion, loading, world entry and reconnect behavior against the existing authoritative server.
- [ ] Preserve all existing gameplay requests, replication, state application and UI workflows specified by existing feature contracts.
- [ ] Preserve UI screen appearance, authored assets, layouts, text, focus, editing, scrolling, layering, input and named-frame automation.
- [ ] Preserve CLI, IPC, screenshot, scene/UI-tree diagnostics, JavaScript automation and debug scenes.

### Evidence

- [ ] Record a complete feature-parity matrix, with missing/blocked/unverified capabilities explicit.
- [ ] Exercise real client/server workflows and inspect matching visual/interaction fixtures.
- [ ] Record matched frame-time, loading and memory evidence without inferring performance from implementation shape.
- [ ] Pass relevant behavioral/integration tests and final Rust formatting/checks at the integrated revision.

## How it works

- [Existing UI contracts](registry-bevy-ui.md)
- [Rendering pipeline](../wiki/systems/rendering-pipeline.md)
- [Networking boundary](../wiki/systems/networking.md)

## Implementation inventory

- `godot/project.godot` — Godot project configuration.
- `godot/game_engine.gdextension` — Rust extension ABI/library declaration.
- `godot/rust/` — native Godot client integration.
- `godot/core/` — reusable Rust asset/data implementation.

## Tests asserting this spec

- `godot/tests/extension_smoke.gd` — native client extension loads and joins a real Godot scene tree; not feature or visual parity proof.

## Known gaps (current cycle)

- [ ] All gameplay/render/UI/automation parity remains open at initial extension bootstrap.
- [ ] Existing Lightyear worker uses a headless Bevy ECS app. Retaining reusable transport must be documented separately from removing Bevy rendering/UI; no server protocol rewrite is authorized.
- [ ] Portable UI registry extraction and Godot projection are unfinished.

## Out of scope

Unrequested server/protocol redesign, new gameplay features, production deployment, and changes to host safety mitigations. No existing client feature is excluded from the conversion target.
