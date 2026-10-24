# Benilla / game-engine Terrain Water, WDT and WDL Fields

Individual serialized inputs, retained representations and located consumers for modern E MH2O, legacy B MCLQ, map selection WDT and distant-terrain WDL. These are different version contracts, not interchangeable water formats. **E modern water exists and has native geometry/material consumers.**

Snapshot: **2026-10-24**. E baseline `354bdb7dbf177047ed46d63168acec531f5c4426`; B `8a6c74036cfe3d00275fd75ad1aeae1f4ea7d0df`, matching [[benilla-wmo-root-fields]]. Static local source inspection only: no execution, imports, builds, tests, extraction, network or visual/runtime proof. Pins identify the requested comparison baselines; this page does not independently certify checkout identity. Index/log maintenance belongs to the parent inventory task.

## Source aliases and evidence boundary

E = `/syncthing/Sync/Projects/world-of-osso/game-engine`; B = `/home/osso/Repos/benilla`. Every alias below names an actual file; source-level module aliases are not filesystem paths.

| Alias | Actual file |
|---|---|
| EW | E `godot/core/src/asset/adt_format/adt_tex_water.rs` |
| EG | E `godot/core/src/asset/adt_format/adt_water_geometry.rs` |
| EN | E `godot/rust/src/terrain/water.rs` |
| EA | E `godot/rust/src/terrain/assets.rs` |
| ET | E `godot/core/src/asset/wdt.rs` |
| EO | E `godot/core/src/asset/adt_format/adt_obj.rs` |
| EH | E `godot/core/src/horizon.rs` |
| EHN | E `godot/rust/src/terrain/horizon.rs` |
| BA | B `crates/benilla-adt/src/lib.rs` |
| BL | B `crates/benilla-formats/src/liquid.rs` |
| BS | B `crates/benilla-world/src/liquid/surface.rs` |
| BT | B `crates/benilla-wdt/src/lib.rs` |
| BH | B `crates/benilla-formats/src/wdl.rs` |
| BHN | B `crates/benilla-world/src/wdl.rs` |

Offsets below are payload-relative unless otherwise stated; integers/floats are little-endian. Array members are named once with count/stride. “No located use” is bounded to these inspected paths, not a repository-wide absence claim. Opaque bytes and unhandled version gates receive no invented semantics.

## MH2O: E modern liquid

EW:8–36 declares 256 chunk headers (12-byte stride), 24-byte layer headers and optional 16-byte attributes. EW:316–375 validates the header table, reads attributes independently, and walks layers only when both instance offset and count are nonzero. Offsets point into the MH2O payload; they are control inputs, not retained pointers.

| Serialized member | Retention / interpretation | Located consumer or boundary |
|---|---|---|
| Chunk `instance_offset`, u32 +0 | Selects layer-header array; discarded after parsing. | Zero means no layers; EW:338–375. |
| Chunk `layer_count`, u32 +4 | Controls layer loop; original integer not retained separately. | Represented by `ChunkWater.layers.len()`; EN:61–70 iterates layers. |
| Chunk `attributes_offset`, u32 +8 | Zero → `None`; otherwise reads attributes. Offset discarded. | EW:355–364; attributes need not imply a layer exists. |
| Attributes `fishable`, u64 +0 | Retained; bit `y*8+x`, helper bounds x/y to 0..7. | Query helper EW:72–105; no read in inspected EN/EG mesh/material path. |
| Attributes `deep`, u64 +8 | Retained independently with same bit indexing. | Same bounded consumer limit; not a geometry-existence mask. |
| Layer `liquid_type`, u16 +0 | Retained. | Material key `(liquid_type, liquid_object)`, EA:299–323, EN:90–105. |
| Layer `liquid_object_or_lvf`, u16 +2 | Stored as `liquid_object`; <42 selects LVF; ≥42 identifies LiquidObject. | EA:311–323 resolves material LVF and calls `decode_object_vertices`; EN uses material key. |
| Layer `min_height`, f32 +4 | Retained. | EG:28–38 supplies missing-height value. |
| Layer `max_height`, f32 +8 | Retained independently. | No located read in EG/EN geometry/material path. |
| Layer `x_offset`, u8 +12 | Retained cell offset. | EG:60–72 adds to generated columns. |
| Layer `y_offset`, u8 +13 | Retained cell offset. | EG:60–72 adds to generated rows. |
| Layer `width`, u8 +14 | Retained cell count. | Vertex count `(width+1)*(height+1)`, mask decoding and quad loop. |
| Layer `height`, u8 +15 | Retained independently. | Same count/loop role on row axis; EW:108–145, EG:105–125. |
| Layer `exists_offset`, u32 +16 | Pointer discarded; zero synthesizes a full rectangular mask. | Nonzero reads packed bit `y*width+x` into eight row bytes; EG emits only set cells. |
| Layer `vertex_offset`, u32 +20 | Pointer discarded; zero gives empty vertex arrays. | EW:217–270 bounds available bytes to at most nine bytes per vertex. |
| Existence bit, one per cell | Packed stream, `ceil(width*height/8)` bytes; retained as `exists[8]`. | EG:24–26,111–117 gates quad emission, unlike fishable/deep. |
| Vertex height, f32 stride4 | Present in LVF0/1/3; retained in `vertex_heights`. | EG assigns mesh height, using `min_height` if absent/missing. |
| Vertex UV U, u16 +0 within stride4 UV array | Present in LVF1/3; retained normalized `/255`. | EG does not read authored UVs; generates grid UVs instead. |
| Vertex UV V, u16 +2 within stride4 UV array | Present in LVF1/3; independently normalized `/255`. | Same retained-but-unbound boundary in EG. |
| Vertex depth, u8 stride1 | Present in LVF0/2/3; retained in `vertex_depths`. | EG:40–50 normalizes `/255` into vertex color alpha; absent depth → 1. |
| LiquidObject raw vertex span | Retained as `object_vertex_bytes`; provisional LVF0 decode only if possible. | EA:311–323 re-decodes with resolved material LVF; failure becomes material error. |

EW:179–214 defines **sequential arrays**, not an interleaved vertex struct: LVF0 = heights then depths; LVF1 = heights then UV pairs; LVF2 = depths only; LVF3 = heights, UV pairs, depths. Each array has `N=(width+1)*(height+1)` elements. Other LVFs error; the ≥42 object-ID gate is not permission to assign unknown layouts.

EN:52–105 walks parsed chunks, requires authored chunk positions, builds EG geometry and binds the resolved native material. EN:403–454 transfers positions, up normals, generated UVs, depth colors and reversed triangle winding to Godot arrays. Native material/shader parameter binding resides in EN:159–206,264–376. This establishes source consumers, not rendered correctness or shader-specific depth response. The preliminary `/tmp` reports left LiquidObject re-decoding unknown; **EA:284–323 closes that handoff**. Fishable/deep and max-height use elsewhere remains unestablished.

## MCLQ: B legacy liquid

BA:487–545 locates reversed `QLCM`, bounds its payload by MCNK liquid size and steps blocks by `0x324`. Each block has eight header bytes, 81 eight-byte vertices, 64 tile flags and a trailing flow area. This does not establish E legacy-MCLQ support.

| Serialized member | B retention / interpretation | Located consumer or boundary |
|---|---|---|
| MCNK flags liquid-kind bits2–5 (`0x3c`) | Set-bit count determines expected block count, with minimum one if MCLQ exists. | BA:499–519 parse control; BL chooses kind from cell nibble, not these bits. |
| MCNK liquid offset, u32 header +0x60 | Locates MCLQ; not retained as liquid metadata. | BA:487–493. |
| MCNK liquid size, u32 header +0x64 | Bounds MCLQ payload; not retained as liquid metadata. | BA:487–493; own subchunk declared size is not used for payload length here. |
| MCLQ tag, four bytes | `QLCM` recognized. | Locator input, not retained. |
| MCLQ declared size word | Not the liquid payload bound in this reader. | BA comment documents zero; no extra field semantics inferred. |
| Block `min_height`, f32 +0 | Retained. | BL:199–210 uses it for invalid unreferenced vertex positions to avoid sentinel-sized bounds. |
| Block `max_height`, f32 +4 | Retained independently. | No located read in BL/BS meshing path. |
| Vertex height, f32 +4 within stride8, 81 entries | Absolute height retained. | BL:149–176 rejects cells with nonfinite/sentinel corners; BL:199–210 generates positions; BS:211–237 installs POSITION. |
| Vertex water/ocean depth, union byte0 | Raw union retained; `depth_byte()` exposes byte0. | BL:218–228 normalizes river depth by42, ocean by255 by default; BS packs depth into UV1.x. |
| Vertex remaining water/ocean union bytes1–3 | Raw bytes retained; BA comment calls union depth/flow. No separate named flow components decoded. | No located use in BL/BS; component meanings deliberately unassigned. |
| Vertex magma/slime S, u16 union +0 | Raw bytes retained; `texcoords()` decodes unsigned S. | Magma BL:214–222 scales by3/256 into UV0; slime has no ADT mesh queue in this path. |
| Vertex magma/slime T, u16 union +2 | Raw bytes retained; independently decoded unsigned T. | Same magma UV0 consumer; not proof of slime rendering. |
| Cell liquid-type nibble, low4 bits, 64 u8 stride1 at +656 | Raw flags retained; `0xf` dry. | BL:149–190 skips dry/invalid cells, builds wet coverage/indices and chooses majority kind/sound nibble. |
| Cell unnamed flag bits4–5 | Preserved in raw flag bytes. | BL mesher masks low nibble; no semantics assigned. |
| Cell fishable bit `0x40` | Preserved in raw flag bytes. | No fishing consumer located in BL/BS path; BA:123–125 comment is not independent proof of server behavior. |
| Cell shared bit `0x80` | Preserved in raw flag bytes. | BL:229–237 sets ADT `shared` all false; no bit-driven ADT render use established. WMO MLIQ is separate. |
| `nFlow`, u32 +720 | Documented by BA block-layout comment, not read or retained. | Reader minimum ends after flags at720 bytes; no downstream field exists here. |
| Flow record bytes, two40-byte slots at +724, stride40 | Documented trailing span, not read or retained. Individual component layout absent from this decoder. | Opaque here; no invented sphere/direction/velocity members or downstream flow claim. |

BA:90–127 defines raw union/depth/UV accessors. BL:139–237 builds one `LiquidMesh` per accepted block: magma uses authored UVs and zero depth; water/ocean generate quarter-cell UVs and depth ramps. Ocean divisor has an environment override in BL:126–137;255 is the default, not an unconditional value.

The located source chain is **BA → BL → B `crates/benilla-world/src/terrain_stream.rs:772–780` → BS:143–190,211–237**. Streaming passes chunk liquids to `spawn_liquids`; BS selects ADT liquid material, installs positions/UV0/depth UV1/indices and carries sound nibble into `LiquidSoundSource`. BS:38–39 describes the depth-swatch lane; material binding begins around333. Thus heights/depth/UVs/tile nibbles are not merely parser-only. Fishable/shared conclusions remain bounded; flow data is unretained. Original-client addresses/comments are repository assertions, not independently authenticated client behavior.

## WDT: recognized map-selection inputs

ET parses MPHD, MAIN, MAID, MWMO and MODF through the ADT chunk iterator. BT:149–236 walks chunks and handles MPHD, MAIN, MWMO and MODF. WDT is not the liquid geometry payload. E `godot/core/src/wdt.rs` is a re-export, not a second decoder.

| Serialized member | E handling | B handling |
|---|---|---|
| Chunk tag, four bytes | Reversed FourCC selects relevant payload; unknown tags skipped by iterator. | Reversed FourCC selects handler; others seek-skipped. |
| Chunk size, u32 | Bounds payload/iteration. | Bounds general payload reads/seeks; MAIN handler reads fixed32768 bytes rather than validating declared size. |
| MVER version, u32 payload +0 | Not interpreted/validated by these WDT entry points. | Not a recognized handler; skipped, no version gate established. |
| MVER remaining payload | Not retained here. | Skipped, unassigned. |
| MPHD first word, u32 +0 | Retained whole as `MphdFlags.raw`. | Only bit0 retained as `one_object`; remaining bits dropped. |
| MPHD bit0 `0x1` | Gates global-WMO parse. | Gates global placement together with nonempty MWMO and MODF. |
| MPHD bit2 `0x4` | Derives big-alpha selection. | No interpretation in BT. |
| MPHD bit7 `0x80` | Derives height-texturing and also big-alpha. | No interpretation in BT. |
| MPHD other first-word bits | Retained raw, no individual interpretation in ET. | Discarded after bit0 test. |
| MPHD bytes after first word | Unread; no names assigned to trailing words. | Payload read, trailing bytes uninterpreted. |
| MAIN entry flags, u32 +0, stride8 ×4096 | Only bit0 determines active tile; other bits dropped. | Only bit0 retained as bool grid; other bits dropped. |
| MAIN entry second word (`area_id` in BT comment), u32 +4 | Unread; no area semantics implemented here. | Unread. |
| MAID root FDID, u32 +0, stride32 ×4096 | Retained for active MAIN slots. | Unrecognized chunk; skipped. |
| MAID obj0 FDID, u32 +4 | Retained for active slots. | Skipped. |
| MAID obj1 FDID, u32 +8 | Retained for active slots. | Skipped. |
| MAID tex0 FDID, u32 +12 | Retained for active slots. | Skipped. |
| MAID lod FDID, u32 +16 | Retained for active slots; direct downstream use not established here. | Skipped. |
| MAID map_texture FDID, u32 +20 | Retained; downstream use not established here. | Skipped. |
| MAID map_normal FDID, u32 +24 | Retained; downstream use not established here. | Skipped. |
| MAID minimap FDID, u32 +28 | Retained; downstream use not established here. | Skipped. |
| MWMO name bytes / NUL delimiters | Last matching names payload selected; derives string-start offsets for MODF name indices. | First NUL-terminated path, lossy UTF-8; later strings ignored. |

ET:124–175 requires MAIN exactly4096×8 and optional MAID exactly4096×32 bytes, including inactive slots in size validation; only active-slot IDs are decoded. Slot index derives coordinates, not a serialized extra member. EA:168–171 calls all three WDT parsers; EA:495 locates tile file IDs. This is bounded source linkage, not exhaustive consumer proof for every retained MAID word.

### Global-WMO MODF: 64-byte entry

EO's raw record and EO:301–345 decoding are shared with ADT objects; ET:71–83 keeps only the first parsed placement and normalizes its path. BT:208–236 reads selected offsets from a single retained payload. Three-component arrays name one member once, with f32 component stride4.

| Serialized member | E handling | B handling |
|---|---|---|
| `name_id`, u32 +0 | Retained; flag0x8 selects FDID, otherwise indexes derived MWMO starts. | Unread; global model comes directly from first MWMO path. |
| `unique_id`, u32 +4 | Retained in placement. | Unread. |
| `position[3]`, f32 +8 | Retained. | Retained as raw position. |
| `rotation[3]`, f32 +20 | Retained. | Retained as raw rotation. |
| Lower bounds[3], f32 +32 | Retained as `extents_min`. | Unread span; no derived bound. |
| Upper bounds[3], f32 +44 | Retained separately as `extents_max`. | Unread span. |
| `flags`, u16 +56 | Retained;0x8 name interpretation,0x4 authored scale. Other bits unassigned here. | Unread, no flag semantics applied. |
| `doodad_set`, u16 +58 | Retained. | Retained. |
| `name_set`, u16 +60 | Retained. | Retained. |
| `scale_raw`, u16 +62 | If flag0x4, `/1024`; otherwise derived1.0. | Unread; no scale member in global result. |

E decodes complete MODF entries before choosing first; partial tails are not fields. B accepts payload size≥64, uses first entry's selected offsets and leaves later bytes uninterpreted. No uninspected global-spawn behavior is claimed.

## WDL: MAOF / MARE / MAHO

The actual E paths are **EH `godot/core/src/horizon.rs` and EHN `godot/rust/src/terrain/horizon.rs`**, not hypothetical asset/WDL or monolithic terrain files. B parser BH is separate from consumer BHN.

| Serialized member | E handling / source consumer | B handling / source consumer |
|---|---|---|
| Top-level tag, four bytes | EH:132–150 scans to first reversed `FOAM` MAOF; other chunks skipped. | BH:205–233 requires first MVER, then scans to MAOF; intervening bodies skipped. |
| Top-level size, u32 | Bounds payload and next-chunk position. | Bounds skips; MAOF must equal16384 bytes. |
| MVER version, u32 | No version validation in EH. | BH:209–215 requires18, does not retain version. |
| MVER tail bytes | Uninterpreted while scanning. | Seek-skipped. |
| MAOF offset, u32 stride4 ×4096 | At least16384 bytes; zero absent, nonzero points to tile header. Extra table bytes ignored. | Exactly16384 bytes; zero leaves `None`, nonzero seeks to tile. |
| MARE tag, four bytes | Requires reversed `ERAM`. | Requires MARE after header decoding. |
| MARE size, u32 | At least1090; complete size locates possible following MAHO. | At least1090; reads only first1090 bytes. |
| MARE outer height, i16 stride2 ×289 at +0 | EH:49–85 converts17×17 corner heights into position Y. | Retained `outer[289]`; BH:111–119 uses mesh height Z before coordinate conversion. |
| MARE inner height, i16 stride2 ×256 at +578 | EH:49–85 converts16×16 cell-centre heights into position Y. | Retained `inner[256]`; BH:121–128 uses centre heights. |
| MARE bytes after1090 | Unread, skipped by full declared size when probing MAHO. | Unread; next tile independently sought via MAOF. |
| MAHO tag, four bytes | Probed only immediately after complete MARE; recognizes reversed `OHAM`. | No MAHO handler in BH:202–264. |
| MAHO size, u32 | Bounds mask; if <32 or absent/truncated/different tag, zero masks used. | Not decoded as MAHO metadata. |
| MAHO row mask, u16 stride2 ×16 | Retained transiently; column bit selects unmarked/marked emission pass. **Both passes emit faces: reorder, not cull.** | No retained mask; fixed fan for every present tile. |
| MAHO bytes after32 | Unread. | Not parsed. |
| Further tile data / other chunk bodies | No additional per-tile decoding established. | No additional per-tile decoding established. |

EH:88–123 emits four triangles per cell, unmarked first then marked, for all256 cells. EHN:136–166 transfers positions and indices into native mesh arrays. EHN:77–89 separately checks loaded-ADT membership and camera distance; these are derived visibility conditions, not MAHO hole culling.

BH:100–150 generates545 vertices and3072 fan indices per present tile. BHN:161–207 selects the camera/focus ring, calls `tile_mesh`, converts coordinates and installs mesh attributes. Different ring/loaded-ADT policies are source observations, not visual-parity findings. No inferred meaning is assigned to skipped WDL low-detail-WMO chunks.

## Coverage and remaining gaps

- Individual known decoded header words, layer fields, vertex attributes and masks above have static declaration/read evidence; distinct min/max and MODF bound members remain separate. Flow submembers are not modeled by BA: only the documented count and two opaque record spans can be named from this decoder.
- E LiquidObject material-format handoff is located in EA, correcting the preliminary leads. Shader-specific depth-alpha effects and other fishable/deep/max-height consumers remain unverified.
- B MCLQ heights/depth/magma UV/type-nibble reach mesh/material paths. Fishable/shared use beyond the bounded ADT path is unknown; trailing flows are unretained, not a hidden supported feature.
- E legacy MCLQ, whole-version compatibility, opaque WDT/WDL extensions and exhaustive WDT world-placement/MAID consumers are outside this page's demonstrated coverage.
- No runtime, visual, asset-corpus acceptance or original-client authentication proof was performed.

## Sources

- EW:8–105,108–228,250–375; EG:24–72,105–125 — MH2O fields, layouts, masks and geometry.
- EA:284–323,663–669; EN:52–105,159–206,264–376,403–454 — resolved LiquidObject LVF, native geometry/material binding.
- BA:90–127,487–545; BL:126–237 — MCLQ raw fields, block stride, unread flows and mesh conversion.
- B `crates/benilla-world/src/terrain_stream.rs:772–780`; BS:38–39,143–190,211–237,333 onward — streaming, ADT spawn and mesh/material lanes.
- ET:12–175; EO:301–345; BT:149–236 — WDT recognized inputs and MODF projection.
- EH:25–150; EHN:55–89,136–166; BH:100–150,202–264; BHN:161–207 — WDL offsets/heights/masks and native/Bevy mesh consumers.
- Local leads `/tmp/adt-liquid-fields.md`, `/tmp/wdt-fields-final.md`, `/tmp/wdl-fields-final.md`, `/tmp/terrain-water-integration.md`, `/tmp/b-terrain-consumer-closure.md` — navigation aids, not substitutes for inspected source; LiquidObject uncertainty corrected above.

## See Also

- [[benilla-asset-field-inventory]] — parent version-qualified comparison and broader field inventory.
- [[benilla-wmo-root-fields]] — shared pins, WMO material and separate interior-liquid context.
- [[adt-format]] — terrain format context, not proof of equivalent legacy/modern water contracts.
- [[open-source-wow-clients]] — reference-client provenance and evidence limits.
