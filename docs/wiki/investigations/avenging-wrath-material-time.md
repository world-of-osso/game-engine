# Avenging Wrath material time

Verified source: 2026-10-12. [Contract](../../specs/m2-material-animation.md).

## Root cause

`m2_material::animate` selected the first timeline of every track and wrapped local time by Stand duration. `WowMaterialAnimation` supplied application age, independent of the owning `WowAnimationPlayer`. Spell effects already switch the model from Stand to Hold and Decay. Bones and material colour/opacity therefore sampled different clips.

Authentic Retail model2368645 (`cfx_paladin_avengingwrath_statechestback`) has Stand0/666ms, Hold158/433ms and Decay159/167ms. Feather batches1–3 have shader0x4014, flags0x15, blend7, textures2373423/2373424. Their local alpha tracks start with keys0/400/666ms →0/0/32767; Hold has no keys (authored default1); Decay0/167ms →32767/0. Sampling application900ms wraps to Stand234ms and hides the sheets despite Hold playback. The initial perspective held counterexample matched8.1% of4,060 pixels; same-view current rendering reached86.1%, outside the oracle's97% gate. Final controlled orthographic comparison uses identical camera/pose/clock/assets: published library34.3% versus corrected sampler100% of6,654 pixels. Mip-zero magnification avoids the fixture's approximated authored mip levels; it is not interchangeable with the perspective measurement.

Reference: local WebWowViewerCpp `wowViewerLib/src/engine/managers/animationManager.cpp:1064–1116` passes `animationInfo` to `animateTrackWithBlend` for colour/alpha/weights, with default colour/alpha1. `m2Object.cpp:2097–2099` skips final transparency below0.0001 before drawing. The pixel oracle formerly omitted that pre-combiner culling rule; its false-positive BlendAdd output was corrected separately. Zero-opacity cases prove culling, not visible feather shading.

## Fix

Material sampling receives the model's sequence index/local time and the global clock separately. The native material controller reads its sibling animation player's existing playback snapshot after animation processing, as the point-light controller already does. Global tracks retain timeline0 and their global period; non-skeletal models use the Stand loop. Variation classification includes differences between clips, not just multiple keys in the first clip. `animate.h:308–310` uses timeline0 when the playing index exceeds a track's table; explicit empty Hold timelines still sample defaults. Authentic165644,356426 and125854 preserve their single-timeline tints across other clips.

Material crossfade remains a named limitation: `animate.h:361–392` blends outgoing/current tracks, whereas `AnimationCore::sampled_base_poses` preserves outgoing sequences or pose snapshots for bones and the material controller reads current playback only. No full transition/no-regression claim. The17 named wing/glow models have no material interpolation2/3 tracks; those curve modes and pre-existing unparsed sequence aliases remain unsupported, not silently claimed as covered.

## Source and reproduction boundaries

Private5931, disposable `fb_m2light_*` characters: Holy/Protection/Retribution31884, Holy216331, Protection389539, Retribution231895/454373 all start wing kit105333 with2368644/2368645 in both skins.454373 expires naturally. These are actual spell requests and replicated effects, not injected visuals. Sanctified Wrath/Radiant Glory passives themselves have no model kit; their downstream active spell IDs are the rendering witnesses, not proof of talent proc logic.

Published manifest version2026-10-11, Last-Modified2026-10-12T04:26:02Z:99 downloaded candidate model/skin/texture and M2-shader files match canonical bytes. Spell-visual cachev11, model index, particle shader and pinned Godot hashes also match. Published Linux extension SHA256213121df81346c40581e62252e0210da5ac8f2f3df2759154b42ab73a0f81b9b has ELF build IDf7de8ece80c88f0e9931431e7f431b3f79e3f07f. No engine Git SHA is established from that binary. Online published proof remains blocked by the message registry: historical server897c6524 with protocol26046db compiles but still rejects this library. Date-selected ef32b4c and the server's declared74e67cf do not compile that server; chronological selection does not establish the binary's wire identity. Published offline native material rendering works.

Only NPC Avenging Wrath267511 →visual74695 →kit107828 uses model2481207, which authentically binds1706679 `spellvisualplaceholder.blp`. It is not the ordinary player wing kit. Do not remove it or replace its texture to hide the reported defect. Missing file-backed textures return no texture and record the FDID; no rainbow/debug fallback was found in the native loader.

Positioner59 remains separate: wing attach378535 has AttachmentID-1/Positioner59. Local CASC59 →state79 →position entry96 references value57; HumanMale HD's skeleton authors attachment57. Entry enum semantics are unsourced, so translating that into a full bone transform would be invented behaviour.

## Native finish proof — 2026-10-12

| Boundary | Proof | Limit |
| --- | --- | --- |
| Modern31884, kit105333 | Actual private casts,2368644/2368645, Modern+Forever; matched frontal camera0/-0.25/4 on master90d02da99 and corrected476c76c07 | No rainbow reproduced on NVIDIA/Linux; not the user's platform/glyph identity |
| Classic renderer witness442416 | Actual private casts, visual141293/kit197638/model165644, both skins and same camera | Glyph visual24929 uses kit43390, not kit24929; witness shares165644 but does not prove glyph57979 availability or its mirrored attachment layout |
| Classic glow166338 | Four additive mesh batches, no point-light node; source cast197636 and isolated published/current rendering | Native global times0/333/700/900 produce identical RGB; initial isolated run accidentally retained the autoload clock and is superseded by `isolated-clock-*` |
| Hold158 feather batch1 | Published34.3% vs corrected100% of6,654 oracle pixels, identical frozen camera/pose/clock/assets | Magnified batch proof, not full spell/particle pixel parity |
| Regression gate |10 authentic material tests, scoped native Cargo check and changed-file rustfmt check pass | Material crossfade, curve2/3, aliases and Positioner59 remain named gaps |

166338's authored global0 keys at0/333/833ms are `(1,132/255,0)`, `(1,216/255,0)`, `(48/255,-17/255,228/255)`; the833ms global period wraps before the last key. Negative green is authored, not corrupt state: the sampler preserves it and `m2.gdshader::blend_output` already clamps at the output boundary. No additional clamping or texture substitution was introduced. `aa44d24b2` checks global-clock independence across local clips and preserves negative green near the final key.

Eight inspected1280x720 frontal PNGs and the192px held-feather comparison are in `/syncthing/AgentShared/2026-10-12/m2-light-effects/`; `same-camera-frontal-gates.json` records matching camera/facing/character, active source models, sizes and hashes. Side-on `final-*` captures are retained diagnostics, superseded for visual acceptance by `frontal-*`. Isolated published model loading does not prove the full live particle/effect lifecycle. The rainbow's responsible model/batch remains unidentified; the pink/blue source placeholder is not evidence that it caused the user's screenshot.

## Evidence

- Slot-local `data/diagnostics/m2-light-effects-2026-10-12/`: published comparisons, source DB2 records, native material pixels and RED Hold rendering.
- `/home/osso/.worktrees/logs/m2-light-effects/`: source census, raw alpha tracks, private account/kit snapshots and both-skin captures.
- `/home/osso/.worktrees/handoff-m2-light-effects.md`: revisions and proof ledger.

The local-time defect is established independently. The user's original rainbow quad is not yet reproduced or explained; no claim that this fix resolves it.

## Sources

- `godot/core/src/m2_material.rs`, `godot/rust/src/assets/uv_animation.rs` — sampler and native playback binding.
- `godot/rust/src/spell_effects/models.rs` — authored Start/Hold/Decay model lifecycle.
- `godot/core/tests/m2_material.rs`, `godot/tests/m2_real_pixels.gd`, `m2_wwv_oracle.gd` — independent source expectations and native pixels.
- Local-CASC2368645.m2/2373426.skin and exported SpellVisualEvent/KitEffect/KitModelAttach/EffectName — exact model/kit/texture chain.
- Local WebWowViewerCpp `animationManager.cpp`, `m2Object.cpp` — track defaults and pre-combiner visibility rule.

## See Also

- [[m2-format]] — authored batch flags and tracks.
- [[rendering-pipeline]] — native M2 materials and pixel oracle.
- [[animation]] — skeletal model playback.
