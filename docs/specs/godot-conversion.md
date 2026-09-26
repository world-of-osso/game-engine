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

- `godot/project.godot` — Godot project configuration and bootstrap scene.
- `godot/game_engine.gdextension` — Rust extension ABI/library declaration.
- `godot/rust/` — native `GameClient` integration; no renderer or game scene implementation.
- `godot/core/` — `game-engine-core`, a Bevy-free authored-byte parser boundary: M2/skin/skeleton data, BLP RGBA8, ADT companions, WDT, and WMO raw data. It excludes CASC/file resolution, Godot conversion, rendering, and playback.
- `../ui-toolkit-godot-conversion/core/` — sibling `ui-toolkit-core`: reusable frame/layout/widget/atlas/screen/registry model. It is not a Godot projection and currently still uses Bevy asset/image/math crates.

## Tests asserting this spec

- `godot/tests/extension_smoke.gd` — native client extension loads and joins a real Godot scene tree; runtime smoke only.
- `godot/core/tests/asset_parsing.rs` — seven fixture behaviors for M2, BLP, ADT, WDT, and WMO. All seven passed before final public-visibility and strict-skeleton-validation edits; current-commit status is unverified.
- `../ui-toolkit-godot-conversion/core/tests/model.rs` — two targeted model tests green: registry mutation/layout publication and named-frame screen diff. No native host or visual proof.

## Known gaps (current cycle)

| Capability | Current boundary | Proof status |
| --- | --- | --- |
| Bootstrap | `GameClient` GDExtension and scene-tree attachment. | Runtime smoke passes. Empty-project editor testing on main did not reproduce the former abort; extension/editor interaction remains unresolved. |
| Reusable parsers | Byte-to-data APIs for M2/BLP/ADT/WDT/WMO only. | Historical 7/7 fixture pass; unverified after final parser edits. |
| Reusable UI model | Sibling registry/layout/screen model only. | Two targeted model tests green; no Godot projection. |
| Rendering and scenes | No Godot mesh/material/terrain/animation/particle/sky/camera scene path. | Open. |
| Client networking and gameplay | No Godot lifecycle, Lightyear/state application, login-to-world flow, collision, equipment, or gameplay migration. | Open. |
| UI parity | No Godot frame projection or screen/workflow/visual/input parity. | Open. |
| Tooling | No Godot automation, screenshots, diagnostics, CLI/IPC, or debug-scene parity. | Open. |

- [ ] Preserve existing Lightyear protocol semantics while replacing the client host; the existing headless Bevy worker does not authorize a server/protocol redesign.
- [ ] Build and prove Godot projection before claiming any UI parity.
- [ ] Exercise actual client/server workflows and visual fixtures before checking any final parity requirement.

## Out of scope

Unrequested server/protocol redesign, new gameplay features, production deployment, and changes to host safety mitigations. No existing client feature is excluded from the conversion target.
