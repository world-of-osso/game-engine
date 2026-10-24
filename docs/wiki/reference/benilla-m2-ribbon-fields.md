# Benilla / game-engine M2 ribbon fields

Individual serialized inputs and their current handling; static source evidence only. This is the ribbon slice of the [M2 declaration census](benilla-m2-field-census.md), not completion of the M2 or cross-format inventory. E = `/syncthing/Sync/Projects/world-of-osso/game-engine`, native source baseline `354bdb7dbf177047ed46d63168acec531f5c4426`; B = `/home/osso/Repos/benilla`, revision `8a6c74036cfe3d00275fd75ad1aeae1f4ea7d0df`. Inspected 2026-10-24. No code, client, imports, builds or tests executed.

## Layout and evidence boundary

E reads a 176-byte record through MD20 array header `0x120`; B reads a 220-byte record through header `0x134`. B targets 1.12.1/build 5875. The tables compare meanings of inputs, **not interchangeable offsets or native-client equivalence**. Source comments naming native addresses are B's claims, not authenticated original-client observations.

E parser reference **EP** = `E/godot/core/src/asset/m2_format/m2_ribbon.rs:7–24,77–103`. B parser reference **BP** = `B/crates/benilla-formats/src/ribbons.rs:210–284`. Every offset below is relative to its own record. E's internal constant `ANGLE_OFFSET` supplies the stored `gravity` member: do not label this an implemented angular parameter. No uncovered bytes remain in E's 176-byte record after expanding the descriptors below. B has two bytes at `+0x06..0x08` not read by BP; its `+0x00..0x04` is also unread there. Those are opaque in this comparison, not assumed E-equivalent fields.

## Record fields

| Field | E serialized input and retention | E production consumer | B input / handling (BP unless stated) |
|---|---|---|---|
| Identifier | `u32 +0x00`, retained (EP) | Error context only in inspected ribbon paths: `E/godot/core/src/m2_ribbons.rs:107–116`, `E/godot/rust/src/ribbons.rs:190–205`; not a demonstrated effect selector. | `+0x00..0x04` unread; no identifier in `RibbonEmitterDef` (`B/crates/benilla-formats/src/ribbons.rs:70–106`). |
| Bone index | `u32 +0x04`, retained (EP) | Range validation and skeleton pose: `E/godot/rust/src/ribbons.rs:166–195`. | `u16 +0x04`, retained `bone`; upper two bytes not interpreted. |
| Position | `3×f32 +0x08`, retained (EP) | Bone/pivot/root transform: `E/godot/rust/src/ribbons.rs:166–186`. | `3×f32 +0x08`, retained; world trail stores bone-relative offset (`B/crates/benilla-world/src/ribbons.rs:49–54`). |
| Texture-reference count | `u32 +0x14`, consumed into vector length (EP). | All paired references loaded: `E/godot/rust/src/ribbons.rs:138–154,190–205`. | `u32 +0x14`; only checks nonzero, resolves first element. |
| Texture-reference offset | `u32 +0x18`, used then discarded (EP). | Address is not runtime state. | `u32 +0x18`; first `u16` becomes resolved texture path. |
| Texture-reference element | `u16[]`, all retained (EP). | Texture FDID/image load: `E/godot/rust/src/ribbons.rs:138–154,243–261`. | Remaining elements not retained by BP. |
| Material-reference count | `u32 +0x1c`, consumed into vector length (EP). | Must equal texture-reference count and be nonempty: `E/godot/rust/src/ribbons.rs:190–205`. | `u32 +0x1c`; only first material selected. |
| Material-reference offset | `u32 +0x20`, used then discarded (EP). | Address is not runtime state. | `u32 +0x20`; follows first `u16`. |
| Material-reference element | `u16[]`, all retained (EP). | One material pass per texture/material pair; material blend/flags resolved: `E/godot/rust/src/ribbons.rs:138–154,243–279`. | First material's blend retained both raw and folded into `ParticleBlend`; material flags not copied by BP. Modes outside 1–4 fold to opaque; not a model for E's complete material policy. |
| Color track | Vec3 track `+0x24`, retained (EP). | RGB sampling: `E/godot/core/src/m2_ribbons.rs:45–64`. | 28-byte track `+0x24`; adapted to `ValueTrack`, rebased on first sequence band. |
| Alpha track | Signed-fixed16 track `+0x38`, retained (EP). | Divide by 32767 and clamp: `E/godot/core/src/m2_ribbons.rs:48–50`. | 28-byte track `+0x40`, signed `/32767`. |
| Height-above track | Float track `+0x4c`, retained (EP). | Sampled into strip width: `E/godot/core/src/m2_ribbons.rs:51–64`. | 28-byte track `+0x5c`, retained as `ValueTrack`. |
| Height-below track | Float track `+0x60`, retained (EP). | Same width sampling, independent value: `E/godot/core/src/m2_ribbons.rs:51–65`. | 28-byte track `+0x78`, retained as `ValueTrack`. |
| Edge density | `f32 +0x74`, retained (EP). | Positive/finite validation; `ceil` then reciprocal emission interval: `E/godot/core/src/m2_ribbons.rs:107–128`. | `f32 +0x94`; retained. |
| Edge lifetime | `f32 +0x78`, retained (EP). | Positive/finite validation; simulation minimum 0.25 seconds: `E/godot/core/src/m2_ribbons.rs:107–128`. | `f32 +0x98`, clamped to at least 0.25 during parse. |
| Gravity | `f32 +0x7c`, retained (EP). | World-Y displacement `gravity × age²`: `E/godot/core/src/m2_ribbons.rs:227–236`. | `f32 +0x9c`; incremental age-squared displacement: `B/crates/benilla-world/src/ribbons.rs:42–46`. Coordinate bases differ. |
| Texture rows | `u16 +0x80`, retained (EP). | Rejects zero; passed as simulation `cols`, following explicit source-order comment: `E/godot/core/src/m2_ribbons.rs:115–129`. | `u16 +0xa0`, forced to at least one. |
| Texture columns | `u16 +0x82`, retained (EP). | Rejects zero; passed as simulation `rows`: same citation. Do not silently normalize this transpose away. | `u16 +0xa2`, forced to at least one. |
| Texture-slot track | `u16` track `+0x84`, retained (EP). | Stepped sampling and atlas-range validation: `E/godot/core/src/m2_ribbons.rs:55–56,137–151`. | 28-byte track `+0xa4`; only first value retained as constant. |
| Visibility track | Byte track `+0x98`, retained (EP). | Stepped sample, missing value defaults on; disabled emission resets continuity: `E/godot/core/src/m2_ribbons.rs:57–58,156–161`. | 28-byte track `+0xc0`; per-animation step keys. Global-sequence visibility treated always on; duplicate variation IDs keep first (`B/crates/benilla-formats/src/ribbons.rs:136–188`). |
| Priority plane | `i16 +0xac`, retained (EP). | Native render priority; rejects outside -128..127: `E/godot/rust/src/ribbons.rs:138–149,264–279`. | BP does not read matching priority metadata; no layout-equivalence claim. |
| Color index | `i8 +0xae`, retained (EP). | No reference found in bounded `E/godot/core/src/m2_ribbons.rs` and `E/godot/rust/src/ribbons.rs` searches. Separate from sampled color track; global unused status unproven. | No counterpart modeled by BP. |
| Texture-transform lookup base | `i8 +0xaf`, retained (EP). | Model flag `0x20000` gates base-plus-pass lookup; native applies animated UV matrix/offset: `E/godot/core/src/m2_ribbons.rs:11–32`, `E/godot/rust/src/ribbons.rs:138–149,223–240`. Negative base means no transform. | No counterpart modeled by BP. |

## Track descriptors: six instances, not six additional record types

Apply each row separately to color, alpha, height-above, height-below, texture-slot and visibility at the E offsets above. All six are 20-byte modern tracks; B's 28-byte tracks have a different flattened/range layout. Parsing is not sufficient proof of authored interpolation support.

| E descriptor member | Relative offset / type | Handling and evidence |
|---|---|---|
| Interpolation type | `+0 u16` | Retained by ordinary track parser. Ribbon vec3/float evaluators interpolate linearly and integer evaluators step rather than dispatch on this field: `E/godot/core/src/asset/m2_format/m2_anim.rs:579–672`; called by ribbon `sample` above. |
| Global-sequence index | `+2 i16` | Retained; `AnimTime::track_time` chooses sequence/time per track: `E/godot/core/src/m2.rs:263–275`; ribbon sampling calls it individually (`m2_ribbons.rs:45–58`). |
| Timestamp outer-array count | `+4 u32` | Sequence-array cardinality consumed during parse. See `E/godot/core/src/asset/m2_format/m2_anim.rs:295–370` track parsing; no raw descriptor runtime member. |
| Timestamp outer-array offset | `+8 u32` | Locates inner descriptors; discarded after decoding. Same parser. |
| Value outer-array count | `+12 u32` | Consumed during parse; values retained as sequence vectors. Same parser. |
| Value outer-array offset | `+16 u32` | Address consumed and discarded. Same parser. |
| Inner timestamp count | Inner descriptor `+0 u32` | Consumed; timestamp payload retained. Same parser. |
| Inner timestamp offset | Inner descriptor `+4 u32` | Address consumed and discarded. Same parser. |
| Inner value count | Inner descriptor `+0 u32` | Consumed; typed payload retained. Same parser. |
| Inner value offset | Inner descriptor `+4 u32` | Address consumed and discarded. Same parser. |
| Timestamp element | `u32` milliseconds | Retained and used for sampling; global clock routing is present, not missing. |
| Value element | Vec3 / i16 / f32 / f32 / u16 / u8 respectively | Sampled as described in field rows. Fixed16 conversion belongs to alpha interpretation, not its address descriptor. |

No execution or shipped-asset reproduction establishes these static findings. Ring edges, sample values, UV matrices, time accumulators, resolved paths and folded blend enums are **derived/runtime representations**, not additional serialized ribbon fields. B's own documented differences in visibility clock and first-sequence sampling remain limitations of that reference.

## Sources

- E parser: `godot/core/src/asset/m2_format/m2_ribbon.rs`; ordinary tracks: `godot/core/src/asset/m2_format/m2_anim.rs`.
- E production: `godot/core/src/m2_ribbons.rs`, `godot/core/src/m2.rs`, `godot/rust/src/ribbons.rs`.
- B parser/adaptation: `crates/benilla-formats/src/ribbons.rs`; production strip state and gravity: `crates/benilla-world/src/ribbons.rs`.
- [M2 format](../formats/m2-format.md) — project format SSOT and separate existing ribbon fixture history; those tests were not rerun here.

## See Also

- [[benilla-m2-field-census]] — named declaration scope, including derived duplicates.
- [[benilla-asset-field-inventory]] — cross-format research scope and version caveats.
- [[m2-format]] — existing modern format documentation.
