# Godot conversion

The Godot replacement is at initial workspace bootstrap only. At game-engine `85794170`, a Godot 4.7 project loads a Rust GDExtension and instantiates its native `Node3D`; all client feature parity remains open.

## Bootstrap

- Official Godot 4.7.2 binary revision: `ed1daf0bf`; archive at `data/tools` verified as SHA-256 `cadd3204e728a35d3f13adb7fd0d7902636b79f6b95c40c265eb73b6c35329e4`.
- `godot/` is a Rust workspace: `core` is Bevy-free asset/parser code; `rust` builds `game-engine-godot` as `cdylib`/`rlib` with `godot` 0.5.5 and API 4.7 bindings.
- `project.godot` starts `scenes/client.tscn`; `game_engine.gdextension` declares `gdext_rust_init` and debug/release Linux libraries.
- `GameClient` is the only exposed native class, a `Node3D` root. `tests/extension_smoke.gd` first failed when the class was absent, then passed at `85794170` by registering, instantiating, and attaching it to a Godot scene tree.

## Current boundary

The headless runtime smoke test succeeded. An editor import scan exited with `SIGABRT`; it is not fixed and is separate from the successful runtime test. Core and UI conversion agents remain in progress. No rendering, gameplay, UI, automation, audio, CLI, IPC, or visual parity is complete.

## Sources

- [Godot conversion specification](../../specs/godot-conversion.md) — full replacement target and explicit open parity matrix.
- [Godot project](../../godot/project.godot) — project runtime configuration.
- [GDExtension declaration](../../godot/game_engine.gdextension) — extension ABI and library paths.
- [Rust extension](../../godot/rust/src/lib.rs) — `GameClient` native `Node3D` registration.
- [Extension smoke](../../godot/tests/extension_smoke.gd) — scene-tree smoke contract.

## See Also

- [[asset-pipeline]] — reusable local-CASC asset boundary.
- [[ui-system]] — existing UI behavior to preserve.
- [[rendering-pipeline]] — existing client rendering behavior to replace.
