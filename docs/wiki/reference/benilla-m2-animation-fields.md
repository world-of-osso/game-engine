# Benilla / game-engine M2 animation fields

Individual bone/sequence/track inputs and current consumers. Static local inspection, 2026-10-24; no execution, builds, tests or asset-corpus reproduction. E = `/syncthing/Sync/Projects/world-of-osso/game-engine`, native baseline `354bdb7dbf177047ed46d63168acec531f5c4426`; B = `/home/osso/Repos/benilla`, pin `8a6c74036cfe3d00275fd75ad1aeae1f4ea7d0df`. B1.12.1/build5875 and modern E layouts are separate. This page does not turn their fields into a compatibility contract.

## Bones

E parser **A** = `E/godot/core/src/asset/m2_format/m2_anim.rs`; bone stride88, scalar decode A:33–86, track decode A:405–461. B bone stride108, with 28-byte rather than20-byte tracks (`B/crates/benilla-formats/src/models/anim.rs:13–44`, `B/crates/benilla-m2/src/lib.rs` bone read). All E bytes are accounted for by the fields below; interpolation payload semantics are separately qualified.

| E field | Serialized location / retention | Production read | B counterpart / boundary |
|---|---|---|---|
| Key-bone ID | `i32 +0`, retained | `E/godot/rust/src/animation/action.rs:110`, `animation/mount_special.rs:132,146` masks. | B skeleton retains `key_bone`; adapted type differs. |
| Flags | `u32 +4`, retained | Billboard interpretation `E/godot/rust/src/animation/billboard.rs:24–26,47,83`; light handling `assets/m2_lights.rs:124`. | B derives billboard and parent-arm metadata (`models/anim.rs:52–69`), while base model retains raw flags. Individual bit-equivalence not established. |
| Parent index | `i16 +8`, retained | Skeleton hierarchy and parent-relative pivots `E/godot/rust/src/assets/mod.rs:402–405`, `animation/mod.rs:227–232`; action parent traversal. | B retains signed parent. |
| Submesh ID | `u16 +0x0a`, retained | No non-copy production read found in bounded E native Rust and core variation paths. Not a global absence proof. | B base bone metadata requires separate consumer tracing; no equivalence asserted. |
| Name CRC | `u32 +0x0c`, retained | Equipment CRC-to-joint remap `E/godot/rust/src/assets/equipment.rs:459–460,476–477`. | No matching CRC consumer established in inspected B skeleton adapter. |
| Translation track | 20-byte Vec3 track `+0x10`, retained separately in `BoneAnimTracks` | Pose sampling `E/godot/rust/src/animation/mod.rs:311–349`. | B28-byte Vec3 track `+0x0c`; sequence/global baking, `models/anim.rs:365–481,605–609`. |
| Rotation track | 20-byte compressed-quaternion track `+0x24` | Same native pose sampling; E ordinary rotation slerp. | B28-byte quaternion track `+0x28`; format adapter reads it at `models/anim.rs:608`. Do not substitute E compressed key encoding for B float quaternion. |
| Scale track | 20-byte Vec3 track `+0x38` | Same native pose sampling. | B28-byte Vec3 track `+0x44` (`models/anim.rs:609`). |
| Pivot | `3×f32 +0x4c`, retained | Skeleton construction above; attachment frame `E/godot/rust/src/assets/attachments.rs:47`, particle/ribbon positioning. | B pivot at `+0x60`; retained in `SkeletonBone`. |

The B rotation/scale/pivot offsets must follow its actual parser, not a stride subtraction rule. Bone remapping descriptions in historical wiki pages do not establish which assets require global-vs-local remap; that question remains outside this source-only inventory.

## Sequences: E64 bytes versus B68 bytes

E all named fields decode at A:128–158. E leaves opaque gaps unread; cooked sequences do not retain their bytes or assign native field names. **BC** = `B/crates/benilla-formats/src/models/anim.rs:302–338`, the cooked animation schema; B raw sequence adapter and consumer limitations are separate from field declarations.

| Field | E offset / type / retention | E production read | B counterpart / layout difference |
|---|---|---|---|
| Animation ID | `+0 u16` | Selection `E/godot/rust/src/animation/mod.rs:426,440,450,487,558,637`. | BC `anim_id +0`, sequence selection key. |
| Variation ID | `+2 u16` | Native selection above; family validation `E/godot/core/src/asset/m2_format/m2_variation.rs:31,61,86`. | No independent authored variation-ID equivalence proved by BC. |
| Duration | `+4 u32`, milliseconds | Playback/end time `E/godot/rust/src/animation/mod.rs:475,643,657,688,715`. | B derives duration from authored start/end `+4/+8`, not E duration. |
| Move speed | `+8 f32` | Current/outgoing locomotion rate `E/godot/rust/src/animation/mod.rs:593,717`. | BC `move_speed` from `+0x0c`. |
| Flags | `+0x0c u32` | Track-source selection A:393–400; external loader `E/godot/core/src/asset/m2_format/parser.rs:477–478`. | B looping metadata differs; BC `looping` is derived, not full raw flags. |
| Frequency | `+0x10 i16` | Weighted family candidates `E/godot/core/src/asset/m2_format/m2_variation.rs:85–99`. | BC `frequency u16` from `+0x14`; signedness differs. |
| Opaque span | `+0x12..0x14` | Not read by E sequence decoder. | No semantics transferred. |
| Replay minimum | `+0x14 u32`, retained in replay[0] | No production read found in bounded native/variation paths. | BC `min_replay +0x18`, retained; BC's policy comments are not runtime proof. |
| Replay maximum | `+0x18 u32`, retained in replay[1] | Same bounded no-use result. | BC `max_replay +0x1c`, retained. |
| Blend time | `+0x1c u16`, milliseconds | Movement/action crossfade `E/godot/rust/src/animation/mod.rs:577`, `animation/action.rs:173`. | BC `blend_time` from `+0x20`, seconds in cooked value. |
| Opaque span | `+0x1e..0x20` | Not read. **Not** `+0x1a..0x1c`, which would overlap replay maximum. | No semantics transferred. |
| Bounds minimum | `+0x20 Vec3`, retained | Sequence AABB culling `E/godot/rust/src/animation/mod.rs:1096` (`sequence_bounds`). | BC `bounds_min +0x24`; cooked center derived from min/max. |
| Bounds maximum | `+0x2c Vec3`, retained | Same AABB conversion. | BC `bounds_max +0x30`. |
| Opaque span | `+0x38..0x3c` | Not read. No sphere-radius semantics assigned by E reader. | B has authored sphere radius `+0x3c`, not the same bytes. |
| Variation-next index | `+0x3c i16`, retained | Family link traversal `E/godot/core/src/asset/m2_format/m2_variation.rs:65`. Candidate list, not temporal successor. | B variation-family policy not proven equivalent. |
| Opaque span | `+0x3e..0x40` | Not read. Alias meaning unassigned. | B final opaque bytes require own record audit. |

## Ordinary track descriptor and payload

Each E translation/rotation/scale track instantiates every descriptor row in [ribbon track descriptors](benilla-m2-ribbon-fields.md#track-descriptors-six-instances-not-six-additional-record-types). Exact decoder A:295–382: outer timestamp/value counts are paired by minimum, inner descriptors remain in model/skeleton data, payload elements can come from external files. Decoded descriptor addresses are discarded; interpolation/global index and per-sequence timestamp/value vectors remain.

| Input / policy | E handling | B handling / evidence |
|---|---|---|
| Interpolation type | Retained, but ordinary Vec3/f32 always linear, quaternion slerp, integer types step (A:579–689). Native bones invoke ordinary evaluators. | `ChannelTrack.step = interp==0`, all other values follow linear bake (`B/crates/benilla-formats/src/models/anim.rs:365–481`). Not general bone cubic support. |
| Global-sequence index | Retained; `AnimTime::track_time` wraps global clock using duration (`E/godot/core/src/m2.rs:263–276,288–294`). | B retains global period and keys; own extraction differs. |
| Timestamp payload | `u32` milliseconds, per-sequence vectors | B flat absolute timeline plus range windows, rebased to sequence bands. |
| Quaternion payload | E bone values compressed signed16×4; texture rotation floats×4 | B bone float quaternion reads must remain version-qualified. |
| Material interpolation | Separate evaluator: type1 linear, others step; quaternion normalized component lerp (`E/godot/core/src/m2_material.rs:296–325,350–365`). | Do not extrapolate B camera sampler to ordinary material or bone channels. |

B 28-byte `ChannelTrack` descriptor fields (`B/crates/benilla-formats/src/models/anim.rs:365–428`) are individually: interpolation `u16 +0`; global-sequence `u16 +2`; range count `u32 +4`; range offset `u32 +8`; timestamp count `u32 +0x0c`; timestamp offset `u32 +0x10`; value count slot `u32 +0x14`; value offset `u32 +0x18`. This particular adapter iterates timestamp count and bounds-checks payload rather than reading the value-count slot. Range elements each contain `lo:u32` and `hi:u32`; retained as windows. Addresses are not retained. These are not modern E missing fields: E uses nested per-sequence arrays instead of B ranges. Base B spline-track readers have their own descriptor behavior, not automatically this adapter's behavior.

## Global sequences and sidecars

| Input | Decode / retention | Downstream boundary |
|---|---|---|
| Global-duration element | `u32[]`, A:182–198 | Wrapped clock via `AnimTime`; not missing global-track support. |
| AFID animation ID | `u16 entry+0`, parser.rs:453–463 | Matches sequence.id. |
| AFID variation | `u16 entry+2`, same parser | Matches sequence.variation_id. |
| AFID FDID | `u32 entry+4`, same parser | Nonzero external dependency; `parser.rs:465–489`. |
| Sequence flag0x20 | Retained raw flag, A:234–236 | In-file payload source. |
| Sequence flag0x40 | Retained raw flag, A:237–239 | Alias path selects in-file payload; this is not proof of full alias-index semantics. |
| SKS1 global count | `u32 +0`, parser.rs:507–517 | Parses durations into skeleton result. |
| SKS1 global offset | `u32 +4`, same parser | Address consumed then discarded. |
| SKS1 sequence count | `u32 +8`, same parser | Parses64-byte sequence records. |
| SKS1 sequence offset | `u32 +12`, same parser | Address consumed then discarded. |
| SKB1 bone descriptor | Parser.rs:520–531; shared bone/track schema | Skeleton bone arrays replace MD20 bones, sequences and clocks (`E/godot/core/src/m2.rs:331–360`). Full SKB1 reserved-byte identity is not inferred here. |
| `.anim` chunk tag | Four bytes, parser.rs:492–505 | Requested AFSB for skeleton bones or AFM2 for model tracks; other chunks skipped. |
| `.anim` chunk size | `u32 +4`, same parser | Advances payload; selected chunk end clamped to file. Chunkless files are treated as own payload. |
| Missing external file | `SequenceData::Missing`, A:315–323 | Empty sequence keys, not a validated animation. |
| External track overrun | A:333–343 | Logs error and retains empty keys for that sequence/track; other tracks continue. In-file errors propagate. |

Array/header slots not consumed by these readers remain in the header inventory; absence of an E cooked `AnimationLookup` member does not justify importing B's pre-Wrath PlayableAnimationLookup at E bone-header offsets. Native flags, all B reserved bytes and every sidecar variant are not exhaustively classified by this slice.

## Sources

- E animation parser, variation, native animation and equipment paths identified above.
- B `crates/benilla-m2/src/{lib,model,track}.rs`, `crates/benilla-formats/src/models/anim.rs` — separate base and cooked schemas.
- [M2 format](../formats/m2-format.md) — format SSOT; historical runtime/fixture evidence is not rerun here.

## See Also

- [[benilla-m2-field-census]] — declaration scope including derived duplicates.
- [[benilla-m2-ribbon-fields]] — expanded ordinary track members.
- [[benilla-m2-particle-fields]] — conditional particle track-retention boundary.
- [[benilla-asset-field-inventory]] — broader inventory and proof limits.
