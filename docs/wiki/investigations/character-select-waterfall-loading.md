# Character-Select Waterfall Loading

Verified: 2026-09-12. Waterfall visibility is confirmed by the user after the loading, UV/timing, particle and placement corrections. Overall scene darkness remains different from Retail; [[character-select-lighting-overwrite]] records the separate, read-only source-level directional-light overwrite finding without claiming a Retail setting or fix.

## Current-WDT preview policy (2026-10-05)

Native previews request only the authored primary tile; they do not resurrect cached tiles absent from the current WDT. Map 2703 resolves through current `Map.csv` to WDT FDID `5493025`: `(31,37)` has MAIN bit 1 and MAID root/obj0/tex0 `5493438/5493439/5493441`; `(31,36)` has MAIN bit 0 and all-zero MAID. Cached named aliases do not authorize the old neighbor. The obsolete scene-1 supplemental-coordinate helper is removed; `Background::load` requests the primary alone. Playable-world WDT validation and generic object selection remain unchanged; no compatibility fallback.

`current_wdt_background_loads_primary_with_all_waterfall_placements` exercises actual `Background::load`, its asynchronous terrain worker and current cached WDT/MAID files. It requires only `(31,37)` to parse, no background map/tile failures, and all 14 primary waterfall/ripple placements to remain admitted by the existing primary selection predicate. These placements lie about 264–350 units from the character; the old neighbor's 42 cached placements are over 500 units away and no longer requested.

Development RED on `380b8aab` plus test/manifest additions failed exactly with `Map 2703 tile (31, 36) is inactive in WDT MAIN` (1 failed test, 548 filtered out). Evidence: `target/skyborn-source-items/preview-wdt-red2-cargo.log`; the same failure is in `native95-client-items.log`. The first test attempt failed compilation on a private test-module import, not a behavioral RED. The manifest adds only existing current-WDT/primary-companion files and their MDID/MHID textures, preserving all prior entries. The fixture also logs an existing missing Forever `Light.csv`; this is not a lighting fix. GREEN and native image acceptance are separate gates: no native image PASS until main captures and inspects the scene.

## Historical split-shadow and renderer investigation

Earlier clients requested primary terrain `2703_31_37` and supplemental `2703_31_36`; all old supplemental root, `_tex0`, and `_obj0` files existed locally. That historical file presence does not override the current-WDT policy above.

The supplemental root contains 227 chunks with the shadow-present flag but no `HSCM` payloads. Their matching 227 shadow payloads reside in `_tex0`, as split-file data. Root-only validation previously returned `MCNK flagged with MCSH but missing HSCM sub-chunk`, aborting the whole tile. `159b2b6f` supplies companion shadows before validation, retaining all maps and strict standalone validation.

`scene_tree::spawn_warband_terrain_tile` previously swallowed that error with `.ok()`, so the supplemental loop skipped its 42 placements. `8e0495a2` reports the path and failure. This explained primary-only loading, but did not by itself restore the visible waterfall.

Native inspection after the parser fix exposed the second blocker: the primary object pass applied a 75-unit prop radius to its own waterfall backdrop. `17b77d68` reuses the existing waterfall/ripple predicate to admit the 14 authored primary-tile effects while keeping ordinary props radius-limited. The real-spawn regression changes from 62 props with no waterfall04 model to 76 placements, including that model. The neighboring tile's 42 placements were supplemental coverage at that revision, not the only waterfall source.

Modern waterfall batches also encode their modulation coordinate in the shader-selected second UV set rather than the legacy coordinate lookup. `458868e9`, with named selector bits from `81417de0`, decodes that second UV source while preserving the authored alpha combine.

`7b0a12eb` preserves per-track global-sequence periods for ordinary `M2EffectMaterial` instances and samples them as loops rather than clamping at their final keyframe. Waterfall model `4661358` has independent 1000 ms and 1333 ms global periods on different batches; its second texture-animation lookup of `-1` remains static. The timing oracle was corrected in `33db3799`; `29e15a64` is its genuine frozen-animation RED boundary, followed by GREEN in `7b0a12eb`. `animation-verification/report.md` records 16 focused CPU tests and `cargo check` passing for this scope. Color-space, opaque, unlit, and cull probes were diagnostic controls only; none is the production fix.

`ca94fe91` adds a matched no-fog GPU control: white `M2EffectMaterial` and `StandardMaterial` quads render identical center RGB `[152, 152, 152]`. This rules out a basic custom-PBR lighting mismatch in that controlled case, not an actual-waterfall or character-select exposure diagnosis.

The placed-waterfall census finds nine authored mist placements that the terrain filter already selects: six `1028937` (`6fx_waterfall_mist01.m2`) and three `2904370` (`8fx_ambient_waterfall_ripple01_misty.m2`), each declaring one particle emitter. The cascade-sheet models `4661357`, `4661358`, and `4661361` declare none. Before `5206d9b4`, preloaded terrain M2 attachment forwarded only mesh batches, so these selected mist emitters were discarded.

Current Retail build `12.1.0.69587` was verified from local CASC; the six relevant model/texture files match extraction byte-for-byte. Retail shader assets are available locally, including `waterfall.bls` (`2990962`) and particle shaders. However, the three cascade models contain no `WFV*` chunk. The specialized WebWowViewer waterfall material consumes `WFV3` data and five waterfall textures, so that shader is not applicable to these ordinary M2 batches. Applying it blindly would be another unsupported rendering assumption. `030090f3` restricted the resulting emitter restoration to the existing `is_waterfall_backdrop_doodad` selection, preserving unrelated terrain props' prior no-emitter behavior (since `50d40c1f` every ADT and WMO doodad spawns its emitters, see [[stockade-entrance]]); the existing graphics particle-effects disable gate remains authoritative. `62e36331` records the meaningful RED: the actual mist attachment expected one emitter but observed zero; its focused GREEN passes.

The restored waterfall path exposed two separate particle prerequisites. Actual generated-WGSL RED/GREEN in `29819999`/`b1663674` corrects random flipbook sprite assignment to Hanabi's integer sprite attribute. Both selected waterfall mist models decode raw gravity as zero; a prior `NaN` generated shader came from an unrelated newly activated prop, not their gravity. Local CASC extraction supplied missing particle texture `2904679`. The later multitexture loader uses the existing asset resolver for every required stage and reports resolution/decode failures before submitting an incomplete GPU material.

`particle-native-fixed/acceptance.md` records a bounded native run with all nine waterfall-backdrop emitters instantiated and substantial localized moving particle output. It found no particle shader-processing failure or Hanabi panic, and no cited orbit-input INFO message. This does **not** accept the overall restoration: the flowing sheet is not independently readable, particle output contaminates its prior silhouette mask, and mist shape, softness, size, density, and reference fidelity remain unproven.

## Raw particle data and multitexture boundary

`2f84b994` corrects scale-variation offsets that previously read alpha-array headers and removes the mistaken runtime-size flag condition from compressed-gravity decoding. `b0d96d84` retains all three packed texture indices and signed secondary UV velocities. Model `1028937` selects texture indices `[0, 1, 1]`, resolving to `1029067/1029068/1029068`; the missing `1029068` was extracted from local CASC. Thirty parser cases pass, including actual-asset regressions.

`d9429b3e` corrects renderer consumers that applied internal `CParticleEmitter` runtime flag values directly to the raw M2 file field. The native and library builders now agree on raw world-space, bone-scale, atlas, orientation and wind semantics. Six actual-data regressions failed before the correction and pass afterward.

`cd4f48f6` corrects the particle-tail/twinkle/burst field offsets: tail `0x15c`; twinkle speed, percent, minimum, and maximum `0x160/0x164/0x168/0x16c`; burst multiplier `0x170`; drag `0x174`. Actual mist model `1028937` now parses burst multiplier `1.0` and drag `5.0` separately. The two-test RED followed by 32 scoped parser cases GREEN is recorded in `fable-parser-fix/`; its synthetic burst fixture writes the literal at `0x170`.

`cda87616` maps raw `XY_QUAD` particles to a world-XZ-plane orientation modifier, retaining authored spin in both native and library effect-builder paths. This corrects geometry rather than assuming a refraction material mode. The GPU regression `ea6a6157` uses actual raw model `2904370` with a controlled white quad and ordinary billboard controls: RED observed `[900, 900, 900, 900]` for XY-top, XY-side, billboard-top, billboard-side; GREEN measured `[900, 0, 900, 900]`.

`581c56be` preserves authored waterfall-backdrop placement height. Terrain grounding had raised the three primary `2904370` ripple placements by 20–38 units to terrain height: UID `49340914` from `448.21677` to `486.1005`, UID `49341153` from `478.3065` to `501.813`, and UID `49341408` from `511.68472` to `532.09875`. The existing waterfall/ripple predicate now bypasses that clamp; 14 ordinary below-terrain props retain it. `authored-placement/{red,green}` records the actual-primary RED and 2/2 GREEN. The earlier M2 `GroundedModelRoot` hypothesis was disproved by `selected_waterfall_attachment_preserves_authored_rest_bounds`; no M2-origin change was made.

Fable CLI successfully inspects the Retail reference and native image pixels where Pi image display is unreliable. Its findings motivated these parser and geometry investigations, but each production conclusion above is tied to direct parser/GPU evidence. No refraction assumption is necessary for the demonstrated field-offset and orientation defects.

`9c81528e` binds the three required images, preserves separate per-particle secondary UV offsets/velocities and implements two-color/three-alpha and three-color/three-alpha composition. A genuine GPU RED failed with one declared texture binding versus three supplied images; GREEN renders the analytic three-color fixture exactly at `[73, 51, 92, 255]`. This constant-texture test does not prove scrolling-atlas fidelity. `461d54a7` preserves the pre-existing first-slot name and resolves the integration readability findings. The subsystem's 77 initially passing cases plus the repaired failing case, three attachment tests and focused check/format evidence are recorded below; one benchmark remains ignored.

**User confirms waterfall visibility (2026-09-12).** The user's direct observation supersedes Fable's earlier claim that the cascade was absent. The remaining reported difference is overall scene darkness versus Retail; brightness matching has not been implemented. The cyan artifacts are removed, and bounded captures contain no shader errors/panics or orbit INFO. Earlier failed image assessments remain historical evidence, not the current visibility verdict. The three-texture GPU test proves its particle composition path, not material equivalence for ordinary cascade sheets. Reference: `reference/wow-reference.png`.

`f53bba1c` keeps character-select orbit-input diagnostics at DEBUG, avoiding INFO-log noise without removing the diagnostic.

Do not discard shadow flags/data, broadly expand prop loading, replace alpha combination, or use a shared/global animation period as substitutes for these corrections. RED/GREEN evidence is under `verification`, `primary-verification`, and `uv-verification`.

## Evidence and sources

- `godot/rust/src/character_select/background.rs` — current primary-only request and actual loader-boundary regression.
- `godot/rust/src/terrain/assets.rs` — authoritative WDT MAIN/MAID validation, unchanged by the preview fix.
- `godot/depot-test-assets.txt` — explicit cached fixture files for host-isolated CPU tests.

- `data/diagnostics/waterfall-missing-20260911/particle-raw-parser/report.md` — actual field values and parser RED/GREEN.
- `data/diagnostics/waterfall-missing-20260911/particle-raw-flags/{red2,green}/` — six raw-flag behavioral regressions.
- `data/diagnostics/waterfall-missing-20260911/fable-parser-fix/report.md` — particle-tail/twinkle/burst offset RED/GREEN, including actual `1028937` burst/drag values.
- `data/diagnostics/waterfall-missing-20260911/xy-quad/{red,green2}/` — actual `2904370` XY-quad GPU RED/GREEN.
- `data/diagnostics/waterfall-missing-20260911/authored-placement/{red,green}/` — actual primary-ripple height-clamp RED/GREEN and ordinary-prop control.
- `data/diagnostics/waterfall-missing-20260911/particle-multitexture-gpu/{red,green}/` — three-texture GPU binding and pixel proof.
- `data/diagnostics/waterfall-missing-20260911/{multitexture-verification,slot-followup-461d54a7}/` — bounded integration and repaired metadata/readability gate.
- `data/diagnostics/waterfall-missing-20260911/multitexture-native-retry/acceptance.md` — failed native appearance acceptance; quantitative method and image-access limitation.

- `data/diagnostics/waterfall-missing-20260911/{verification,primary-verification,uv-verification}/` — RED/GREEN regression evidence.
- `data/diagnostics/waterfall-missing-20260911/split-shadow-layout.json` — root/companion chunk census.
- `src/asset/adt_format/adt/parsing.rs` — shadow validation.
- `src/asset/adt_format/adt.rs` — root parse error propagation.
- `src/rendering/terrain/terrain_objects.rs` — primary backdrop admission.
- `src/asset/m2_batch.rs` — shader-selected waterfall UV source.
- `data/diagnostics/waterfall-missing-20260911/animation-verification/report.md` — 16 focused CPU tests and check evidence for timing.
- `data/diagnostics/waterfall-missing-20260911/lit-comparison/output.txt` — matched lit-material GPU pixels.
- `data/diagnostics/waterfall-missing-20260911/particle-census/{report.md,census.json}` — placed-waterfall emitter census.
- `data/diagnostics/waterfall-missing-20260911/{particle-red,particle-green,particle-verification}/` — actual mist-attachment RED/GREEN and scoped verification.
- `data/diagnostics/waterfall-missing-20260911/particle-sprite-index/` — actual generated-WGSL sprite-index RED/GREEN evidence.
- `data/diagnostics/waterfall-missing-20260911/particle-native-fixed/acceptance.md` — bounded native emitter, particle-motion, error/panic, and orbit-log assessment; overall sheet/mist fidelity remains unaccepted.
- `src/rendering/model/m2_effect_material.rs` — per-track global-sequence texture-offset sampling.
- `src/rendering/model/m2_spawn.rs`, `src/rendering/terrain/terrain_objects.rs` — waterfall-backdrop-only terrain emitter forwarding.
- `src/rendering/model/m2_spawn_material.rs` — preserves global-sequence timing for ordinary effect materials.
- `src/scenes/char_select/scene_tree.rs` — terrain-load error reporting.
- `src/scenes/char_select/warband/mod.rs` — neighboring-tile selection.

Related: [terrain](../systems/terrain.md), [character-selection visibility](../../specs/character-selection-visibility.md).
