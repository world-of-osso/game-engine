# Benilla / game-engine M2 header and container fields

Literal header inputs read by the inspected implementations, not a reconstructed universal M2 schema. Static local inspection on 2026-10-24; no source/client execution, builds, tests or asset extraction. E root `/syncthing/Sync/Projects/world-of-osso/game-engine`, native baseline `354bdb7dbf177047ed46d63168acec531f5c4426`; B root `/home/osso/Repos/benilla`, pin `8a6c74036cfe3d00275fd75ad1aeae1f4ea7d0df`.

## E MD20 payload header

All locations below are relative to the payload selected from MD21. Each count/offset is a distinct serialized u32 input. Counts bound attempted decoding, not necessarily final vector cardinality: individual readers can truncate, filter or reject inputs. Addresses are consumed and discarded, not retained runtime field values. Source abbreviations in this table are relative to `E/godot/core/src/asset/m2_format/`. `E/godot/core/src/m2.rs:311–437` wires these into the production Model; family pages trace runtime consumers. E does not use B's sequential pre-Wrath header walk.

| Field | Offset / type | Handling / source |
|---|---|---|
| Payload magic bytes | `0x00..0x04` | Not validated by scoped MD21 payload parser; outer MD21 tag is required (`parser.rs:187–224`). No universal plain-MD20 acceptance inferred. |
| Version | `0x04 u32` | Particle stride branch (`pure.rs:38`, `m2_particle.rs:725–740,782–788`); camera independently accepts272–274 (`m2_camera.rs`). Not a general model-version compatibility gate. |
| Global flags | `0x10 u32` | Retained (`parser.rs:353–355`, `m2.rs:372,421,435`); flag0x8 derives texture-combiner policy; ribbon flag0x20000 gates UV lookup. Other bits need individual consumers, not blanket unsupported classification. |
| Global sequences count | `0x14 u32` | Durations -> shared clocks; animation field page; `m2_anim.rs:182–198`. |
| Global sequences offset | `0x18 u32` | Payload address consumed then discarded; `m2_anim.rs:182–198`. |
| Sequences count | `0x1c u32` | Sequence records -> playback/variation; animation field page; `m2_anim.rs:128–179`. |
| Sequences offset | `0x20 u32` | Payload address consumed then discarded; `m2_anim.rs:128–179`. |
| Bones count | `0x2c u32` | Metadata plus bone tracks; animation field page; `m2_anim.rs:65–92,455–461`. |
| Bones offset | `0x30 u32` | Payload address consumed then discarded; `m2_anim.rs:65–92,455–461`. |
| Vertices count | `0x3c u32` | Decoded vertex vectors -> Model.vertices; `pure.rs:43; parser.rs:227–229`. |
| Vertices offset | `0x40 u32` | Payload address consumed then discarded; `pure.rs:43; parser.rs:227–229`. |
| Colors count | `0x48 u32` | RGB/opacity tracks -> material model; `pure.rs:44; m2_anim.rs:479–492`. |
| Colors offset | `0x4c u32` | Payload address consumed then discarded; `pure.rs:44; m2_anim.rs:479–492`. |
| Textures count | `0x50 u32` | Type/flags retained; legacy filename descriptors dropped; `pure.rs:45; parser.rs:312–333`. |
| Textures offset | `0x54 u32` | Payload address consumed then discarded; `pure.rs:45; parser.rs:312–333`. |
| Transparency count | `0x58 u32` | Fixed16 tracks -> material opacity; `pure.rs:46; m2_anim.rs:464–477`. |
| Transparency offset | `0x5c u32` | Payload address consumed then discarded; `pure.rs:46; m2_anim.rs:464–477`. |
| Texture animation count | `0x60 u32` | Translation/rotation/scale; constant is named TEXTURE_WEIGHTS, actual reader is texture animation; `pure.rs:47; m2_anim.rs:504–520`. |
| Texture animation offset | `0x64 u32` | Payload address consumed then discarded; `pure.rs:47; m2_anim.rs:504–520`. |
| Materials count | `0x70 u32` | Flags/blend records -> Model.materials; `pure.rs:48; parser.rs:335–337`. |
| Materials offset | `0x74 u32` | Payload address consumed then discarded; `pure.rs:48; parser.rs:335–337`. |
| Texture lookup count | `0x80 u32` | u16 lookup -> batch resolution; `pure.rs:49; parser.rs:345–351`. |
| Texture lookup offset | `0x84 u32` | Payload address consumed then discarded; `pure.rs:49; parser.rs:345–351`. |
| Texture-unit lookup count | `0x88 u32` | i16 UV/environment lookup -> batch resolution; `pure.rs:50; parser.rs:357–363`. |
| Texture-unit lookup offset | `0x8c u32` | Payload address consumed then discarded; `pure.rs:50; parser.rs:357–363`. |
| Transparency lookup count | `0x90 u32` | i16 lookup -> batch opacity; `parser.rs:13,365–371`. |
| Transparency lookup offset | `0x94 u32` | Payload address consumed then discarded; `parser.rs:13,365–371`. |
| UV-animation lookup count | `0x98 u32` | i16 lookup -> texture transforms; `parser.rs:14,373–375`. |
| UV-animation lookup offset | `0x9c u32` | Payload address consumed then discarded; `parser.rs:14,373–375`. |
| Collision indices count | `0xd8 u32` | u16 triangle list -> validated collision mesh; `m2_collision.rs:9,27–38`. |
| Collision indices offset | `0xdc u32` | Payload address consumed then discarded; `m2_collision.rs:9,27–38`. |
| Collision vertices count | `0xe0 u32` | Vec3 payload -> validated collision mesh; `m2_collision.rs:10,36–39`. |
| Collision vertices offset | `0xe4 u32` | Payload address consumed then discarded; `m2_collision.rs:10,36–39`. |
| Attachments count | `0xf0 u32` | Can be replaced by skeleton/model SKA1; `pure.rs:53; m2_attach.rs:39–57`. |
| Attachments offset | `0xf4 u32` | Payload address consumed then discarded; `pure.rs:53; m2_attach.rs:39–57`. |
| Attachment lookup count | `0xf8 u32` | i16 indices -> attachment selection; `pure.rs:54; m2_attach.rs:86–112`. |
| Attachment lookup offset | `0xfc u32` | Payload address consumed then discarded; `pure.rs:54; m2_attach.rs:86–112`. |
| Events count | `0x100 u32` | Timestamped records -> selected action events; `m2_event.rs:22–25,39–66`. |
| Events offset | `0x104 u32` | Payload address consumed then discarded; `m2_event.rs:22–25,39–66`. |
| Lights count | `0x108 u32` | Full seven-track light records; `m2_light.rs:7,133–145`. |
| Lights offset | `0x10c u32` | Payload address consumed then discarded; `m2_light.rs:7,133–145`. |
| Cameras count | `0x110 u32` | Snapshot reader, not part of returned Model; `m2_camera.rs:6,115–128,151–179`. |
| Cameras offset | `0x114 u32` | Payload address consumed then discarded; `m2_camera.rs:6,115–128,151–179`. |
| Ribbons count | `0x120 u32` | 176-byte records -> native trails; `m2_ribbon.rs:7,51–65`. |
| Ribbons offset | `0x124 u32` | Payload address consumed then discarded; `m2_ribbon.rs:7,51–65`. |
| Particles count | `0x128 u32` | Version-selected emitter records -> native particles; `pure.rs:55; m2_particle.rs:725–740`. |
| Particles offset | `0x12c u32` | Payload address consumed then discarded; `pure.rs:55; m2_particle.rs:725–740`. |
| Bounding-box minimum | `0xa0 Vec3` | Retained Model bounds, `pure.rs:51,57–63`, `m2.rs:369,429–430`. |
| Bounding-box maximum | `0xac Vec3` | Same, independent corner. |
| Collision-box minimum | `0xbc Vec3` | Retained after finite/ordered validation, `m2_collision.rs:39–48`. |
| Collision-box maximum | `0xc8 Vec3` | Same, independent corner. |

Unread header intervals in this scoped reader: `0x08..0x10`, `0x24..0x2c`, `0x34..0x3c`, `0x44..0x48`, `0x68..0x70`, `0x78..0x80`, `0xb8..0xbc`, `0xd4..0xd8`, `0xe8..0xf0`, `0x118..0x120`; bytes after `0x130` are not assigned a header meaning by the inspected parser. Some correspond to known format concepts, but this table does not invent semantics from B's different offsets. In particular **E0x70 is materials**, **E0x60 is texture animation**, and **E0x118 is not its ribbon descriptor**. Header coverage here accounts for inspected reads and opaque intervals; it does not authenticate unnamed modern fields. Array payload layout is covered by family pages, not counted again as unique headers.

## Container and external-reference fields

E `parser.rs:187–224,339–343,384–388,453–505` reads modern chunk framing; `m2.rs:311–376` chooses inputs. Unknown chunks and unconsumed trailing fragments are not retained as raw container data.

| Field | Serialized input | Handling |
|---|---|---|
| Chunk tag | Four bytes | Dispatch MD21,SKA1,TXID,EXP2,SKID,SFID,AFID. Unknown tags skipped. |
| Chunk payload size | `u32 header+4` | Advances by8+size, errors if chunk exceeds model container; incomplete final header is not parsed. |
| MD21 payload | Counted bytes | Required model payload; later recognized MD21 replaces earlier stored slice. |
| SKID FDID | First `u32`, size>=4 | Retained optional skeleton ID; supplied skeleton required when present (`m2.rs:317–320`). Extra payload bytes not modeled. |
| SFID element | Each complete `u32` | Retained geometry-skin FDIDs; incomplete tail dropped. |
| TXID element | Each complete `u32` | Retained texture FDIDs; incomplete tail dropped. Index-to-FDID is derived, not an extra disk field. |
| AFID animation ID | `u16 entry+0` | Retained tuple key and matching sequence ID. |
| AFID variation | `u16 entry+2` | Retained tuple key and matching variation. |
| AFID FDID | `u32 entry+4` | External `.anim` dependency; zero/unmatched/in-file sequences not loaded. Incomplete8-byte tail dropped. |
| SKA1 payload | Counted bytes | Attachment parser selects nonempty skeleton/model data before MD20; four descriptor words (`m2_attach.rs:60–83,104–121`) individually listed in [scene descriptors](benilla-m2-scene-fields.md#descriptor-members-and-omitted-spans). |
| EXP2 payload | Counted bytes | Particle supplemental header/28-byte records; [particle field table](benilla-m2-particle-fields.md#partial-tracks-multitexture-and-exp2). |
| External `.anim` tag | Four bytes | Selects requested AFM2/AFSB; chunkless input treated as payload (`parser.rs:492–505`). |
| External `.anim` size | `u32 header+4` | Selected end clamped to file; not same strict framing as model parser. |
| SKS1 global count | `u32 payload+0` | Skeleton durations; `parser.rs:508–517`. |
| SKS1 global offset | `u32 payload+4` | Address consumed. |
| SKS1 sequence count | `u32 payload+8` | Skeleton64-byte sequence array. |
| SKS1 sequence offset | `u32 payload+12` | Address consumed. |
| SKB1 bone count | `u32 payload+0` | Skeleton88-byte bone array; `parser.rs:520–531`. |
| SKB1 bone offset | `u32 payload+4` | Same array supplies tracks; address consumed. |

SKS1 bytes beyond its read16-byte prefix and SKB1 beyond its read8-byte prefix are not given semantic header labels here. Skeleton chunk dispatch requires at least24/16bytes respectively (`parser.rs:409–435`); a minimum accepted chunk size is not proof all those bytes are decoded. AFSB supplies skeleton bone payloads; AFM2 supplies model tracks including events. Missing external elements retain empty per-sequence key vectors, not verified animation support.

## B pre-Wrath header walk

B `crates/benilla-m2/src/lib.rs:39–155` validates MD20 and admits versions256–263; all admitted versions take embedded views and PlayableAnimationLookup. The table gives **both count and offset individually**. `walk-only` means that base reader consumes the pair but does not retain its payload there; other B adapters can independently use raw bytes. No repository-wide unused claim follows.

| B field | Offset / type | Base-reader handling |
|---|---|---|
| Magic | `0x00 bytes4` | Requires MD20. |
| Version | `0x04 u32` | Requires256–263. |
| Flags | `0x10 u32` | Cursor skips scalar in this walk. |
| Name count | `0x08 u32` | walk-only. |
| Name offset | `0x0c u32` | Address consumed; walk-only. |
| Global sequences count | `0x14 u32` | Initial `_global` pair unused, but same base reader rereads/decodes durations (`B/crates/benilla-m2/src/lib.rs:409–421`) and retains them (`:452`). |
| Global sequences offset | `0x18 u32` | Address reread/consumed by same base reader; duration vector retained (same sources). |
| Sequences count | `0x1c u32` | walk-only here; separately read by animation adapter. |
| Sequences offset | `0x20 u32` | Address consumed; walk-only here; separately read by animation adapter. |
| Animation lookup count | `0x24 u32` | decoded lookup. |
| Animation lookup offset | `0x28 u32` | Address consumed; decoded lookup. |
| Playable animation lookup count | `0x2c u32` | decoded substitute/direction records. |
| Playable animation lookup offset | `0x30 u32` | Address consumed; decoded substitute/direction records. |
| Bones count | `0x34 u32` | decoded bones. |
| Bones offset | `0x38 u32` | Address consumed; decoded bones. |
| Key-bone lookup count | `0x3c u32` | walk-only. |
| Key-bone lookup offset | `0x40 u32` | Address consumed; walk-only. |
| Vertices count | `0x44 u32` | decoded vertices. |
| Vertices offset | `0x48 u32` | Address consumed; decoded vertices. |
| Views count | `0x4c u32` | embedded skin descriptor retained. |
| Views offset | `0x50 u32` | Address consumed; embedded skin descriptor retained. |
| Colors count | `0x54 u32` | decoded RGB/alpha tracks. |
| Colors offset | `0x58 u32` | Address consumed; decoded RGB/alpha tracks. |
| Textures count | `0x5c u32` | decoded type/flags/filename. |
| Textures offset | `0x60 u32` | Address consumed; decoded type/flags/filename. |
| Transparency count | `0x64 u32` | decoded tracks. |
| Transparency offset | `0x68 u32` | Address consumed; decoded tracks. |
| Texture flipbooks count | `0x6c u32` | cursor skips pair. |
| Texture flipbooks offset | `0x70 u32` | Cursor skips pair without reading count or address (`B/crates/benilla-m2/src/lib.rs:96–98`). |
| Texture animation count | `0x74 u32` | decoded transforms. |
| Texture animation offset | `0x78 u32` | Address consumed; decoded transforms. |
| Color replacement count | `0x7c u32` | walk-only. |
| Color replacement offset | `0x80 u32` | Address consumed; walk-only. |
| Render flags count | `0x84 u32` | decoded materials. |
| Render flags offset | `0x88 u32` | Address consumed; decoded materials. |
| Bone lookup count | `0x8c u32` | walk-only. |
| Bone lookup offset | `0x90 u32` | Address consumed; walk-only. |
| Texture lookup count | `0x94 u32` | decoded lookup. |
| Texture lookup offset | `0x98 u32` | Address consumed; decoded lookup. |
| Texture-unit lookup count | `0x9c u32` | decoded lookup. |
| Texture-unit lookup offset | `0xa0 u32` | Address consumed; decoded lookup. |
| Transparency lookup count | `0xa4 u32` | decoded lookup. |
| Transparency lookup offset | `0xa8 u32` | Address consumed; decoded lookup. |
| Texture-animation lookup count | `0xac u32` | decoded lookup. |
| Texture-animation lookup offset | `0xb0 u32` | Address consumed; decoded lookup. |
| Collision triangles count | `0xec u32` | raw collision indices retained. |
| Collision triangles offset | `0xf0 u32` | Address consumed; raw collision indices retained. |
| Collision vertices count | `0xf4 u32` | raw collision positions retained. |
| Collision vertices offset | `0xf8 u32` | Address consumed; raw collision positions retained. |
| Collision normals count | `0xfc u32` | walk-only. |
| Collision normals offset | `0x100 u32` | Address consumed; walk-only. |
| Attachments count | `0x104 u32` | decoded with narrowing/filtering. |
| Attachments offset | `0x108 u32` | Address consumed; decoded with narrowing/filtering. |
| Attachment lookup count | `0x10c u32` | remapped to surviving records. |
| Attachment lookup offset | `0x110 u32` | Address consumed; remapped to surviving records. |
| Events count | `0x114 u32` | positional marker projection; full events read separately by animation adapter. |
| Events offset | `0x118 u32` | Address consumed; positional marker projection; full events read separately by animation adapter. |
| Bounds minimum | `0xb4 Vec3` | Retained header min. |
| Bounds maximum | `0xc0 Vec3` | Retained header max. |
| Bounds sphere radius | `0xcc f32` | Retained. |
| Collision minimum | `0xd0 Vec3` | Retained header min. |
| Collision maximum | `0xdc Vec3` | Retained header max. |
| Collision sphere radius | `0xe8 f32` | Retained. |

B later header reads are independent fixed-offset adapters: lights `0x11c`, cameras `0x124`, camera lookup `0x12c`, ribbons `0x134`, particles `0x13c`; each has `u32 count` then `u32 offset +4`. The v257 combiner descriptor at `0x144` is separately conditional, **not an E flag0x8 table equivalence** (`B/crates/benilla-m2/src/lib.rs` camera/combiner decoding, `B/crates/benilla-formats/src/models/records.rs`, `ribbons.rs`, `particles.rs`). Detailed field consumers remain in the family comparisons. B remaining optional bytes and format versions not admitted by its gate are not authenticated by this walk.

## Sources

- E `godot/core/src/asset/m2_format/{pure,parser,m2_anim,m2_attach,m2_event,m2_light,m2_camera,m2_collision,m2_ribbon,m2_particle}.rs`; `godot/core/src/m2.rs` — literal supported header inputs.
- B `crates/benilla-m2/src/lib.rs`; `crates/benilla-formats/src/{ribbons,particles}.rs`, `models/records.rs` — separate pre-Wrath walk/adapters.
- [M2 format](../formats/m2-format.md) — format SSOT, not independent byte authentication.

## See Also

- [[benilla-m2-field-census]] — declarations do not enumerate implicit header fields.
- [[benilla-m2-animation-fields]] — shared tracks and skeleton/animation source selection.
- [[benilla-m2-particle-fields]] — EXP2/particle fields.
- [[benilla-m2-ribbon-fields]] — ribbon descriptors and consumers.
- [[benilla-asset-field-inventory]] — cross-format scope and limits.
