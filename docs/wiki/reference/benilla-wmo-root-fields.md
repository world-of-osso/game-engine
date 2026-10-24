# Benilla / game-engine WMO Root Header and Material Fields

Individual MOHD/MOMT inputs and their parser-to-consumer boundaries. E's modern FDID/shader-oriented layout is not Benilla's WoW1.12.1/build5875 contract, even when records have the same stride. Other root records, group geometry and collision are separate inventory slices; this page is not a whole-WMO completeness claim.

Snapshot: **2026-10-24**. E native baseline `354bdb7dbf177047ed46d63168acec531f5c4426`; B `8a6c74036cfe3d00275fd75ad1aeae1f4ea7d0df`. Roots/evidence vocabulary follow [[benilla-asset-field-inventory]]. Static local text only; no build, test, parser/client execution, extraction or visual/runtime proof.

Abbreviations: **T**=`E/godot/core/src/asset/wmo_format/parser_types.rs`; **P**=`parser.rs`; **C**=`parser_chunks.rs` in that directory; **S**=`E/godot/rust/src/wmo/scene.rs`; **B0**=`B/crates/benilla-wmo/src/lib.rs`; **BR**=`B/crates/benilla-formats/src/models/wmo/root.rs`.

## Framing and header

Root dispatch C:7–34,133–150 recognizes reversed FourCC tags and skips unknown chunks. The chunk tag and little-endian payload-size word are framing inputs, not retained root members. Fixed-entry parsing P:17–37 decodes complete records and ignores partial record tails; this does not preserve arbitrary unknown bytes. The source dispatch, rather than an ADT parser's similarly named iterator, governs this WMO path.

MOHD has64 bytes (T:446–460; C:152–159,103–110). Header counts do not imply that their values drive vector lengths: separate chunks are decoded by payload size.

| Serialized member | E retention and located use | B handling / limit |
|---|---|---|
| `nMaterials` `u32 +0` | Read as `_n_materials`, dropped; MOMT payload supplies actual records. | B0:187–188 comments call this `nTextures`; base parser does not retain it. No cross-version name equivalence inferred. |
| `nGroups` `u32 +4` | Retained `n_groups`; native group-file enumeration `E/godot/rust/src/wmo/assets.rs:106–125`. | Retained B0:23–24,187–188; BR:64–70,135–154 and `models/wmo/mod.rs:14–30` enumerate legacy indexed filenames. |
| `nPortals` `u32 +8` | Read, dropped; portal tables use their payloads. | Base header parser does not retain count; separate portal adapter reads chunk data. |
| `nLights` `u32 +0x0c` | Read, dropped; MOLT payload parsed independently. | Header count not retained; separate `models/records.rs` MOLT adapter exists. |
| `nModels` `u32 +0x10` | Read, dropped; MODI/MODN tables independent. | Not retained by base header parser. This does not imply that B's separate doodad adapter lacks model names. |
| `nDoodads` `u32 +0x14` | Read, dropped; MODD records from payload. | Base count not retained; BR:327–369 separately decodes placements/sets. |
| `nSets` `u32 +0x18` | Read, dropped; MODS records from payload. | Header count not retained; separate set adapter exists. |
| Ambient BGRA `u32 +0x1c` | Normalized RGBA retained; S:339 and `wmo/doodad_light.rs:57` include root ambient. | Not represented by B0 root result. No broader B ambient-policy claim. |
| WMO ID `u32 +0x20` | Read `_wmo_id`, dropped by header application. | BR:319–324 extracts raw ID; `B/crates/benilla-assets/src/wmo.rs:547–549` carries identity for WMOAreaTable lookup. |
| Bounds minimum `Vec3 +0x24` | Retained root corner; no precise production consumer established in this bounded header audit. | Not retained by B0 root result; separate group bounds are not this field. |
| Bounds maximum `Vec3 +0x30` | Retained independently; same bounded consumer limit. | Same distinction. |
| Flags `u16 +0x3c` | Projects bits1/2/4/8 to `do_not_attenuate_vertices`, `use_unified_render_path`, `use_liquid_type_dbc_id`, `do_not_fix_vertex_color_alpha` (T:194–209). Other raw bits not represented by these booleans. Consumer semantics not established solely by parsing. | Not retained by B0 header parser. |
| `nLod` `u16 +0x3e` | Read `_n_lod`, dropped. | Not retained by base header parser. |
| Bytes after64 | Not modeled by this header decoder. | No opaque-tail preservation established. |

## MOMT: all sixteen32-bit words

E64-byte raw record T:462–479, conversion P:72–98. B's selected legacy fields are B0:27–48,194–208. **E use** below refers to core mesh predicates and native material binding, with separate terrain/sound consumers identified explicitly; a bounded unknown is not global absence.

| Serialized member | E retention / production consumer | B handling / version difference |
|---|---|---|
| `flags` `u32 +0` | Raw word retained plus normalized `material_flags` (T:177–190). S:560–561,593–610 uses unlit/unfogged/cull/clamp; core `mesh_data.rs:103–176` consumes material predicates. | Raw flags retained; legacy material/renderer policy differs. |
| `shader` `u32 +4` | Retained; S:276–296,359–360 maps/rejects shader IDs; core mesh selects needed UV/color/tangent streams. | Not retained in legacy `Material`. Equal stride does not imply equal shader support. |
| `blend_mode` `u32 +8` | Retained; S:361–365 rejects outside0–3, :573 binds uniform, :598–610 selects blended variant for2/3. | Retained legacy blend mode. |
| `texture_1` `u32 +0x0c` | Retained FDID; texture slot0. S:537–549 loads/binds only sampled slots; zero/unselected slots use black. | Legacy MOTX **byte offset**, resolved to name via B0:216–234; not a modern FDID. |
| `sidnColor` BGRA `+0x10` | Retained normalized `sidn_color`; no use established in inspected core mesh/S paths. S:552–580 does not bind it as active emission. | Retains RGB, discards alpha; SIDN metadata in `B/crates/benilla-assets/src/wmo.rs:498–500`, night curve/renderer described in parent inventory. |
| `frameSidnColor` BGRA `+0x14` | Raw word read then dropped by P:82–96. | Not represented in selected legacy material. |
| `texture_2` `u32 +0x18` | Retained FDID, sampled slot1. | Not retained in legacy material. |
| `diffColor` BGRA `+0x1c` | Retained normalized RGBA; native material base color is white, not this value (S:552–553). Its alpha **is** read by terrain material-priority heuristic (`E/godot/rust/src/terrain/assets.rs:729–747`). | RGB retained, alpha discarded; BR:98–115 and assets/wmo.rs:572–573 resolve interior-liquid color. Do not transfer Vanilla tint policy blindly. |
| `groundType` `u32 +0x20` | Retained; terrain heuristic reads whether nonzero (`terrain/assets.rs:729–747`), then classifies material textures. This is not exact face-ground-type footstep selection. | Retained; BR:88–95 and assets/wmo.rs:572–573 carry material ground types for face-based surface lookup. |
| `texture_3` `u32 +0x24` | Retained FDID, sampled slot2. | Not retained in legacy material. |
| `color_2` `u32 +0x28` | Retained `extra_texture_fdids[0]`; maps to slot3 for pixel shaders19/20. Storage/use is shader-specific, not proof of universal texture semantics. | Not retained in selected legacy material. |
| `flags_2` `u32 +0x2c` | Retained extras[1], slot4 for pixel19/20. | Not retained. |
| `runTimeData[0]` `u32 +0x30` | Retained extras[2], slot5 for pixel19/20. | Not retained. |
| `runTimeData[1]` `u32 +0x34` | Retained extras[3], slot6 for pixel20. | Not retained. |
| `runTimeData[2]` `u32 +0x38` | Retained extras[4], slot7 for pixel20. | Not retained. |
| `runTimeData[3]` `u32 +0x3c` | Retained extras[5], slot8 for pixel20. | Not retained. |

T:164–174 produces the nine ordered texture slots. S:90–98 sampling masks: pixel0/1/2/4/18→slot0; -1/3/5/6/8/9/12/13/16→0–1; 7/10/11/14/15/17→0–2;19→0,2,3,4,5;20→0–8. MOMT shader22/23 maps to pixel19/20 (S:52–55). Retention is therefore distinct from a particular shader sampling an extra word.

The eight normalized material booleans are `unlit`, `unfogged`, `unculled`, `exterior_light`, `sidn`, `window`, `clamp_s`, `clamp_t`. Direct production reads of the first three and clamps are above; no direct `sidn/window/exterior_light` read was established in the bounded mesh/S audit. Group exterior-light selection is a separate input. `uv_translation_speed` comes from MOUV, not a seventeenth MOMT word; no UV-motion consumer established in these material paths.

Both E and B iterate complete64-byte MOMT records; incomplete tails are not retained and no count-match validation against MOHD was established (P:17–37; B0:194–208).

## Sources

- E `godot/core/src/asset/wmo_format/{parser_types,parser,parser_chunks,mesh_data}.rs` — raw declarations, conversion and mesh predicates.
- E `godot/rust/src/{wmo/scene,wmo/assets,wmo/doodad_light,terrain/assets}.rs` — located native consumers.
- B `crates/benilla-wmo/src/lib.rs`, `crates/benilla-formats/src/models/wmo/root.rs`, `crates/benilla-assets/src/wmo.rs` — version-specific parser/adapter boundaries.

## See Also

- [[benilla-wmo-root-aux-fields]] — remaining root records and production consumers.
- [[benilla-wmo-extension-fields]] — modern extension fields and opaque intervals.
- [[benilla-wmo-group-fields]] — group mesh, liquid and BSP fields.

- [[benilla-asset-field-inventory]] — parent comparison, actionable differences and broader scope.
- [[wmo-format]] — engine WMO system/format reference, not independent original-client authentication.
- [[open-source-wow-clients]] — reference-client version and evidence limits.
