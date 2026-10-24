# M2 Declaration-to-Handling Map

The [census](benilla-m2-field-census.md) individually lists **630 named members in90 file-qualified representations: E362/B268**. This map assigns every census representation, and therefore every member listed under it, a representation role and handling worksheet. It does not count630 unique disk fields or claim every byte/version is supported. Names/types stay in the census rather than being duplicated here.

Snapshot: **2026-10-24**. E native source baseline `354bdb7dbf177047ed46d63168acec531f5c4426`; B `8a6c74036cfe3d00275fd75ad1aeae1f4ea7d0df`. Roots, version limits and static-only proof follow [[benilla-asset-field-inventory]]. No source/client execution, build, test or asset-corpus validation.

## Role vocabulary

- **D — decoded/staging representation:** groups source values/descriptors or raw reads. Does not imply every member survives parsing or has a consumer; worksheet records drops/unknowns.
- **P — projection:** copied, narrowed, selected, remapped or derived values, sometimes mixed with defaults/decoded fields. Not a second serialized record. Field-specific distinctions are in the worksheet and exceptions below.
- **A — aggregate:** owns/borrows component arrays, tracks or another record. Its members route to their component inputs, not new enclosing disk fields.
- **H — helper/default/sample state:** configuration, clocks, calculated selection or evaluated output. Any backing file inputs are already listed in the relevant worksheet.

The role classifies the representation, not a blanket parsing/consumer verdict for its members. A row with a bounded unknown consumer is still an inventoried field, not an omitted field. Source lines below locate the named declarations; parsing and production-use evidence comes from the linked handling rows, not declarations alone.

## All census representations

### B: `crates/benilla-formats/src/emit_timing.rs`

| Record | Members | Declaration lines | Role | Handling |
|---|---:|---|---|---|
| `EmitTiming` | 3 | 17–22 | P | [Particle](benilla-m2-particle-fields.md) |
| `ParamsNow` | 9 | 154–171 | H | [Particle](benilla-m2-particle-fields.md) |
| `EmitParams` | 1 | 196–196 | A | [Particle](benilla-m2-particle-fields.md) |
### B: `crates/benilla-formats/src/models/anim.rs`

| Record | Members | Declaration lines | Role | Handling |
|---|---:|---|---|---|
| `SkeletonBone` | 5 | 17–25 | P | [Animation](benilla-m2-animation-fields.md), [Projection details](benilla-m2-representation-fields.md) |
| `Skeleton` | 1 | 31–31 | A | [Animation](benilla-m2-animation-fields.md), [Projection details](benilla-m2-representation-fields.md) |
| `M2Attachment` | 3 | 74–76 | P | [Scene](benilla-m2-scene-fields.md) |
| `EventMarker` | 3 | 103–105 | P | [Scene](benilla-m2-scene-fields.md) |
| `StringAnchors` | 2 | 128–129 | P | [Projection details](benilla-m2-representation-fields.md), [Scene](benilla-m2-scene-fields.md) |
| `PlayableAnim` | 2 | 190–192 | P | [Animation](benilla-m2-animation-fields.md), [Projection details](benilla-m2-representation-fields.md) |
| `BoneKeys` | 4 | 262–265 | P | [Animation](benilla-m2-animation-fields.md), [Projection details](benilla-m2-representation-fields.md) |
| `GlobalSeqChannel` | 2 | 272–273 | P | [Animation](benilla-m2-animation-fields.md), [Projection details](benilla-m2-representation-fields.md) |
| `GlobalSeqBone` | 4 | 279–282 | P | [Animation](benilla-m2-animation-fields.md), [Projection details](benilla-m2-representation-fields.md) |
| `AnimEvent` | 5 | 289–299 | P | [Scene](benilla-m2-scene-fields.md) |
| `ModelAnimation` | 17 | 307–338 | P | [Animation](benilla-m2-animation-fields.md), [Projection details](benilla-m2-representation-fields.md) |
| `ChannelTrack` | 4 | 367–374 | P | [Animation](benilla-m2-animation-fields.md), [Projection details](benilla-m2-representation-fields.md) |
| `EventRecord` | 5 | 563–567 | D | [Scene](benilla-m2-scene-fields.md) |
### B: `crates/benilla-formats/src/models/bounds.rs`

| Record | Members | Declaration lines | Role | Handling |
|---|---:|---|---|---|
| `M2Bounds` | 8 | 20–48 | P | [Projection details](benilla-m2-representation-fields.md), [Header/container](benilla-m2-header-fields.md), [Animation](benilla-m2-animation-fields.md) |
### B: `crates/benilla-formats/src/models/collision.rs`

| Record | Members | Declaration lines | Role | Handling |
|---|---:|---|---|---|
| `CollisionMesh` | 2 | 18–19 | P | [Scene](benilla-m2-scene-fields.md) |
### B: `crates/benilla-formats/src/models/records.rs`

| Record | Members | Declaration lines | Role | Handling |
|---|---:|---|---|---|
| `M2Light` | 11 | 13–28 | P | [Scene](benilla-m2-scene-fields.md) |
| `M2PortraitCamera` | 6 | 212–218 | P | [Scene](benilla-m2-scene-fields.md) |
| `M2PaneCamera` | 3 | 360–364 | A | [Scene](benilla-m2-scene-fields.md) |
| `M2CameraTracks` | 5 | 371–375 | A | [Scene](benilla-m2-scene-fields.md) |
### B: `crates/benilla-formats/src/particles.rs`

| Record | Members | Declaration lines | Role | Handling |
|---|---:|---|---|---|
| `CellRamp` | 4 | 47–50 | P | [Particle](benilla-m2-particle-fields.md) |
| `OverLife` | 6 | 86–96 | P | [Particle](benilla-m2-particle-fields.md) |
| `OverLifeSample` | 4 | 103–107 | H | [Particle](benilla-m2-particle-fields.md) |
| `SplineData` | 2 | 154–156 | P | [Particle](benilla-m2-particle-fields.md) |
| `ParticleEmitterDef` | 30 | 241–301 | P | [Particle](benilla-m2-particle-fields.md) |
### B: `crates/benilla-formats/src/ribbons.rs`

| Record | Members | Declaration lines | Role | Handling |
|---|---:|---|---|---|
| `RibbonEmitterDef` | 16 | 73–101 | P | [Ribbon](benilla-m2-ribbon-fields.md) |
| `RibbonVisibility` | 1 | 108–108 | P | [Ribbon](benilla-m2-ribbon-fields.md) |
### B: `crates/benilla-m2/src/model.rs`

| Record | Members | Declaration lines | Role | Handling |
|---|---:|---|---|---|
| `C3` | 3 | 12–14 | D | [Geometry/material](benilla-m2-geometry-fields.md) |
| `C2` | 2 | 19–20 | D | [Geometry/material](benilla-m2-geometry-fields.md) |
| `M2ArrayString` | 1 | 47–47 | P | [Geometry/material](benilla-m2-geometry-fields.md) |
| `M2Texture` | 4 | 52–57 | P | [Geometry/material](benilla-m2-geometry-fields.md) |
| `M2Material` | 2 | 79–80 | D | [Geometry/material](benilla-m2-geometry-fields.md) |
| `M2Bone` | 4 | 95–99 | P | [Animation](benilla-m2-animation-fields.md) |
| `M2Vertex` | 5 | 111–116 | D | [Geometry/material](benilla-m2-geometry-fields.md) |
| `M2RawData` | 3 | 121–125 | A | [Geometry/material](benilla-m2-geometry-fields.md), [Scene](benilla-m2-scene-fields.md) |
| `M2Header` | 6 | 131–136 | A | [Header/container](benilla-m2-header-fields.md), [Scene](benilla-m2-scene-fields.md) |
| `M2PlayableAnim` | 2 | 144–147 | P | [Animation](benilla-m2-animation-fields.md) |
| `M2Attachment` | 3 | 155–158 | P | [Scene](benilla-m2-scene-fields.md) |
| `M2EventMarker` | 3 | 167–170 | P | [Scene](benilla-m2-scene-fields.md) |
| `M2TextureTransform` | 3 | 176–178 | A | [Geometry/material](benilla-m2-geometry-fields.md) |
| `M2Camera` | 9 | 188–205 | D | [Scene](benilla-m2-scene-fields.md) |
| `M2Model` | 25 | 210–262 | A | [Header/container](benilla-m2-header-fields.md), [Geometry/material](benilla-m2-geometry-fields.md), [Animation](benilla-m2-animation-fields.md), [Scene](benilla-m2-scene-fields.md) |
| `M2Format` | 1 | 318–318 | A | [Header/container](benilla-m2-header-fields.md), [Geometry/material](benilla-m2-geometry-fields.md), [Animation](benilla-m2-animation-fields.md), [Scene](benilla-m2-scene-fields.md) |
### B: `crates/benilla-m2/src/skin.rs`

| Record | Members | Declaration lines | Role | Handling |
|---|---:|---|---|---|
| `SkinSection` | 3 | 12–14 | D | [Geometry/material](benilla-m2-geometry-fields.md) |
| `SkinBatch` | 10 | 20–36 | D | [Geometry/material](benilla-m2-geometry-fields.md) |
| `Skin` | 4 | 41–44 | A | [Geometry/material](benilla-m2-geometry-fields.md) |
### B: `crates/benilla-m2/src/track.rs`

| Record | Members | Declaration lines | Role | Handling |
|---|---:|---|---|---|
| `M2Track` | 4 | 12–21 | D | [Animation](benilla-m2-animation-fields.md), [Scene](benilla-m2-scene-fields.md) |
| `M2SplineKey` | 3 | 129–131 | D | [Animation](benilla-m2-animation-fields.md), [Scene](benilla-m2-scene-fields.md) |
### E: `godot/core/src/asset/m2_format/m2_anim.rs`

| Record | Members | Declaration lines | Role | Handling |
|---|---:|---|---|---|
| `M2Bone` | 6 | 53–61 | D | [Animation](benilla-m2-animation-fields.md) |
| `M2AnimSequence` | 10 | 113–127 | D | [Animation](benilla-m2-animation-fields.md) |
| `AnimTrack` | 3 | 205–208 | D | [Animation](benilla-m2-animation-fields.md) |
| `BoneAnimTracks` | 3 | 214–216 | A | [Animation](benilla-m2-animation-fields.md) |
| `TextureAnimTracks` | 3 | 221–224 | A | [Animation](benilla-m2-animation-fields.md) |
| `ColorAnimTracks` | 2 | 229–230 | A | [Animation](benilla-m2-animation-fields.md) |
### E: `godot/core/src/asset/m2_format/m2_attach.rs`

| Record | Members | Declaration lines | Role | Handling |
|---|---:|---|---|---|
| `M2Attachment` | 3 | 32–36 | P | [Scene](benilla-m2-scene-fields.md) |
### E: `godot/core/src/asset/m2_format/m2_camera.rs`

| Record | Members | Declaration lines | Role | Handling |
|---|---:|---|---|---|
| `M2CameraSnapshot` | 7 | 18–24 | P | [Scene](benilla-m2-scene-fields.md) |
### E: `godot/core/src/asset/m2_format/m2_collision.rs`

| Record | Members | Declaration lines | Role | Handling |
|---|---:|---|---|---|
| `M2CollisionMesh` | 4 | 16–20 | P | [Scene](benilla-m2-scene-fields.md) |
### E: `godot/core/src/asset/m2_format/m2_event.rs`

| Record | Members | Declaration lines | Role | Handling |
|---|---:|---|---|---|
| `M2Event` | 5 | 26–31 | D | [Scene](benilla-m2-scene-fields.md) |
### E: `godot/core/src/asset/m2_format/m2_light.rs`

| Record | Members | Declaration lines | Role | Handling |
|---|---:|---|---|---|
| `M2Light` | 10 | 34–44 | D | [Scene](benilla-m2-scene-fields.md) |
| `EvaluatedLight` | 5 | 49–53 | H | [Scene](benilla-m2-scene-fields.md) |
| `LightTrackTime` | 4 | 179–182 | H | [Scene](benilla-m2-scene-fields.md) |
### E: `godot/core/src/asset/m2_format/m2_particle.rs`

| Record | Members | Declaration lines | Role | Handling |
|---|---:|---|---|---|
| `M2ParticleMultiTexture` | 5 | 32–36 | P | [Particle](benilla-m2-particle-fields.md) |
| `M2ParticleEmitter` | 62 | 42–149 | P | [Particle](benilla-m2-particle-fields.md) |
| `EmitterHeaderCore` | 12 | 219–230 | P | [Particle](benilla-m2-particle-fields.md) |
| `EmitterTracks` | 11 | 586–598 | A | [Particle](benilla-m2-particle-fields.md) |
### E: `godot/core/src/asset/m2_format/m2_ribbon.rs`

| Record | Members | Declaration lines | Role | Handling |
|---|---:|---|---|---|
| `M2RibbonEmitter` | 19 | 28–48 | P | [Ribbon](benilla-m2-ribbon-fields.md) |
### E: `godot/core/src/asset/m2_format/m2_variation.rs`

| Record | Members | Declaration lines | Role | Handling |
|---|---:|---|---|---|
| `VariationFamily` | 4 | 16–20 | H | [Animation](benilla-m2-animation-fields.md) |
### E: `godot/core/src/asset/m2_format/parser.rs`

| Record | Members | Declaration lines | Role | Handling |
|---|---:|---|---|---|
| `M2Vertex` | 6 | 19–24 | D | [Geometry/material](benilla-m2-geometry-fields.md) |
| `M2Submesh` | 5 | 28–32 | P | [Geometry/material](benilla-m2-geometry-fields.md) |
| `RawM2Submesh` | 13 | 38–50 | D | [Geometry/material](benilla-m2-geometry-fields.md) |
| `M2Material` | 2 | 56–57 | D | [Geometry/material](benilla-m2-geometry-fields.md) |
| `M2TextureUnit` | 12 | 61–72 | P | [Geometry/material](benilla-m2-geometry-fields.md) |
| `RawM2TextureUnit` | 13 | 78–90 | D | [Geometry/material](benilla-m2-geometry-fields.md) |
| `M2TextureTypeEntry` | 4 | 96–99 | D | [Geometry/material](benilla-m2-geometry-fields.md) |
| `SkinHeader` | 11 | 105–115 | D | [Geometry/material](benilla-m2-geometry-fields.md) |
| `SkinData` | 4 | 119–122 | A | [Geometry/material](benilla-m2-geometry-fields.md) |
| `M2Chunks` | 7 | 126–134 | A | [Header/container](benilla-m2-header-fields.md) |
| `TextureTables` | 4 | 138–141 | H | [Geometry/material](benilla-m2-geometry-fields.md) |
| `SkelData` | 4 | 391–394 | A | [Animation](benilla-m2-animation-fields.md), [Header/container](benilla-m2-header-fields.md) |
### E: `godot/core/src/asset/m2_particle_defaults.rs`

| Record | Members | Declaration lines | Role | Handling |
|---|---:|---|---|---|
| `EmitterTrackDefaults` | 27 | 73–99 | H | [Particle](benilla-m2-particle-fields.md) |
| `EmitterVisualDefaults` | 14 | 137–150 | H | [Particle](benilla-m2-particle-fields.md) |
### E: `godot/core/src/m2.rs`

| Record | Members | Declaration lines | Role | Handling |
|---|---:|---|---|---|
| `Vertex` | 6 | 15–20 | P | [Geometry/material](benilla-m2-geometry-fields.md) |
| `Submesh` | 5 | 25–29 | P | [Geometry/material](benilla-m2-geometry-fields.md) |
| `Material` | 2 | 34–35 | P | [Geometry/material](benilla-m2-geometry-fields.md) |
| `Model` | 32 | 42–79 | A | [Header/container](benilla-m2-header-fields.md), [Geometry/material](benilla-m2-geometry-fields.md), [Animation](benilla-m2-animation-fields.md), [Scene](benilla-m2-scene-fields.md), [Particle](benilla-m2-particle-fields.md), [Ribbon](benilla-m2-ribbon-fields.md) |
| `AssetReferences` | 2 | 125–126 | A | [Header/container](benilla-m2-header-fields.md) |
| `AnimTime` | 4 | 282–285 | H | [Animation](benilla-m2-animation-fields.md), [Header/container](benilla-m2-header-fields.md) |
| `SubmeshArrays` | 7 | 446–452 | P | [Geometry/material](benilla-m2-geometry-fields.md) |

## Mixed and non-wire member boundaries

### E helper and aggregate members

| Representation/member | Exact role/input boundary | Source evidence |
|---|---|---|
| `VariationFamily.linked` | Number of linked candidates visited, not the current/root sequence index. | `E/godot/core/src/asset/m2_format/m2_variation.rs:14–20,85` |
| `VariationFamily.playable` | `(sequence index, weight)` pairs admitted only for positive duration and nonzero frequency; second tuple member is not duration. | Same `:79–100` |
| `VariationFamily.total` | Checked sum of admitted weights, not duration sum. | Same `:90–98` |
| `VariationFamily.shortest_ms` | Minimum admitted duration, initially `u32::MAX`; not an extra authored sequence scalar. | Same `:36,99` |
| `TextureTables.tex_lookup` | Borrowed `Model.texture_lookup`. | `E/godot/core/src/m2.rs:100–106` |
| `TextureTables.tex_types` | Borrowed `Model.texture_types`. | Same |
| `TextureTables.txid` | Borrowed `Model.texture_fdids`, originally TXID. | Same; header worksheet |
| `TextureTables.skin_fdids` | Caller-supplied **skin texture replacements**, not the model's geometry-SFID list. Similar names do not establish shared provenance. | `E/godot/core/src/m2.rs:85–107`, assignment `skin_fdids: skin_texture_fdids` |
| `SkelData.bones`, `bone_tracks` | Decoded SKB1 arrays/tracks. Neither is populated by SKS1. | `E/godot/core/src/asset/m2_format/parser.rs:519–534` |
| `SkelData.sequences`, `global_sequences` | Decoded SKS1 arrays. | Same `:508–517` |
| `AssetReferences.skin_fdids`, `skeleton_fdid` | SFID/SKID projection for dependency resolution; not replacement texture IDs. | `E/godot/core/src/m2.rs:122–135` |
| `AnimTime.sequence`, `time_ms`, `global_ms` | Caller-supplied selected index/local clock/global clock; not file input. | Same `:281–295` |
| `AnimTime.global_sequences` | Borrowed decoded durations used for global-clock wrapping. | Same `:263–295` |
| `LightTrackTime.sequence_index`, `local_ms`, `global_ms`, `global_sequences` | Arguments/borrowed table for the parser-module light helper; not serialized or proof this helper is the production light evaluator. | `E/godot/core/src/asset/m2_format/m2_light.rs:178–216`; scene worksheet distinguishes native point-light path |
| `EvaluatedLight.visible`, `color`, `intensity`, `attenuation_start`, `attenuation_end` | Computed samples/defaults/clamps from light tracks. No five new source fields. | Same `:204–237,265–281` |
| `EmitterTrackDefaults` all27 members; `EmitterVisualDefaults` all14 members | Constructor constants/empty vectors and zero-state overrides, copied into the emitter; later parser writes are separate. Census lists each name. | `E/godot/core/src/asset/m2_particle_defaults.rs:72–172,174–205,227–287` |
| `EmitterHeaderCore` all12 members | Temporary decoded prefix, copied to emitter; not another serialized prefix. | `E/godot/core/src/asset/m2_format/m2_particle.rs:471–500`; `m2_particle_defaults.rs:212–225` |
| `M2ParticleMultiTexture.texture_indices`, `texture_fdids` | Packed index decomposition and TXID resolution, respectively. | `E/godot/core/src/asset/m2_format/m2_particle.rs:669–723` |
| `M2ParticleMultiTexture.uv_scale_bytes`, `velocity_midpoints`, `velocity_ranges` | Decoded packed scale bytes/fixed-point vectors; exact serialized inputs and native motion consumers remain in particle worksheet. | Same; particle worksheet |
| `SubmeshArrays.positions`, `normals`, `uv`, `uv2`, `bones`, `weights`, `indices` | Referenced vertex deduplication/remapping, coordinate conversion, widened bone indices and normalized weights. Not seven new vertex attributes on disk. | `E/godot/core/src/m2.rs:445–495`; geometry worksheet |
| `Model.uses_texture_combiner_combos` | Boolean derived from raw global flags bit0x8; retained in batch inputs/resolved batches and consumed by sky combine selection. | `E/godot/core/src/m2.rs:114,421`; `asset/m2_batch_data.rs:233`; `E/godot/rust/src/sky_model.rs:562–576` |

### B mixed/projected members

All names remain individually enumerated in the census. These are classification exceptions, not a second wire layout.

| Representation/member | Origin/derivation | Handling/source |
|---|---|---|
| `C2.x/y`, `C3.x/y/z` | Scalar components of decoded vectors/UVs, not additional nested file records. | Geometry/scene worksheets; `B/crates/benilla-m2/src/lib.rs:23–29,162–177` |
| `M2ArrayString.string` | Filename descriptor's byte payload, truncated at first NUL and wrapped as CString; descriptor addresses are not members. | Geometry worksheet; `B/crates/benilla-m2/src/lib.rs:188–200` |
| `M2Texture.wrap_x`, `wrap_y` | Bits0/1 of one flags word; `filename` wraps the above payload, `texture_type` maps the raw type. | Geometry worksheet; same `:181–200` |
| `M2Model.pivot_attach_z` | Attachment-ID17 selection; not an extra authored header pivot. | Representation bounds table; same `:155` |
| `M2Model.raw_data`, `header`, `views`, `version` | Raw-array aggregate, bounds aggregate, embedded-view descriptor pair and decoded version, respectively. Other members collect corresponding worksheet arrays. | Header/geometry worksheets; `B/crates/benilla-m2/src/model.rs:208–263` |
| `SkeletonBone.parent`, `pivot`, `key_bone` | Copies from base bones; `billboard`, `parent_arm` derive flag policy. `Skeleton.bones` owns the resulting collection. | `B/crates/benilla-formats/src/models/anim.rs:13–69`; animation worksheet |
| `ModelAnimation.seq_index`, `duration`, `looping`, `bounds_center` | Enumeration index, `(end-start)/1000`, bit0-clear predicate and mean of box corners. `blend_time` converts milliseconds to seconds; `bones/events` are nested projections. Remaining members copy decoded sequence values. | Same `:617–696`; animation worksheet |
| `ChannelTrack.step` | `interp==0`; `gseq/ranges/keys` retain descriptor metadata/decoded payload. Not a replacement interpolation wire field. | Same `:365–481` |
| `AnimEvent.time` | Timestamp rebased to sequence-relative seconds; other members copy record payload. `EventRecord.times` is decoded timestamp data. | Same `:558–600,660–672`; scene worksheet |
| `M2Light.bone_z`, `visibility_off` | Bone/snapshot-derived fields, not additional fixed light-record words. Other fields retain fixed values or sampled channels. | Scene worksheet, `B/crates/benilla-formats/src/models/records.rs:12–40,164–202` |
| `M2PortraitCamera`, `M2PaneCamera.still`, `tracks`, `M2CameraTracks` | Snapshot and optional track aggregates projected from base camera; no new serialized camera records. | Scene worksheet; same camera adapters |
| `EmitTiming.rate`, `enabled`, `looping` | Baked per-sequence rate/enabled tracks and sequence-loop policy. | Particle worksheet; `B/crates/benilla-formats/src/particles.rs:503–530,629–641` |
| `ParamsNow` all9 members | Current sampled parameter outputs/defaults; not nine additional emitter scalars. `EmitParams.channels` owns nine baked track families. | Particle worksheet; `B/crates/benilla-formats/src/emit_timing.rs:154–267` |
| `CellRamp.begin/end`, `base/span` | Parsed endpoint pair versus calculated sampling base/span. | Particle worksheet; `B/crates/benilla-formats/src/particles.rs:39–77,595–612` |
| `OverLife.mid/color/scale/head_cells/tail_cells/repeat` | Authored midpoint, decoded packed colors/scales, endpoint-derived cell ramps and repeat values. `OverLifeSample` all4 members are evaluated outputs. | Same `:79–151,575–624`; particle worksheet |
| `SplineData.points`, `knots` | Authored control points versus calculated normalized arc-length boundaries; no authored knot array. | Same `:152–238,600–613`; particle worksheet |
| `ParticleEmitterDef.shape/blend/lit/texture/timing/params` | Enum/lighting predicates, resolved texture and cooked timing/parameter tracks. Other members retain input values or nested decoded/projection helpers as detailed in the particle worksheet. | Same `:241–395,583–698`; particle worksheet |
| `RibbonEmitterDef.texture/blend/visible`, `RibbonVisibility.by_anim` | Resolved texture, derived blend policy and visibility keys grouped by animation; other members retain fixed/sampled/track inputs from the ribbon worksheet. | `B/crates/benilla-formats/src/ribbons.rs`; ribbon worksheet |

## Residual limits

Every census record is assigned a role and worksheet; this is not a proof of universal semantic coverage. The family pages retain specific unresolved consumers and dropped/unread inputs: camera curves, raw batch metadata, some event payload paths, particle geometry/recursion dependencies in E, follow parameters and alpha cutoff, ribbon color index, version-specific opaque bytes and the shorter particle branch's decoder boundary. Unknown semantics remain unknown rather than becoming invented field names. Generic descriptors and raw intervals are accounted by family/header tables, not inferred from the630 declarations.

## Sources

- [[benilla-m2-field-census]] — authoritative per-member names/types/locations;90 source-qualified records.
- E/B production source paths and anchored ranges in the tables, at the revisions above — constructor and helper boundaries.
- Six linked handling worksheets — serialized inputs, actual production consumers and bounded unknowns; source presence is not runtime certification.

## See Also

- [[benilla-m2-representation-fields]] — individual B bounds and marker/animation projection members.
- [[benilla-asset-field-inventory]] — parent comparison and later formats.
