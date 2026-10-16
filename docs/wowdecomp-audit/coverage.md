# Coverage, exclusions and proof ledger

[Finding index and source pins](README.md). **28 findings do not mean 28 total project gaps.** They describe completed paired-source comparisons within the slices below. Absence of a finding is not parity; unreviewed rows are not implementation findings.

## Reviewed capability matrix

| Slice | Mismatched / missing behavior | Implemented or matching evidence | Uncertain / still unreviewed | Proof |
|---|---|---|---|---|
| Generic M2 track evaluation | A1–A3 | Material sampler separately honors interpolation and short-table rules | Asset census, spline interpolation families, native outputs | Paired functions + explicit counterexamples |
| Model events / replay | A4–A5 | Six selected action/missile IDs, variation families and crossfades have native paths | Other event handlers and real finite-repeat usage; independent event-clock/pose semantics and original-binary fidelity need separate proof | Paired runtime paths; no execution |
| Bone transforms / attachments / skinning | A6–A7: low-bit parent adjustment and attachment activation, using C | Hierarchy/billboards and static bone attachment points exist | IK, runtime flag producers, child color-pass/reactivation behavior and weighted normal transforms | [Paired C/E4 paths](cataclysm-434.md#a6-low-bone-inheritance-bits-do-not-affect-native-pose-composition); no native fixture |
| Authored model cameras | None promoted to finding | C samples/outputs camera tracks; E4 snapshots drive creation/portrait framing | No established native time-varying camera requirement or affected asset | [Paired capability difference, applicability incomplete](cataclysm-434.md#matched-mechanisms-and-leads-not-promoted) |
| Particle geometry / birth / motion | P1–P5, P7 | Plane/sphere simulation, force integration, model/world axes, keyframed emitters, head quads, several appearance flags | Asset/layout provenance for spline/model/velocity scale; full flag census | Paired native consumer paths |
| Particle ordering | P6 | Object/pool-level transparent sorting is not claimed absent | Cross-emitter global sort semantics and actual overlapping assets | Paired generation/upload order |
| M2 mesh material order | R1 | Priority plane/material layer are retained | Projected batches, clip-plane passes, exhaustive texcoord generation | Authored metadata discarded; reference sort consumer traced |
| M2/particle shader lighting | S1 | Scene light, combiners and particle fog/blend exist; C/E4 centered UV rotation/scale/translation paths match | Alpha cutoff/quat/fog candidates, lit flag polarity and exhaustive variants | Paired shader/binder paths; [UV lead rejected](cataclysm-434.md#matched-mechanisms-and-leads-not-promoted); no pixels |
| WMO materials | S2 | 24-entry native shader mapping, texture combinations, interior ambient, point lights and shadow flags exist | Equation-by-equation parity, special lightmap branch and asset applicability | Paired SIDN parser/binder/shader trace |
| Terrain compositing/detail | S3: C terrain-surface MCSH consumer has no E4 binding | Detail-doodad mask sampling exists; terrain layers/colors/dynamic lighting exist | Modern surface modulation equation, split-file applicability and full material parity | [Paired C surface pass/native binding](cataclysm-434.md#s3-authored-mcsh-shadows-do-not-reach-terrain-surface-shading); no pixels |
| Weather / storm environment | ENV1 | E clear-time-of-day sky, stars, planets, skyboxes and fog exist | Weather authority/wire input, all precipitation families and asset appearance | Reference lifecycle vs native clear-only environment inputs |
| Camera immersion | ENV2: underwater scene-light/fog selection | E samples underwater light data and fogs refracted water-surface samples | V5 depth-darkening producer-sign/update-order conflict; glare depth fade, underwater sound and modern parameter correspondence | Native scene-light input/output trace, separate from surface shader |
| Walking contacts | M1–M2 | E slope/support sampling, wall clamping, ordinary jump/fall and transport carry exist | Matched geometry, body parameters, transport fall/seat edges, launch/knockback producer | Paired contact-response algorithms |
| Camera collision / target fade / WMO visibility | C1–C2: finite-width obstruction query and camera-target opacity binding | Camera ray/smoothing and WMO portal/collision paths exist | Modern camera dimensions/fade policy, matched portal outputs and vehicle specifics | [Paired C/E4 active paths](cataclysm-434.md#c1-camera-obstruction-probes-have-no-lateral-extent); no native pixels |
| WMO liquids | None established | C hidden-tile query and E4 hidden-tile geometry have matching low-nibble predicate | Query flags, gameplay height/type consumer and edge interpolation | [Bounded render/query mechanism comparison](cataclysm-434.md#matched-mechanisms-and-leads-not-promoted), not full liquid parity |
| World audio | AU1–AU4 | Native zone channels, footsteps, spell-kit/voice and UI audio paths exist | WMO-specific spatial emitter producer, audio occlusion, full kit/concurrency/doppler rules | Paired contextual selection/scheduling paths |
| Server authority | None established | S has independent ground/movement/navmesh implementations | Combat/persistence/authority cannot be recovered from W alone | Client-only reference limit |

## Domains not systematically audited

These are **audit gaps**, not a claim that every named feature is missing from E/S. They prevent an exhaustive conclusion.

| Reference area | Outstanding comparison |
|---|---|
| `Engine/Source/Frame*`, `WoW/Source/Ui`, `Glue`, `UIUtil` | Full authored layout/input/focus/scroll/tooltips, UI model scenes and glue transitions vs native UI; no blanket missing-UI claim |
| `WoW/Source/Net`, `Battlenet`, `WowServices`, `WowSvcs` | Session/reconnect/security/state ordering; historical wire formats are not a migration requirement for the project's protocol |
| `WoW/Source/Magic`, `Game`, `Object`, `Minigame` | Full spell visual/impact/aura timelines, object visibility/interaction, combat presentation and minigames; server gameplay requires independent contracts |
| Model/world special paths | IK and runtime inheritance-flag producers; animated authored camera applicability; projected textures and shadow-batch eligibility; collision-material details, streaming/occlusion/LOD boundaries and exhaustive shader variants |
| Remaining sound/environment/backend paths | Audio occlusion, doppler and concurrency; WMO spatial emitters; liquid glare, uncommon weather/material families; device/backend behavior and performance |

## Checked claims that must not become rework tasks

### Existing ribbon UVs use the newest slot in the reference

`W/Engine/Source/Services/RibbonEmitter.cpp:349–354` rewrites surviving edge UVs using the current slot every update. `E/godot/core/src/m2_ribbons.rs:231–242` likewise derives them from the trail's current slot. Retaining each edge's birth slot would introduce a difference.

### Ribbon interpolation uses current width for both endpoints

`W/Engine/Source/Services/RibbonEmitter.cpp:95–107` constructs previous/current endpoints with current `m_above`/`m_below`. `E/godot/core/src/m2_ribbons.rs:258–277` does the equivalent with current values and the direction-difference bend. Previous-frame width retention was not reference behavior in this path.

### Long-frame particle replay caps whole steps on both sides

`W/Engine/Source/Services/ParticleSystem2.cpp:1037–1063` computes remainder before capping whole steps to one lifespan. `E/godot/core/src/m2_particles.rs:528–540` does the same. Lifespan 1 s and delta 2.05 s replay approximately 1.05 s on both. Follow displacement distribution is a separate omission, P2.

### A 45-degree slope is not rejected by cos(50 degrees)

`W/WoW/Source/Object/Movement.cpp:2037–2039` uses normal Z `<= 0.64278764`, approximately cos(50 degrees). `S/crates/shared/src/movement.rs:3–4,77–84` uses a 50-degree maximum. A 45-degree plane has normal Z approximately 0.7071; these snippets do not prove a 45-versus-50-degree policy defect. M1/M2 concern response/clearance, not that false numerical lead.

### Root-owned top-level ribbon meshes are not proof of a leak

`E/godot/rust/src/ribbons.rs:206–222` adds meshes as children before making transforms top-level; `:27–32` hides empty geometry. This alone does not establish surviving nodes after owner removal. Natural effect-tail teardown still needs its own lifecycle fixture.

### MOUV and the original terrain-surface MCSH lead

E parses MOUV (`godot/core/src/asset/wmo_format/parser.rs:100–108`) and notes unported UV animation (`godot/shaders/wmo.gdshader:255–259`), but the pinned W producer/animation formula was not established. Record this as an uncompleted comparison, not a wowdecomp-proven defect. W's inspected MCSH use at `WoW/Source/WorldClient/MapChunk.cpp:1331–1339` concerns detail-doodad vertex alpha, which E handles at `godot/core/src/ground_detail.rs:166–169,207–211,293–299`; it does not prove a missing terrain-surface shadow. The separate [C/E4 comparison now establishes S3](cataclysm-434.md#s3-authored-mcsh-shadows-do-not-reach-terrain-surface-shading) through an actual C surface pass; this does not upgrade the old W citation. MOUV remains unproven against either reference.

### Audio and model events are partial, not absent

Native action events at `E/godot/rust/src/animation/action.rs:29–35,69–103,244–256` feed real melee/voice consumers. Native spell audio at `E/godot/rust/src/spell_effects/kits.rs:198–229` and `spell_sounds.rs:130–150,184–208,321–332` includes authored kit selection and weighted sound choice. A4 and AU1–AU4 must not be paraphrased as no model events or no real spell audio. Likewise, no WMO emitter semantics are inferred from an unverified chunk name such as MOSB.

## Server boundary and prerequisites

W reconstructs a client. Collision queries, weather consumers and packet handling reveal client expectations, not authoritative server decisions. Do not change S slope policy, world scheduling, protocols, combat or persistence from this audit alone. ENV1's producer contract, model-particle asset resolution, modern inherited-velocity scale and applicable historic/new material rules are named dependencies—not permission to invent fields, constants or fallback behavior.

## Proof ledger

| Check | Scope / revision | Evidence and limits |
|---|---|---|
| E/W/S baseline metadata | Pins in index; expansion began at E docs HEAD `201d6288a` | No tracked source changes; W/S clean; unrelated E untracked paths preserved |
| Initial independent artifact gate | Original nine findings through `c9eb3880a` | All nine supported; 32/32 links resolved; docs-only diff/whitespace passed; citation/fixture corrections required |
| Initial narrow follow-up | `201d6288a` corrections | PASS: affected citations, inherited-velocity overrides/variance and four-point Bézier fixture; whitespace check exit 0; unaffected proof retained |
| Expansion source inspection | P6–P7, A4–A5, R1, S2, ENV1–ENV2, M1–M2, AU1–AU4 | Fourteen additional conditional capabilities traced; independent gate required event-timing, sort-citation, weather-input, finite-repeat and step-acceptance corrections. Depth darkening moved to V5 reference uncertainty. Final follow-up recorded in task delivery, not presumed from explorer reports |
| Runtime/asset/original-binary checks | None | Not run; no native visual/acoustic/gameplay reproduction, affected-asset census, performance or exhaustive parity claim |
| Builds/tests/deployment | None | Not run; documentation-only work, no implementation or deployment acceptance claim |

Documentation verification validates source citations, links, coherent classification and Git scope. It cannot upgrade this evidence into runtime proof.
