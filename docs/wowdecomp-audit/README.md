# wowdecomp behavior rework backlog

Source audit: **2026-10-15**. Eleven source-demonstrated semantic/capability gaps, plus separately gated shader candidates. **Not eleven reproduced visual bugs:** no client, server, shader, build, or reference executable was run. Asset prevalence and visible impact remain unmeasured.

## Read first

`wowdecomp/README.md:3–14` identifies the reference as reconstructed **WoW 3.3.5a, build 12340**, not current Retail. Its original-address annotations aid inspection; they do not independently certify reconstruction fidelity. Do not replace newer rendering behavior merely because it differs from Wrath.

| Source alias | Checkout | Inspected HEAD |
|---|---|---|
| W | `~/Repos/wowdecomp` | `0e4e9fe0a4a9fa08027c5eed4b2c5e873739cfd3` |
| E | game-engine, this repository | `3bdfb57b2e99205614e81bb7736ee3130677f24d` |
| S | sibling `../game-server` | `0dcaa23bde680678d72b3c43a786eb574bd809b8` |

All `W`, `E`, and `S` path/line citations refer to these revisions, not subsequent HEADs. Reference and server were clean; engine had no tracked modifications and had unrelated untracked files, left untouched. Source was treated as data only. No implementation changes or deployment form part of this audit.

## Rework order

Order reflects directness of the demonstrated gap, not measured visual severity. Each detailed entry gives reference behavior, current behavior, the required change and a discriminating acceptance check.

| Order | ID | Behavior not reproduced | Owner | Evidence |
|---|---|---|---|---|
| 1 | A1 | Step bone/vector/scalar tracks are smoothed | Client/core | [Track interpolation](m2-animation.md#a1-step-tracks-are-interpolated) |
| 2 | A2 | Animated ribbon alpha holds keys instead of interpolating | Client/core | [Ribbon opacity](m2-animation.md#a2-ribbon-alpha-is-always-stepped) |
| 3 | A3 | Short track timeline tables lose authored timeline-zero values | Client/core | [Timeline selection](m2-animation.md#a3-short-track-tables-do-not-select-timeline-zero) |
| 4 | P1 | Tail-only particles disappear; head-and-tail particles lose tails | Client/rendering | [Tail geometry](particles-and-shaders.md#p1-particle-tails-are-not-drawn) |
| 5 | S1 | Lit particle flag has no lighting path | Client/shaders | [Particle lighting](particles-and-shaders.md#s1-lit-particles-are-rendered-unshaded) |
| 6 | P2 | Follow-position emitters do not carry surviving particles | Client/core | [Emitter following](particles-and-shaders.md#p2-follow-position-is-ignored) |
| 7 | P3 | Burst/inherited emitter velocity is not added at birth | Client/core | [Inherited velocity](particles-and-shaders.md#p3-emitter-velocity-is-not-inherited) |
| 8 | P4 | Spline emitters are filtered out | Client/assets/rendering | [Spline emission](particles-and-shaders.md#p4-spline-emitters-are-filtered-out) |
| 9 | P5 | Model-particle and recursive-emitter model references are unused | Client/assets/rendering | [Model particles](particles-and-shaders.md#p5-model-particles-and-recursive-emitters-have-no-native-path) |
| 10 | P6 | Authored per-particle depth sorting is ignored | Client/rendering | [View-depth ordering](particle-order-and-ground.md#p6-authored-per-particle-depth-sorting-is-ignored) |
| 11 | P7 | Ground-projected particle births do not query world geometry | Client/core/world | [Particle projection](particle-order-and-ground.md#p7-ground-projected-particle-births-do-not-query-world-geometry) |

**Evidence level for every row:** paired source paths and a conditional input/output counterexample. No runtime reproduction, affected-asset census, original-binary comparison or native pixel proof. Missing capabilities are distinguished from wrong output in an otherwise supported path. Existing known particle gaps are corroborated, not presented as newly discovered features.

This count covers established findings only, not all project gaps. An unreviewed subsystem is **unknown**, not matching. The initial pass concentrated on M2/particles; broader world, environment, motion and audio comparisons are tracked in the coverage ledger.

## Shader candidates and coverage limits

[Validation candidates](validation-candidates.md) records alpha-key thresholds, quaternion interpolation, fog and the missing modern refraction pass without misclassifying them as proven target-version defects. [Coverage and rejected claims](coverage.md) records what was inspected, what remains unknown, and why no game-server rework is prescribed from this client-only reference.

Before implementation, identify a relevant authored asset and its target product/build, preserve its hashes and reproduce the counterexample through the actual native consumer. Reconcile the governing feature spec in the same change. This backlog describes desired observable behavior; it does not authorize implementation or prescribe an architecture.

## Related knowledge

- [M2 format](../wiki/formats/m2-format.md) and [animation](../wiki/systems/animation.md).
- [Particle contract](../specs/m2-particles.md) and [ribbon contract](../specs/m2-ribbons.md).
- [Retail lighting](../wiki/systems/retail-lighting.md) and [reference clients](../wiki/reference/open-source-wow-clients.md).
