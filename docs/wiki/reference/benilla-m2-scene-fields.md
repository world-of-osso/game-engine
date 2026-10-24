# Benilla / game-engine M2 scene fields

Static source comparison only. No executable, import, build, test, network, download, or repo edit performed. Source line numbers are current local checkout observations (2026-10-24); E native source baseline `354bdb7dbf177047ed46d63168acec531f5c4426`; B pin `8a6c74036cfe3d00275fd75ad1aeae1f4ea7d0df`. Root E `/syncthing/Sync/Projects/world-of-osso/game-engine`; B `/home/osso/Repos/benilla`. `E` is the modern Godot client; `B` is Benilla's WoW 1.12.1 client. These are version-distinct layouts, not interchangeable structures.

Source shorthand: bare `m2_*.rs` means `E/godot/core/src/asset/m2_format/`; `core/src/` and `godot/rust/` are E; `benilla-*` means `B/crates/benilla-*`. Named family tables are research, not runtime acceptance.

## Descriptor convention

Paths below are relative to the stated E or B repository root. Where rows say **track descriptor**, each descriptor field applies separately to each named payload: E camera snapshots consume only first populated sequence/key values and retain no interpolation/global-sequence/timestamp/tangent arrays; E light tracks retain interpolation (`u16`), global sequence (`i16`), per-sequence u32 timestamp arrays and payload value arrays (no tangents in these channels). B camera tracks retain `interp`, `gseq`, ranges, timestamp/value keys; camera spline keys retain value, in-tangent and out-tangent (`benilla-m2/src/track.rs:1-8,126-137`; camera types `benilla-m2/src/model.rs:181-206`). B light parser snapshots each named channel's first key only, discarding descriptor timing/interpolation and later keys (`benilla-formats/src/models/records.rs:164-202`). Payload components are named per row. No claim that identically named descriptors share byte layout.

## Attachments

| Field | E — modern parser → retained/derived/dropped → consumer | B — 1.12.1 counterpart → retained/derived/dropped → consumer |
|---|---|---|
| Header attachment array | MD20 array pair at `0xF0`; 40-byte stride (`m2_attach.rs:3-6,21,39-46`). Loader selects nonempty SKA1 data first, else MD20 (`core/src/m2.rs:174-190,367-368`). | Header array resolved in pre-Wrath sequential header walk (`benilla-m2/src/lib.rs:147-153`); 48-byte stride (`:246-265`). B version gate 256–263 (`benilla-m2/src/lib.rs:40-50`). |
| Record ID | u32 at record +0 retained as u32 (`m2_attach.rs:23,48-50`). | Read u32 +0, checked/narrowed to u16; out-of-range record dropped (`benilla-m2/src/lib.rs:249-265`; type `model.rs:150-159`). |
| Bone index | u16 +4 retained (`m2_attach.rs:24,48-50`). | Read u32 +4, checked/narrowed to u16; absent-bone records dropped (`benilla-m2/src/lib.rs:249-265`). |
| Position X/Y/Z | Three f32 at +8/+12/+16 retained as model-local WoW-axis vector (`m2_attach.rs:25,48-55`). | C3 at +8 read and retained as three f32 (`benilla-m2/src/lib.rs:250-265`). |
| Unknown attachment word | E +6 u16 not represented (`m2_attach.rs:8–13,48–56`). | B uses u32 bone at+4; these are not independent unknown bytes in that layout. |
| Attachment trailing track | E bytes+20..+39 explicitly skipped, not decoded to visibility/timing (`m2_attach.rs:8–13,48–56`). | B bytes+20..+47 are not represented;48-byte stride (`benilla-m2/src/lib.rs:246–265`). Different descriptor size; no visibility support inferred. |
| Lookup table | MD20 `0xF8`, i16 entries retained as `Vec<i16>`; SKA1 lookup similarly preferred when present (`m2_attach.rs:6,86-112`; loader `core/src/m2.rs:184-190`). IDs index the lookup; core accessor handles empty lookup separately (`core/src/m2.rs:228-241`). | Lookup header `+0x10c`; raw u16 record indices remapped to emitted attachment indices, invalid/dropped records become `0xffff` (`benilla-m2/src/lib.rs:268-279`; type docs `model.rs:248-252`). `M2Model::attachment(id)` returns none for sentinel (`model.rs:305-313`). |
| B-only derived follow pivot | E has `above_character_position` lookup (`core/src/m2.rs:228-241`); no equivalent named `pivot_attach_z` field was found in the inspected E model. | Derived from attachment ID 17's position Z and stored as `pivot_attach_z` (`benilla-m2/src/lib.rs:152-155`; model field/docs `model.rs:245-247`). Distinct from the raw lookup/record fields. |
| Scene binding | Native creates a `BoneAttachment3D`, sets authored bone, and applies `position - bone.pivot` as child offset (`godot/rust/src/assets/attachments.rs:38–49`); model load calls it (`assets/mod.rs:494-496`). | B loader retains records/lookup; `M2Model::attachment` resolves IDs (`benilla-m2/src/model.rs:305-313`), and animation attachment extraction calls it (`benilla-formats/src/models/anim.rs:83-92`). The breath handler documents tag 0x11 as AttachmentID but defers actual tag resolution (`benilla-app/src/creature_anim/breath.rs:37-39,177-185`). |

## Events

| Field | E — modern parser → retained/derived/dropped → consumer | B — 1.12.1 counterpart → retained/derived/dropped → consumer |
|---|---|---|
| Event table / record | MD20 header `0x100`, 36-byte records, track at +0x18 (`m2_event.rs:22-25,39-42`). `core/src/m2.rs:361-362,433-435` supplies sequence source metadata and retains `Vec<Event>`. | 1.12.1 event table header `0x114/0x118`, 44-byte records; `benilla-m2` marker reader parses event table at `benilla-m2/src/lib.rs:280-299`. Separately, the animation parser builds complete per-sequence `AnimEvent` records (`benilla-formats/src/models/anim.rs:558-568,593-600,660-669`); this is not marker-only. |
| 4CC identifier | Four raw bytes +0 retained as `[u8;4]` (`m2_event.rs:26-29,47-51`); native action filters identifiers (`godot/rust/src/animation/action.rs:29-35,68-83`). | Full `AnimEvent.ident` retained, then copied into per-sequence events (`anim.rs:285-300,660-669`). The separate `M2EventMarker` projection retains ident/bone/position, not payload or time (`benilla-m2/src/model.rs:161-170`; parser `lib.rs:283-299`). |
| Event data / payload | u32 +4 retained by E `M2Event` (`m2_event.rs:26-29,62-66`). No direct use of this field was found in the native Godot event-action/sound chain searched; that chain forwards event IDs/times. This is a scoped negative, not a claim that all downstream systems never use it. | Full `AnimEvent.data` retained and passed into event sound routing (`anim.rs:287-300,660-669`; `benilla-app/src/creature_anim/events.rs:11-17,221-254`; `sound/anim_events.rs:124-145`). `$SND`/`$DSO` and `$CSD` use `ev.data` as sound ID. In contrast, B's marker-only projection drops data (`benilla-m2/src/lib.rs:280-299`). |
| Bone index | u32 +8 retained (`m2_event.rs:26-29,62-66`). No direct native Godot consumer of `event.bone` found in searched Rust source. | Full event parser casts raw u32 to u16 without checking, losing high16 bits (`anim.rs:562–568,582–586,660–669`). Separate marker parser uses checked narrowing and drops invalid bones (`benilla-m2/src/lib.rs:287–299`). Asset `ClipEvent` carries narrowed bone and pivot-relative offset (`benilla-assets/src/m2.rs:550–564`); `EventFrame::point` composes through live pose (`benilla-app/src/creature_anim/events.rs:30–46`) before terminal sound dispatch (`sound/anim_events.rs:124–145`); consumers of positional markers are described in `benilla-m2/src/model.rs:161-170`. |
| Position X/Y/Z | Three f32 +0x0c retained (`m2_event.rs:26-29,62-66`). No direct native Godot use found in event action/sound path searched. | Full `AnimEvent.position` retained (`anim.rs:562-568,593-600,663-669`); sound/marker consumers can use fired point (`benilla-app/src/sound/anim_events.rs:124-145`; marker semantics `benilla-m2/src/model.rs:161-170`). |
| Event track descriptor | E keeps per-sequence timestamp lists only, with `.anim` AFM2 external/missing handling (`m2_event.rs:29-34,50-65`). Track interpolation and global-sequence fields are not in `M2Event`. Native action gathers timestamps only for selected IDs and missile release (`godot/rust/src/animation/action.rs:68-100`). | Full animation parser reads timestamp arrays and emits one `AnimEvent` per key inside each sequence band, as seconds (`benilla-formats/src/models/anim.rs:558-591,645-669`). Its output `AnimEvent` represents fired keys, not a retained raw descriptor. The `benilla-m2::M2EventMarker` projection skips the 20-byte event track entirely (`benilla-m2/src/lib.rs:280-299`). |
| B-only specialized event-derived anchors | No specialized `$WTT`/`$WTB` string-anchor or `$CCH` marker projection was found in the inspected E parser/native consumer paths. | B has separate parsers deriving string anchors from `$WTT`/`$WTB` and a `$CCH` marker from the same raw 1.12 event table (`benilla-formats/src/models/anim.rs:124-177`). These are projections from event records, not extra `AnimEvent` fields. |
| Sound / firing consumer audit | E's IDs/times route through `animation/action.rs:29-35,68-100`, `animation/mod.rs:273-276,976-982`, `world_combat.rs:303-314`, and `spell_melee.rs:99-105` (`$SCD`, `$CSS`, `$CAH`). Search found no direct `event.data`, `event.bone`, or `event.position` read in this Godot action/sound chain. | B's animation clips carry event keys; `creature_anim/events.rs:1-17,221-254` emits `AnimSoundEvent`; `sound/anim_events.rs:35-45,90-145,167-189` dispatches cues, including payload-driven `$SND`/`$DSO`/`$CSD`. Therefore B events are not merely marker records. |

## Lights

E uses modern MD20 light header `0x108`, stride `0x9c` (156 bytes) and seven track descriptors. B uses 1.12 header count/offset `0x11c/0x120`, stride `0xd4` (212 bytes); layout mismatch is explicit.

| Field | E — modern | B — 1.12.1 |
|---|---|---|
| Type | u16 at +0; retained (`m2_light.rs:10-19,119-129`). | u16 at +0; retained (`benilla-formats/src/models/records.rs:12-21,164-182`). B `is_point` means type 1; type 0 feeds ambient accumulator (`records.rs:31-40`). |
| Bone index | i16 at +2; retained (`m2_light.rs:11-19,120-129`). | u16 at +2 reinterpreted as same-width i16; retained (`records.rs:164-170`). B additionally derives `bone_z` from bone pose (`records.rs:22-28,164-170`). |
| Position X/Y/Z | Vec3 f32 at +4; retained as local position (`m2_light.rs:12,121-129`). | Vec3 f32 at +4/+8/+0x0c; retained (`records.rs:170-174`). |
| Ambient color track | Vec3 payload, descriptor +0x10; full track retained (`m2_light.rs:15-16,102-104,123`). | Vec3 first-key snapshot at +0x10; fallback `[1,1,1]`; times/remaining keys/interpolation not retained (`records.rs:175-176`). |
| Ambient intensity track | f32 payload, descriptor +0x24; full track retained (`m2_light.rs:16,106-108,124`). | f32 first key at +0x2c; fallback 0; descriptor and later keys dropped (`records.rs:176`). |
| Diffuse color track | Vec3 payload, descriptor +0x38; full track retained (`m2_light.rs:17,102-104,125`). | Vec3 first key at +0x48; fallback white; descriptor and later keys dropped (`records.rs:177`). |
| Diffuse intensity track | f32 payload, descriptor +0x4c; full track retained (`m2_light.rs:18,106-108,126`). | f32 first key at +0x64; fallback 1; descriptor and later keys dropped (`records.rs:178`). |
| Attenuation-start track | f32 payload, descriptor +0x60; full track retained (`m2_light.rs:18,106-108,127`). | f32 first key at +0x80; fallback 0; descriptor and later keys dropped (`records.rs:179`). |
| Attenuation-end track | f32 payload, descriptor +0x74; full track retained (`m2_light.rs:19,106-108,128`). | f32 first key at +0x9c; fallback 0; descriptor and later keys dropped (`records.rs:180`). |
| Visibility track | u8 payload, descriptor +0x88; full track retained (`m2_light.rs:19,110-112,129`). | First u8 at +0xb8 reduced to `visibility_off` boolean (`records.rs:25-28,181`); descriptor and remaining timeline lost. |
| Remaining record bytes | Seven 20-byte track descriptors plus 16-byte fixed prefix fill 156-byte record; offsets/constants `m2_light.rs:10-19`. | Seven vanilla 28-byte track descriptors plus fixed 16-byte prefix fill the 212-byte record; first-key values are extracted, but descriptor metadata/ranges and later keys are not retained (`records.rs:164-202`; descriptor layout `benilla-m2/src/track.rs:9–22`). B additionally derives runtime `bone_z` and `casts()` (`records.rs:22-40`). |
| Consumer path | Core stores lights on model (`core/src/m2.rs:79,436`). Native installs `WowM2Lights` (`godot/rust/src/assets/mod.rs:569–572`): type1 lights become bone-following `OmniLight3D` children, updated from animation and global clocks (`godot/rust/src/assets/m2_lights.rs:31–57,107–181`). Production `core/src/m2_lights.rs:70–102` samples diffuse RGB/intensity and visibility; authored attenuation tracks apply only under model flag0x8000, otherwise fixed ranges. Ambient RGB/intensity separately feed scene ambient at time0 (`m2_light.rs:162–180`, `godot/rust/src/char_create/scene.rs:208–228`). Do not confuse the parser-module `evaluate_light` with the native point-light evaluator. | `benilla-assets/src/m2.rs:325,397–406,674–680` retains parsed lights. Placed-light spawn (`benilla-world/src/terrain_stream/spawn/fx.rs:48–71`) gates type/visibility, transforms rest position and uses diffuse RGB/intensity; fixed range48 (`:14–25`), not authored attenuation. `benilla-world/src/lighting/global_light.rs:193–266,354–363` packs/uploads lights; `shaders/static_gx.wgsl:79–114` applies nearest-light diffuse falloff. No animated placed-light bone poses established. |

## Cameras

**Correction verified in source:** E `CAMERA_RECORD_SIZE = 116` decimal (`0x74`), not 0x68 (`godot/core/src/asset/m2_format/m2_camera.rs:5-8`). B camera stride is `0x7c` = 124 bytes. Header locations and version families differ; never infer equivalence from similar field names.

| Field | E — modern camera snapshot | B — version-256–263 camera |
|---|---|---|
| Header/lookup | Camera array at MD20 `0x110`; parser walks 116-byte records (`m2_camera.rs:5-8,120-148`). E snapshot selects camera index 0; portrait helper selects type 0 (`:159-179`). | Camera array header `0x124/0x128`, lookup `0x12c/0x130`, stride `0x7c` (`benilla-m2/src/lib.rs:483-538`; model docs `model.rs:181-206,239-243`). Parser accepts only versions 256–263 (`lib.rs:40-50`). B portrait path indexes lookup slot 0; pane path can index directly (`benilla-formats/src/models/records.rs:221-244`). |
| Type | i32 +0 retained in snapshot (`m2_camera.rs:139-147`). | u32 read +0, interpreted as i32 (`benilla-m2/src/lib.rs:495-500`); retained (`model.rs:187-188`). |
| FoV | Track descriptor +96, first populated scalar value; default 1 radian when no key (`m2_camera.rs:7,90-104,144-145`). | f32 base +4 and retained (`model.rs:189-191`; parser `benilla-m2/src/lib.rs:495-500`). |
| Far clip | f32 base +4, retained (`m2_camera.rs:146-147`). | f32 +8, retained (`model.rs:192-193`; parser `lib.rs:495-500`). |
| Near clip | f32 base +8, retained (`m2_camera.rs:146-147`). | f32 +0x0c, retained (`model.rs:194-195`; parser `lib.rs:495-500`). |
| Position track descriptor | Track +12; Vec3 first key offset added componentwise to base at +32 (`m2_camera.rs:130-138`). First-key snapshot only; interpolation/timestamps/tangents/global-seq discarded. | Spline track +0x10 and base Vec3 +0x2c; both retained (`model.rs:196-200`; parser `lib.rs:503-513`). Pane camera samples full retained track (`records.rs:367-393`). |
| Position X/Y/Z payload | Each component added to corresponding base component; E stores only the computed snapshot (`m2_camera.rs:131-138`). | All Vec3 value components retained per key, with spline track key data (`model.rs:196-200`; track schema `benilla-m2/src/track.rs:126–137,221–245`). |
| Target track descriptor | Track +44; first Vec3 key plus base at +64; snapshot only (`m2_camera.rs:130-138`). | Spline track +0x38 and base Vec3 +0x54 (`model.rs:200-203`; parser `lib.rs:503-513`); timed sampling in `records.rs:367-393`. |
| Target X/Y/Z payload | Componentwise base + first key; no later keys/tangents retained (`m2_camera.rs:133-138`). | Three values retained for each key in full track (`model.rs:200-203`; `track.rs:126–137,221–245`). |
| Roll track descriptor/payload | Track +76, first scalar offset (default zero) retained only as snapshot (`m2_camera.rs:143-144`). | Scalar spline +0x60 retained (`model.rs:204-206`), full track in `M2CameraTracks` (`records.rs:367-375`). |
| In/out tangents | E snapshot reader does not retain camera key tangents (`m2_camera.rs:90-112,130-148`). | B `M2Track`/spline representation carries tangent values for spline interpolation (`benilla-m2/src/track.rs:126–137,221–245`; `M2Camera` fields `model.rs:196-206`). |
| Lookup consumer | No camera lookup table in E camera parser. Native creation and portrait callers consume parsed snapshots (`godot/rust/src/char_create/scene.rs:208-220`; `unit_portraits.rs:474-475`). | `camera_lookup` retained by `M2Model`; portrait selects lookup[0], while UI model panes index selected camera (`benilla-m2/src/model.rs:239-243`; `benilla-formats/src/models/records.rs:221-244`; `benilla-app/src/ui_models/mod.rs:1132-1134`). |
| Dynamic camera consumer | E snapshot function explicitly does not evaluate animation (`m2_camera.rs:159-160`). | `M2PaneCamera::at(ms)` samples retained dynamic position/target/roll tracks (`benilla-formats/src/models/records.rs:357-393`); adapter constructs camera tracks (`benilla-assets/src/m2.rs:651–657`), asset sampler evaluates them (`:128–142`) and pane invokes it (`benilla-app/src/ui_models/mod.rs:1144`). |

## Bounds and collision

| Field | E — modern | B — 1.12.1 |
|---|---|---|
| Render bounds minimum | Vec3 at MD20 `0xa0`, retained (`pure.rs:51–62`; `core/src/m2.rs:369,429–430`). | Vec3 header minimum retained (`benilla-m2/src/model.rs:130–137`; `lib.rs:129–134,461–467`). |
| Render bounds maximum | Vec3 at MD20 `0xac`, retained (same E sources). | Independent Vec3 header maximum retained (same B sources). |
| Render sphere radius | No corresponding E field in the inspected `Model` or bounding-box parser (`core/src/m2.rs:65-70`; `pure.rs:57-62`). | f32 retained in `M2Header.bounding_sphere_radius` (`benilla-m2/src/model.rs:130-137`; `lib.rs:129-134,461-467`). |
| Collision AABB minimum | Vec3 at MD20 `0xbc`, validated and retained (`m2_collision.rs:8–10,39–48`; `core/src/m2.rs:370,431`). | Independent header minimum (`benilla-m2/src/model.rs:130–137`; `lib.rs:135–140,461–467`); `benilla-formats/src/creatures.rs:879–883` is a test comparison, not production use. No direct collision-box consumer found in bounded B format/native Rust search; production height comes from catalog (`creatures.rs:349–374`). |
| Collision AABB maximum | Vec3 at MD20 `0xc8`, validated and retained (same E sources). | Independent header maximum (same B sources), not render bounds. |
| Collision sphere radius | No field in E `M2CollisionMesh` parser/model (`m2_collision.rs:15-20`; `core/src/m2.rs:68-70`). | f32 retained (`benilla-m2/src/model.rs:130-137`; `lib.rs:135-140,461-467`). |
| Triangle indices | Array header at MD20 +0xD8; u16 indices; validates triangle multiple and vertex range (`m2_collision.rs:9,27-35,93-108`). Retained typed (`core/src/m2.rs:370`). | Raw u16 `bounding_triangles` decoded into u32 `CollisionMesh.indices`; rejects invalid hull (`benilla-formats/src/models/collision.rs:42-72`; raw header arrays in `benilla-m2/src/lib.rs:141-145`). |
| Vertex positions X/Y/Z | Array header at +0xE0; each vertex C3/f32×3; retained typed (`m2_collision.rs:10-11,36-40,76-88`). | `bounding_vertices` C3 bytes decoded to `[f32;3]` (`benilla-formats/src/models/collision.rs:43-59`). |
| Native collision use | E converts triangles to engine axes `(x,z,-y)`, creates `ConcavePolygonShape3D` and named StaticBody (`godot/rust/src/terrain/doodad_collision.rs:19-55`); model parsing stores the optional mesh (`core/src/m2.rs:367-370`). | `CollisionMesh` is supplied to terrain placement collider (`benilla-world/src/terrain_stream/collider.rs:163-164`); B raw geometry path is `benilla-formats/src/models/collision.rs:42-72`. |

## Version/layout and opaque-byte cautions

- E cameras: MD20 camera-array offset `0x110`, 116-byte (0x74) record. B cameras: header `0x124`, 124-byte (0x7c) record and separate lookup; B parser explicitly targets 256–263. Field offsets differ, including position track (+12 E vs +0x10 B), base (+32 vs +0x2c), target (+44 vs +0x38), roll (+76 vs +0x60), FoV (+96 track vs +4 base). These are version-specific layouts.
- E lights use header `0x108`, 156-byte record; B lights use header `0x11c`, 212-byte record. Same conceptual seven channels do not imply same offset/descriptor encoding.
- E attachments are 40-byte records with a known/skipped 20-byte AnimBlock; B attachments are 48-byte records where only ID/bone/position are retained. Their tails are opaque and not equivalent.
- E event records are 36 bytes; B records are 44 bytes. B full animation parser reads 1.12 raw event payload/times, while its separate marker API discards data/timeline. E preserves timestamps but native reported-action path selects a subset of IDs.
- E collision parser treats triangle vertices/indices and collision bounds as one parsed mesh. B's M2 header separately carries render and collision boxes/radii; B collision hull arrays are in `bounding_vertices`/`bounding_triangles`. Do not equate AABBs, hull fields, or consumer semantics from names alone.
- Other opaque ranges were not decoded into schema fields in these source slices; no layout equivalence is inferred for them.

## Coverage gaps / bounded claims

1. Static parser-to-source data flow is traced for requested fields, but this is not rendered/runtime parity, visual equivalence, nor validation against real bytes.
2. E camera path represents snapshots; full animated camera equivalence to B's `M2PaneCamera::at` is not present in this source path. Confirm additional consumers before treating this as an exhaustive whole-client statement.
3. B placed-light chain is established below; other ambient/directional/carried-light paths are not exhaustively traced. E native point-light path is established separately from scene ambient: type1/diffuse/visibility and conditional attenuation tracks bind native nodes. Static wiring does not prove rendered light fidelity.
4. B's `AnimEvent` full per-key route is established; B marker-only projection is a separate API. E direct sound chain uses event identifiers/times, and source search found no direct reads of `M2Event.data/bone/position` there; other non-Rust/indirect consumers are outside the claim.
5. E and B bounds/collision field names and payloads differ. Full header format/version evidence beyond source parser definitions was not independently cross-checked against an external format spec (network intentionally not used).
6. No repo files were changed; old `/tmp/m2-scene-fields.md` was treated as fallible input, not authority.


## Descriptor members and omitted spans

For each of the seven light tracks, apply every ordinary descriptor row in [ribbon descriptors](benilla-m2-ribbon-fields.md#track-descriptors-six-instances-not-six-additional-record-types): interpolation, global-sequence, outer timestamp count/offset, outer value count/offset, inner timestamp count/offset, inner value count/offset, timestamps and typed values. Light-specific decoder is `m2_light.rs:60–100`, not the ordinary decoder's external-source path. Native point-light global routing is `E/godot/core/src/m2_lights.rs:49–65`; ordinary interpolation limitations still apply.

| Descriptor input | Location / type | Handling |
|---|---|---|
| Event interpolation | `record+0x18 u16` | Skipped; no retained field (`m2_event.rs:39–74`). |
| Event global-sequence index | `record+0x1a i16` | Skipped; event parser instead indexes supplied sequence sources. |
| Event outer timestamp count | `record+0x1c u32` | Decodes number of sequence arrays. |
| Event outer timestamp offset | `record+0x20 u32` | Locates inner descriptors; discarded after decoding. |
| Event inner timestamp count | `inner+0 u32` | Decodes timestamp vector length (`m2_anim.rs:354–366`). |
| Event inner timestamp offset | `inner+4 u32` | Address in selected model/external payload; discarded. |
| Event timestamp element | `u32` ms | Retained; selected identifiers/times reduced by native action path above. |
| SKA1 attachment count | `chunk+0 u32` |40-byte records; `m2_attach.rs:60–83`. |
| SKA1 attachment offset | `chunk+4 u32` | Address consumed. |
| SKA1 lookup count | `chunk+8 u32` | Signed16 lookup; `m2_attach.rs:104–121`. |
| SKA1 lookup offset | `chunk+12 u32` | Address consumed. |
| Camera interpolation | Each track `+0 u16` | Not read; snapshot only (`m2_camera.rs:49–104`). |
| Camera global-sequence index | Each track `+2 i16` | Not read; snapshot does not route clocks. |
| Camera outer timestamp count | Each track `+4 u32` | Read/validated against value sequence count. |
| Camera outer timestamp offset | Each track `+8 u32` | Validates inner arrays; no timeline retained. |
| Camera outer value count | Each track `+12 u32` | Must match timestamp sequence count. |
| Camera outer value offset | Each track `+16 u32` | Finds first populated sequence's values. |
| Camera inner timestamp count | Inner descriptor `+0 u32` | Must match corresponding value count. |
| Camera inner timestamp offset | Inner descriptor `+4 u32` | Range validated; timestamp contents not sampled. |
| Camera inner value count | Inner descriptor `+0 u32` | Determines nonempty sequence. |
| Camera inner value offset | Inner descriptor `+4 u32` | First key selected; later keys not retained. |
| Camera key value | Vec3 for position/target, float for roll/FoV | First value retained into snapshot; position/target bases added. |
| Camera in tangent | Same dimension as value | Key stride reserves all three components, tangents not read. |
| Camera out tangent | Same dimension as value | Not read. |
| Camera position base | `Vec3 record+32` | Authored position base read by `m2_camera.rs:130–138`; added to first track value. B base `+0x2c`. |
| Camera target base | `Vec3 record+64` | Independent authored target base, same decoder; B base `+0x54`. |

Camera position/target bases are independently authored fields, not derived offsets. E record accounting: type/far/near12 + position-track20 + position-base12 + target-track20 + target-base12 + roll-track20 + FoV-track20 =116 bytes. Snapshot positions/targets are derived sums. B `M2PaneCamera::at` actually samples its retained tracks; `B/crates/benilla-m2/src/track.rs:168–209` dispatches step/linear/Bézier/Hermite, but that sampler itself does not route ranges/global clocks. B camera ability is not universal cubic-bone support.

## B descriptor members

Offsets below are relative to each B track, not E offsets. Apply light rows to all seven named B channels; `B/crates/benilla-formats/src/models/records.rs:64–91,164–202` reads first values. Event descriptor is relative to the event record's track base+24 (`B/crates/benilla-formats/src/models/anim.rs:558–600`).

| B serialized member | Offset / type | Handling |
|---|---|---|
| Light interpolation | `+0 u16` | Not decoded by first-value reader. |
| Light global-sequence index | `+2 u16` | Not decoded; no retained clock. |
| Light range count | `+4 u32` | Not decoded. |
| Light range offset | `+8 u32` | Not decoded; range payload unretained. |
| Light timestamp count | `+12 u32` | Not decoded. |
| Light timestamp offset | `+16 u32` | Not decoded; timestamp payload unretained. |
| Light value count | `+20 u32` | Checks nonzero. |
| Light value offset | `+24 u32` | Reads first typed value only. |
| Event interpolation | `+0 u16` | Not decoded by full animation-event adapter. |
| Event global-sequence index | `+2 u16` | Not decoded. |
| Event range count | `+4 u32` | Not decoded. |
| Event range offset | `+8 u32` | Not decoded; range payload unretained. |
| Event timestamp count | `+12 u32` | Iterates flat timestamps with bounds checks. |
| Event timestamp offset | `+16 u32` | Reads flat u32 milliseconds; emits keys into matching sequence bands. |

## Sources

- E parser files `godot/core/src/asset/m2_format/{m2_attach,m2_event,m2_light,m2_camera,m2_collision,pure}.rs`; production `godot/core/src/{m2,m2_lights}.rs`, `godot/rust/src/assets/{attachments,m2_lights,mod}.rs` and named action/scene consumers above.
- B base `crates/benilla-m2/src/{lib,model,track}.rs`; adapters `crates/benilla-formats/src/models/{anim,records,collision}.rs`; named asset/app consumers above.
- [M2 format](../formats/m2-format.md) — project format SSOT; no tests rerun by this inventory.

## See Also

- [[benilla-m2-header-fields]] — separate count/offset and container fields.
- [[benilla-m2-animation-fields]] — ordinary track evaluation and external sources.
- [[benilla-m2-field-census]] — named declaration scope, not disk-field count.
- [[benilla-asset-field-inventory]] — broader research scope.
