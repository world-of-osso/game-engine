# Godot conversion

The Godot replacement remains an early bootstrap. The project loads a Rust GDExtension and instantiates `GameClient` as a native `Node3D`; `21e83ff7` adds reusable asset-parser APIs and sibling `c7bc1e2` adds a portable UI model crate. Neither is a Godot renderer, scene system, or parity implementation.

## Bootstrap

- Official Godot 4.7.2 binary revision: `ed1daf0bf`; archive at `data/tools` verified as SHA-256 `cadd3204e728a35d3f13adb7fd0d7902636b79f6b95c40c265eb73b6c35329e4`.
- `godot/` is a Rust workspace: `core` is Bevy-free asset/parser code; `rust` builds `game-engine-godot` as `cdylib`/`rlib` with `godot` 0.5.5 and API 4.7 bindings.
- `project.godot` starts `scenes/client.tscn`; `game_engine.gdextension` declares `gdext_rust_init` and debug/release Linux libraries.
- `GameClient` is the only exposed native class, a `Node3D` root. `tests/extension_smoke.gd` first failed when the class was absent, then passed at `85794170` by registering, instantiating, and attaching it to a Godot scene tree.

## Reusable-core boundary

`godot/core` (`game-engine-core`) is Bevy-free and exposes authored-byte parsing only:

- M2 model/skin data, including external SKID skeleton input and strict skeleton hierarchy validation; it returns vertices, indices, skin batches/materials, bones, sequences, tracks, and texture FDIDs without rendering or coordinate conversion.
- BLP mip-0 RGBA8 decoding.
- ADT root, texture, and object companions; WDT map flags/global WMO; WMO roots and raw group geometry.
- No CASC resolution, file loading, mesh/material construction, animation playback, terrain spawning, or Godot resource conversion.

Sibling `../ui-toolkit-godot-conversion/core` (`ui-toolkit-core`) extracts the UI model: frames, layout/anchors, strata, widget definitions/data, atlas metadata, `Screen`/`SharedContext`, and `FrameRegistry` mutation/dirty publication. It is not a Godot projection or a complete Bevy removal: its current manifest still depends on `bevy_asset`, `bevy_image`, and `bevy_math`.

## Capability and proof matrix

| Area | Current capability | Proof / limit |
| --- | --- | --- |
| Bootstrap | Rust GDExtension registers `GameClient` and runtime smoke attaches it to a Godot scene tree. | Runtime smoke passed. Empty-project editor testing on main did not reproduce the prior abort; extension/editor interaction remains unresolved. |
| Asset parser core | M2/BLP/ADT/WDT/WMO byte-to-data APIs above. | Seven fixture tests passed before final public-visibility and strict-skeleton-validation edits; current-commit status is unverified. |
| UI model core | Registry/layout/screen-diff model extracted in sibling workspace. | Two targeted model tests green: mutation publication/layout and named-frame screen diff. No Godot projection proof. |
| Rendering, scenes, networking, UI, tooling | None migrated. | Rendering, game scenes, client networking/state application, UI projection/visual parity, automation, audio, CLI/IPC, diagnostics, and gameplay remain open. |

The legacy editor import `SIGABRT` is not evidence of a current empty-project failure: the empty project was tested on main without that abort. It does not establish extension/editor compatibility.

## Sources

- [Godot conversion specification](../../specs/godot-conversion.md) — full replacement target and explicit open parity matrix.
- [Godot project](../../godot/project.godot) — project runtime configuration.
- [GDExtension declaration](../../godot/game_engine.gdextension) — extension ABI and library paths.
- [Rust extension](../../godot/rust/src/lib.rs) — `GameClient` native `Node3D` registration.
- [Godot core API](../../godot/core/src/lib.rs) — reusable parser module surface.
- [Godot parser fixtures](../../godot/core/tests/asset_parsing.rs) — seven parser behaviors and failure boundaries.
- [Sibling UI core registry](../../../../ui-toolkit-godot-conversion/core/src/registry.rs) — extracted frame/model registry boundary.
- [Sibling UI core tests](../../../../ui-toolkit-godot-conversion/core/tests/model.rs) — targeted model behavior proof.
- [Extension smoke](../../godot/tests/extension_smoke.gd) — scene-tree smoke contract.

## See Also

- [[asset-pipeline]] — reusable local-CASC asset boundary.
- [[ui-system]] — existing UI behavior to preserve.
- [[rendering-pipeline]] — existing client rendering behavior to replace.
