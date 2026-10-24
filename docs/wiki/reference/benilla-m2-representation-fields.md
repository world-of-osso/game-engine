# Benilla M2 Representation Boundaries

The [declaration census](benilla-m2-field-census.md) counts source members, not unique serialized fields. This companion separates retained source inputs from cooked copies, calculated values and helper state. Wire layouts and version-specific losses remain in the six handling pages; representation names do not establish new disk records or modern/Vanilla equivalence. Coverage reconciliation is in progress.

Snapshot: **2026-10-24**. E native source baseline `354bdb7dbf177047ed46d63168acec531f5c4426`; B `8a6c74036cfe3d00275fd75ad1aeae1f4ea7d0df`. Roots and static-only evidence restrictions follow [[benilla-asset-field-inventory]]. No client, parser, build or test was executed.

## B bounds projections: eight members

`B/crates/benilla-formats/src/models/bounds.rs` defines `M2Bounds` at lines17–48 and constructs it at134–168. These members are not eight additional serialized fields. Header inputs are in [[benilla-m2-header-fields]]; sequence boxes and indices in [[benilla-m2-animation-fields]].

| Member | Source or derivation | Located production use / boundary |
|---|---|---|
| `sphere_radius` | Copy of `model.header.bounding_sphere_radius` (`bounds.rs:159`). | No consumer established in the bounded bounds audit; not a global-unused claim. |
| `bbox_min` | Copy of header bounding minimum (`:160`). | `B/crates/benilla-app/src/entities/display.rs:362–378`: fallback pivot/box height and bake center. |
| `bbox_max` | Copy of header bounding maximum (`:161`). | Same display path. |
| `vert_max_from_origin` | Maximum parsed vertex `sqrt(x²+y²+z²)`, initial fold0 (`:139–147,162`). | No production consumer established by the bounded audit. Not another authored radius. |
| `ring_footprint` | Stand-box X/Y extents, else header extents; both zero →1.2, otherwise `sqrt(0.5*sqrt(rx²+ry²))` (`:67–116,149–157,163`). | Display `:358` selects ground-radius input. Stand resolves lookup ID0, else sequence slot0. |
| `pivot_z` | Copy of `model.pivot_attach_z` (`:164`), selected from attachment ID17 by `B/crates/benilla-m2/src/lib.rs:155`. | Display `:362–365` adds0.0972 when present; otherwise uses90% of nonnegative header-box height. Not a header pivot scalar. |
| `stand_box_z` | Stand Z extent or header Z extent, clamped nonnegative (`bounds.rs:99–116,149–153,165`). | Display `:369–371` propagates Stand-box height. |
| `swim_pivot_drop` | `max(Stand.max.z - Swim.max.z,0)`;0 when either query unavailable (`bounds.rs:123–131,166`). Stand lookup fallback as above; Swim lookup ID42. | Display `:366` propagates swim pivot drop. Not a separately authored camera field. |

## B marker and animation projections: twelve members

Here **A** is `B/crates/benilla-formats/src/models/anim.rs`. The original event/bone/track inputs remain in [[benilla-m2-scene-fields]] and [[benilla-m2-animation-fields]]. A synthesized record index is not a new bone-index field in the file.

| Member | Source or derivation | Located production use / boundary |
|---|---|---|
| `StringAnchors.top` | Last `$WTT` record encountered in the bounded44-byte event scan; tuple contains `u32(+8) as u16` bone and `f32(+12,+16,+20)` position. Both top and bottom must exist (A:127–160). | Asset conversion `B/crates/benilla-assets/src/m2.rs:660`; equipment `B/crates/benilla-app/src/entities/equipment/spawn.rs:284–296`; posed top endpoint in `bowstring.rs:50–57`. Bone narrowing loses high16 bits. |
| `StringAnchors.bottom` | Same projection for last `$WTB` (A:127–160). Not the checked-narrowing general marker API. | Same conversion/equipment chain; posed bottom endpoint `bowstring.rs:50–57`. |
| `GlobalSeqChannel.period_ms` | Global-duration entry selected by track global index; valid index and positive period required (A:700–730,735–754). | `B/crates/benilla-assets/src/model.rs:602–645` converts period/keys to seconds; `B/crates/benilla-world/src/rig_anim/global_seq.rs:109–119` wraps instance-relative clock and samples channels. |
| `GlobalSeqChannel.keys` | Timestamp/value pairs, scan order retained; channel emitted only with more than one valid key (A:714–730). | Same asset conversion and live pose sampler. A lone key is omitted from this global-channel representation, not proof it disappears from the ordinary clip path. |
| `GlobalSeqBone.bone` | Bone-record loop index `i as u16`, not an additional payload read (A:758–777). | Asset conversion indexes skeleton/rest translation; live drive maps joint or rig index (`global_seq.rs:32–59`). |
| `GlobalSeqBone.translation` | Optional channel from bone track `+0x0c`, using the above index/period/multiple-key rules (A:766–777). | Asset conversion applies coordinate conversion and rest translation; live sampler writes translation. |
| `GlobalSeqBone.rotation` | Optional channel from `+0x28`, same rules (A:766–777). | Asset conversion conjugates quaternion into target frame; live sampler writes rotation. |
| `GlobalSeqBone.scale` | Optional channel from `+0x44`, same rules (A:766–777). | Asset conversion permutes scale axes; live sampler writes scale. |
| `BoneKeys.bone` | Bone-track enumeration index cast to `u16` (A:645–657). | `m2_bone_spins` selects skeleton bone (A:230–251); ordinary clip construction consumes the sequence's bones. |
| `BoneKeys.translation` | Bone translation track selected with sequence slot/start/end; `ChannelTrack::band` rebases timestamps to seconds from start (A:365–481,602–609,645–657). | Spin classifier rejects nonempty translation (A:233); clip construction `B/crates/benilla-assets/src/model.rs:433` consumes sequence bone keys. |
| `BoneKeys.rotation` | Sequence-band projection of rotation track, same rebasing. | Spin classifier retains rotation keys (A:233–251); ordinary clip construction. |
| `BoneKeys.scale` | Sequence-band projection of scale track, same rebasing. | Spin classifier rejects nonempty scale (A:233); ordinary clip construction. |

The previous bounded report that had not located global-channel consumers is superseded here: the asset conversion and live `GlobalSeqDrive` path above are present. That is static consumer evidence, not an executed timing or pose-parity result.

## Sources

- [[benilla-m2-field-census]] — literal field names/types and source-record scope.
- B `crates/benilla-formats/src/models/{bounds,anim}.rs`, `crates/benilla-m2/src/lib.rs` — source inputs and projections at the pinned revision.
- B `crates/benilla-assets/src/{m2,model}.rs`, `crates/benilla-world/src/rig_anim/global_seq.rs`, `crates/benilla-app/src/{entities/display,entities/equipment/spawn,bowstring}.rs` — bounded production consumers, not test-only call sites.

## See Also

- [[benilla-asset-field-inventory]] — cross-format research goal and evidence vocabulary.
- [[benilla-m2-header-fields]] — serialized header/chunk inputs.
- [[benilla-m2-animation-fields]] — authored tracks and sequence boundaries.
- [[benilla-m2-scene-fields]] — source events, attachments and bounds.
- [[benilla-m2-geometry-fields]] — raw/cooked mesh representations.
- [[benilla-m2-particle-fields]] — emitter inputs/default conversions.
- [[benilla-m2-ribbon-fields]] — ribbon inputs versus strip/runtime state.
