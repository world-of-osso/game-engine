# M2 Format

M2 is Blizzard's model format for characters, creatures, doodads, and spell effects. Each model consists of a root `.m2` file (the MD21 chunk container), one or more `.skin` files for render batches, and optionally an external `.skel` file for HD models.

## File Structure

The root `.m2` file is a chunked binary. The primary chunk is `MD21` (magic `MD20` at offset 0), which contains the main header. Subsequent top-level chunks are identified by 4-byte tags:

| Chunk | Purpose |
|-------|---------|
| `MD21` | Main header: vertices, bones, sequences, materials, texture units, attachments, lights, particles |
| `TXID` | Texture FDIDs (supersedes legacy path strings in MD21) |
| `SFID` | Skin file FDIDs |
| `SKID` | Points to external `.skel` file FDID (HD models only) |

For HD models (e.g. `humanmale_hd.m2`), the `.skel` file is a separate chunked binary:

| Chunk | Purpose |
|-------|---------|
| `SKB1` | Bones (216 for human male HD) + per-bone animation tracks |
| `SKS1` | Animation sequences (422 for human male HD) + global sequences |

Legacy models embed bones and sequences directly in `MD21`.

## Bones and Skeleton

- Bone indices in M2 vertices are **global** skeleton indices — not local per-geoset indices.
- The skin file's bone lookup table remaps local vertex bone indices to global indices; this remap must be applied or vertices bind to wrong bones.
- HD models carry bones in `SKB1`; legacy models carry them inline in `MD21`.
- Key bone IDs: jaw = 7 (bone index 88 on human male HD, parent = head at 39).

## Animation Sequences

Each sequence has a `blend_time` used for crossfade transitions (minimum 150ms enforced). Tracks are stored per-bone per-sequence as M2Track arrays (translation, rotation, scale). External `.anim` files (referenced via `AFID` chunk) may carry additional per-sequence track data not present in the root file.

Sequence records retain signed `frequency` at0x10, replay bounds at0x14/0x18, and `variation_next` at0x3c. The link enumerates weighted alternatives of the same animation ID, not the animation to play next in time. The alias field at0x3e is separate and was not reinterpreted. See [weighted loop playback](../systems/animation.md#weighted-loop-variations) for implemented coverage and the remaining nonzero-replay limitation. Primary format reference: [wowlib M2 records](https://skarndev.github.io/wowlib/python/m2/records/).

## Geosets and Skin Files

Geosets are sub-meshes selected at runtime based on `mesh_part_id` (mpid = `group * 100 + variant`). Variant 0 = hidden, 1+ = visible options. See [[geosets]] for the full group table.

Skin files contain render batches that pair a submesh with a material and a texture unit. The `indexStart` field is u16 but HD models overflow it (>65535 triangle indices); the engine computes `triangle_start` as a cumulative sum instead of reading the overflowing field directly.

## Materials and Render Flags

The `M2Material` table (parsed from MD20 offset `0x70`) holds per-batch `flags` and `blend_mode`:

| Flag | Meaning |
|------|---------|
| `0x01` | Unlit |
| `0x04` | Two-sided (no backface cull) |

Blend modes follow WMVx conventions: 0=Opaque, 1=AlphaMask, 2=AlphaBlend, 3=Additive (SRC_COLOR), 4=Additive alpha (SRC_ALPHA), 5=Modulate, 6=ModulateX2, 7=BlendAdd.

## Texture Types

| Type | Source |
|------|--------|
| 0 | Hardcoded FDID (TXID chunk) — e.g. eye reflection |
| 1 | Body skin (composited character texture atlas) |
| 6 | Face/hair replaceable texture |

Type-6 geosets share a single composited 512×512 atlas built from DB2-traced face and hair texture FDIDs.

## Lights

The MD20 header light array starts at `0x108`. Current modern records are 156 bytes: a 16-byte prefix followed by seven 20-byte animation tracks at offsets `0x10`, `0x24`, `0x38`, `0x4c`, `0x60`, `0x74`, and `0x88`. Those tracks are ambient color/intensity, diffuse color/intensity, attenuation start/end, and byte visibility.

`4238519` has two type-1 cauldron point lights and `5149702`/`5140152` each have four lantern point lights. Local tracks sample the owning model sequence; global tracks use their declared global-sequence period. Static M2 attachments retain their authored lights and run the model-default local sequence independently of a parent skeletal player. See [[character-select-lighting-overwrite]] for the asset-backed regression evidence.

## Authored Cameras

For MD20 version 274 inside the `MD21` chunk, the camera array header is at payload offsets `0x110` (count) and `0x114` (offset). The outer chunk's 8-byte header is **not** part of these offsets. A camera record is 116 bytes: signed type and far/near clips at `0x00/0x04/0x08`; position spline track and base at `0x0c/0x20`; target spline track and base at `0x2c/0x40`; roll and FOV spline tracks at `0x4c/0x60`. The spline's first value is an offset added to the position/target base; FOV is a diagonal angle in radians. All coordinates remain raw WoW model-local coordinates.

`m2_camera::parse_camera_snapshot` extracts camera index zero and each track's first key only; it does not evaluate animation. It rejects missing/truncated records or required keys and unsupported MD20 versions rather than inventing FOV defaults. The three cached creation backdrops (`623712`, `623714`, `623716`) each have one type-`-1` camera, with FOV `0.84570783`, near clip `0.22222222`, and far clip `513.91522`. This parser does not yet wire a scene camera.

## Particles

Particle emitter data lives in the MD21 header at offset `0x128` (Cata+ layout, 476-byte stride). Each emitter references a bone index, a texture FDID (from TXID), and carries M2Track fields for emission speed, gravity, lifespan, etc. See [[particle-system]] for the renderer details and known limitations.

## Sources

- [wowlib M2 camera records](https://skarndev.github.io/wowlib/python/m2/records/) — versioned camera fields and spline/FOV interpretation
- [M2 camera parser](../../../src/asset/m2_format/m2_camera.rs) — MD20 layout, bounds checks, and cached creation-model fixtures
- [docs/particle-system.md](../particle-system.md) — M2 particle parser layout, field list, renderer architecture
- [docs/geosets.md](../geosets.md) — geoset groups, bone indices, texture types, HD geoset observations
- [docs/hd-skeleton-status.md](../hd-skeleton-status.md) — external .skel loading, skin index overflow fix, render flags, bone remap
- AGENTS.md (`asset/m2_format/` section) — module structure and chunk overview

## See Also

- [[geosets]] — mesh_part_id table, equipment slots, texture type assignments
- [[blp-format]] — texture format loaded for M2 texture units
- [[casc-format]] — how M2 files and their FDID references are resolved
- [[particle-system]] — particle emitter renderer built on M2 parser output
- [[character-select-lighting-overwrite]] — modern M2 light record and runtime ownership evidence
- [[db2-format]] — DB2 tables used to resolve character customization texture FDIDs
