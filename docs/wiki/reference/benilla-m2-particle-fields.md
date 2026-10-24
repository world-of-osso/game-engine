# Benilla / game-engine M2 particle fields

Individual particle-record inputs, decoded representations and production boundaries. Static inspection only, 2026-10-24: E `/syncthing/Sync/Projects/world-of-osso/game-engine`, native source baseline `354bdb7dbf177047ed46d63168acec531f5c4426`; B `/home/osso/Repos/benilla`, `8a6c74036cfe3d00275fd75ad1aeae1f4ea7d0df`. No source execution, builds, tests, extraction or visual comparison. This slice does not complete the broader [inventory](benilla-asset-field-inventory.md).

## Layout and table keys

E parser **P** = `E/godot/core/src/asset/m2_format/m2_particle.rs`. Offsets are relative to an E emitter. `P:154–217` declares offsets, `P:472–502` reads the prefix, `P:518–577` fills decoded values, `P:634–650` reads generator tracks. E selects stride **492 bytes (0x1ec) for version >=272**, otherwise **376 bytes (0x178)** (`P:782–788`). That branch is not proof of general older-layout compatibility: later reads use a slice extending beyond the record and default missing reads. Do not call either record “272 bytes.” B uses different 28-byte tracks and a different emitter layout; its field offsets below are B-only.

**G** = `E/godot/core/src/m2_particles.rs:419–470,544–640`: generator values enter simulation, choosing per-track global/sequence clocks, then plane/sphere spawning and motion. **V** = same file `:283–315`: lifetime keys determine color, alpha, size and head cell. **N** = `E/godot/rust/src/particles.rs`: native placement/material wiring. **BP** = `B/crates/benilla-formats/src/particles.rs:550–665`: paired parser/adaptation evidence. A B parser row alone is not runtime proof.

## Prefix and generator fields

| Field | E serialized location / handling | Production fate | B counterpart / distinction (BP) |
|---|---|---|---|
| Opaque leading word | `+0x00..0x04`, not read by `P:477–502` | No identifier retained by this path; meaning unassigned. | BP also starts retained metadata at `+0x04`; do not invent identifier semantics. |
| Flags | `u32 +0x04`, retained | Simulation/material branches; decoded bits listed below. | Raw flags retained; same bit width is not cross-version equivalence. |
| Position | `3×f32 +0x08`, retained | Native emitter transform, N:460–474. | Bone-relative vector `+0x08`. |
| Bone index | `u16 +0x14`, retained | Native skeleton pose selection, N:460–474. | `u16 +0x14`. |
| Texture index word | `u16 +0x16`; ordinary index masked to low five bits | TXID resolution `P:705–722`, native texture binding N:102–110. | BP reads whole `u16 +0x16` as index into texture paths. |
| Geometry filename count | `u32 +0x18`, used for optional string | Filename retained; no consumer found in inspected E core simulation/native particle paths. | `u32 +0x18`; B does load geometry-model dependency, separate trace below. |
| Geometry filename offset | `u32 +0x1c`, address discarded after decoding | Same retained filename boundary. | `u32 +0x1c`, file-relative. |
| Geometry filename bytes | Counted bytes, first NUL then trim/lossy UTF-8 (`P:266–282`) | Retained `particle_model_filename`, not a demonstrated model-mesh emitter. | B stores `geometry_model`. |
| Recursion filename count | `u32 +0x20`, used for optional string | Retained filename; no recursion consumer found in inspected E particle paths. | `u32 +0x20`; B stores recursion dependency separately. |
| Recursion filename offset | `u32 +0x24`, address discarded | Same boundary. | `u32 +0x24`, file-relative. |
| Recursion filename bytes | Same optional-string decoder | Retained `child_emitters_model_filename`. | B stores `recursion_model`. |
| Blend type | Byte `+0x28`, retained | Native blend/depth/alpha-test selection, N:42–71,125–148. | Byte `+0x28`, folded blend plus derived lighting. |
| Emitter type | Byte `+0x29`, retained | Only 1 plane / 2 sphere accepted; other shapes filtered, G file:125–134 and N:315–325. | B reads `u16 +0x2a`; supports spline with control-point parsing. Not the same offset/type. |
| Particle type | Byte `+0x2a`, retained | No production read found in bounded E simulation/native particle paths; debug display is not rendering support. | B geometry selection is driven by geometry-model field; do not equate it with this byte. |
| Head/tail selector | Byte `+0x2b`, retained | Same bounded no-use result; E quad path explicitly emits heads only (`m2_particles.rs:646–655`). | B byte `+0x2c`, actual head/tail quad branch. |
| Texture rows | `u16 +0x30`, retained | Lifetime cell count V; native inverse atlas dimensions N:134–137. | BP requires power-of-two row/column pair, otherwise uses 1×1. |
| Texture columns | `u16 +0x32`, retained | Same independent dimension consumers. | B `u16 +0x32`. |
| Emission-speed track | 20-byte track `+0x34` | Static value plus conditionally retained track; G. | B28-byte track `+0x34` baked per sequence. |
| Speed-variation track | Track `+0x48` | G. | B track `+0x50`. |
| Vertical-range track | Track `+0x5c` | G. | B latitude track `+0x6c`. |
| Horizontal-range track | Track `+0x70` | G. | B longitude track `+0x88`. |
| Gravity track | Track `+0x84`, float or compressed four-byte vector key | G; compressed decoder `P:331–359,613–632`. Decoded scalar/vector are representations of this input, not two disk fields. | B float track `+0xa4`. |
| Lifespan track | Track `+0x98` | G and pool sizing `m2_particles.rs:191–208`. | B track `+0xc0`. |
| Lifespan variation | `f32 +0xac` | Seed variation and pool sizing `m2_particles.rs:199–202,261–267`. | No independently mapped B scalar in BP. |
| Emission-rate track | Track `+0xb0` | G and pool sizing. | B track `+0xdc` baked in `EmitTiming`. |
| Emission-rate variation | `f32 +0xc4` | Rate accumulation `m2_particles.rs:598–611`, pool sizing. | No independently mapped B scalar in BP. |
| Area-length track | Track `+0xc8` | G. | B track `+0xf8`. |
| Area-width track | Track `+0xdc` | G. | B track `+0x114`. |
| Z-source track | Track `+0xf0` | G unless EXP2 clears/replaces it. | B track `+0x130`. |
| Enabled track | Track `+0x1c8`, only when selected stride passes gate | Conditional retention; absence defaults **on** in `EmitterSim::set_animation` (`m2_particles.rs:489–503`). | B byte-value track `+0x1dc`, baked by `EmitTiming`. |

Each E generator track has the six outer descriptor fields and four inner descriptor fields enumerated in [ribbon descriptors](benilla-m2-ribbon-fields.md#track-descriptors-six-instances-not-six-additional-record-types), plus `u32` timestamps and typed values. Those are descriptor instances, not new generator parameters. Particle parser **discards the entire track unless at least one sequence has more than one timestamp** (`P:604–610`). Thus all-single-key sequences with distinct constants lose their per-sequence distinction; all-single-key enabled=0 is discarded and simulation defaults on. If any sequence has >1 key, other single-key sequences survive. This is a static source counterexample, not a reproduced shipped-asset failure. Native does call `set_animation` (N:484–489); no separate corrective override was found in that path. B bakes per-sequence constants (`BP:621–643`, `B/crates/benilla-formats/src/emit_timing.rs`), but this does not establish whole-system parity.

## Lifetime and scalar fields

Every row is independently retained by `P:533–577` unless marked opaque. Lifetime partial-track descriptor expansion follows this table.

| Field | E location / representation | Production consumer or bounded unknown | B counterpart (BP) |
|---|---|---|---|
| Color lifetime track | 16-byte partial track `+0x104`, Vec3 values | Full `color_keys` consumed by V; first-three `colors` representation not used by V. | Three packed colors `+0x150,+0x154,+0x158`; different encoding. |
| Opacity lifetime track | Partial track `+0x114`, signed fixed16 values | Full `opacity_keys` consumed by V. | Alpha embedded in B packed colors. |
| Scale lifetime track | Partial track `+0x124`, two-float values | Full `scale_keys` consumed by V. | Three scalar sizes `+0x15c,+0x160,+0x164`; no direct anisotropic layout match. |
| Scale variation X | `f32 +0x134` | V varies both axes or X independently by flag. | No independent BP equivalent established. |
| Scale variation Y | `f32 +0x138` | V when 2D variation flag set. | No independent BP equivalent established. |
| Head-cell lifetime track | Partial track `+0x13c`, u16 values | Full keys consumed by V; first-three `head_cell_track` are duplicate decoded representation. | B head begin/end/repeat ramps, not modern partial tracks. |
| Tail-cell lifetime track | Partial track `+0x14c`, u16 values | No consumer found in bounded E simulation/native particle paths. | B independent tail ramps feed tail-quad UV. |
| Tail length | `f32 +0x15c` | Retained, no consumer found in those paths. | `tail_time f32 +0x17c`, used by tail geometry. |
| Twinkle speed | `f32 +0x160` | `m2_particles.rs:332–343` phase selection. | `+0x180`. |
| Twinkle percent | `f32 +0x164` | Same function, particle visibility threshold. | `+0x184`. |
| Twinkle minimum scale | `f32 +0x168` | Same function, size floor. | `+0x188`. |
| Twinkle maximum scale | `f32 +0x16c` | Same function, size range. | `+0x18c`. |
| Burst multiplier | `f32 +0x170`, nonpositive replaced by 1 | Retained; no consumer found in bounded E simulation/native particle paths. | No exact BP counterpart established. |
| Drag | `f32 +0x174` | Motion damping `m2_particles.rs:584–591`. | `+0x194`. |
| Base spin | `f32 +0x178` | Rotation sampling `m2_particles.rs:771–787`. | No independent B base-angle counterpart established. |
| Base-spin variation | `f32 +0x17c` | Same rotation sampler. | No independent BP counterpart established. |
| Spin speed | `f32 +0x180` | Same rotation sampler and quad orientation. | `spin +0x198`; B also authors vector angular velocity bounds. |
| Spin-speed variation | `f32 +0x184` | Same rotation sampler. | No scalar one-to-one BP counterpart established. |
| Opaque span | `+0x188..0x1a0`, not decoded by inspected parser | No retained payload; no invented semantics. | B fields at same offsets belong to different layout. |
| Wind vector | `3×f32 +0x1a0` | Coordinate conversion and early-life acceleration `m2_particles.rs:553–583`. | No BP counterpart mapped. |
| Wind time | `f32 +0x1ac` | Same early-life acceleration gate. | No BP counterpart mapped. |
| Follow speed 1 | `f32 +0x1b0` | Retained; no consumer found in bounded E simulation/native particle paths. | `+0x1c4`; pair contributes B follow relation. |
| Follow scale 1 | `f32 +0x1b4` | Same bounded unknown. | `+0x1c8`. |
| Follow speed 2 | `f32 +0x1b8` | Same bounded unknown. | `+0x1cc`. |
| Follow scale 2 | `f32 +0x1bc` | Same bounded unknown. | `+0x1d0`. |
| Spline-point descriptor | `+0x1c0..0x1c8`, not decoded by E parser | Type3 filtered; cannot establish payload or layout from a code comment alone (`P:601–602`, shape gate above). | B count/offset `+0x1d4/+0x1d8`, cubic points retained, arc knots derived (BP:568–579). |
| Inheritance scale | **Not an established E disk field**; `inherit_velocity_scale` defaults 1 (`E/godot/core/src/asset/m2_particle_defaults.rs:47`) | No consumer found in bounded E particle paths. | Authored B `f32 +0x190`; passed to `inherit_trigger` (`B/crates/benilla-world/src/particles/sim.rs:781–790`). Exact helper arithmetic not audited here. |
| Midpoint | **Derived** from second color timestamp, default0.5 (`P:451–461`) | No separate midpoint consumer in V, which samples full keys instead. | Independent B `f32 +0x14c`; not the same disk field. |

## Partial tracks, multitexture and EXP2

For each of color/opacity/scale/head-cell/tail-cell and EXP2 alpha cutoff, partial track members are: timestamp count `u32 +0`, timestamp offset `u32 +4`, value count `u32 +8`, value offset `u32 +12`; timestamp elements are normalized `clamp(u16 /32767,0,1)`, values are the row-specific type. Counts/addresses are consumed then discarded; decoded `(time,value)` pairs retained up to the smaller count (`P:284–295,428–448`). This schema is distinct from ordinary tracks: no interpolation/global-sequence words. Address base is MD20 for ordinary particle lifetime data, EXP2 chunk for cutoff.

| Field | Serialized input | Handling / consumer |
|---|---|---|
| Packed texture layer0 index | Low5 bits of `u16 +0x16` | Decoded and TXID-resolved; N:102–110. |
| Packed texture layer1 index | Bits5–9 of same word | Same; only when multitexture flag set (`P:667–685`). |
| Packed texture layer2 index | Bits10–14 of same word | Same; bit15 not assigned semantics by this decoder. |
| UV scale layer1 | Byte `+0x2c` | Decoded as low5 fraction/high3 integer, `m2_particles.rs:180–186`. |
| UV scale layer2 | Byte `+0x2d` | Same, independent value. |
| Opaque bytes | `+0x2e..0x30` | Not decoded by inspected prefix/multitexture parser. |
| UV velocity midpoint layer1 | Two i16 `+0x1dc`, `/512` | Retained; particle UV-motion initialization `m2_particles.rs` `spawn_texture_motion`. |
| UV velocity midpoint layer2 | Two i16 `+0x1e0`, `/512` | Same independent layer. |
| UV velocity range layer1 | Two i16 `+0x1e4`, `/512` | Same initialization; exact algorithm not a native-parity claim. |
| UV velocity range layer2 | Two i16 `+0x1e8`, `/512` | Same. E492-byte record fully ends after this field. |
| EXP2 record count | `u32 chunk+0` | Limits emitter-order assignment (`P:747–775`). |
| EXP2 record offset | `u32 chunk+4` | Locates 28-byte records; descriptor address discarded. |
| EXP2 zSource | `f32 record+0` | Replaces MD20 scalar; presence clears all z-source tracks even if a later record cannot decode (`P:756–774`); G consumes value. |
| EXP2 color multiplier | `f32 record+4` | V multiplies RGB. |
| EXP2 alpha multiplier | `f32 record+8` | V multiplies alpha. |
| EXP2 alpha-cutoff track | Partial track `record+12`, signed-fixed16 values | Retained `alpha_cutoff_keys`; no consumer found in inspected E core/native particle paths. Native blend-based alpha test is a different input. |

## Flag decoding and unknowns

Raw flags remain retained. Definitions at `E/godot/core/src/m2_particles.rs:14–30`; individual tested bits below. Unlisted bits remain uninterpreted here, not declared unused. This table is E behavior, not a same-bit Vanilla equivalence map.

| Bit | Named branch | Consumer evidence |
|---|---|---|
| `0x4` | Velocity-oriented | Quad axes `m2_particles.rs:825–846`. |
| `0x10` | Model-space | Simulation coordinate basis and spawn transforms `m2_particles.rs:553–560,625–637`. |
| `0x20` | Inherit-scale | Quad scale `m2_particles.rs:672–676`; not velocity inheritance. |
| `0x80` | Sphere implode | Particle retirement against emitter center `m2_particles.rs:570–593`. |
| `0x100` | Sphere-up | Sphere spawn `m2_particles.rs:750–760`. |
| `0x200` | Spin flip | Per-seed rotation sign `m2_particles.rs:803–811`. |
| `0x1000` | Local orientation | Quad-axis basis `m2_particles.rs:825–846`. |
| `0x10000` | Random cell | Lifetime cell selection V. |
| `0x20000` | Head | Required for head quad generation `m2_particles.rs:646–655`. |
| `0x80000` | 2D scale variation | Independent Y variation V. |
| `0x100000` | Refraction | Core shader selection `m2_particles.rs:152–165`; filtered from native forward rendering N:315–325. |
| `0x800000` | Compressed gravity | Parser vector decode `P:331–359,613–632`. |
| `0x2000000` | No global density scale | Native density selection N:476–481. |
| `0x10000000` | Multitexture | Packed input decode P:667–685, core shader selection and native texture binding. |
| `0x40000000` | Three color | Multitexture color/alpha combiner selection `m2_particles.rs:152–165`. |

Refraction filtering is not missing ordinary multitexture support. Inherit-scale does **not** establish consumption of the default `inherit_velocity_scale` member.

## Benilla model particles: resolved contradiction

B support is not limited to recursion. Geometry and recursion filenames become separate `M2Model` dependency handles (`B/crates/benilla-assets/src/m2.rs:343–350`). Runtime copies the geometry handle (`B/crates/benilla-world/src/particles.rs:158–163,532–535`); simulation emits particles but suppresses ordinary quads when geometry is authored (`particles/sim.rs:876–914,967`). `particles/model.rs:46–126,132–180` resolves the loaded model, creates static submesh instances and updates per-particle transforms/color/size. Scheduling invokes this after simulation (`particles.rs:679–687`). This establishes a source chain to static model meshes, **not animated geometry-model fidelity or successful load of every path**.

Recursion separately resolves the recursion model and wires up to four emitter definitions (`B/crates/benilla-world/src/particles.rs:559–606`). Head/tail quads use `head_tail` and separate UV/lifetime inputs (`particles/quads.rs:168–239`); those are neither model-mesh rendering nor a consumer of E's stored filename. B-only/differently encoded payload members follow. Each is read in BP; normalized colors, ramp base/span and sampled outputs are derived, not extra disk fields.

| B authored member | B offset / type | Adaptation / consumer boundary |
|---|---|---|
| Lifetime midpoint | `+0x14c f32` | `OverLife::sample` segment split (`B/crates/benilla-formats/src/particles.rs:109–143`). |
| RGBA start | `+0x150 packed BGRA` | Normalized to color[0], same lifetime sampler. |
| RGBA middle | `+0x154 packed BGRA` | color[1], same sampler. |
| RGBA end | `+0x158 packed BGRA` | color[2], same sampler. |
| Size start | `+0x15c f32` | scale[0], same sampler. |
| Size middle | `+0x160 f32` | scale[1], same sampler. |
| Size end | `+0x164 f32` | scale[2], same sampler. |
| Head A begin | `+0x168 u16` | `CellRamp::new` derives signed base/span; lifetime sampler produces head cell. |
| Head A end | `+0x16a u16` | Same ramp, independent endpoint. |
| Segment A repeat | `+0x16c u16` | Converted to float; fractional repeat affects cells only (`particles.rs:131–143`). |
| Head B begin | `+0x16e u16` | Second head ramp. |
| Head B end | `+0x170 u16` | Second head ramp endpoint. |
| Segment B repeat | `+0x172 u16` | Second segment cell repeat. |
| Tail A begin | `+0x174 u16` | Independent tail cell ramp. |
| Tail A end | `+0x176 u16` | Same ramp endpoint. |
| Tail B begin | `+0x178 u16` | Second tail cell ramp. |
| Tail B end | `+0x17a u16` | Same ramp endpoint. |
| Angular-velocity minimum | `+0x19c Vec3` | Retained in `ParticleEmitterDef`; model-particle rotation birth branch `particles/sim.rs:876–914`. |
| Angular-velocity maximum | `+0x1a8 Vec3` | Same branch, independent bound. |
| Spline point count | `+0x1d4 u32` | BP derives point cardinality and validates payload. |
| Spline point offset | `+0x1d8 u32` | Address used then discarded. |
| Spline point | `Vec3[]` | Cubic control points retained; arc knots derived; spline emission `B/crates/benilla-world/src/particles/emit.rs:49–59`. |

Full B reserved-byte accounting and individual B flag consumers remain open. E parser-input list is not a claim that every B record byte has a settled semantic identity.

## Sources

- E parser, defaults, simulation and native paths identified above; B parser, asset loader and particle runtime identified above.
- [M2 declared-field census](benilla-m2-field-census.md) — raw/cooked/derived declaration roster; not a unique disk-field count.
- [M2 format](../formats/m2-format.md) — format SSOT; existing fixture results are separate historical evidence, not tests run for this research.

## See Also

- [[benilla-m2-ribbon-fields]] — ordinary track descriptors and ribbon handling.
- [[benilla-asset-field-inventory]] — cross-format scope.
- [[m2-format]] — format documentation.
