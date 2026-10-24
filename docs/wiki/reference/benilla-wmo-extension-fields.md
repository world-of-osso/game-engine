# Benilla / game-engine WMO Root Extension Fields

Individual serialized fields and opaque intervals for MAVG, MAVD, MBVD, MNLD, MOVV, MOVB, MCVP, MOUV, GFID and MDDI, with raw-to-cooked retention and located consumers. These are E's modern extension layouts, **not Vanilla offsets inferred from Benilla**. MOGN/MOSB belong to [[benilla-wmo-root-aux-fields]]; their fact tables are deliberately not duplicated here.

Snapshot: **2026-10-24**. Comparison pins inherited from [[benilla-wmo-root-fields]]: E native baseline `354bdb7dbf177047ed46d63168acec531f5c4426`; B `8a6c74036cfe3d00275fd75ad1aeae1f4ea7d0df`. Local source text checked for this page; pins are comparison provenance, not a newly executed checkout verification. E=`/syncthing/Sync/Projects/world-of-osso/game-engine`; B=`/home/osso/Repos/benilla`. No project execution, imports, builds, tests, network, downloads, extraction or nested agents. Source tests mentioned below were read, not run; source use is not rendered/runtime proof.

**Anchors:** T=`E/godot/core/src/asset/wmo_format/parser_types.rs`; P=`parser.rs`; C=`parser_chunks.rs` in that directory; L=`E/godot/rust/src/wmo/doodad_light.rs`; A=`E/godot/rust/src/wmo/assets.rs`. Line ranges below refer to these actual files, not aliases for another parser's lines.

## Framing, retention and bounded evidence

C:7–34 dispatches reversed FourCCs. For this slice: MAVG=`GVAM`, MAVD=`DVAM`, MBVD=`DVBM`, MNLD=`DNLM`, MOVV=`VVOM`, MOVB=`VBOM` **and** `BVOM`, MCVP=`PVCM`, MOUV=`VUOM`, GFID=`DIFG`, MDDI=`IDDM`. Both MOVB spellings select the same handler; this does not establish a second record schema. C:133–150 skips unrecognized tags. Chunk tag/size framing is not an extra field in each record below.

All byte intervals are inclusive, decimal, relative to **one payload record**, not file offsets or Rust memory layout. Multi-byte primitives are little-endian. Raw `BinRead` declaration order and explicit strides establish offsets; no host struct padding is assumed. P:17–37 decodes `floor(payload_size / stride)` complete records and ignores any incomplete tail. GFID P:162–167 and MDDI C:202–208 instead use `chunks_exact(4)`, likewise dropping trailing 1–3 bytes. These tails are unretained opaque intervals, not preserved extension data. Each handler assigns its vector (C:166–168,202–253); repeated chunks replace the preceding vector rather than concatenate it.

**Color conversion** P:467–474 maps bytes `[B,G,R,A]` to `[R/255,G/255,B/255,A/255]`; alpha survives parsing even when a consumer takes only RGB. **Bounded unknown (U)** below means no production consumer located beyond parser/conversion/root retention in the inspected E paths. A literal search of `E/godot/rust/src/**/*.rs` for `visible_block_vertices`, `visible_blocks`, `convex_volume_planes`, `baked_ambient_box_volumes`, `dynamic_lights`, `uv_translation_speed` returned no matches. A broader `E/godot/**/*.rs` search located declarations, conversions and source tests. This is not proof of absence in every language, external client or repository.

## MAVG and MAVD — two chunks, one 48-byte record

Raw `RawWmoAmbientVolume` T:550–562; stride T:430; cooked `WmoAmbientVolume` T:295–304. Both call **the same** P:169–185 `parse_mavd`, but C:220–228 stores MAVG in `global_ambient_volumes` and MAVD in `ambient_volumes`; C:122–123 retains both independently. The following table enumerates every field of **each** chunk, not one record shared between their arrays.

L:57–73 **uses** the first MAVG whose `doodad_set_id` is in the active set list, otherwise first MAVG, otherwise first MAVD, otherwise MOHD ambient. Selection is list order, not distance, volume containment or active-set-list priority. For MAVD, only entry zero is selected and its set ID is not consulted by this path.

| Field in each MAVG/MAVD record | Bytes / disk type | Retention and actual use / bound |
|---|---|---|
| `position` | 0–11; `[f32;3]` (`x`0–3, `y`4–7, `z`8–11) | Copied unchanged; U. L does not spatially evaluate it. |
| `start` | 12–15; `f32` | Copied; U, no radius/falloff inference. |
| `end` | 16–19; `f32` | Copied; U. |
| `color_1` | 20–23; `[u8;4]` BGRA | Converted RGBA; L:66–72 **uses RGB** as ambient, and repeats it for all three channels when flag bit0 is clear. Parsed alpha is not used there. |
| `color_2` | 24–27; BGRA bytes | Converted RGBA; L:66–70 **uses RGB** as horizon only with flag bit0 set. Alpha not read there. |
| `color_3` | 28–31; BGRA bytes | Converted RGBA; same conditional **RGB use** as ground. Alpha not read there. |
| `flags` | 32–35; `u32` | Whole word retained; L:66 tests `flags & 1`. Other bits U. This bit controls three-color selection, not spatial inclusion. |
| `doodad_set_id` | 36–37; `u16` | Retained. **MAVG USED** by L:59–63 membership selection. **MAVD retained**, not read for selection in L:64. |
| `_padding` | 38–47; `[u8;10]` opaque | Raw read consumes all ten bytes; P conversion omits them. Dropped, no cooked field or consumer. |

The ambient values feed `LightInputs.ambient` at L:132–140 during interior doodad lighting. Selected color_1 RGB also feeds WMO surface batches: `scene.rs:181–198,216–223,339–340,385–393` prepares `interior_ambient`, `scene.rs:550–553` binds it, and `godot/shaders/wmo.gdshader:307` uses it in interior ambient lighting. That surface path takes only the selector's first RGB color, not separate color_2/color_3 or alpha. These are native consumers, not proof every surface/exterior doodad uses these colors or of visual parity.

## MBVD — 128-byte ambient box record

Raw `RawWmoAmbientBoxVolume` T:564–575; stride T:431; cooked T:306–315; conversion P:187–202; C:230–233 assigns `baked_ambient_box_volumes`, retained C:124. Every named input and opaque interval follows. U applies to **each retained field**; no native box-volume ambient evaluation established.

| Field | Bytes / disk type | Retention / consumer |
|---|---|---|
| `planes[0]` | 0–15; `[f32;4]` | Copied to cooked plane0; U. |
| `planes[1]` | 16–31; `[f32;4]` | Copied to plane1; U. |
| `planes[2]` | 32–47; `[f32;4]` | Copied to plane2; U. |
| `planes[3]` | 48–63; `[f32;4]` | Copied to plane3; U. |
| `planes[4]` | 64–79; `[f32;4]` | Copied to plane4; U. |
| `planes[5]` | 80–95; `[f32;4]` | Copied to plane5; U. |
| `end` | 96–99; `f32` | Copied unchanged; U. |
| `color_1` | 100–103; BGRA bytes | Converted RGBA; U, including alpha. |
| `color_2` | 104–107; BGRA bytes | Converted RGBA; U. |
| `color_3` | 108–111; BGRA bytes | Converted RGBA; U. |
| `flags` | 112–115; `u32` | Entire word retained; U, no located bit interpretation. |
| `doodad_set_id` | 116–117; `u16` | Copied; U. Do not transfer MAVG's located consumer to MBVD. |
| `_padding` | 118–127; `[u8;10]` opaque | Consumed in raw read, omitted by conversion; dropped. |

Each plane stores components0/1/2/3 at its start+0/+4/+8/+12, four bytes each. Source declares four floats, not separately named normal/distance members; their geometric interpretation is not authenticated here.

## MNLD — 60-byte new-light record

Raw `RawWmoNewLight` T:577–591; stride T:432; cooked `WmoNewLight` T:331–343; P:204–223 converts; C:235–238 assigns `dynamic_lights`, retained C:125. No opaque interior interval: named fields cover bytes0–59. This is distinct from MOLT/`WmoLight` and group MOLP/`WmoPointLight`; their consumers do not prove an MNLD consumer.

| Field | Bytes / disk type | Retention / consumer |
|---|---|---|
| `light_type` | 0–3; `i32` | Normalized to `WmoNewLightType`: exactly1→`Spot`, every other value→`Point` (T:317–329). Original non-1 discriminator lost; cooked value U. |
| `light_index` | 4–7; `i32` | Copied; U. No MOLT-index lookup established. |
| `flags` | 8–11; `i32` | Copied; U, no bit consumer located. |
| `doodad_set` | 12–15; `i32` | Copied; U; not the MAVG `u16` member. |
| `inner_color` | 16–19; BGRA bytes | Converted RGBA; U. |
| `position` | 20–31; `[f32;3]` | Copied (x20–23/y24–27/z28–31); U. No axis conversion in this reader. |
| `rotation` | 32–43; `[f32;3]` | Copied (components32–35/36–39/40–43); U. Not MOLT's four-float rotation; no angle/unit claim. |
| `attenuation_start` | 44–47; `f32` | Copied; U. |
| `attenuation_end` | 48–51; `f32` | Copied; U. |
| `intensity` | 52–55; `f32` | Copied; U. |
| `outer_color` | 56–59; BGRA bytes | Converted RGBA; U. |

## MOVV, MOVB and MCVP — separate visibility representations

MOVV uses P:453–455 `parse_vec3_array`, stride12 T:440, dispatched C:240–243. MOVB raw T:529–534, cooked T:280–283, stride4 T:427, conversion P:283–293, dispatch C:245–248. MCVP raw T:536–542, cooked T:285–289, stride20 T:428, conversion P:295–307, dispatch C:250–253. C:118–120 retains these as independent root vectors. All listed fields are U; neither naming nor retention proves visibility/culling integration.

| Chunk / field | Bytes / disk type | Retention / consumer |
|---|---|---|
| MOVV vertex `x` | 0–3; `f32` | Element[0] of `visible_block_vertices`; unchanged, U. |
| MOVV vertex `y` | 4–7; `f32` | Element[1]; unchanged, U. |
| MOVV vertex `z` | 8–11; `f32` | Element[2]; unchanged, U. |
| MOVB `start_vertex` | 0–1; `u16` | `visible_blocks[].start_vertex`; copied, U. |
| MOVB `vertex_count` | 2–3; `u16` | `visible_blocks[].vertex_count`; copied, U. No range-resolution step located. |
| MCVP `normal` | 0–11; `[f32;3]` | `convex_volume_planes[].normal`; copied, U. Components0–3/4–7/8–11. |
| MCVP `distance` | 12–15; `f32` | Copied, U; no normalization or plane evaluation here. |
| MCVP `flags` | 16–19; `u32` | Entire word copied, U. |

No interior padding is declared for these records. MOVV is not MOPV portal-vertex storage: the latter has a separate accumulator and portal-resolution step C:255–263,100–102.

## MOUV — 16-byte per-material transform

Raw `RawWmoMaterialUvTransform` T:544–548 and cooked helper T:291–293 contain one named array `translation_speed: [[f32;2];2]`, stride16 T:429. P:101–110 copies it; C:166–168 retains a temporary transform vector. The four serialized elements are explicit below; no invented layer/unit names.

| Array element | Bytes / disk type | Retention / consumer |
|---|---|---|
| `translation_speed[0][0]` | 0–3; `f32` | Copied to material array[0][0]; native consumer U. |
| `translation_speed[0][1]` | 4–7; `f32` | Copied to material array[0][1]; native consumer U. |
| `translation_speed[1][0]` | 8–11; `f32` | Copied to material array[1][0]; native consumer U. |
| `translation_speed[1][1]` | 12–15; `f32` | Copied to material array[1][1]; native consumer U. |

Finalization C:100–101 calls C:282–289 `apply_material_uv_transforms`: positional `zip` with MOMT materials sets `WmoMaterialDef.uv_translation_speed = Some(array)` (T:105). No material ID is serialized in MOUV. Extra transforms beyond material count are discarded; materials beyond transform count retain `None` initialized at P:87. This **helper use is established**, but not animated UV application by a native shader/material clock. No interior opaque bytes declared.

## GFID and MDDI — scalar arrays, positional metadata

| Chunk / serialized field | Bytes / disk type | Retention / actual use |
|---|---|---|
| GFID `file_data_id[i]` | `4*i`–`4*i+3`; `u32` LE | P:162–167 decodes to `Vec<u32>` using the size constant `MODI_ENTRY_SIZE=4` (T:424), **not a MODI record association**. C:215–218/121 retains `group_file_data_ids`. A:192–220 **uses** the first `n_groups` entries as group file IDs when enough exist; zero in that selected prefix errors. Surplus IDs remain stored but are not loaded by this path. |
| MDDI `intensity[i]` | `4*i`–`4*i+3`; `f32` LE | C:202–208 decodes directly, C:116 retains `doodad_intensities`; no raw struct. T:16–18 documents per-MODD indexing. L:125–147 **uses** `doodad.index` to retrieve the scalar, but only accepts it when MODD flag `USE_INTENSITY=0x10` (L:29) is set; missing entry or clear flag gives1.0. |

The `i` in each table row is **array position**, not an extra serialized index. MDDI contains no doodad flags; `doodad.index` and `doodad.flags` are companion inputs. GFID contains no serialized group-count/LOD header in this decoder. A:209–220 resolves legacy indexed filenames when the vector is shorter than `n_groups`; this is observed existing behavior, not a proposed compatibility policy. MDDI intensity feeds `LightInputs`, rather than becoming an independent light object. L:189–212 `adjust_lighting` computes `color * inputs.intensity.max(1.0)` unless `NO_INTENSITY_COLOR` suppresses it, then selects direct/ambient adjustment using MODD flags; thus retention is not a universal unconstrained multiplication of final light by the raw scalar. Neither array has interior padding; incomplete trailing bytes are dropped as above.

## Serialized records versus helpers / projections

`RawWmoAmbientVolume`, `RawWmoAmbientBoxVolume`, `RawWmoNewLight`, `RawWmoVisibleBlock`, `RawWmoConvexVolumePlane` and `RawWmoMaterialUvTransform` are disk readers. Their cooked counterparts T:280–343 are retained projections, **not another set of disk fields**. Color float components, `WmoNewLightType`, `Option<...>` for material transforms, and the root vectors do not add serialized words. MOVV/GFID/MDDI use arrays directly without corresponding named raw records. `WmoRootAccum`, dispatch function pointers and `apply_material_uv_transforms` are parser helpers; `DoodadLight`/`LightInputs` L:34–45,143–153 are native lighting results/inputs, not extension chunks. This distinction prevents double-counting declarations as independently authored fields.

## Benilla comparison boundary

B is explicitly a 1.12.1/build5875 client (`B/docs/METHOD.md:1–6`; `B/docs/MAP.md` crate roster). Its bounded base root parser `B/crates/benilla-wmo/src/lib.rs:177–214` handles MOHD/MOTX/MOMT in that function and skips other tags; its independent root adapter `B/crates/benilla-formats/src/models/wmo/root.rs:135–154` also calls doodad, group-info, portal, fog and skybox readers. Therefore absence from the base function alone is **not repository-wide absence**.

A literal search in `B/crates/**/*.rs` for the ten chunk names and reversed tags listed in Framing found only MOVV/MOVB mentions: `benilla-formats/src/models/records.rs:268–300` scans past zero-size MOVV/MOVB on its way to MOLT; `benilla-formats/tests/wmo_lights.rs:1–2` describes that regression. Those hits establish traversal context, **not decoded MOVV vertices/MOVB ranges**. No field-level counterpart for the modern extension tables above was located by that bounded tag search or in the two inspected root parser paths. Alternate names, other file types and runtime behavior were not exhaustively excluded. Modern 48/128/60-byte records must not be projected onto Vanilla layouts merely because both clients load WMO roots.

## Sources

- E `godot/core/src/asset/wmo_format/parser_types.rs:3–31,94–108,280–343,420–442,529–591` — retained members, helper/projection distinction, strides and serialized layouts.
- E `godot/core/src/asset/wmo_format/parser.rs:17–37,101–110,162–223,283–307,453–474` — complete-record parsing, conversion and BGRA normalization.
- E `godot/core/src/asset/wmo_format/parser_chunks.rs:7–34,59–131,166–168,202–253,282–289` — actual dispatch, independent arrays, finalization and material association.
- E `godot/rust/src/wmo/doodad_light.rs:29,34–73,119–160,189–212` — located MAVG/MAVD colors/set/flag and MDDI index/MODD-flag consumers; not inferred from parser retention.
- E `godot/rust/src/wmo/assets.rs:192–220` — GFID-selected group resolution and validation boundary.
- E `godot/rust/src/wmo/scene.rs:181–198,216–223,339–340,385–393,550–553`; `godot/shaders/wmo.gdshader:19,307` — selected ambient RGB feeds native surface batches and interior shader lighting.
- B `crates/benilla-wmo/src/lib.rs:177–214`; `crates/benilla-formats/src/models/wmo/root.rs:135–154`; `crates/benilla-formats/src/models/records.rs:268–300`; `crates/benilla-formats/tests/wmo_lights.rs:1–2` — bounded parser/adapter and traversal evidence.
- B `docs/METHOD.md:1–6`, `docs/MAP.md` — stated Vanilla version scope. `/tmp/wmo-root-extension-fields.md` was a working lead, corrected against actual source anchors rather than treated as authoritative.

## See Also

- [[benilla-wmo-root-fields]] — MOHD/MOMT fields and comparison pins; MOUV is not a seventeenth MOMT word.
- [[benilla-wmo-root-aux-fields]] — MOGN/MOSB and other auxiliary root inputs; owns their fact tables.
- [[benilla-asset-field-inventory]] — broader comparison and evidence vocabulary.
- [[wmo-format]] — engine WMO format/system reference.
- [[open-source-wow-clients]] — version-qualified external reference limits.
