# BLP qualification — before-only, no extraction approval

Approved scope: inventory current behavior and freeze independent CPU/GPU/icon receipts before proposing a crate/policy split. No production logic, Cargo pins, assets or dependencies change. Server icon behavior must remain unchanged. A fifth crate is not approved.

## Existing engine contract

`core/src/blp.rs` exposes `RgbaImage`, `decode_rgba`, `GpuFormat`, `GpuImage`, `decode_gpu`. CPU strips mipmap flags, swaps Raw1 palette BGRA channels, repairs all-zero/one-bit alpha, and handles BC5 separately. GPU retains authored DXT mip data, adjusts truncated level-zero dimensions, repairs zero DXT3/5 alpha, and decompresses alpha-bearing BC1 for Godot's RGB-only compressed upload format. `asset/blp_format.rs` exposes `strip_mipmaps` and `fix_1bit_alpha`; those helpers are re-exported through the existing asset namespace. No behavior moves in this qualification.

`core/src/blp_tests.rs`:12 tests, of which4 are synthetic BC5 cases and8 authored fixtures (7 present, palette180071 missing). `core/tests/asset_parsing.rs` adds the authored boar+bad-input BLP case. `core/tests/m2_material.rs` also exercises GPU decoding of material-resolved Avenging Wrath textures. NPC/atlas sweep integrations consume RGBA; they are not an exhaustive BLP format oracle.

## Observer

`core/tests/blp_qualification.rs::capture_cpu_gpu_before_only` is ignored unless explicitly invoked with frozen input/output paths. It calls the actual current APIs independently, recording dimensions, format/mipmap flag, complete pixels/upload bytes, exact errors and panic messages. Panic capture is observation only; no decoder error is repaired. A fresh output directory is mandatory.

```text
WOW_BLP_INPUTS=/home/osso/.worktrees/logs/wow_formats-phase7/inputs.json
WOW_BLP_ENGINE_OUTPUT=/home/osso/.worktrees/logs/wow_formats-phase7/engine
agent-run wow_formats python3 scripts/depot-build.py --root <engine-slot> --test -p game-engine-core --test blp_qualification -- --ignored --exact capture_cpu_gpu_before_only
```

Use normal capacity ADD and the existing protocol sibling. Before-only corpus/receipt manifests and run logs live under `/home/osso/.worktrees/logs/wow_formats-phase7*`; no assets are copied into this repository. Frozen source hashes and complete input hashes must be checked after capture. The missing authored180071 test remains a named gap, not an ignored success or synthetic replacement.

## Server boundary

Server `site_snapshot_catalog::icon` currently loads/converts BLP directly through image-blp1.2, then encodes PNG using image0.24.9. It does not apply the engine's normalization. PNG preference, missing-file returns, cached bundles, icon-prefixed errors and complete encoded output are separate server policy. Its observer invokes that exact function; qualification does not migrate it.

Results and split proposal are pending capture. No compatibility or cross-consumer equality is assumed.
