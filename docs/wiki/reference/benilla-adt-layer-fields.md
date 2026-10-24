# ADT Terrain Samples and Texture-Layer Fields

Field-level static inventory of E root/`_tex0` inputs versus B's Vanilla monolithic path. E baseline `354bdb7dbf177047ed46d63168acec531f5c4426`; B `8a6c74036cfe3d00275fd75ad1aeae1f4ea7d0df`. No execution, imports, builds, tests, extraction or runtime/asset compatibility proof. Arrays describe one repeated field with explicit stride/count, not an assertion that every element is a separate source declaration.

## Sources and projection boundaries

**D**=`godot/core/src/asset/adt_format/adt.rs`, **P**=`.../adt/parsing.rs`, **T**=`godot/core/src/asset/adt_format/adt_tex.rs`, **M**=`godot/core/src/rendering/terrain/terrain_material_data.rs`, **N**=`godot/rust/src/terrain/`; **B**=`/home/osso/Repos/benilla/crates/benilla-adt/src/lib.rs`. E root `McnkData` D82–102 is cooked, not another serialized header. Public `Chunk` (`godot/core/src/adt.rs:30–47,77–140`) copies these inputs. B adapters/production shaders live outside its parser crate: absence of a renderer in `benilla-adt` is not absence of a consumer.

E recognizes reversed root subchunk tags P702–715; B uses header offset/magic lookup B386–498. MCNK header offsets and flags have their own worksheet; modern companion object references are separate from root MCRF.

## Root samples and emitters

| Chunk/member | Serialized bytes | E read → retention → actual consumer / limit | B counterpart and production boundary |
|---|---|---|---|
| MCVT `height[i]` | 145 LE f32, `4*i` | P MCVT reader/dispatch, D99; D399–419 adds base height and staggered-grid origin. Native mesh N`material.rs:231–238`, `mod.rs:154–200` installs positions | B414–428 retains up to145 values; `benilla-formats/src/terrain.rs:311–359` builds geometry; `benilla-assets/src/adt.rs:80–129` accumulates and installs mesh attributes (actual setters124–129) |
| MCNR `normal[i].x` | i8 at `3*i` | P49–73 divides127; resulting normalized vector includes this x; D100 → native NORMAL array at N`mod.rs:154–200` | B430–445 retains signed x; formats terrain315–355 and assets ADT86–129 produces/installs NORMAL (setter125); `benilla-assets/src/shaders/terrain.wgsl:120–150` consumes lighting normal |
| MCNR `normal[i].z` | i8 at `3*i+1` | P49–73 reads `nz`; cooked vector's third component is `-nz`, normalized; same native path | B retains signed z and normalizes/reorders through its own coordinate helper; not proof of equal coordinate spaces |
| MCNR `normal[i].y` | i8 at `3*i+2` | P49–73 reads `ny`; cooked vector's second component; same native path | B retains signed y; same B production path |
| MCCV blue | u8 at `4*i`, 145 colors | P76–99 → float B/127 in RGBA; native COLOR packing adjusts127/255, N`mod.rs:154–177`; `godot/shaders/terrain.gdshader:133–134` restores255/127 and multiplies diffuse | No MCCV member/dispatch in inspected B parser + formats terrain path; B mesh COLOR contains **layer indices**, not authored MCCV (`benilla-assets/src/adt.rs:80–129`) |
| MCCV green | u8 at `4*i+1` | G/127, same retained/native shader path | Same bounded B nonmatch |
| MCCV red | u8 at `4*i+2` | R/127, same retained/native shader path | Same bounded B nonmatch |
| MCCV alpha | u8 at `4*i+3` | A/255 retained in RGBA and native COLOR; cited terrain diffuse reads RGB, not a demonstrated alpha use | Same bounded B nonmatch |
| MCSH shadow bits | 512 bytes,64×64 samples; LSB detail addressing, distinct MSB repair below | P128–168,635–679,752–765 parses/repairs edges and checks root/companion conflicts. Repair helpers use **MSB-first** `1<<(7-col%8)`; actual detail sampler uses **LSB-first** `1<<(column&7)`. No runtime artifact establishes a visible defect; D88/public `Chunk.shadow_map` retains. Core `ground_detail.rs:125–143,166–170` copies/reads shadow for detail shading via native N`ground_detail.rs:229–236`; core `ground_detail.rs:295–300,525` converts shadow into placement alpha/color, N`ground_detail.rs:399–429` installs COLOR and `godot/shaders/ground_detail.gdshader:35–45` applies darkening. Native **terrain-surface** material has no MCSH texture binding in N`material.rs:271–299,350–401`; this is not absence of all consumers | B75–87,477–488 zero-pads short payload to512 and provides `is_shadowed`; formats terrain409–419 expands raw MCSH into shadow bytes → assets ADT150–220 → terrain shader203–229 sampling/combination; formats terrain77–92,232–235 separately supports cooked queries. World terrain-stream queries36–52 also consume shadow |
| MCSE `sound_entry_id` | u32 `+0` of28B record | D189–193/P171–187; retained in `sound_emitters`; bounded native terrain/audio search found no actual consumer | No dispatch/member in inspected B ADT parser |
| MCSE `position` | `[f32;3]` `+4` | Same retained emitter; no native audio placement consumer established | Same bounded nonmatch |
| MCSE `size_min` | `[f32;3]` `+16` | Same retained emitter; no native attenuation/size consumer established | Same bounded nonmatch |
| Root MCRF reference indices | u32 sequence, no E retained list | No dispatch in P702–715; root header offset/count dropped | No MCRF handling in inspected B reader; do not conflate with E companion MCRD/MCRW |

P741–749 defaults absent MCCV to white unless the has-MCCV flag is set, when it errors. MCNR requires145 triples; the all-zero normal becomes up. These are source branches, not executed malformed-input tests. MCLV vertex lighting, MCDD detail masks and MBBB/MBMH/MBNV blend records are separately inventoried in [[benilla-adt-extension-fields]], not silently omitted from root input scope.

## MCLY entry:16 bytes

T127–132 raw `MclyEntry` → T327–360 public `TextureLayer`. B48–60,448–463 retains analogous Vanilla members. The leading field's literal E name is `texture_index`, not B's `texture_id`.

| Offset/member | E retention / consumer | B retention / consumer |
|---|---|---|
| `+0 texture_index: u32` | Retained; addresses `AdtTexData.texture_fdids` and native decoded texture map, N`material.rs:360–399` | `texture_id`; formats terrain370–410 assembles layer textures → assets ADT130–215 texture arrays → terrain shader sampling |
| `+4 flags: u32` | Retained as `MclyFlags.raw`; named predicates below | Retained as `MclyFlags.value`; compressed-alpha accessor B41–60; actual layer construction formats terrain370–410, not merely parser-only |
| `+8 offset_in_mcal: u32` | Used to bound this alpha layer against next alpha-mapped layer/T303–319; not a public `TextureLayer` field | Retained; formats terrain401–407 → `benilla-adt/src/combined_alpha.rs:46–135,152–173` consumes offset during alpha decode |
| `+12 _effect_id: u32` | **Copied to `TextureLayer.effect_id`**, T351–357; native footstep surface resolver N`assets.rs:359–380,582–585`; detail collection N`ground_detail.rs:229–236,354–370` | Retained → `ChunkMesh.layer_effect_ids`, formats terrain391–397; `ground_effect_at`95–106 → world terrain-stream queries75–80; `benilla-formats/src/ground_effects.rs:153–210` resolves clutter doodads |

| Flags predicate | E actual handling | B boundary |
|---|---|---|
| Bits0–2 | `animation_rotation`, T25–27; M56–83 derives velocity direction | No corresponding selector use established in inspected B terrain path |
| Bits3–5 | `animation_speed`, T29–31; speed lookup multiplied into M64–70 velocity | Same bounded unknown; raw bits survive |
| Bit6/`0x40` | `animation_enabled`, T33–35 gates velocity | Raw retained; no corresponding animated-layer consumer established |
| Bit7/`0x80` | `overbright`, T37–39 → M34–53 brightness multiplier | Raw retained; no matching consumer established |
| Bit8/`0x100` | `use_alpha_map`, T41–43,327–350 gates alpha read | Raw retained; B alpha construction is version-specific, `benilla-adt/src/combined_alpha.rs:46–135,152–173` |
| Bit9/`0x200` | `alpha_compressed`, T45–47,242–262 selects RLE | Named accessor; formats terrain401–407 → `benilla-adt/src/combined_alpha.rs:46–135,152–173` selects compressed/big/small decode and edges, not proof all E modes match |
| Bit10/`0x400` | `use_cube_map_reflection`, T49–51 → M64–83 reflection channel | Raw retained; matching consumer not established |
| Bits11–31 | Raw retained, semantics not assigned by these predicates | Raw retained; meanings/use not established |

Native N`material.rs:284–292,397–399` binds animation vectors; **`godot/shaders/terrain.gdshader:106–109` applies velocity × animation time to each slot's UV**, and137 uses reflection weights. N`material.rs:360–399` truncates rendered layer inputs to four; retaining additional MCLY records is not proof all are rendered.

## Alpha payload and other `_tex0` fields

| Chunk/field | Bytes and read | Retention / actual E use | B contrast |
|---|---|---|---|
| MCAL alpha payload | Variable; T157–224,242–319 | RLE/big8-bit/small4-bit decode → per-layer `alpha_map`; edge policy uses WDT big-alpha and root do-not-fix-alpha. M86–101 packs layers1/2/3 into RGB, opaque A; native material consumes packed texture | Raw B MCAL copied using header size; formats terrain401–407 calls `benilla-adt/src/combined_alpha.rs:46–135,152–173` for actual big/small/RLE mapping and edge handling; absence of decompression in parser alone is not end-to-end absence |
| MCMT `material_id[i]` | First4 u8, T388–395 | T351–357 maps slots to `TextureLayer.material_id`, missing slot/default0; M34–53 puts ID in layer parameter z | No dispatch in inspected B monolithic reader |
| MAMP amplifier | Exactly one u32, T485–494 | Retained `texture_amplifier`; M29–32 clamps exponent≤8 and derives repeat8×2^exponent; N`material.rs:378–393` binds config | No corresponding reader established |
| MDID texture FDID[i] | u32 array, T397–411,449–465 | `texture_fdids`; native texture loading and layer lookup | B MTEX NUL strings are older texture identity input, B249–277, not modern FDIDs |
| MHID height FDID[i] | u32 array, same reader | `height_texture_fdids`; N`textures.rs:46–70` resolves height texture; native material `has_height` gates height scale | No corresponding reader established |
| MTXF flags[i] | u32 array, same reader | `texture_flags` retained; no actual native use established in this bounded pass | No corresponding reader established |
| MTXP `flags` | u32 `+0` of16B, T136–141,413–433 | Retained `TextureParams.flags`; M34–53 uses height scale/offset, not this raw flag; other consumers unknown | No corresponding reader established |
| MTXP `height_scale` | f32 `+4` | Retained; M42–49 uses only when height texture present → native layer params | No corresponding reader established |
| MTXP `height_offset` | f32 `+8` | Retained; M42–49 → native layer params | No corresponding reader established |
| MTXP `_padding` | u32 `+12` | Read then omitted from public params; no meaning assigned | No corresponding reader established |

`AdtTexData.map_flags` comes from the caller's WDT projection, not an additional `_tex0` wire field. Accumulators, chunk counters, normalized maps, packed RGBA, velocities and native material arrays are helpers/derived values; their inputs are enumerated above. Unrecognized tags and bytes are not assigned speculative names.

## Sources

- D21–102; P49–187,581–765; T20–141,157–494 — declarations, actual sample/texture readers, policies and projections.
- `godot/core/src/ground_detail.rs:125–170` — real detail-map/shadow consumers, distinct from terrain-surface binding.
- M20–101; N`mod.rs:154–200`, `material.rs:231–292,360–399`, `textures.rs:46–70`, `assets.rs:359–380,582–585`, `ground_detail.rs:229–236,354–370`; `godot/shaders/terrain.gdshader:106–109,133–137` — production native/shader consumers.
- B20–155,249–277,353–498; `crates/benilla-formats/src/terrain.rs:77–106,232–235,311–410`; `crates/benilla-formats/src/ground_effects.rs:153–210`; `crates/benilla-assets/src/adt.rs:80–220`, `src/shaders/terrain.wgsl:120–220`; `crates/benilla-world/src/terrain_stream/queries.rs:36–80` — B parser versus actual terrain consumers.

## See Also

- [[benilla-adt-header-fields]] — header/version byte distinctions.
- [[benilla-adt-placement-fields]] — object companion references and placement inputs.
- [[benilla-adt-extension-fields]] — remaining recognized root/LOD inputs.
- [[benilla-terrain-water-fields]] — liquid/map/horizon inputs.
- [[benilla-asset-field-inventory]] — parent scope and static-only boundaries.
