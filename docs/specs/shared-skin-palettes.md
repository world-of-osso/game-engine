# Shared skin palettes

Share identical skinning palettes across render batches without changing mesh or joint entities. Implementation lives in `vendor/bevy_pbr/src/render/skin.rs`; engine M2 batching supplies the existing shared joints and inverse-bindpose handles. See [rendering pipeline](../wiki/systems/rendering-pipeline.md).

## What it must do

### Identity and rendering

- [ ] Visible meshes with the same ordered joint entities and inverse-bindpose asset identity use one palette allocation. Different joint order, joint identity, or asset identity remains separate.
- [ ] Every mesh retains its skin lookup and draw identity; shaders, joint entities, batching policy, animation, and equipment behavior remain unchanged.
- [ ] Joint-transform and inverse-bindpose changes update the shared matrices correctly, including same-frame membership changes.

### Lifetime

- [ ] Hiding, removing, or rebinding one mesh does not invalidate another mesh's shared palette. The last user releases the allocation.
- [ ] New visibility and removal/re-addition produce current matrices without stale membership or skipped extraction.
- [ ] Current/previous skin data retains existing motion-vector semantics and valid offsets in storage and uniform modes.

### Evidence

- [ ] Behavioral tests exercise actual extraction and lifecycle boundaries; GPU upload/history claims require GPU evidence.
- [ ] Comparable workload measurements justify retaining the prototype. Baseline and patched executables retain their matching dynamic libraries; vendoring preserves dependency optimization settings.

## How it works

- [Rendering pipeline](../wiki/systems/rendering-pipeline.md)
- [CPU investigation](../wiki/investigations/movement-performance.md)

## Implementation inventory

- `vendor/bevy_pbr/src/render/skin.rs` — canonical palette membership/allocation, unique palette extraction, constant-time per-mesh lookup, and unchanged buffer preparation.
- `Cargo.toml` / `Cargo.lock` — exact local Bevy PBR override and preserved dev optimization.
- `vendor/README.md` — provenance, licenses, and retirement conditions.

## Tests asserting this spec

- `vendor/bevy_pbr/src/render/skin/tests.rs` — planned real extraction and lifetime regressions.
- `tests/unit/equipment_live_tests.rs` — existing production rig/equipment behavior; not allocation-sharing proof.

## Known gaps (current cycle)

- [ ] Establish actual batch multiplicity and baseline allocation/extraction cost.
- [ ] Implement sharing and prove the requirements above.
- [ ] Record comparable native CPU results; no gain claimed at import.

## Out of scope

No shader redesign, joint pruning, inverse-bindpose deduplication by matrix contents, unrelated renderer repairs, frame caps, focus/worker restrictions, or host safety changes.
