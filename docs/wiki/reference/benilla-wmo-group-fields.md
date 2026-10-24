# Benilla / game-engine WMO Group, Liquid and BSP Fields

Individual group inputs and their static parser-to-consumer boundaries. Modern E records and Benilla's WoW 1.12.1/build 5875 reader are different contracts; matching offsets do not establish cross-version semantics. This page complements [[benilla-wmo-root-fields]], not a whole-WMO compatibility or runtime claim.

Snapshot: **2026-10-24**. E native baseline `354bdb7dbf177047ed46d63168acec531f5c4426`; B `8a6c74036cfe3d00275fd75ad1aeae1f4ea7d0df`. Static local text inspection only: no builds, tests, source execution/imports, network, downloads or extraction.

## Sources and notation

- **E** = `/syncthing/Sync/Projects/world-of-osso/game-engine`; **B** = `/home/osso/Repos/benilla`.
- **T**, **P**, **C**, **M** = E `godot/core/src/asset/wmo_format/{parser_types,parser,parser_chunks,mesh_data}.rs` respectively.
- **S** = E `godot/rust/src/wmo/scene.rs`; **B0** = B `crates/benilla-wmo/src/lib.rs`; **BA** = B `crates/benilla-formats/src/models/wmo/group.rs`.
- **Q** = `/syncthing/Sync/Projects/world-of-osso/shared-protocol/src/ground/wmo.rs`, revision `5107483b61745a3378d1a88d1cf2cd960123e998`. E `godot/Cargo.toml:18–19` selects this local patch; `godot/rust/Cargo.toml:37` declares git `74e67cf`, **not the selected source** for this audit.

Anchors below refer to these source snapshots. “No located consumer” means only the inspected paths did not establish one; it is not a repository-wide absence claim. B base-parser omissions do not imply omissions in its separate raw-byte adapters. Temporary leads `/tmp/wmo-{group-fields,group-integration,collision-fields}.md` guided inspection; primary source overrides their stale claims, especially for MOC2, MOLP/MLSP, bounds and BSP.

## MOGP: 68-byte little-endian header

T:649–667; P:318–339 retain every member below. E `godot/core/src/wmo.rs:60–68` requires the complete header before parsing subchunks. B0:236–257 retains only flags/group liquid in its base result and returns a group without geometry for a short header; BA:58–80 separately reads selected fields, requiring at least40 bytes and defaulting later unavailable members.

| Serialized member | E representation / located consumer | B base versus raw adapter |
|---|---|---|
| Name offset `u32 +0x00` | `group_name_offset`; no resolved-name consumer located in inspected native paths. | Neither B0 group result nor BA header retains it. |
| Descriptive-name offset `u32 +0x04` | `descriptive_group_name_offset`; same bounded unknown. | Neither inspected header projection retains it. |
| Flags `u32 +0x08` | Raw `flags`; derived exterior `0x8`, interior `0x2000`, antiportal `0x04000000` (T:72–90). Portal classification, fog, liquid and collision inspect flags independently. Q:43–80 suppresses geometry for unreachable `0x80` or antiportal. Other bits remain raw, not universally interpreted. | B0 retains raw flags; BA retains flags and its interior policy tests `flags & 0x48 == 0` (BA:497–500), not E's derived interior boolean. |
| Bounds minimum `f32[3] +0x0c` | Retained `bbox_min`; **native doodad lighting uses this corner in box center**, `wmo/doodad_light.rs:127–147`. Q independently reads it for query bounds/local bounds. | Not retained in B0/BA header projections; this says nothing about other B raw readers. |
| Bounds maximum `f32[3] +0x18` | Retained `bbox_max`; same center and Q bounds consumers. | Same projection limit. |
| Portal start `u16 +0x24` | `portal_start`; portal/fog reference slice, `wmo/portals.rs:36–58`, core `wmo_format/fog.rs:70–90`. | BA retains `portal_ref_start`; absent from B0 result. |
| Portal count `u16 +0x26` | `portal_count`; same slice. | BA retains `portal_ref_count`. |
| Transparent batch count `u16 +0x28` | `trans_batch_count`; M:103–160,186–241 classify/fix batch colors. | BA:484–493 reads directly from MOGP for render classification; absent from B0 result. |
| Interior batch count `u16 +0x2a` | `int_batch_count`; same batch boundary. | BA raw-byte classification reads this separately. |
| Exterior batch count `u16 +0x2c` | `ext_batch_count`; retained batch metadata (M:103–160). | BA:484–529 uses remaining batches as exterior rather than retaining this count. |
| Batch type D `u16 +0x2e` | `batch_type_d`; no located native consumer. | Not retained by inspected projections. |
| Fog ID 0 `u8 +0x30` | `fog_ids[0]`; core fog copies array into group selection data (fog.rs:70–90). | BA `fog_indices[0]`, zero for short header. |
| Fog ID 1 `u8 +0x31` | `fog_ids[1]`; same fog input. | BA `fog_indices[1]`. |
| Fog ID 2 `u8 +0x32` | `fog_ids[2]`; same fog input. | BA `fog_indices[2]`. |
| Fog ID 3 `u8 +0x33` | `fog_ids[3]`; same fog input; no claim all four are always selected. | BA `fog_indices[3]`. |
| Group liquid `u32 +0x34` | `group_liquid`; root mode/group flags resolve LiquidType, core `wmo_liquid.rs:31–58`. | B0 and BA retain; BA calls `0xf` no whole-group liquid. Do not equate this policy with E's modern type conversion. |
| Unique ID `u32 +0x38` | `unique_id`; no located native use in inspected group consumers. | BA explicitly projects `area_table_id` for WMOAreaTable identity; B0 omits it. |
| Flags2 `u32 +0x3c` | `flags2`, opaque; no located interpretation. | Not retained by inspected projections. |
| Parent split group `i16 +0x40` | `parent_split_group_index`; no located consumer. | Not retained by inspected projections. |
| Next split child `i16 +0x42` | `next_split_child_group_index`; no located consumer. | Not retained by inspected projections. |

The subchunk stream starts at68, not at a variable header extent. No opaque extra-header preservation is established. Root MOGI bounds used in portal camera classification are separate from these MOGP corners; their use does not negate the native MOGP box-center consumer.

## Geometry, references and color streams

E raw fields T:357–385; handlers C:394–524. B0 dispatch:258–323. Component arrays below are repeated record fields, not additional headers.

| Chunk / individual record input | E retention / located consumer | B base versus adapter |
|---|---|---|
| MOVT `x,y,z: f32` at +0,+4,+8; stride12 | `vertices`; M:248–278 basis-converts mesh positions; Q independently retains collision vertices. | B0:260–267 appends positions; BA:500–538 builds indexed submeshes. |
| MONR `x,y,z: f32`; stride12 | `normals`; M:349–356 uses authored normals when counts match, otherwise up vectors. | B0:269–276 retains; BA:500–506 reads per-vertex normals. |
| MOTV `u:f32 +0`, `v:f32 +4`; stride8 | C:502–515 fills first **empty** UV destination through four sets; later sets ignored. Empty chunks do not advance a slot. S:101–124,462 onward binds UV2 and packed UV3/UV4, beyond M's material-specific streams. | B0:278–285 concatenates repeated chunks into one vector, not four indexed sets; BA selects by vertex index. |
| MOVI `index:u16`; stride2 | `indices`; M:324–338 batch index slices; Q collision triangles independently read same encoding. | B0:286–291 appends indices; BA:530–538 selects batch ranges. |
| MOPY `flags:u8 +0`; stride2 | Widened to `WmoTriangleMaterial.flags:u16`; Q independently reduces flags/material to collision bool. | B0:313–320 retains raw byte; BA footprint adapter:115–147 rejects `0x88` faces and retains remaining flags. |
| MOPY `material_id:u8 +1` | `0xff` becomes `None`; M:379–398 excludes no-material triangles in whole-group render path. | B0 preserves raw byte including `0xff`; footprint adapter keeps material for surviving faces. MOBA submesh slicing is a different path. |
| MPY2 `flags:u16 +0`; stride4 | Same widened face representation, C:399–402; Q independently computes collision bool. | No MPY2 case in inspected B0 dispatch. No universal B-reader absence inferred. |
| MPY2 `material_id:u16 +2` | `0xffff` becomes `None`. MOPY/MPY2 assign the same vector: last encountered assignment wins. | No MPY2 field in inspected base result. |
| MOCV blue `u8 +0`; stride4 | First nonempty color destination: normalized RGBA blue component. Subsequent chunks discard RGB. | B0:303–311 appends byte colors; BA fixed-color/footprint paths use RGB. |
| MOCV green `u8 +1` | Same, green component. | Same byte retention. |
| MOCV red `u8 +2` | Same, red component. | Same byte retention. |
| MOCV alpha `u8 +3` | First stream retains normalized alpha; later stream replaces `second_color_blend_alphas`. M:186–241 color fix; S binds second alpha. | Base retains alpha; BA fixed-color path interprets lighting alpha separately from opacity. |
| MOC2 blue `u8 +0`; stride4 | Normalized blue in `moc2_colors`; S:101–124,462 onward binds **CUSTOM1 RGBA8**, not unused. | No MOC2 field/case in inspected B0. |
| MOC2 green `u8 +1` | Normalized green, same native stream. | Same base boundary. |
| MOC2 red `u8 +2` | Normalized red, same native stream. | Same base boundary. |
| MOC2 alpha `u8 +3` | Normalized alpha, same native stream. | Same base boundary. |
| MODR `doodad_ref:u16`; stride2 | `doodad_refs`; native `wmo/assets.rs:26–43` passes refs to placed-doodad selection. | Not B0 field; BA:165–175 raw `RDOM` reader returns root MODD indices. |
| MOLR `light_ref:u16`; stride2 | `light_refs`; precise native consumer not established in inspected asset/point-light/doodad-light paths. MOLP is separate. | Not B0 field; BA:178–188 raw `RLOM` reader returns MOLT refs. |

### MOBA: 24-byte batch

T:620–645; P:486–499. B0:292–301 only retains start/count/small material; BA:484–538 consumes these plus separately read MOGP counts. B's comment labels the first12 bytes as i16 box corners; E explicitly treats bytes0–9 as opaque and bytes10–11 as a selectable large material ID. Do not overwrite either source's version-qualified interpretation.

| Serialized member | E fate / use | B0 fate |
|---|---|---|
| `_unknown_box:u8[10] +0` | Read opaque, dropped; no box interpretation established. | First12 bytes skipped; comment describes legacy bounds, not retained bounds. |
| `material_id_large:u16 +10` | Selected only when flags bit `0x02`; retained as cooked material ID. | Skipped; no large-ID selection. |
| `start_index:u32 +12` | Retained; selects MOVI range. | Retained; same range role. |
| `count:u16 +16` | Retained index count. | Retained. |
| `min_index:u16 +18` | Retained; mesh vertex range. | Skipped. |
| `max_index:u16 +20` | Retained; mesh vertex range. | Skipped. |
| `flags:u8 +22` | Bit `0x02` selects large ID; raw byte otherwise dropped, other bits opaque here. | Skipped. |
| `material_id_small:u8 +23` | Widened/selected when large-ID flag clear; unused alternative dropped. | Retained unconditionally as material ID. |

### MOLP / MLSP: active native point-light inputs

C:414–460; core `wmo.rs:34–58` selects MLSP ranges for deduplicated active doodad sets, transforms positions and multiplies RGB by intensity. Native `wmo/point_lights.rs:14–29` creates OmniLight3D children, uses attenuation end as range and start fraction as specular. These are located source consumers, not observed lighting results. No MOLP/MLSP case exists in inspected B0 dispatch; auxiliary B coverage outside BA is unestablished.

| Serialized member | E fate / located use |
|---|---|
| MOLP `light_id:u32 +0` | Retained; no ID-based lookup in inspected active-point-light consumer. |
| MOLP blue `u8 +4` | Normalized RGB blue; intensity multiplication and native color. |
| MOLP green `u8 +5` | Normalized green; same consumer. |
| MOLP red `u8 +6` | Normalized red; same consumer. |
| MOLP alpha `u8 +7` | Discarded, not native opacity. |
| MOLP position `f32[3] +8` | Retained; file-to-engine conversion and native position. |
| MOLP attenuation start `f32 +20` | Retained; PointLight/start fraction. |
| MOLP attenuation end `f32 +24` | Retained; native range/start fraction. |
| MOLP intensity `f32 +28` | Retained; scales RGB before native binding. |
| MOLP bytes32–43 | Skipped opaque12-byte interval. Comment says rotation, but no decoded rotation or four-float quaternion is established; stride44. |
| MLSP offset `u32 +0`; stride8 | Retained pair start into MOLP, selected by active set index. |
| MLSP count `u32 +4` | Retained range length; saturating range end and bounds-checked light access. |

## MLIQ: 30-byte header, 8-byte vertices, 1-byte tiles

T:386–410,678–694; P:375–450. Core `wmo_liquid.rs:31–125` resolves type and builds geometry; native `terrain/wmo_liquid.rs:31–70` attaches liquid surfaces. B0:329–373 parses grids; BA:217–303 builds liquid meshes. Field retention is not proof that every retained input affects native water.

| Serialized member | E fate / located use | B handling |
|---|---|---|
| `x_verts:i32 +0` | Retained; vertex count and grid column indexing. | Reads/retains `u32 xverts`. |
| `y_verts:i32 +4` | Retained; vertex count. | Reads/retains `u32 yverts`. |
| `x_tiles:i32 +8` | Retained; tile count/row stride. | Reads/retains `u32 xtiles`. |
| `y_tiles:i32 +12` | Retained; tile count/row loop. | Reads/retains `u32 ytiles`. |
| Base X `f32 +16` | Retained `position[0]`; grid vertex X. | Retained base X. |
| Base Y `f32 +20` | Retained `position[1]`; grid vertex Y. | Retained base Y. |
| Base Z `f32 +24` | Retained `position[2]`; inspected E vertices use authored per-vertex height instead, no base-Z addition. | Retained base Z; not evidence of a universal relative-height convention. |
| `material_id:i16 +28` | Retained header value; exact native material selection consumer not established in inspected E geometry path. | Reads/retains `u16`; distinct signedness. |
| Vertex raw byte0 `u8 +0` | Retained; low byte of signed magma S for type19. Ordinary path uses positional UV, not opacity byte. | Retains as opacity-ramp coordinate; BA:25–29 normalizes byte/255. |
| Vertex raw byte1 `u8 +1` | Retained; high byte of signed magma S. | Discarded. |
| Vertex raw byte2 `u8 +2` | Retained; low byte of signed magma T. | Discarded. |
| Vertex raw byte3 `u8 +3` | Retained; high byte of signed magma T. | Discarded. |
| Vertex height `f32 +4` | Retained; actual surface height. Magma UV uses signed raw pairs ×3/256. | Retained heights; BA comments explicitly say authored magma UV pair is not parsed. |
| Tile low nibble `u8 &0x0f` | Retained `liquid_type`; `0xf` suppresses tile geometry. Whole surface type separately resolved from MOGP/root mode. | Retains whole byte; BA picks first wet nibble unless groupLiquid overrides. |
| Tile bit `0x10` | Dropped, no interpretation established. | Preserved in raw tile byte; use not established here. |
| Tile bit `0x20` | Dropped, no interpretation established. | Preserved; use not established here. |
| Tile bit `0x40` | Retained `fishable`; no consumer in inspected E liquid geometry/native surface path. | Preserved raw byte. |
| Tile bit `0x80` | Retained `shared`; no consumer in inspected E surface path. | Preserved raw byte; BA:217–252 derives shared/holes. |

E count helper checked-multiplies signed dimensions then casts to usize; no explicit negative-dimension rejection there. P:390–408 checks requested payload spans; no claim of complete malformed-input safety. B rejects zero, oversized (>2^20 entries) or short grids with no liquid. Bytes after the calculated MLIQ grid are not retained by either inspected decoder. MOGP groupLiquid/root mode is not an extra MLIQ header field.

## BSP: MOBN 16-byte nodes and MOBR u16 face references

**MOBN/MOBR query support exists.** It is in selected shared Q, not the E core render-handler table. E native `wmo/assets.rs:161–181` parses both core group and shared collision from the same bytes. Q:43–129,160–211 provides BSP walkable-hit traversal; Q:264–290 decodes node/ref arrays.

| Serialized member | Q retention / query interpretation |
|---|---|
| MOBN flags `u16 +0` | Retained raw. `0x04` means leaf; otherwise low two bits choose axis, value3 clamped to2. Other bits retained but not interpreted by traversal. |
| MOBN negative child `i16 +2` | Retained signed; nonnegative child visited when segment minimum is at/below plane. |
| MOBN positive child `i16 +4` | Retained signed; nonnegative child visited when segment maximum is at/above plane. |
| MOBN face count `u16 +6` | Retained; leaf reference-range length, clamped to available MOBR extent. |
| MOBN face start `u32 +8` | Retained; first **MOBR entry**, not MOVI index. |
| MOBN plane distance `f32 +12` | Retained; split-plane comparisons. |
| MOBR face reference `u16`; stride2 | Retained `face_refs`; leaf ranges select faces, collidable predicate filters, face number maps to MOVI triple/MOVT corners. |

Q traversal starts at node0; absent BSP nodes produce no walkable hits. It bounds-tests the segment, intersects triangles and applies transformed walkable-normal threshold. Missing face flags default noncollidable; invalid corner lookup yields no face. No general graph validation/cycle safety claim follows from these checks.

Q independently reduces MOPY/MPY2 to booleans: collision bit `0x08`, or render `0x20` without detail `0x04`, or collision-only material (`0xff`/`0xffff`). Raw flags/material are not retained by Q; E core does retain its separate face representation. Duplicate assignments replace earlier values, including mixed MOPY/MPY2.

**Native concave shapes are not BSP meshes.** Q:83–96 `collidable_triangles()` enumerates MOVI triples using collision booleans/MOVT, converts `[x,y,z]` to `[x,z,-y]`, and does not traverse MOBN/MOBR. Native `wmo/collision.rs:224–242` caches a ConcavePolygonShape3D from that iterator. Thus shared BSP floor queries and native wall/body triangles have different enumeration boundaries.

B0's inspected dispatch has no MOBN/MOBR case; BA's inspected render/reference/header adapters do not establish a production BSP consumer. B `crates/benilla-formats/examples/wmo_bsp.rs:1–7,200 onward` separately inspects reachable BSP faces: an example is not proof of production integration. Broader B collision consumers remain outside this bounded audit.

## Framing, tails and version limits

E C:36–51 dispatches reversed group tags; unrecognized render chunks are skipped, not stored opaque. P:17–37 decodes complete fixed records and ignores partial tails for MOVT/MONR/MOTV/MOBA/MOPY. MOVI/MODR/MOLR ignore an odd final byte; MOCV/MOC2 ignore partial four-byte colors. **MPY2 rejects a nonmultiple of4**, MOLP rejects a nonmultiple of44, MLSP rejects a nonmultiple of8: the earlier blanket “tails ignored” lead is wrong for these core paths.

Q instead ignores incomplete MOBN/MOBR/MOPY/MPY2/MOVI/MOVT records via complete-entry iteration. Core and collision error behavior therefore differs. C:526 onward requires nonempty vertices/indices, not universal cross-array consistency. B0 appends repeated geometry/color/MOPY/batch records and ignores incomplete records; E generally replaces streams, except empty-slot UV/color selection. B MLIQ is separately assigned.

No inspected dispatch gates each modern E extension on a specific MVER value. MPY2, MOC2, third/fourth UV sets, large material IDs, split-header indices and MOLP/MLSP must not be advertised as Vanilla5875 features merely because E accepts them. Unknown raw flag meanings, opaque intervals, and unlocated consumers remain unknown; no visual, physical correctness, malformed-file acceptance or version-wide compatibility was executed here.

## See Also

- [[benilla-wmo-root-fields]] — MOHD/MOMT fields and modern versus Vanilla material semantics.
- [[benilla-asset-field-inventory]] — parent comparison and evidence vocabulary.
- [[wmo-format]] — engine WMO format/system context.
- [[open-source-wow-clients]] — reference-client version boundaries.
