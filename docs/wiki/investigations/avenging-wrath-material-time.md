# Avenging Wrath material time

Verified source: 2026-10-12. [Contract](../../specs/m2-material-animation.md).

## Root cause

`m2_material::animate` selected the first timeline of every track and wrapped local time by Stand duration. `WowMaterialAnimation` supplied application age, independent of the owning `WowAnimationPlayer`. Spell effects already switch the model from Stand to Hold and Decay. Bones and material colour/opacity therefore sampled different clips.

Authentic Retail model2368645 (`cfx_paladin_avengingwrath_statechestback`) has Stand0/666ms, Hold158/433ms and Decay159/167ms. Feather batches1–3 have shader0x4014, flags0x15, blend7, textures2373423/2373424. Their local alpha tracks start with keys0/400/666ms →0/0/32767; Hold has no keys (authored default1); Decay0/167ms →32767/0. Sampling application900ms wraps to Stand234ms and hides the sheets despite Hold playback. The held native pixel counterexample matched only8.1% of4,060 compared pixels before the sampler change.

Reference: local WebWowViewerCpp `wowViewerLib/src/engine/managers/animationManager.cpp:1064–1116` passes `animationInfo` to `animateTrackWithBlend` for colour/alpha/weights, with default colour/alpha1. `m2Object.cpp:2097–2099` skips final transparency below0.0001 before drawing. The pixel oracle formerly omitted that pre-combiner culling rule; its false-positive BlendAdd output was corrected separately. Zero-opacity cases prove culling, not visible feather shading.

## Fix

Material sampling receives the model's sequence index/local time and the global clock separately. The native material controller reads its sibling animation player's existing playback snapshot after animation processing, as the point-light controller already does. Global tracks retain timeline0 and their global period; non-skeletal models use the Stand loop. Variation classification includes differences between clips, not just multiple keys in the first clip.

## Source and reproduction boundaries

Private5931, disposable `fb_m2light_*` characters: Holy/Protection/Retribution31884, Holy216331, Protection389539, Retribution231895/454373 all start wing kit105333 with2368644/2368645 in both skins.454373 expires naturally. These are actual spell requests and replicated effects, not injected visuals. Sanctified Wrath/Radiant Glory passives themselves have no model kit; their downstream active spell IDs are the rendering witnesses, not proof of talent proc logic.

Published manifest version2026-10-11, Last-Modified2026-10-12T04:26:02Z:99 downloaded candidate model/skin/texture and M2-shader files match canonical bytes. Spell-visual cachev11, model index, particle shader and pinned Godot hashes also match. Published Linux extension SHA256213121df81346c40581e62252e0210da5ac8f2f3df2759154b42ab73a0f81b9b has ELF build IDf7de8ece80c88f0e9931431e7f431b3f79e3f07f. No engine Git SHA is established from that binary. Its first private login rejects the newer server's message/channel registry; historical matching-server proof remains pending.

Only NPC Avenging Wrath267511 →visual74695 →kit107828 uses model2481207, which authentically binds1706679 `spellvisualplaceholder.blp`. It is not the ordinary player wing kit. Do not remove it or replace its texture to hide the reported defect. Missing file-backed textures return no texture and record the FDID; no rainbow/debug fallback was found in the native loader.

Positioner59 remains separate: wing attach378535 has AttachmentID-1/Positioner59. Local CASC59 →state79 →position entry96 references value57; HumanMale HD's skeleton authors attachment57. Entry enum semantics are unsourced, so translating that into a full bone transform would be invented behaviour.

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
