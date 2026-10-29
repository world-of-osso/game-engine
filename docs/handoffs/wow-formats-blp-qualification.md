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
agent-run wow_formats python3 scripts/engine-build.py --root <engine-slot> --test -p game-engine-core --test blp_qualification -- --ignored --exact capture_cpu_gpu_before_only
```

Use normal capacity ADD and the existing protocol sibling. Before-only corpus/receipt manifests and run logs live under `/home/osso/.worktrees/logs/wow_formats-phase7*`; no assets are copied into this repository. Frozen source hashes and complete input hashes must be checked after capture. The missing authored180071 test remains a named gap, not an ignored success or synthetic replacement.

## Server boundary

Server `site_snapshot_catalog::icon` currently loads/converts BLP directly through image-blp1.2, then encodes PNG using image0.24.9. It does not apply the engine's normalization. PNG preference, missing-file returns, cached bundles, icon-prefixed errors and complete encoded output are separate server policy. Its observer invokes that exact function; qualification does not migrate it.

## Captured before-only evidence

Frozen202 inputs comprise9 existing authored files and193 synthetic cases. Engine observer1/0:CPU33 success/162 error/7 panic; GPU34 success/162 error/6 panic. All9 authored inputs succeed. Complete engine inventory:68 files22,476,152B. Server observer1/0 captures207 icon cases including five source-selection edges:32 success/166 error/7 panic/2 None, with7 authored successes and2 authored BC5 refusals. Complete icon inventory:64 files4,123,240B. All202 input hashes and5 decoder/lockfile hashes remain unchanged.

`/home/osso/.worktrees/logs/wow_formats-phase7/before-receipt.json` records full filenames/counts/sizes/SHA-256 and exact independent outcomes. `wow_formats-phase7-audit.py` rechecks all inputs, production sources/lockfiles and complete receipt inventories without decoding again. This is a before-only oracle, not old/new equivalence.

Sixteen CPU/icon divergences include real464043 (65,536 alpha bytes), real1022933 (965,880 channel bytes), both realBC5 refusals, zero-alpha synthetic DXT3/5 and palette0/1/4/8 plus syntheticBLP1. Six truncated DXT block cases panic on all paths;8x8header/4x4payload additionally panics on CPU/icon while GPU returns4x4compressed. Those outcomes are retained, not fixed. Full error and panic strings live in each outcomes.json.

Retained engine `blp::tests::`:11 pass/1 fail, the already absent180071 fixture,0 ignored. The authored boar+invalid-byte integration passes1/0. No authoredBLP0/BLP1/JPEG coverage is claimed; a bounded4096-entry header scan found an authoredDXT3 but noBLP1. Synthetic cases do not substitute for missing authored proof. Qualification is not an all-green release claim. Scoped helper core check and workspace fmt check both exit0; manual changed-line readability audit found no warning suppressions or production changes. Decoder/lockfile hashes still match the capture source after merging master (only unrelated GDScript/docs changed). No receipt test is rerun for docs-only commits.

## Smallest boundary proposal

Recommend **no fifth crate and no consumer migration now**. The common raw parser is already image-blp1.2.0, used by both consumers. Keep engine CPU palette/alpha/mip normalization and BC5 support local; keep all Godot upload policy local; keep server source selection/direct conversion/errors/PNG bytes unchanged. A facade merely re-exporting image-blp would not remove a duplicate decoder. Exporting the entire engine wrapper would turn consumer policy into shared format behavior and change icons.

If a separate portability/reuse goal later justifies a custom public decoder, the narrow candidate is BC5 validation/decode only. It currently has one consumer, so it is not recommended as another consolidation step. It requires explicit approval and replay against each consumer's own receipts, including unchanged icon refusals. No extraction follows this qualification without approval.
