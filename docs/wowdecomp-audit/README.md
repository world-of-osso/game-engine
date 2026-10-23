# wowdecomp behavior rework backlog

Source audit: **2026-10-15**. **28 source-supported conditional gaps across eight behavior domains**, plus five separately gated validation candidates and three additional unresolved 4.3.4 leads. This is a growing reviewed-slice inventory—not an upper bound on project gaps or a claim of exhaustive compatibility coverage. Unreviewed means **unknown**, not matching.

No client, server, shader, build or reference executable was run. Findings establish wrong/missing semantics for specified source inputs; affected-asset prevalence, visible/audible impact and target-client parity remain unmeasured.

## Sources and applicability

`wowdecomp/README.md:3–14` identifies reconstructed **WoW 3.3.5a, build 12340**, not current Retail. Original-address annotations aid inspection but do not independently certify reconstruction fidelity. Preserve newer behavior unless comparable target-build evidence supports a change.

| Alias | Checkout | Source revision |
|---|---|---|
| W | `~/Repos/wowdecomp` | `0e4e9fe0a4a9fa08027c5eed4b2c5e873739cfd3` |
| E | game-engine, this repository | `3bdfb57b2e99205614e81bb7736ee3130677f24d` |
| S | sibling `../game-server` | `0dcaa23bde680678d72b3c43a786eb574bd809b8` |
| C | `~/Repos/wowdecomp15595` — 4.3.4/build 15595 | `959b0b85c5f55c565328d7c3ca33027a65e41327` |
| E4 | game-engine, 4.3.4 comparison baseline | `3abdb6e6cd3273e08d65c97ddfa76090372af01a` |

Path/line citations refer to these pinned source revisions. Expansion began at engine `201d6288a`, whose changes from E were documentation only; implementation sources remained identical. Reference/server were clean; engine's unrelated untracked files were preserved. This work changes documentation only and prescribes no server policy from a client-only reference.

The [4.3.4 expansion](cataclysm-434.md#pins-and-historical-boundary) adds C as a separate historical reference, not a replacement for W. E4's intervening changes are documentation only. C explicitly carries some older-build bodies and disclaims behavioral correctness from address-tag coverage; the original 23 findings have not been wholesale revalidated against C.

## Finding index

IDs identify behaviors, not measured severity. Each entry contains paired source/call-path evidence, required observable changes, concrete acceptance fixtures and unresolved applicability.

**Implementation snapshot: lead-reported, 2026-10-16.** `[x]` means reported landed on agent-server, not independently revalidated or native-parity complete; `[ ]` includes partial, gap, in-progress and not-started work. Descriptions remain the historical findings at the pinned E/E4 baselines, not assertions about today's remote master. Status provenance, remaining scope and unavailable commit verification are recorded in [implementation follow-through](coverage.md#implementation-follow-through).

### M2 track sampling and model runtime — 7

| ID | Historical incorrect or omitted behavior | Detail | Implementation status / SHA (reported) |
|---|---|---|---|
| [ ] A1 | Step bone/vector/scalar tracks are smoothed | [Interpolation](m2-animation.md#a1-step-tracks-are-interpolated) | Not started / awaiting approval. No implementation SHA reported. |
| [ ] A2 | Ribbon alpha holds keys instead of interpolating | [Ribbon opacity](m2-animation.md#a2-ribbon-alpha-is-always-stepped) | Not started / awaiting approval. No implementation SHA reported. |
| [ ] A3 | Short timeline tables lose authored timeline-zero transforms | [Timeline selection](m2-animation.md#a3-short-track-tables-do-not-select-timeline-zero) | Not started / awaiting approval. No implementation SHA reported. |
| [ ] A4 | General model events lack dispatch, timing context and payload; selected action events already work | [Event coverage](model-runtime.md#a4-general-authored-events-lose-dispatch-timing-context-and-payload) | Not started / awaiting approval. No implementation SHA reported. |
| [ ] A5 | Replay bounds do not extend finite sequence residence | [Repeat scheduling](model-runtime.md#a5-replay-bounds-do-not-control-finite-sequence-residence) | Not started / awaiting approval. No implementation SHA reported. |
| [x] A6 | Low bone inheritance bits cannot adjust the parent contribution | [Bone composition, C](cataclysm-434.md#a6-low-bone-inheritance-bits-do-not-affect-native-pose-composition) | Reported landed: `19d9850e8` |
| [x] A7 | Authored attachment activation is discarded | [Child participation, C](cataclysm-434.md#a7-authored-attachment-activation-is-discarded) | Reported landed: `514c05a5e`, `65beaaa4e` |

### Particle geometry and motion — 7

| ID | Historical incorrect or omitted behavior | Detail | Implementation status / SHA (reported) |
|---|---|---|---|
| [x] P1 | Tail-only particles disappear; head-and-tail particles lose tails | [Tail geometry](particles-and-shaders.md#p1-particle-tails-are-not-drawn) | Reported landed: `1ce73e656`, `443e43b4d` |
| [x] P2 | Follow-position does not carry eligible surviving particles | [Emitter following](particles-and-shaders.md#p2-follow-position-is-ignored) | Reported landed: `05de65324` |
| [ ] P3 | Authored emitter velocity is not inherited at birth | [Inherited velocity](particles-and-shaders.md#p3-emitter-velocity-is-not-inherited) | Partial: modern scale decode landed; timer/reset gap remains. SHA not supplied. |
| [ ] P4 | Spline emitters are filtered out | [Spline emission](particles-and-shaders.md#p4-spline-emitters-are-filtered-out) | Partial: spline layout/curve landed; per-unit density gap remains. SHA not supplied. |
| [ ] P5 | Model-particle and recursive-emitter model references are unused | [Model particles](particles-and-shaders.md#p5-model-particles-and-recursive-emitters-have-no-native-path) | Named gap: product resolution and child timing. No completion SHA. |
| [x] P6 | Authored per-particle depth sorting is ignored | [View-depth order](particle-order-and-ground.md#p6-authored-per-particle-depth-sorting-is-ignored) | Reported landed: `5b76a724d` |
| [x] P7 | Ground-projected births do not query world geometry | [World contact](particle-order-and-ground.md#p7-ground-projected-particle-births-do-not-query-world-geometry) | Reported landed: `7cfed41dd` |

### Mesh ordering and material lighting — 4

| ID | Historical incorrect or omitted behavior | Detail | Implementation status / SHA (reported) |
|---|---|---|---|
| [ ] R1 | M2 section sort center/radius cannot affect transparent ordering | [Authored sort metadata](world-shading.md#r1-authored-m2-sort-center-and-radius-cannot-influence-draw-order) | Not started / awaiting approval. No implementation SHA reported. |
| [ ] S1 | Lit/unlit particle distinction has no native lighting branch | [Particle lighting](particles-and-shaders.md#s1-lit-particles-are-rendered-unshaded) | Named gap: lit-flag polarity lacks allowed-source oracle. No completion SHA. |
| [ ] S2 | WMO SIDN color/curve contribution is retained but not bound | [Time-dependent emissive](world-shading.md#s2-authored-wmo-sidn-emissive-color-is-not-bound) | Not started / awaiting approval. No implementation SHA reported. |
| [ ] S3 | MCSH mask does not reach terrain-surface shading | [Authored terrain shadows, C](cataclysm-434.md#s3-authored-mcsh-shadows-do-not-reach-terrain-surface-shading) | In progress, unlanded: mask transport proven; modulation on `terrain-mcsh-cleanroom`. SHA not supplied. |

### Environment — 2

| ID | Historical incorrect or omitted behavior | Detail | Implementation status / SHA (reported) |
|---|---|---|---|
| [ ] ENV1 | No precipitation/weather transition or storm-light consumer | [Weather](environment.md#env1-weather-state-has-no-precipitation-or-storm-light-consumer) | Not started / awaiting approval. No implementation SHA reported. |
| [ ] ENV2 | Camera immersion does not select underwater scene fog/light | [Underwater environment](environment.md#env2-camera-immersion-does-not-select-underwater-scene-fog-and-light) | Not started / awaiting approval. No implementation SHA reported. |

### Ground contact — 2

| ID | Historical incorrect or omitted behavior | Detail | Implementation status / SHA (reported) |
|---|---|---|---|
| [ ] M1 | Oblique wall contact discards residual tangential travel | [Wall sliding](movement.md#m1-oblique-wall-contact-discards-remaining-tangential-movement) | Not started / awaiting approval. No implementation SHA reported. |
| [ ] M2 | Floor snapping lacks clearance-aware obstacle step-up | [Step clearance](movement.md#m2-sampled-floor-snapping-does-not-implement-clearance-aware-step-up) | Not started / awaiting approval. No implementation SHA reported. |

### Camera clearance and target opacity — 2

| ID | Historical incorrect or omitted behavior | Detail | Implementation status / SHA (reported) |
|---|---|---|---|
| [ ] C1 | Camera obstruction queries lack lateral extent | [Clearance corridor, C](cataclysm-434.md#c1-camera-obstruction-probes-have-no-lateral-extent) | Not started / awaiting approval. No implementation SHA reported. |
| [ ] C2 | Close-camera fade has no target-model binding | [Camera target opacity, C](cataclysm-434.md#c2-close-camera-target-fade-has-no-native-camera-to-player-binding) | Not started / awaiting approval. No implementation SHA reported. |

### World audio — 4

| ID | Historical incorrect or omitted behavior | Detail | Implementation status / SHA (reported) |
|---|---|---|---|
| [ ] AU1 | Parsed MCSE terrain sound emitters never reach playback | [Spatial world emitters](audio.md#au1-parsed-terrain-mcse-emitters-never-reach-native-playback) | Not started / awaiting approval. No implementation SHA reported. |
| [ ] AU2 | Subarea/WMO identity and inside state cannot select sound overrides | [Contextual selection](audio.md#au2-subarea-and-wmo-context-cannot-select-world-musicambience) | Not started / awaiting approval. No implementation SHA reported. |
| [ ] AU3 | Music immediately cycles a track cursor instead of authored scheduling | [Music timing/arbitration](audio.md#au3-zone-music-is-immediate-cursor-cycling-not-the-authored-scheduler) | Not started / awaiting approval. No implementation SHA reported. |
| [ ] AU4 | Interior/underwater reverb preferences are not applied | [Acoustic environment](audio.md#au4-contextual-interiorunderwater-reverb-is-not-applied) | Not started / awaiting approval. No implementation SHA reported. |

The eight domains are M2 sampling/runtime, particles, mesh ordering, material lighting, environment, movement, camera and audio. Some share files; counts enumerate distinct conditional behaviors, not independent fixes or confirmed affected assets.

## What this inventory does not establish

[Coverage](coverage.md) records reviewed, implemented/matching, mismatched, uncertain and unreviewed areas—including substantial UI, networking, gameplay and model-transform gaps in the **audit itself**. [Validation candidates](validation-candidates.md) separates alpha-key cutoff, quaternion curve, fog-generation differences, modern refraction and reference depth-darkening conflicts from confirmed paired omissions. No unsupported behavior is automatically a defect in a deliberately narrower project contract.

Before rework, identify the applicable product/build and a real authored asset, preserve its hashes and reproduce the counterexample through the native consumer. Reconcile the governing feature spec in that implementation change. This is a behavioral backlog, not authorization to implement or an architecture prescription.

## Related knowledge

- [12340 tooltip padding and aura files](tooltip-padding-buff-12340.md): source-derived width arithmetic and 3.4.3 cache versus unavailable 12340 UI files (2026-10-23).
- [12340 CloseAuctionHouse](closeauctionhouse-12340.md): Lua binding, local state cleanup, event order and proof limits (2026-10-22).
- [12340 UI source questions](ui-source-questions-12340.md): texture filenames, auction return values, addon dependencies, and unresolved FrameXML ordering (2026-10-22).

- [M2 format](../wiki/formats/m2-format.md) and [animation](../wiki/systems/animation.md).
- [Particle contract](../specs/m2-particles.md) and [ribbon contract](../specs/m2-ribbons.md).
- [Retail lighting](../wiki/systems/retail-lighting.md), [sound](../wiki/systems/sound.md) and [reference clients](../wiki/reference/open-source-wow-clients.md).
