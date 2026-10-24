# Benilla / game-engine WMO Root Auxiliary Fields

Individual ROOT MOGI/MOGN/MOSB/MOPV/MOPT/MOPR/MODS/MODN/MODI/MODD/MOLT/MFOG inputs, their raw-to-helper projections, and located production consumers. Header/material fields belong to [[benilla-wmo-root-fields]]; group records and modern root extensions are outside this slice.

Snapshot: **2026-10-24**. E baseline `354bdb7dbf177047ed46d63168acec531f5c4426`; B pin `8a6c74036cfe3d00275fd75ad1aeae1f4ea7d0df`. Roots and evidence vocabulary follow [[benilla-asset-field-inventory]]: E=`/syncthing/Sync/Projects/world-of-osso/game-engine`, B=`/home/osso/Repos/benilla`. B targets WoW **1.12.1/build 5875**; E's modern layouts/FDID paths are not a claim of version equivalence. These pins are the supplied comparison baseline, not newly executed Git verification. All evidence here is **static local source inspection**, not passing tests, asset-corpus validation, visual fidelity or runtime observations. Original-client addresses in B comments are leads, not independently authenticated evidence.

## Source aliases and reading rules

| Alias | Exact path relative to E or B |
|---|---|
| T / P / C / F | `E/godot/core/src/asset/wmo_format/{parser_types.rs,parser.rs,parser_chunks.rs,fog.rs}` respectively |
| W / Q | `E/godot/core/src/wmo.rs`; `E/godot/core/src/asset/wmo_format/portal.rs` |
| A / D / N / S | `E/godot/rust/src/wmo/{assets.rs,doodad_light.rs,portals.rs,scene.rs}` respectively |
| O / L | `E/godot/rust/src/terrain/objects.rs`; `E/godot/rust/src/lighting/mod.rs` |
| BR / BA / BL | `B/crates/benilla-formats/src/models/wmo/root.rs`; `B/crates/benilla-assets/src/wmo.rs`; `B/crates/benilla-formats/src/models/records.rs` |
| BS / BP / BF / BK | `B/crates/benilla-world/src/terrain_stream/spawn/mod.rs`; `B/crates/benilla-world/src/wmo_portal/mod.rs`; `B/crates/benilla-world/src/wmo_portal/fog.rs`; `B/crates/benilla-world/src/skybox.rs` |

Offsets below are **payload-relative for string blobs, record-relative otherwise**, hexadecimal. Multicomponent named members stay one row, with component offsets specified. Integers/floats are little-endian; BGRA bytes become RGBA floats in E (P:467–474). A derived byte offset, vector length or array index is not another serialized field. **Retained** means present in the returned model; **used** names an inspected consumer; **unknown** is bounded to the stated paths, never repository-wide absence.

E dispatch C:7–34,133–150 recognizes reversed chunk tags; C:99–130 finalizes the cooked root. Fixed records use floor(payload length / stride), dropping partial tails (P:17–37); MODI likewise uses `chunks_exact(4)` (P:155–161). B's BR fixed-record adapters use `as_chunks::<N>().0` (BR:179–196,222–264,294–317,330–369), likewise not retaining incomplete tails. No count validation against MOHD is established for these adapters. Unknown chunks are skipped, not retained as opaque byte blobs. B's separate legacy base reader is not the complete B pipeline: `benilla-wmo/src/lib.rs:178–213` handles MOHD/MOTX/MOMT, while BR:135–155 adds doodads, group metadata, portals, fog and skybox, and BL:268–305 independently adds MOLT.

## Group directory and skybox

MOGI stride32: T:613–619; conversion C:332–343. MOGN is a variable string blob: P:148–152,241–260. MOSB is a single string: C:32–33,277–280 (`apply_mobs_chunk` is the function spelling, not a different on-disk chunk).

| Chunk / individual named field: offset, disk type | E retention → consumer or bounded unknown | B handling → consumer or bounded unknown |
|---|---|---|
| MOGI `flags`: +0x00, u32 | Copied whole into `WmoGroupInfo.flags` (C:332–343). Native N:196–220 takes bounds from MOGI but exterior status from **MOGP**, not this word; no direct MOGI-flags consumer established in N/S. | BR:179–196 projects `(flags & 0x48)==0` to `interior`, bit0x40000 to `show_skybox`; raw flags/other bits lost. `interior` drives MODR-owned doodad bases (BA:314–350). The production skybox resolver instead tests group-nav flags (BK:145–161); do not mistake its same-named bit for a direct read of this MOGI helper. |
| MOGI `bbox_min`: +0x04, f32[3] (x/y/z +4/+8/+0xc) | Retained; N:196–220 axis-converts/reorders min/max for camera interior-group bounds; N:179–192 also requires a floor below camera. | Retained BR:183; BA:567 carries `group_bounds`. Downstream exact use of these MOGI bounds beyond that asset boundary remains untraced here; not equivalent to render/collision-derived bounds. |
| MOGI `bbox_max`: +0x10, f32[3] (x/y/z +0x10/+0x14/+0x18) | Retained; same N:196–220/179–192 bounds path, independently from minimum. | Retained BR:184; BA:567; same bounded downstream gap. |
| MOGI `_name_offset`: +0x1c, u32 in E (B comment calls i32) | Read into raw field then dropped (T:613–619; C:332–343). No MOGI→MOGN name join survives this conversion. | BR:179–196 uses 32-byte stride but never reads this word; no retained name offset in `WmoGroupInfo` (BR:41–50). Signedness is a source-description difference, not validated cross-version semantics. |
| MOGN string bytes: variable offset, NUL-delimited bytes | Lossy UTF-8 string plus computed original start offset stored as `WmoGroupName` (P:148–152,241–260; T:253–256). No production name consumer established in N/S/A. | No MOGN reader in the inspected BR composition or legacy base dispatch; this is a parser-path limit, not a global B absence claim. |
| MOGN terminator: end of each name, u8=0 | Consumed delimiter, not retained. P:241–253 stops at an empty entry; a missing final NUL is accepted by P:57–64. | No handling established in those same B readers. |
| MOSB skybox path: +0 through first NUL/end, bytes | `skybox_wow_path: Option<String>` retained by C:277–280/130; P:57–64 does not trim or normalize model suffix. No production MOSB binding established in inspected A/N/S/L paths; L:227–230 synchronizes **Light-derived skyboxes**, not this root member. | BR:158–169 parses UTF-8, trims, rejects empty and passes through `model_path` (.mdx→.m2 per adapter contract). BA:556 retains it. BK:114–174 resolves it against visible group-nav SHOW_SKYBOX and crossfade; BK:177 onward builds the requested model. Static production wiring, not fresh rendering proof. |
| MOSB terminator/trailing bytes: first NUL and remainder | Delimiter/remainder not retained; empty prefix→None (P:57–64; C:277–280). | Delimiter/remainder not retained; four zero bytes produce no name (BR:158–169). |

The MOGN/MODN helper `offset` is computed from string traversal, not a u32 embedded before each string. E advances by the **decoded String's** byte length +1 (P:241–253); lossless offsets for malformed UTF-8 are therefore not established. Leading/embedded empty entries stop that traversal. B MODN resolution below slices raw bytes by MODD's requested offset instead; these are not equivalent malformed/empty-entry policies.

## Portal pool, planes and graph

MOPV stride12 uses `parse_vec3_array` (C:252–255; P:453–455). MOPT stride20 and MOPR stride8 declarations T:595–609; conversion/finalization C:291–330. B paired declarations/decoders BR:198–264. E's temporary MOPV pool is **not MOVV/visible_block_vertices**.

| Chunk / individual named field: offset, disk type | E retention → consumer or bounded unknown | B handling → consumer or bounded unknown |
|---|---|---|
| MOPV vertex: +0, f32[3] (x/y/z +0/+4/+8) | Temporary `portal_vertices` pool; copied into each cooked portal polygon by C:306–317, then pool itself omitted from root (C:99–130). N:68–72 axis-converts polygons; N:156–175 tests near-plane/frustum visibility. F:87–89,133–147 uses polygon distance for fog exit weighting. | Raw triples retained as `WmoPortals.vertices` (BR:226–235), BA:551 copies pool. BP:690–714 uses polygon/plane/edge traversal; BK's visible-group gate is downstream of PVS, not an alternate MOPV parser. |
| MOPT `start_vertex`: +0x00, u16 | Retained only in temporary `(start,count)` range (C:291–303); C:306–317 expands to vertices. No cooked start field. | Retained `start_vertex` BR:210–214,236–247; BA:552 copies infos; BP traversal uses indexed portal polygons. |
| MOPT `vert_count`: +0x02, u16 | Same temporary projection; end clamped to pool length, out-of-range start leaves empty polygon (C:306–317). No cooked count field. | Retained as `count` in same BR adapter; not replaced by expanded polygon length. |
| MOPT `normal`: +0x04, f32[3] (x/y/z +4/+8/+0xc) | Copied into `WmoPortal.normal` (C:296–299). Core helper Q:5–16 uses it for camera-side tests, Q:75–91 uses side to recurse. **Native N instead recomputes a plane from vertices** (N:256–274); no native use of this stored normal established in inspected N path. | Retained as normal components `plane[0]`, `[1]`, `[2]` BR:245; BP:703–711 directly computes signed eye distance from this plane. |
| MOPT `_unknown`: +0x10, f32 | Read then dropped (T:595–600; C:291–303). Unknown meaning in this E raw declaration, not an assertion that bytes have no format semantics. | Retained as `plane[3]` (distance) BR:207–214,245. BP:703–711 adds it in normal·eye+d. B interpretation is version-qualified evidence, not permission to relabel E without target-version validation. |
| MOPR `portal_index`: +0x00, u16 | Retained C:319–330. N:233–253 groups links by portal; F:87–89/133–147 uses it to choose exit polygons. | Retained as `portal` BR:253–260; BP:690–714 resolves info by index. |
| MOPR `group_index`: +0x02, u16 | Retained. Native N:233–253 connects all distinct group IDs sharing a portal; those links feed BFS N:130–154. | Retained as neighbour `group` BR:253–260; BP traversal follows the edge's neighbour rather than reconstructing it solely from same-portal group IDs. |
| MOPR `side`: +0x04, i16 | Retained; Q:79–90 has a sign-gated helper consumer. Native N:233–253 adjacency construction does **not read side**; native visibility checks polygon/frustum instead. Helper support is not production proof for side gating. | Retained BR:258; BP:703–711 flips signed plane distance when side<0 and rejects the wrong half-space. |
| MOPR `_padding`: +0x06, u16 | Read, dropped by C:319–330; no cooked slot. | Unread bytes in each 8-byte BR stride, not retained (BR:249–260). |

## Doodad sets, model resolution and placements

MODS stride32: T:495–502/P:130–141. MODD stride40: T:504–512/P:224–239. MODN variable strings: P:142–146,241–260. MODI u32 table: P:155–161. All enter the root through C:173–196/114–118. **B MODS name is dropped; B MODD transform/color is not absent.** BR:327–369 bypasses the base reader and resolves raw MODN offsets itself.

| Chunk / individual named field: offset, disk type | E retention → consumer or bounded unknown | B handling → consumer or bounded unknown |
|---|---|---|
| MODS `name`: +0x00, u8[20] | Fixed C-string→String, bytes after first NUL lost (P:66–68,130–141); stored as `WmoDoodadSet.name` (T:242–246). No production name use established in W/A/O; activation is range/index-driven. | **Not read or retained**: helper only `start,count` (BR:32–35); BR:363–367 reads +20/+24, not name. Do not infer retention from a layout comment listing name[20]. |
| MODS `start_doodad`: +0x14, u32 | Retained; W:133–145 tests active set's half-open range against referenced MODD index; A:25–43 supplies active sets. | Retained `start` BR:364–365. BS:776–800 selects set0 plus nonzero placement-selected set, then skips/takes this range. |
| MODS `n_doodads`: +0x18, u32 | Retained; same W:133–145 saturating range end. | Retained `count`; BS:793–800 uses take(count). |
| MODS `_unused`: +0x1c, u32 | Raw read, absent from cooked set (T:495–502; P:130–141). | Unread trailing four bytes at stride32 (BR:359–369), not retained. |
| MODN model-path bytes: variable byte offset through NUL/end | String plus computed original byte offset retained (P:142–146,241–260). W:148–160 uses exact offset only if MODI vector is empty. A:25–43 derives placed doodads from group MODR and selected set. | BR:330–340 slices raw MODN using low24 MODD bits, resolves lossy UTF-8 and stores model String (BR:348–357). BA:564 copies defs; BS:802–805 skips empty model, BS:821–829 requests model and creates placement instance. |
| MODN terminator: end of name, u8=0 | Delimiter not retained; empty entry halts E table traversal, missing final NUL accepted (P:57–64,241–253). | BR:336–340 stops at NUL or blob end for each requested offset; offset outside blob returns empty String. Does not require offset be a pre-enumerated string start. |
| MODI FileDataID: +0x00 per4, u32 | Retained vector. MODD low24 becomes **array index**, not byte offset, when vector nonempty; zero/missing entry fails model resolution (W:148–155). | No MODI lookup in inspected BR:330–369 adapter; it always chooses MODN byte-offset resolution. This is a specific modern-versus-Vanilla path distinction, not blanket B doodad absence. |
| MODD `name_index_and_flags`: +0x00, u32 | Raw word split: low24→`name_offset`, high8→`flags` (P:224–239). Low24 consumed W:148–160 as MODI index or MODN offset. Flags retained W:125 and consumed D:15–20,108–242 for light policy. Raw packed word itself not retained. | BR:347–349 masks low24 and resolves MODN; resulting model retained, numeric offset and top-byte flags discarded. B helper has no flags field (BR:14–27). |
| MODD `position`: +0x04, f32[3] (x/y/z +4/+8/+0xc) | Retained; W:116–120 maps (x,y,z)→(x,z,-y). O:1088–1095 composes world/model transform; actual placement and light binding O:1080–1101. | Retained BR:350; BS:805 converts via `wmo_doodad_local`, BS:823 composes placement world transform; BS:630–650 passes that transform to production model assembly. |
| MODD `rotation`: +0x10, f32[4] (x/y/z/w +0x10/+0x14/+0x18/+0x1c) | Retained; W:164–176 normalizes quaternion and conjugates basis, used as doodad rotation W:121/O:1092–1094. | Retained `orientation` BR:351; same BS:805/823 transform path, not an opaque unread span. |
| MODD `scale`: +0x20, f32 | Retained W:122; O:1081 sets model scale and O:1092 includes it in world/model transform. | Retained BR:352; BS:805/823 transform path. |
| MODD `color`: +0x24, BGRA u8[4] (B/G/R/A +0x24/+0x25/+0x26/+0x27) | Retained as normalized RGBA P:235. RGB feeds D:127–133; alpha is rounded back to a MOLT **index**,255=none (D:144–170,295–297), not shader opacity. Flags/MDDI modify lighting D:173–242; O:1097 binds derived light. | Retained as reordered RGBA bytes `[38,37,36,39]` BR:353–355; BA:340–348 uses **RGB** for interior ambient/diffuse. Alpha survives raw adapter/asset but no alpha consumer established in this inspected lighting/spawn path. |

W:99–130 deduplicates MODR references by MODD index and carries all owning groups. This join, plus MODS selection, determines E placement eligibility; merely retaining all MODD entries does not mean every entry is spawned. A:25–43 computes each selected doodad's light, then O's model placement path binds it. B's BS:770–829 resolves set ranges and queues `WmoDoodadInst`; its actual assembly call BS:630–650 receives the composed transform. Those source call chains establish production handling without asserting a client was run. B MODD RGB-to-light use is separate from MOLT's dynamic light path and from any alpha interpretation.

## MOLT: retained fields versus actual native lighting

48-byte record T:481–493; cooked WmoLight T:231–241; P:112–128 converts enum, boolean and color. E retains **all named nonpadding fields**, but the inspected native use is narrower. A:25–43 → D:77–97,108–170 → D:173–242,286–294 → O:1097/D:247–282 is the production doodad-light chain. S:339–340 delegates root ambient to D:46–73; that is not a consumer of every MOLT field.

| Individual named field: offset, disk type | E retention → production consumer or bounded unknown | B handling → production consumer or bounded unknown |
|---|---|---|
| `light_type`: +0x00, u8 | Enum from_raw maps1/2/3 to Spot/Directional/Ambient, all other values to Omni (T:213–228; P:117). Original unrecognized byte lost. No direct read established in D:108–294; do not call this native spot/directional-light support. | Raw u8 retained BL:252; `is_omni()` checks ==0 (BL:58–62). BS:808–817 filters group-referenced MOLT lights through it before constructing prop lobes. |
| `use_attenuation`: +0x01, u8 | Retained boolean!=0 (P:118), raw nonzero magnitude lost. No read established in inspected D path. | Retained boolean BL:254; no direct use established in BS:808–817, which passes attenuation distances regardless. |
| `_padding`: +0x02, u8[2] | Read by raw decoder, dropped by P:112–128. | Unread by BL:251–266, not retained. |
| `color`: +0x04, BGRA u8[4] | RGBA floats retained; **RGB used**, multiplied by intensity in D:286–294; normalized if maximum>1, otherwise incoming doodad color remains. Alpha retained but no read established in this D consumer. | RGB floats only BL:256–260; alpha byte unread/dropped. BS:812–817 multiplies RGB by nonnegative intensity into prop-lobe light. |
| `position`: +0x08, f32[3] (x/y/z +8/+0xc/+0x10) | Retained; D:226–241 axis-converts selected MOLT position into personal sun source. D:255–259 transforms source to world and derives direction toward model center. | Retained BL:261; BS:811 transforms it into world point for prop light. |
| `intensity`: +0x14, f32 | Retained; **used D:288** in MOLT RGB×intensity; not merely a parsed-but-unbound scalar. Distinct from per-doodad MDDI intensity D:116–123. | Retained BL:262; BS:812 uses max(0). Policy differs from E's selected-light normalization. |
| `rotation`: +0x18, f32[4] (+0x18/+0x1c/+0x20/+0x24) | Retained P:122, but no read established in inspected D/native scene paths. Name/semantic meaning of these four floats is not authenticated by retention. | BL:248–250 documents four other floats in this span; BL:251–266 does not read them. **Unread span**, not a parsed rotation quaternion in B. |
| `attenuation_start`: +0x28, f32 | Retained P:123; no use established in D:108–294. | Retained BL:263; BS:813 passes to prop-lobe attenuation input. |
| `attenuation_end`: +0x2c, f32 | Retained P:124; no use established in same D path. | Retained BL:264; BS:814 passes to prop-lobe attenuation input. |

MOLT array membership is not itself proof of a native point-light node per record. In E's traced path, MODD alpha chooses one record and MODD flags determine how its RGB/intensity/position affect M2 shader uniforms; D:272–282 writes ambient/direct/sun parameters. Other MOLT consumers outside these inspected paths remain bounded unknowns.

## MFOG: all ten named inputs and the native boundary

48-byte record T:516–527; ten fields copied P:263–281 into WmoFog (T:267–278). F:69–91 constructs a camera evaluator, F:149–165 projects **all ten** fields; F:97–130,172–221,248–270 evaluates flags/radii and both land/underwater triples. B BR:269–317 retains all ten in its own layout names, including raw color dwords. Neither retention nor an evaluator alone establishes native binding of every output.

| Individual named field: offset, disk type | E retention → evaluator / production outcome | B retention → located consumer / limit |
|---|---|---|
| `flags`: +0x00, u32 | Retained P:268/F:152. F:104–109 uses record0 volume/disabled-volume bits; F:186–195 filters bit1; other bits retained but semantics untraced here. | Retained BR:305. BF:43–48 filters bit1; remaining flag semantics not established in this selector. |
| `position`: +0x04, f32[3] (x/y/z +4/+8/+0xc) | Retained P:269/F:153; F:186–195 computes camera distance to candidate centers. | Retained `pos` BR:306; BF:48–50 computes camera distance. |
| `smaller_radius`: +0x10, f32 | Retained P:270/F:154; radial influence F:172–183/207–210. | Retained `radius_inner` BR:307; BF:56–62 derives radial weight. |
| `larger_radius`: +0x14, f32 | Retained P:271/F:155; radius admission F:193 and radial falloff F:172–183. | Retained `radius_outer` BR:308; BF:49–50 admission and BF:56–62 weight. |
| `fog_end`: +0x18, f32 | Retained P:272/F:157; land triple blended in F, then **native L:489–493** passes end to wmo_fog conversion. | Retained BR:309; BF:73–78 constructs land target and BF:66 blends end. |
| `fog_start_multiplier`: +0x1c, f32 | Retained P:273/F:158; land start_scalar blended then **L:491** converts it. | Retained `fog_start_scalar` BR:310; BF:77 target and BF:67 blending. |
| `color_1`: +0x20, BGRA u8[4] | Retained RGBA P:274; F:150/159 rounds RGB back to authored bytes, drops alpha from evaluator. L:481/489–502 blends authored land RGB and converts result to linear. | Raw u32 `color` retained BR:311; BF:73–89 extracts RGB from ARGB dword, alpha not in WmoFogTarget. |
| `underwater_fog_end`: +0x24, f32 | Retained P:275/F:161; **evaluated/blended** into WmoFogBlend.underwater (F:119–129,207–221). **Not bound by native L:477–505**, which reads only `.fog`. | Retained `uw_fog_end` BR:312. BF:73–79 produces land-only target; no underwater-field consumer established in this selector/native handoff. |
| `underwater_fog_start_multiplier`: +0x28, f32 | Retained P:276/F:162; underwater triple evaluated, same unbound native L boundary. | Retained `uw_fog_start_scalar` BR:313; same bounded BF land-only projection. |
| `color_2`: +0x2c, BGRA u8[4] | Retained RGBA P:277; F:150/163 copies RGB only into underwater evaluator triple. Alpha drops there; underwater RGB not selected by L:477–505. | Raw u32 `uw_color` retained BR:314; no read established in BF:33–97 target projection. |

### Production integration, not evaluator-only evidence

E A:84–93 constructs WmoFogVolume from root plus loaded MOGP headers; O:298–315 stores it per spawned WMO. N:119–127 transforms the camera to file-local coordinates, identifies the interior group using native bounds/floor logic, then calls F.camera_fog. O:1106–1112 chooses the first matching spawned WMO. `E/godot/rust/src/lib.rs:2643–2668` gets the world camera sample and passes the result to WorldLighting.sync; L:197–231 applies the land fog and binds scene lighting. L:474–505 explicitly does not port underwater fog: all ten raw inputs reach the evaluator, but its underwater result is not consumed by this native binder. Character-select also calls its objects.camera_fog before lighting (`E/godot/rust/src/character_select/background.rs:168`). This is source-wired integration, not a demonstrated runtime result.

E F:133–147 derives `dist_to_exit` from MOPR/MOPV polygons; WmoFogBlend.weight clamps distance×0.04 (F:33–37), feeding L:492. The containing-group choice is the native floor/bounds implementation, **not** proof of original-client BSP-equivalent selection. F also rejects exterior/exterior-lit groups and uses MOGP fog IDs; those group-file inputs are joins, not additional root fields.

B BA:555 copies fog records; BP:407 calls `select_wmo_fog(&model.fogs, n.fog_indices, eye_local)`. BF:33–79 produces the land target, seeded by record0, rejecting fewer than two records and blending referenced records nearest-last. `B/crates/benilla-world/src/lighting/resolve.rs:182` receives CameraWmoFog in the production lighting resolver. That establishes a native handoff; the remainder of that resolver's final fog staging and any underwater MFOG consumption outside BF are not expanded in this audit. B's land-only selector does not justify calling its raw underwater fields unparsed. E's volume mode and portal-distance weighting are not the same policy as BF's Vanilla selector.

## Raw fields versus helper projections

| Representation | What it really contains / does not contain | Located proof boundary |
|---|---|---|
| E `WmoGroupName` / `WmoDoodadName` | Derived byte offset plus decoded String; terminators and arbitrary raw blob bytes lost. | T:247–256; P:142–152,241–260; W:155–160 uses doodad offsets only. |
| E `WmoPortal` | Copied polygon plus retained normal; **no serialized start/count/final plane word** after finalization. | C:291–317; N:68–72/156–175 consumes polygon, Q consumes stored normal. |
| E `WmoPortalRef` | portal index, group index and signed side copied; pad lost. Native adjacency is a derived all-pairs connection among group IDs sharing a portal. | C:319–330; N:233–253. Native graph does not establish use of side. |
| E `WmoDoodadDef` → `WmoDoodad` → `DoodadLight` | Packed word split; model resolved, basis-converted transform, owning-group list and flags/color retained; lighting is derived from those and MOLT/MDDI/ambient. Not a second set of raw ROOT fields. | P:224–239; W:99–176; A:25–43; D:108–242/286–294; O:1080–1101. |
| B `WmoDoodadSet` / `WmoDoodad` | Set has **only start/count**. Doodad has resolved model, position, orientation, scale and RGBA; no MODD flags/raw name offset. | BR:14–35,327–369; BA:564–565; BS:770–829/630–650. |
| E FogRecord / WmoFogBlend versus B WmoFogTarget | E retains ten-field semantics in evaluator but projects colors to RGB and yields land+underwater+derived exit distance; native binder uses land only. B raw keeps ten fields, target projects land RGB/end/start only. | F:42–48,149–165,119–129; L:477–505; BR:269–317; BF:21–29,73–89. |

## Remaining bounded gaps

- MOGN names, retained E MOGI flags and MOSB production binding remain untraced in the named native paths; B's MOGI-bound downstream uses beyond BA are not expanded. These are not global absence claims.
- E MOLT type/use-attenuation/rotation/attenuation distances and color alpha have no consumer established in D's inspected production chain. B use_atten and MODD alpha similarly remain untraced after retention; B's MOLT +0x18..0x27 floats are explicitly unread by its light adapter.
- E MOPT final float is dropped; B calls it plane distance. Target-version meaning is unresolved. E stored normal/MOPR side have core helper uses but are not proven native culling inputs; native path demonstrably derives polygon planes and shared-portal adjacency instead.
- Native E underwater MFOG binding is absent **at L:477–505**, despite complete evaluator input use. B retains underwater raw fields but its inspected BF selector omits them; wider submerged-lighting equivalence is not established.
- No byte corpus, malformed-input run, original-client authentication, native rendering or performance validation was performed. Partial record tails, reserved words and string trailing bytes are not opaque retained payloads in the inspected conversions. Header/materials, modern extensions and group-file field completeness remain separate scopes.

## Sources

- E `godot/core/src/asset/wmo_format/{parser_types,parser,parser_chunks,fog,portal}.rs` — raw members/strides, loss boundaries, evaluator and helper-only portal uses.
- E `godot/core/src/wmo.rs`, `godot/rust/src/wmo/{assets,doodad_light,portals,scene}.rs` — model resolution, native selection, light derivation and portal construction.
- E `godot/rust/src/{terrain/objects,lighting/mod,lib,character_select/background}.rs` — actual placement, fog query, production lighting handoff and land-only binding.
- B `crates/benilla-{wmo/src/lib,formats/src/models/wmo/root,formats/src/models/records,assets/src/wmo}.rs` — separate base parser/raw adapters, ten-field fog, set-name loss, real transform/RGBA retention and asset transport.
- B `crates/benilla-world/src/{terrain_stream/spawn/mod,wmo_portal/mod,wmo_portal/fog,skybox,lighting/resolve}.rs` — production placement/assembly, portal-side plane use, land fog projection, skybox selection and lighting handoff. `/tmp/wmo-root-final.md`, `/tmp/wmo-doodad-portals.md` and `/tmp/b-wmo-root-corrections.md` were leads only; contradictory name/MODD claims were resolved against these source files.

## See Also

- [[benilla-wmo-root-fields]] — separate MOHD/MOMT inventory and the shared comparison pins.
- [[benilla-asset-field-inventory]] — parent comparison, evidence vocabulary and version limits.
- [[wmo-format]] — E format/system context, not independent client authentication.
- [[open-source-wow-clients]] — reference-client scope and provenance limits.
