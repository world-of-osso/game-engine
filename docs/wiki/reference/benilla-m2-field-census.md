# Benilla M2 Declared-Field Census

**Working census; not final consumer audit.** One row per named field declaration in the reader/model source scope below. Raw, decoded, duplicate and derived representations are intentionally visible. These are not canonical on-disk field counts and not proof that every declared field is parsed or consumed. Header walks, skipped byte ranges and descriptor expansion require the companion field-handling inventory.

Snapshot: 2026-10-24. E source baseline `354bdb7dbf177047ed46d63168acec531f5c4426`, unchanged in native paths through `9b8d42e683621de4e71cbca6fe58585b6b661b7c`; B `8a6c74036cfe3d00275fd75ad1aeae1f4ea7d0df`. E root: `/syncthing/Sync/Projects/world-of-osso/game-engine`; B root: `/home/osso/Repos/benilla`.

Method: static text enumeration of named struct fields; no reader, client, build or test executed. Generic track payloads and M2Array count/offset members are not expanded by declaration extraction. Enum variants, constants and implicit fields read directly from byte offsets are not counted here. B WmoLight is excluded as a separate format. Helpers sharing selected files are representations, not new serialized fields.

[Representation boundaries](benilla-m2-representation-fields.md) classify copied, derived and helper members separately from serialized inputs; reconciliation remains in progress.

## Inventory scope

Declared representation fields: E **362**, B **268**; **90** file-qualified struct representations. Names/types are literal source representation, not cross-version equivalence. No missing/unsupported claim follows solely from this table.

Reader modules: E `godot/core/src/asset/m2_format/*.rs`, model `godot/core/src/m2.rs`, default-conversion `godot/core/src/asset/m2_particle_defaults.rs`; B `crates/benilla-m2/src/*.rs`, format adapters `models/{anim,records,bounds,collision}.rs`, `particles.rs`, `ribbons.rs`, `emit_timing.rs`. Header scalar/array walks, offset reads, named flag constants and opaque spans need the field-handling worksheet.

## B: EmitTiming

Source: `B/crates/benilla-formats/src/emit_timing.rs`. Declared fields: 3.

| Field | Literal type | Source line |
|---|---|---:|
| `rate` | `Vec<Option<ScalarAnim>>` | 17 |
| `enabled` | `Vec<Option<ScalarAnim>>` | 20 |
| `looping` | `Vec<bool>` | 22 |

## B: ParamsNow

Source: `B/crates/benilla-formats/src/emit_timing.rs`. Declared fields: 9.

| Field | Literal type | Source line |
|---|---|---:|
| `emission_speed` | `f32` | 154 |
| `speed_variation` | `f32` | 156 |
| `vertical_range` | `f32` | 158 |
| `horizontal_range` | `f32` | 160 |
| `gravity` | `f32` | 162 |
| `lifespan` | `f32` | 165 |
| `area_length` | `f32` | 167 |
| `area_width` | `f32` | 169 |
| `z_source` | `f32` | 171 |

## B: EmitParams

Source: `B/crates/benilla-formats/src/emit_timing.rs`. Declared fields: 1.

| Field | Literal type | Source line |
|---|---|---:|
| `channels` | `[Vec<Option<ScalarAnim>>; 9]` | 196 |

## B: SkeletonBone

Source: `B/crates/benilla-formats/src/models/anim.rs`. Declared fields: 5.

| Field | Literal type | Source line |
|---|---|---:|
| `parent` | `i16` | 17 |
| `pivot` | `[f32; 3]` | 18 |
| `key_bone` | `i16` | 21 |
| `billboard` | `Option<crate::BillboardKind>` | 23 |
| `parent_arm` | `Option<crate::ParentArm>` | 25 |

## B: Skeleton

Source: `B/crates/benilla-formats/src/models/anim.rs`. Declared fields: 1.

| Field | Literal type | Source line |
|---|---|---:|
| `bones` | `Vec<SkeletonBone>` | 31 |

## B: M2Attachment

Source: `B/crates/benilla-formats/src/models/anim.rs`. Declared fields: 3.

| Field | Literal type | Source line |
|---|---|---:|
| `id` | `u16` | 74 |
| `bone` | `u16` | 75 |
| `position` | `[f32; 3]` | 76 |

## B: EventMarker

Source: `B/crates/benilla-formats/src/models/anim.rs`. Declared fields: 3.

| Field | Literal type | Source line |
|---|---|---:|
| `ident` | `[u8; 4]` | 103 |
| `bone` | `u16` | 104 |
| `position` | `[f32; 3]` | 105 |

## B: StringAnchors

Source: `B/crates/benilla-formats/src/models/anim.rs`. Declared fields: 2.

| Field | Literal type | Source line |
|---|---|---:|
| `top` | `(u16, [f32; 3])` | 128 |
| `bottom` | `(u16, [f32; 3])` | 129 |

## B: PlayableAnim

Source: `B/crates/benilla-formats/src/models/anim.rs`. Declared fields: 2.

| Field | Literal type | Source line |
|---|---|---:|
| `resolved_id` | `u16` | 190 |
| `dir_flags` | `u16` | 192 |

## B: BoneKeys

Source: `B/crates/benilla-formats/src/models/anim.rs`. Declared fields: 4.

| Field | Literal type | Source line |
|---|---|---:|
| `bone` | `u16` | 262 |
| `translation` | `Vec<(f32, [f32; 3])>` | 263 |
| `rotation` | `Vec<(f32, [f32; 4])>` | 264 |
| `scale` | `Vec<(f32, [f32; 3])>` | 265 |

## B: GlobalSeqChannel

Source: `B/crates/benilla-formats/src/models/anim.rs`. Declared fields: 2.

| Field | Literal type | Source line |
|---|---|---:|
| `period_ms` | `u32` | 272 |
| `keys` | `Vec<(u32, T)>` | 273 |

## B: GlobalSeqBone

Source: `B/crates/benilla-formats/src/models/anim.rs`. Declared fields: 4.

| Field | Literal type | Source line |
|---|---|---:|
| `bone` | `u16` | 279 |
| `translation` | `Option<GlobalSeqChannel<[f32; 3]>>` | 280 |
| `rotation` | `Option<GlobalSeqChannel<[f32; 4]>>` | 281 |
| `scale` | `Option<GlobalSeqChannel<[f32; 3]>>` | 282 |

## B: AnimEvent

Source: `B/crates/benilla-formats/src/models/anim.rs`. Declared fields: 5.

| Field | Literal type | Source line |
|---|---|---:|
| `time` | `f32` | 289 |
| `ident` | `[u8; 4]` | 291 |
| `data` | `u32` | 293 |
| `bone` | `u16` | 296 |
| `position` | `[f32; 3]` | 299 |

## B: ModelAnimation

Source: `B/crates/benilla-formats/src/models/anim.rs`. Declared fields: 17.

| Field | Literal type | Source line |
|---|---|---:|
| `anim_id` | `u16` | 307 |
| `seq_index` | `usize` | 309 |
| `start_ms` | `u32` | 311 |
| `end_ms` | `u32` | 312 |
| `duration` | `f32` | 313 |
| `looping` | `bool` | 314 |
| `move_speed` | `f32` | 317 |
| `blend_time` | `f32` | 319 |
| `bounds_center` | `[f32; 3]` | 322 |
| `bounds_radius` | `f32` | 324 |
| `bounds_min` | `[f32; 3]` | 327 |
| `bounds_max` | `[f32; 3]` | 328 |
| `frequency` | `u16` | 330 |
| `min_replay` | `u32` | 334 |
| `max_replay` | `u32` | 335 |
| `bones` | `Vec<BoneKeys>` | 336 |
| `events` | `Vec<AnimEvent>` | 338 |

## B: ChannelTrack

Source: `B/crates/benilla-formats/src/models/anim.rs`. Declared fields: 4.

| Field | Literal type | Source line |
|---|---|---:|
| `step` | `bool` | 367 |
| `gseq` | `u16` | 369 |
| `ranges` | `Vec<(u32, u32)>` | 372 |
| `keys` | `Vec<(u32, T)>` | 374 |

## B: EventRecord

Source: `B/crates/benilla-formats/src/models/anim.rs`. Declared fields: 5.

| Field | Literal type | Source line |
|---|---|---:|
| `ident` | `[u8; 4]` | 563 |
| `data` | `u32` | 564 |
| `bone` | `u16` | 565 |
| `position` | `[f32; 3]` | 566 |
| `times` | `Vec<u32>` | 567 |

## B: M2Bounds

Source: `B/crates/benilla-formats/src/models/bounds.rs`. Declared fields: 8.

| Field | Literal type | Source line |
|---|---|---:|
| `sphere_radius` | `f32` | 20 |
| `bbox_min` | `[f32; 3]` | 22 |
| `bbox_max` | `[f32; 3]` | 23 |
| `vert_max_from_origin` | `f32` | 25 |
| `ring_footprint` | `f32` | 30 |
| `pivot_z` | `Option<f32>` | 33 |
| `stand_box_z` | `f32` | 38 |
| `swim_pivot_drop` | `f32` | 48 |

## B: CollisionMesh

Source: `B/crates/benilla-formats/src/models/collision.rs`. Declared fields: 2.

| Field | Literal type | Source line |
|---|---|---:|
| `positions` | `Vec<[f32; 3]>` | 18 |
| `indices` | `Vec<u32>` | 19 |

## B: M2Light

Source: `B/crates/benilla-formats/src/models/records.rs`. Declared fields: 11.

| Field | Literal type | Source line |
|---|---|---:|
| `light_type` | `u16` | 13 |
| `bone` | `i16` | 14 |
| `position` | `[f32; 3]` | 15 |
| `ambient_color` | `[f32; 3]` | 16 |
| `ambient_intensity` | `f32` | 17 |
| `diffuse_color` | `[f32; 3]` | 18 |
| `diffuse_intensity` | `f32` | 19 |
| `attenuation_start` | `f32` | 20 |
| `attenuation_end` | `f32` | 21 |
| `bone_z` | `[f32; 3]` | 24 |
| `visibility_off` | `bool` | 28 |

## B: M2PortraitCamera

Source: `B/crates/benilla-formats/src/models/records.rs`. Declared fields: 6.

| Field | Literal type | Source line |
|---|---|---:|
| `fov` | `f32` | 212 |
| `far_clip` | `f32` | 213 |
| `near_clip` | `f32` | 214 |
| `position` | `[f32; 3]` | 216 |
| `target` | `[f32; 3]` | 217 |
| `roll` | `f32` | 218 |

## B: M2PaneCamera

Source: `B/crates/benilla-formats/src/models/records.rs`. Declared fields: 3.

| Field | Literal type | Source line |
|---|---|---:|
| `camera_type` | `i32` | 360 |
| `still` | `M2PortraitCamera` | 362 |
| `tracks` | `Option<Box<M2CameraTracks>>` | 364 |

## B: M2CameraTracks

Source: `B/crates/benilla-formats/src/models/records.rs`. Declared fields: 5.

| Field | Literal type | Source line |
|---|---|---:|
| `positions` | `benilla_m2::M2Vec3SplineTrack` | 371 |
| `position_base` | `[f32; 3]` | 372 |
| `target` | `benilla_m2::M2Vec3SplineTrack` | 373 |
| `target_base` | `[f32; 3]` | 374 |
| `roll` | `benilla_m2::M2ScalarSplineTrack` | 375 |

## B: CellRamp

Source: `B/crates/benilla-formats/src/particles.rs`. Declared fields: 4.

| Field | Literal type | Source line |
|---|---|---:|
| `begin` | `u16` | 47 |
| `end` | `u16` | 48 |
| `base` | `i32` | 49 |
| `span` | `i32` | 50 |

## B: OverLife

Source: `B/crates/benilla-formats/src/particles.rs`. Declared fields: 6.

| Field | Literal type | Source line |
|---|---|---:|
| `mid` | `f32` | 86 |
| `color` | `[[f32; 4]; 3]` | 88 |
| `scale` | `[f32; 3]` | 90 |
| `head_cells` | `[CellRamp; 2]` | 92 |
| `tail_cells` | `[CellRamp; 2]` | 94 |
| `repeat` | `[f32; 2]` | 96 |

## B: OverLifeSample

Source: `B/crates/benilla-formats/src/particles.rs`. Declared fields: 4.

| Field | Literal type | Source line |
|---|---|---:|
| `color` | `[f32; 4]` | 103 |
| `size` | `f32` | 105 |
| `head_cell` | `u16` | 106 |
| `tail_cell` | `u16` | 107 |

## B: SplineData

Source: `B/crates/benilla-formats/src/particles.rs`. Declared fields: 2.

| Field | Literal type | Source line |
|---|---|---:|
| `points` | `Vec<[f32; 3]>` | 154 |
| `knots` | `Vec<f32>` | 156 |

## B: ParticleEmitterDef

Source: `B/crates/benilla-formats/src/particles.rs`. Declared fields: 30.

| Field | Literal type | Source line |
|---|---|---:|
| `flags` | `u32` | 241 |
| `position` | `[f32; 3]` | 242 |
| `bone` | `u16` | 243 |
| `shape` | `ParticleShape` | 244 |
| `blend` | `ParticleBlend` | 245 |
| `lit` | `bool` | 247 |
| `geometry_model` | `Option<String>` | 249 |
| `recursion_model` | `Option<String>` | 252 |
| `texture` | `Option<String>` | 254 |
| `tile_rows` | `u16` | 257 |
| `tile_cols` | `u16` | 258 |
| `head_tail` | `u8` | 260 |
| `timing` | `EmitTiming` | 262 |
| `params` | `EmitParams` | 265 |
| `drag` | `f32` | 268 |
| `tail_time` | `f32` | 271 |
| `angular_velocity_min` | `[f32; 3]` | 274 |
| `angular_velocity_max` | `[f32; 3]` | 275 |
| `inherit_scale` | `f32` | 278 |
| `follow_speed1` | `f32` | 280 |
| `follow_scale1` | `f32` | 281 |
| `follow_speed2` | `f32` | 282 |
| `follow_scale2` | `f32` | 283 |
| `twinkle_speed` | `f32` | 287 |
| `twinkle_percent` | `f32` | 289 |
| `twinkle_min` | `f32` | 290 |
| `twinkle_max` | `f32` | 291 |
| `spline` | `Option<SplineData>` | 297 |
| `spin` | `f32` | 300 |
| `over_life` | `OverLife` | 301 |

## B: RibbonEmitterDef

Source: `B/crates/benilla-formats/src/ribbons.rs`. Declared fields: 16.

| Field | Literal type | Source line |
|---|---|---:|
| `bone` | `u16` | 73 |
| `position` | `[f32; 3]` | 75 |
| `texture` | `Option<String>` | 77 |
| `blend` | `ParticleBlend` | 79 |
| `blend_mode` | `Option<u16>` | 81 |
| `color` | `ValueTrack<[f32; 3]>` | 83 |
| `alpha` | `ValueTrack` | 85 |
| `height_above` | `ValueTrack` | 87 |
| `height_below` | `ValueTrack` | 88 |
| `edges_per_second` | `f32` | 90 |
| `edge_lifetime` | `f32` | 92 |
| `gravity` | `f32` | 94 |
| `tile_rows` | `u16` | 96 |
| `tile_cols` | `u16` | 97 |
| `tex_slot` | `u16` | 98 |
| `visible` | `Option<RibbonVisibility>` | 101 |

## B: RibbonVisibility

Source: `B/crates/benilla-formats/src/ribbons.rs`. Declared fields: 1.

| Field | Literal type | Source line |
|---|---|---:|
| `by_anim` | `std::collections::HashMap<u16, Vec<(f32, bool)>>` | 108 |

## B: C3

Source: `B/crates/benilla-m2/src/model.rs`. Declared fields: 3.

| Field | Literal type | Source line |
|---|---|---:|
| `x` | `f32` | 12 |
| `y` | `f32` | 13 |
| `z` | `f32` | 14 |

## B: C2

Source: `B/crates/benilla-m2/src/model.rs`. Declared fields: 2.

| Field | Literal type | Source line |
|---|---|---:|
| `x` | `f32` | 19 |
| `y` | `f32` | 20 |

## B: M2ArrayString

Source: `B/crates/benilla-m2/src/model.rs`. Declared fields: 1.

| Field | Literal type | Source line |
|---|---|---:|
| `string` | `CString` | 47 |

## B: M2Texture

Source: `B/crates/benilla-m2/src/model.rs`. Declared fields: 4.

| Field | Literal type | Source line |
|---|---|---:|
| `texture_type` | `M2TextureType` | 52 |
| `wrap_x` | `bool` | 54 |
| `wrap_y` | `bool` | 56 |
| `filename` | `M2ArrayString` | 57 |

## B: M2Material

Source: `B/crates/benilla-m2/src/model.rs`. Declared fields: 2.

| Field | Literal type | Source line |
|---|---|---:|
| `flags` | `M2RenderFlags` | 79 |
| `blend_mode` | `M2BlendMode` | 80 |

## B: M2Bone

Source: `B/crates/benilla-m2/src/model.rs`. Declared fields: 4.

| Field | Literal type | Source line |
|---|---|---:|
| `key_bone` | `i16` | 95 |
| `flags` | `M2BoneFlags` | 96 |
| `parent` | `i16` | 98 |
| `pivot` | `C3` | 99 |

## B: M2Vertex

Source: `B/crates/benilla-m2/src/model.rs`. Declared fields: 5.

| Field | Literal type | Source line |
|---|---|---:|
| `position` | `C3` | 111 |
| `bone_weights` | `[u8; 4]` | 113 |
| `bone_indices` | `[u8; 4]` | 114 |
| `normal` | `C3` | 115 |
| `tex_coords` | `C2` | 116 |

## B: M2RawData

Source: `B/crates/benilla-m2/src/model.rs`. Declared fields: 3.

| Field | Literal type | Source line |
|---|---|---:|
| `texture_lookup_table` | `Vec<u16>` | 121 |
| `bounding_triangles` | `Vec<u8>` | 123 |
| `bounding_vertices` | `Vec<u8>` | 125 |

## B: M2Header

Source: `B/crates/benilla-m2/src/model.rs`. Declared fields: 6.

| Field | Literal type | Source line |
|---|---|---:|
| `bounding_box_min` | `[f32; 3]` | 131 |
| `bounding_box_max` | `[f32; 3]` | 132 |
| `bounding_sphere_radius` | `f32` | 133 |
| `collision_box_min` | `[f32; 3]` | 134 |
| `collision_box_max` | `[f32; 3]` | 135 |
| `collision_sphere_radius` | `f32` | 136 |

## B: M2PlayableAnim

Source: `B/crates/benilla-m2/src/model.rs`. Declared fields: 2.

| Field | Literal type | Source line |
|---|---|---:|
| `resolved_id` | `u16` | 144 |
| `dir_flags` | `u16` | 147 |

## B: M2Attachment

Source: `B/crates/benilla-m2/src/model.rs`. Declared fields: 3.

| Field | Literal type | Source line |
|---|---|---:|
| `id` | `u16` | 155 |
| `bone` | `u16` | 156 |
| `position` | `[f32; 3]` | 158 |

## B: M2EventMarker

Source: `B/crates/benilla-m2/src/model.rs`. Declared fields: 3.

| Field | Literal type | Source line |
|---|---|---:|
| `ident` | `[u8; 4]` | 167 |
| `bone` | `u16` | 168 |
| `position` | `[f32; 3]` | 170 |

## B: M2TextureTransform

Source: `B/crates/benilla-m2/src/model.rs`. Declared fields: 3.

| Field | Literal type | Source line |
|---|---|---:|
| `translation` | `M2Vec3Track` | 176 |
| `rotation` | `M2QuatTrack` | 177 |
| `scaling` | `M2Vec3Track` | 178 |

## B: M2Camera

Source: `B/crates/benilla-m2/src/model.rs`. Declared fields: 9.

| Field | Literal type | Source line |
|---|---|---:|
| `camera_type` | `i32` | 188 |
| `fov` | `f32` | 191 |
| `far_clip` | `f32` | 193 |
| `near_clip` | `f32` | 195 |
| `positions` | `M2Vec3SplineTrack` | 197 |
| `position_base` | `[f32; 3]` | 199 |
| `target` | `M2Vec3SplineTrack` | 201 |
| `target_base` | `[f32; 3]` | 203 |
| `roll` | `M2ScalarSplineTrack` | 205 |

## B: M2Model

Source: `B/crates/benilla-m2/src/model.rs`. Declared fields: 25.

| Field | Literal type | Source line |
|---|---|---:|
| `vertices` | `Vec<M2Vertex>` | 210 |
| `textures` | `Vec<M2Texture>` | 211 |
| `materials` | `Vec<M2Material>` | 212 |
| `color_alpha_tracks` | `Vec<M2ScalarTrack>` | 215 |
| `color_rgb_tracks` | `Vec<M2Vec3Track>` | 218 |
| `transparency_tracks` | `Vec<M2ScalarTrack>` | 220 |
| `transparency_lookup` | `Vec<u16>` | 223 |
| `texture_unit_lookup` | `Vec<u16>` | 226 |
| `texture_transforms` | `Vec<M2TextureTransform>` | 228 |
| `texture_combiners` | `Vec<u16>` | 232 |
| `texture_transform_lookup` | `Vec<u16>` | 235 |
| `global_sequences` | `Vec<u32>` | 237 |
| `bones` | `Vec<M2Bone>` | 238 |
| `cameras` | `Vec<M2Camera>` | 240 |
| `camera_lookup` | `Vec<u16>` | 243 |
| `raw_data` | `M2RawData` | 244 |
| `header` | `M2Header` | 245 |
| `pivot_attach_z` | `Option<f32>` | 248 |
| `attachments` | `Vec<M2Attachment>` | 250 |
| `attach_lookup` | `Vec<u16>` | 252 |
| `event_markers` | `Vec<M2EventMarker>` | 254 |
| `animation_lookup` | `Vec<u16>` | 257 |
| `playable_animation_lookup` | `Vec<M2PlayableAnim>` | 259 |
| `views` | `(u32, u32)` | 261 |
| `version` | `u32` | 262 |

## B: M2Format

Source: `B/crates/benilla-m2/src/model.rs`. Declared fields: 1.

| Field | Literal type | Source line |
|---|---|---:|
| `model` | `M2Model` | 318 |

## B: SkinSection

Source: `B/crates/benilla-m2/src/skin.rs`. Declared fields: 3.

| Field | Literal type | Source line |
|---|---|---:|
| `id` | `u16` | 12 |
| `triangle_start` | `u16` | 13 |
| `triangle_count` | `u16` | 14 |

## B: SkinBatch

Source: `B/crates/benilla-m2/src/skin.rs`. Declared fields: 10.

| Field | Literal type | Source line |
|---|---|---:|
| `flags` | `u16` | 20 |
| `shader_id` | `u16` | 22 |
| `skin_section_index` | `u16` | 23 |
| `texture_combo_index` | `u16` | 24 |
| `texture_coord_combo_index` | `u16` | 27 |
| `material_index` | `u16` | 28 |
| `color_index` | `u16` | 30 |
| `weight_combo_index` | `u16` | 32 |
| `texture_count` | `u16` | 34 |
| `texture_transform_combo_index` | `u16` | 36 |

## B: Skin

Source: `B/crates/benilla-m2/src/skin.rs`. Declared fields: 4.

| Field | Literal type | Source line |
|---|---|---:|
| `indices` | `Vec<u16>` | 41 |
| `triangles` | `Vec<u16>` | 42 |
| `submeshes` | `Vec<SkinSection>` | 43 |
| `batches` | `Vec<SkinBatch>` | 44 |

## B: M2Track

Source: `B/crates/benilla-m2/src/track.rs`. Declared fields: 4.

| Field | Literal type | Source line |
|---|---|---:|
| `interp` | `u16` | 12 |
| `gseq` | `u16` | 14 |
| `ranges` | `Vec<(u32, u32)>` | 19 |
| `keys` | `Vec<(u32, V)>` | 21 |

## B: M2SplineKey

Source: `B/crates/benilla-m2/src/track.rs`. Declared fields: 3.

| Field | Literal type | Source line |
|---|---|---:|
| `value` | `V` | 129 |
| `in_tan` | `V` | 130 |
| `out_tan` | `V` | 131 |

## E: M2Bone

Source: `E/godot/core/src/asset/m2_format/m2_anim.rs`. Declared fields: 6.

| Field | Literal type | Source line |
|---|---|---:|
| `key_bone_id` | `i32` | 53 |
| `flags` | `u32` | 54 |
| `parent_bone_id` | `i16` | 55 |
| `submesh_id` | `u16` | 56 |
| `name_crc` | `u32` | 59 |
| `pivot` | `[f32; 3]` | 61 |

## E: M2AnimSequence

Source: `E/godot/core/src/asset/m2_format/m2_anim.rs`. Declared fields: 10.

| Field | Literal type | Source line |
|---|---|---:|
| `id` | `u16` | 113 |
| `variation_id` | `u16` | 114 |
| `duration` | `u32` | 115 |
| `movespeed` | `f32` | 116 |
| `flags` | `u32` | 117 |
| `blend_time` | `u16` | 118 |
| `frequency` | `i16` | 120 |
| `replay` | `[u32; 2]` | 122 |
| `variation_next` | `i16` | 124 |
| `bounds` | `[[f32; 3]; 2]` | 127 |

## E: AnimTrack

Source: `E/godot/core/src/asset/m2_format/m2_anim.rs`. Declared fields: 3.

| Field | Literal type | Source line |
|---|---|---:|
| `interpolation_type` | `u16` | 205 |
| `global_sequence` | `i16` | 206 |
| `sequences` | `Vec<(Vec<u32>, Vec<T>)>` | 208 |

## E: BoneAnimTracks

Source: `E/godot/core/src/asset/m2_format/m2_anim.rs`. Declared fields: 3.

| Field | Literal type | Source line |
|---|---|---:|
| `translation` | `AnimTrack<[f32; 3]>` | 214 |
| `rotation` | `AnimTrack<[i16; 4]>` | 215 |
| `scale` | `AnimTrack<[f32; 3]>` | 216 |

## E: TextureAnimTracks

Source: `E/godot/core/src/asset/m2_format/m2_anim.rs`. Declared fields: 3.

| Field | Literal type | Source line |
|---|---|---:|
| `translation` | `AnimTrack<[f32; 3]>` | 221 |
| `rotation` | `AnimTrack<[f32; 4]>` | 223 |
| `scale` | `AnimTrack<[f32; 3]>` | 224 |

## E: ColorAnimTracks

Source: `E/godot/core/src/asset/m2_format/m2_anim.rs`. Declared fields: 2.

| Field | Literal type | Source line |
|---|---|---:|
| `color` | `AnimTrack<[f32; 3]>` | 229 |
| `opacity` | `AnimTrack<i16>` | 230 |

## E: M2Attachment

Source: `E/godot/core/src/asset/m2_format/m2_attach.rs`. Declared fields: 3.

| Field | Literal type | Source line |
|---|---|---:|
| `id` | `u32` | 32 |
| `bone` | `u16` | 34 |
| `position` | `[f32; 3]` | 36 |

## E: M2CameraSnapshot

Source: `E/godot/core/src/asset/m2_format/m2_camera.rs`. Declared fields: 7.

| Field | Literal type | Source line |
|---|---|---:|
| `camera_type` | `i32` | 18 |
| `position` | `[f32; 3]` | 19 |
| `target` | `[f32; 3]` | 20 |
| `roll` | `f32` | 21 |
| `fov` | `f32` | 22 |
| `near_clip` | `f32` | 23 |
| `far_clip` | `f32` | 24 |

## E: M2CollisionMesh

Source: `E/godot/core/src/asset/m2_format/m2_collision.rs`. Declared fields: 4.

| Field | Literal type | Source line |
|---|---|---:|
| `bounds_min` | `[f32; 3]` | 16 |
| `bounds_max` | `[f32; 3]` | 17 |
| `vertices` | `Vec<[f32; 3]>` | 18 |
| `indices` | `Vec<u16>` | 20 |

## E: M2Event

Source: `E/godot/core/src/asset/m2_format/m2_event.rs`. Declared fields: 5.

| Field | Literal type | Source line |
|---|---|---:|
| `identifier` | `[u8; 4]` | 26 |
| `data` | `u32` | 27 |
| `bone` | `u32` | 28 |
| `position` | `[f32; 3]` | 29 |
| `timestamps` | `Vec<Vec<u32>>` | 31 |

## E: M2Light

Source: `E/godot/core/src/asset/m2_format/m2_light.rs`. Declared fields: 10.

| Field | Literal type | Source line |
|---|---|---:|
| `light_type` | `u16` | 34 |
| `bone_index` | `i16` | 35 |
| `position` | `[f32; 3]` | 37 |
| `ambient_color` | `AnimTrack<[f32; 3]>` | 38 |
| `ambient_intensity` | `AnimTrack<f32>` | 39 |
| `diffuse_color` | `AnimTrack<[f32; 3]>` | 40 |
| `diffuse_intensity` | `AnimTrack<f32>` | 41 |
| `attenuation_start` | `AnimTrack<f32>` | 42 |
| `attenuation_end` | `AnimTrack<f32>` | 43 |
| `visibility` | `AnimTrack<u8>` | 44 |

## E: EvaluatedLight

Source: `E/godot/core/src/asset/m2_format/m2_light.rs`. Declared fields: 5.

| Field | Literal type | Source line |
|---|---|---:|
| `visible` | `bool` | 49 |
| `color` | `[f32; 3]` | 50 |
| `intensity` | `f32` | 51 |
| `attenuation_start` | `f32` | 52 |
| `attenuation_end` | `f32` | 53 |

## E: LightTrackTime

Source: `E/godot/core/src/asset/m2_format/m2_light.rs`. Declared fields: 4.

| Field | Literal type | Source line |
|---|---|---:|
| `sequence_index` | `usize` | 179 |
| `local_ms` | `u32` | 180 |
| `global_ms` | `u64` | 181 |
| `global_sequences` | `&'a [u32]` | 182 |

## E: M2ParticleMultiTexture

Source: `E/godot/core/src/asset/m2_format/m2_particle.rs`. Declared fields: 5.

| Field | Literal type | Source line |
|---|---|---:|
| `texture_indices` | `[u16; 3]` | 32 |
| `texture_fdids` | `[Option<u32>; 3]` | 33 |
| `uv_scale_bytes` | `[u8; 2]` | 34 |
| `velocity_midpoints` | `[[f32; 2]; 2]` | 35 |
| `velocity_ranges` | `[[f32; 2]; 2]` | 36 |

## E: M2ParticleEmitter

Source: `E/godot/core/src/asset/m2_format/m2_particle.rs`. Declared fields: 62.

| Field | Literal type | Source line |
|---|---|---:|
| `flags` | `u32` | 42 |
| `position` | `[f32; 3]` | 44 |
| `bone_index` | `u16` | 45 |
| `texture_index` | `u16` | 46 |
| `particle_model_filename` | `Option<String>` | 48 |
| `child_emitters_model_filename` | `Option<String>` | 50 |
| `texture_fdid` | `Option<u32>` | 52 |
| `multi_texture` | `Option<M2ParticleMultiTexture>` | 53 |
| `blend_type` | `u8` | 54 |
| `emitter_type` | `u8` | 56 |
| `particle_type` | `u8` | 58 |
| `head_or_tail` | `u8` | 60 |
| `tile_rows` | `u16` | 61 |
| `tile_cols` | `u16` | 62 |
| `emission_speed` | `f32` | 63 |
| `speed_variation` | `f32` | 64 |
| `vertical_range` | `f32` | 65 |
| `horizontal_range` | `f32` | 66 |
| `gravity` | `f32` | 67 |
| `gravity_vector` | `[f32; 3]` | 69 |
| `lifespan` | `f32` | 70 |
| `lifespan_variation` | `f32` | 72 |
| `emission_rate` | `f32` | 73 |
| `emission_rate_variation` | `f32` | 74 |
| `area_length` | `f32` | 75 |
| `area_width` | `f32` | 76 |
| `z_source` | `f32` | 77 |
| `tail_length` | `f32` | 79 |
| `drag` | `f32` | 80 |
| `scale_variation` | `f32` | 82 |
| `scale_variation_y` | `f32` | 84 |
| `base_spin` | `f32` | 86 |
| `base_spin_variation` | `f32` | 88 |
| `spin` | `f32` | 90 |
| `spin_variation` | `f32` | 92 |
| `wind_vector` | `[f32; 3]` | 94 |
| `wind_time` | `f32` | 96 |
| `follow_speed1` | `f32` | 98 |
| `follow_scale1` | `f32` | 100 |
| `follow_speed2` | `f32` | 102 |
| `follow_scale2` | `f32` | 104 |
| `tracks` | `EmitterTracks` | 106 |
| `inherit_velocity_scale` | `f32` | 111 |
| `colors` | `[[f32; 3]; 3]` | 113 |
| `color_keys` | `Vec<(f32, [f32; 3])>` | 115 |
| `opacity` | `[f32; 3]` | 117 |
| `opacity_keys` | `Vec<(f32, f32)>` | 119 |
| `scales` | `[[f32; 2]; 3]` | 121 |
| `scale_keys` | `Vec<(f32, [f32; 2])>` | 123 |
| `twinkle_speed` | `f32` | 125 |
| `twinkle_percent` | `f32` | 127 |
| `twinkle_scale_min` | `f32` | 129 |
| `twinkle_scale_max` | `f32` | 131 |
| `head_cell_track` | `[u16; 3]` | 133 |
| `tail_cell_track` | `[u16; 3]` | 135 |
| `head_cell_keys` | `Vec<(f32, u16)>` | 137 |
| `tail_cell_keys` | `Vec<(f32, u16)>` | 139 |
| `color_mult` | `f32` | 141 |
| `alpha_mult` | `f32` | 143 |
| `alpha_cutoff_keys` | `Vec<(f32, f32)>` | 145 |
| `burst_multiplier` | `f32` | 147 |
| `mid_point` | `f32` | 149 |

## E: EmitterHeaderCore

Source: `E/godot/core/src/asset/m2_format/m2_particle.rs`. Declared fields: 12.

| Field | Literal type | Source line |
|---|---|---:|
| `flags` | `u32` | 219 |
| `position` | `[f32; 3]` | 220 |
| `bone_index` | `u16` | 221 |
| `texture_index` | `u16` | 222 |
| `particle_model_filename` | `Option<String>` | 223 |
| `child_emitters_model_filename` | `Option<String>` | 224 |
| `blend_type` | `u8` | 225 |
| `emitter_type` | `u8` | 226 |
| `particle_type` | `u8` | 227 |
| `head_or_tail` | `u8` | 228 |
| `tile_rows` | `u16` | 229 |
| `tile_cols` | `u16` | 230 |

## E: EmitterTracks

Source: `E/godot/core/src/asset/m2_format/m2_particle.rs`. Declared fields: 11.

| Field | Literal type | Source line |
|---|---|---:|
| `emission_speed` | `Option<AnimTrack<f32>>` | 586 |
| `speed_variation` | `Option<AnimTrack<f32>>` | 587 |
| `vertical_range` | `Option<AnimTrack<f32>>` | 588 |
| `horizontal_range` | `Option<AnimTrack<f32>>` | 589 |
| `gravity` | `Option<AnimTrack<[f32; 3]>>` | 591 |
| `lifespan` | `Option<AnimTrack<f32>>` | 592 |
| `emission_rate` | `Option<AnimTrack<f32>>` | 593 |
| `area_length` | `Option<AnimTrack<f32>>` | 594 |
| `area_width` | `Option<AnimTrack<f32>>` | 595 |
| `z_source` | `Option<AnimTrack<f32>>` | 597 |
| `enabled` | `Option<AnimTrack<u8>>` | 598 |

## E: M2RibbonEmitter

Source: `E/godot/core/src/asset/m2_format/m2_ribbon.rs`. Declared fields: 19.

| Field | Literal type | Source line |
|---|---|---:|
| `id` | `u32` | 28 |
| `bone_index` | `u32` | 29 |
| `position` | `[f32; 3]` | 31 |
| `texture_indices` | `Vec<u16>` | 32 |
| `material_indices` | `Vec<u16>` | 33 |
| `color` | `AnimTrack<[f32; 3]>` | 34 |
| `alpha` | `AnimTrack<i16>` | 35 |
| `height_above` | `AnimTrack<f32>` | 36 |
| `height_below` | `AnimTrack<f32>` | 37 |
| `texture_slot` | `AnimTrack<u16>` | 38 |
| `visibility` | `AnimTrack<u8>` | 39 |
| `edges_per_second` | `f32` | 40 |
| `edge_lifetime` | `f32` | 41 |
| `gravity` | `f32` | 43 |
| `texture_rows` | `u16` | 44 |
| `texture_cols` | `u16` | 45 |
| `priority_plane` | `i16` | 46 |
| `color_index` | `i8` | 47 |
| `texture_transform_lookup` | `i8` | 48 |

## E: VariationFamily

Source: `E/godot/core/src/asset/m2_format/m2_variation.rs`. Declared fields: 4.

| Field | Literal type | Source line |
|---|---|---:|
| `linked` | `usize` | 16 |
| `playable` | `Vec<(usize, u32)>` | 18 |
| `total` | `u32` | 19 |
| `shortest_ms` | `u32` | 20 |

## E: M2Vertex

Source: `E/godot/core/src/asset/m2_format/parser.rs`. Declared fields: 6.

| Field | Literal type | Source line |
|---|---|---:|
| `position` | `[f32; 3]` | 19 |
| `bone_weights` | `[u8; 4]` | 20 |
| `bone_indices` | `[u8; 4]` | 21 |
| `normal` | `[f32; 3]` | 22 |
| `tex_coords` | `[f32; 2]` | 23 |
| `tex_coords_2` | `[f32; 2]` | 24 |

## E: M2Submesh

Source: `E/godot/core/src/asset/m2_format/parser.rs`. Declared fields: 5.

| Field | Literal type | Source line |
|---|---|---:|
| `mesh_part_id` | `u16` | 28 |
| `vertex_start` | `u16` | 29 |
| `vertex_count` | `u16` | 30 |
| `triangle_start` | `u32` | 31 |
| `triangle_count` | `u16` | 32 |

## E: RawM2Submesh

Source: `E/godot/core/src/asset/m2_format/parser.rs`. Declared fields: 13.

| Field | Literal type | Source line |
|---|---|---:|
| `mesh_part_id` | `u16` | 38 |
| `_level` | `u16` | 39 |
| `vertex_start` | `u16` | 40 |
| `vertex_count` | `u16` | 41 |
| `_triangle_start` | `u16` | 42 |
| `triangle_count` | `u16` | 43 |
| `_bone_count` | `u16` | 44 |
| `_bone_combo_index` | `u16` | 45 |
| `_bone_influences` | `u16` | 46 |
| `_root_bone` | `u16` | 47 |
| `_center_mass` | `[f32; 3]` | 48 |
| `_sort_center_position` | `[f32; 3]` | 49 |
| `_sort_radius` | `f32` | 50 |

## E: M2Material

Source: `E/godot/core/src/asset/m2_format/parser.rs`. Declared fields: 2.

| Field | Literal type | Source line |
|---|---|---:|
| `flags` | `u16` | 56 |
| `blend_mode` | `u16` | 57 |

## E: M2TextureUnit

Source: `E/godot/core/src/asset/m2_format/parser.rs`. Declared fields: 12.

| Field | Literal type | Source line |
|---|---|---:|
| `flags` | `u8` | 61 |
| `priority_plane` | `i8` | 62 |
| `shader_id` | `u16` | 63 |
| `submesh_index` | `u16` | 64 |
| `color_index` | `i16` | 65 |
| `render_flags_index` | `u16` | 66 |
| `material_layer` | `u16` | 67 |
| `texture_count` | `u16` | 68 |
| `texture_id` | `u16` | 69 |
| `texture_coord_index` | `u16` | 70 |
| `transparency_index` | `u16` | 71 |
| `texture_animation_id` | `u16` | 72 |

## E: RawM2TextureUnit

Source: `E/godot/core/src/asset/m2_format/parser.rs`. Declared fields: 13.

| Field | Literal type | Source line |
|---|---|---:|
| `flags` | `u8` | 78 |
| `priority_plane` | `i8` | 79 |
| `shader_id` | `u16` | 80 |
| `submesh_index` | `u16` | 81 |
| `_submesh_index2` | `u16` | 82 |
| `color_index` | `i16` | 83 |
| `render_flags_index` | `u16` | 84 |
| `material_layer` | `u16` | 85 |
| `texture_count` | `u16` | 86 |
| `texture_id` | `u16` | 87 |
| `texture_coord_index` | `u16` | 88 |
| `transparency_index` | `u16` | 89 |
| `texture_animation_id` | `u16` | 90 |

## E: M2TextureTypeEntry

Source: `E/godot/core/src/asset/m2_format/parser.rs`. Declared fields: 4.

| Field | Literal type | Source line |
|---|---|---:|
| `texture_type` | `u32` | 96 |
| `flags` | `u32` | 97 |
| `_filename_length` | `u32` | 98 |
| `_filename_offset` | `u32` | 99 |

## E: SkinHeader

Source: `E/godot/core/src/asset/m2_format/parser.rs`. Declared fields: 11.

| Field | Literal type | Source line |
|---|---|---:|
| `_magic` | `[u8; 4]` | 105 |
| `lookup_count` | `u32` | 106 |
| `lookup_offset` | `u32` | 107 |
| `indices_count` | `u32` | 108 |
| `indices_offset` | `u32` | 109 |
| `_bone_indices_count` | `u32` | 110 |
| `_bone_indices_offset` | `u32` | 111 |
| `submesh_count` | `u32` | 112 |
| `submesh_offset` | `u32` | 113 |
| `batch_count` | `u32` | 114 |
| `batch_offset` | `u32` | 115 |

## E: SkinData

Source: `E/godot/core/src/asset/m2_format/parser.rs`. Declared fields: 4.

| Field | Literal type | Source line |
|---|---|---:|
| `lookup` | `Vec<u16>` | 119 |
| `indices` | `Vec<u16>` | 120 |
| `submeshes` | `Vec<M2Submesh>` | 121 |
| `batches` | `Vec<M2TextureUnit>` | 122 |

## E: M2Chunks

Source: `E/godot/core/src/asset/m2_format/parser.rs`. Declared fields: 7.

| Field | Literal type | Source line |
|---|---|---:|
| `md20` | `&'a [u8]` | 126 |
| `ska1` | `Option<&'a [u8]>` | 127 |
| `txid` | `Option<&'a [u8]>` | 128 |
| `exp2` | `Option<&'a [u8]>` | 130 |
| `skid` | `Option<u32>` | 131 |
| `sfid` | `Vec<u32>` | 132 |
| `afid` | `Vec<(u16, u16, u32)>` | 134 |

## E: TextureTables

Source: `E/godot/core/src/asset/m2_format/parser.rs`. Declared fields: 4.

| Field | Literal type | Source line |
|---|---|---:|
| `tex_lookup` | `&'a [u16]` | 138 |
| `tex_types` | `&'a [u32]` | 139 |
| `txid` | `&'a [u32]` | 140 |
| `skin_fdids` | `&'a [u32]` | 141 |

## E: SkelData

Source: `E/godot/core/src/asset/m2_format/parser.rs`. Declared fields: 4.

| Field | Literal type | Source line |
|---|---|---:|
| `bones` | `Vec<super::m2_anim::M2Bone>` | 391 |
| `sequences` | `Vec<super::m2_anim::M2AnimSequence>` | 392 |
| `bone_tracks` | `Vec<super::m2_anim::BoneAnimTracks>` | 393 |
| `global_sequences` | `Vec<u32>` | 394 |

## E: EmitterTrackDefaults

Source: `E/godot/core/src/asset/m2_particle_defaults.rs`. Declared fields: 27.

| Field | Literal type | Source line |
|---|---|---:|
| `emission_speed` | `f32` | 73 |
| `speed_variation` | `f32` | 74 |
| `vertical_range` | `f32` | 75 |
| `horizontal_range` | `f32` | 76 |
| `gravity` | `f32` | 77 |
| `gravity_vector` | `[f32; 3]` | 78 |
| `lifespan` | `f32` | 79 |
| `lifespan_variation` | `f32` | 80 |
| `emission_rate` | `f32` | 81 |
| `emission_rate_variation` | `f32` | 82 |
| `area_length` | `f32` | 83 |
| `area_width` | `f32` | 84 |
| `z_source` | `f32` | 85 |
| `tail_length` | `f32` | 86 |
| `drag` | `f32` | 87 |
| `scale_variation` | `f32` | 88 |
| `scale_variation_y` | `f32` | 89 |
| `base_spin` | `f32` | 90 |
| `base_spin_variation` | `f32` | 91 |
| `spin` | `f32` | 92 |
| `spin_variation` | `f32` | 93 |
| `wind_vector` | `[f32; 3]` | 94 |
| `wind_time` | `f32` | 95 |
| `follow_speed1` | `f32` | 96 |
| `follow_scale1` | `f32` | 97 |
| `follow_speed2` | `f32` | 98 |
| `follow_scale2` | `f32` | 99 |

## E: EmitterVisualDefaults

Source: `E/godot/core/src/asset/m2_particle_defaults.rs`. Declared fields: 14.

| Field | Literal type | Source line |
|---|---|---:|
| `colors` | `[[f32; 3]; 3]` | 137 |
| `color_keys` | `Vec<(f32, [f32; 3])>` | 138 |
| `opacity` | `[f32; 3]` | 139 |
| `opacity_keys` | `Vec<(f32, f32)>` | 140 |
| `scales` | `[[f32; 2]; 3]` | 141 |
| `scale_keys` | `Vec<(f32, [f32; 2])>` | 142 |
| `twinkle_speed` | `f32` | 143 |
| `twinkle_percent` | `f32` | 144 |
| `twinkle_scale_min` | `f32` | 145 |
| `twinkle_scale_max` | `f32` | 146 |
| `head_cell_track` | `[u16; 3]` | 147 |
| `tail_cell_track` | `[u16; 3]` | 148 |
| `burst_multiplier` | `f32` | 149 |
| `mid_point` | `f32` | 150 |

## E: Vertex

Source: `E/godot/core/src/m2.rs`. Declared fields: 6.

| Field | Literal type | Source line |
|---|---|---:|
| `position` | `[f32; 3]` | 15 |
| `normal` | `[f32; 3]` | 16 |
| `tex_coords` | `[f32; 2]` | 17 |
| `tex_coords_2` | `[f32; 2]` | 18 |
| `bone_weights` | `[u8; 4]` | 19 |
| `bone_indices` | `[u8; 4]` | 20 |

## E: Submesh

Source: `E/godot/core/src/m2.rs`. Declared fields: 5.

| Field | Literal type | Source line |
|---|---|---:|
| `mesh_part_id` | `u16` | 25 |
| `vertex_start` | `u16` | 26 |
| `vertex_count` | `u16` | 27 |
| `triangle_start` | `u32` | 28 |
| `triangle_count` | `u16` | 29 |

## E: Material

Source: `E/godot/core/src/m2.rs`. Declared fields: 2.

| Field | Literal type | Source line |
|---|---|---:|
| `flags` | `u16` | 34 |
| `blend_mode` | `u16` | 35 |

## E: Model

Source: `E/godot/core/src/m2.rs`. Declared fields: 32.

| Field | Literal type | Source line |
|---|---|---:|
| `vertices` | `Vec<Vertex>` | 42 |
| `indices` | `Vec<u16>` | 43 |
| `submeshes` | `Vec<Submesh>` | 44 |
| `batches` | `Vec<TextureUnit>` | 45 |
| `materials` | `Vec<Material>` | 46 |
| `bones` | `Vec<Bone>` | 47 |
| `sequences` | `Vec<Sequence>` | 48 |
| `bone_tracks` | `std::sync::Arc<Vec<BoneAnimTracks>>` | 50 |
| `global_sequences` | `Vec<u32>` | 51 |
| `texture_types` | `Vec<u32>` | 52 |
| `texture_flags` | `Vec<u32>` | 54 |
| `texture_fdids` | `Vec<u32>` | 55 |
| `texture_lookup` | `Vec<u16>` | 56 |
| `texture_unit_lookup` | `Vec<i16>` | 57 |
| `transparency_lookup` | `Vec<i16>` | 58 |
| `uv_animation_lookup` | `Vec<i16>` | 59 |
| `uses_texture_combiner_combos` | `bool` | 60 |
| `color_tracks` | `Vec<m2_anim::ColorAnimTracks>` | 61 |
| `transparency_tracks` | `Vec<m2_anim::AnimTrack<i16>>` | 62 |
| `texture_animations` | `Vec<m2_anim::TextureAnimTracks>` | 63 |
| `skin_fdids` | `Vec<u32>` | 64 |
| `skeleton_fdid` | `Option<u32>` | 65 |
| `attachments` | `Vec<m2_attach::M2Attachment>` | 66 |
| `attachment_lookup` | `Vec<i16>` | 67 |
| `bounding_box_min` | `[f32; 3]` | 68 |
| `bounding_box_max` | `[f32; 3]` | 69 |
| `collision` | `Option<M2CollisionMesh>` | 70 |
| `particle_emitters` | `Vec<ParticleEmitter>` | 72 |
| `ribbon_emitters` | `Vec<format::m2_ribbon::M2RibbonEmitter>` | 74 |
| `events` | `Vec<Event>` | 76 |
| `flags` | `u32` | 78 |
| `lights` | `Vec<m2_light::M2Light>` | 79 |

## E: AssetReferences

Source: `E/godot/core/src/m2.rs`. Declared fields: 2.

| Field | Literal type | Source line |
|---|---|---:|
| `skin_fdids` | `Vec<u32>` | 125 |
| `skeleton_fdid` | `Option<u32>` | 126 |

## E: AnimTime

Source: `E/godot/core/src/m2.rs`. Declared fields: 4.

| Field | Literal type | Source line |
|---|---|---:|
| `sequence` | `usize` | 282 |
| `time_ms` | `u32` | 283 |
| `global_ms` | `f64` | 284 |
| `global_sequences` | `&'a [u32]` | 285 |

## E: SubmeshArrays

Source: `E/godot/core/src/m2.rs`. Declared fields: 7.

| Field | Literal type | Source line |
|---|---|---:|
| `positions` | `Vec<[f32; 3]>` | 446 |
| `normals` | `Vec<[f32; 3]>` | 447 |
| `uv` | `Vec<[f32; 2]>` | 448 |
| `uv2` | `Vec<[f32; 2]>` | 449 |
| `bones` | `Vec<i32>` | 450 |
| `weights` | `Vec<f32>` | 451 |
| `indices` | `Vec<i32>` | 452 |

## Sources

- Pinned E/B source files named above; declaration census only.
- [Paired asset inventory](benilla-asset-field-inventory.md) — existing bounded field/consumer findings.

## See Also

- [[m2-format]] — current parser layout and effect references.
- [[open-source-wow-clients]] — reference sources and version boundaries.
