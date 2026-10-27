# Spell visual census gaps

## Type 1: procedural effect semantics unresolved

The 70448 census identifies type 1 in 220 distinct supplied spell IDs (476 spec memberships). WoWDBDefs `SpellVisualKitEffect.dbd` identifies `Effect` as a `SpellProceduralEffect` ID; its `SpellProceduralEffect.dbd` identifies `Type` and four `Value` fields. [wowdev's legacy SpellVisualKit procedures](https://wowdev.wiki/DB/SpellVisualKit) names procedure 27 Weapon Trail and procedure 14 SetAlphaMod; its modern [SpellProceduralEffect page](https://wowdev.wiki/DB/SpellProceduralEffect) contains no subtype or parameter documentation. The local reference clients searched under `~/Repos` have no `SpellProceduralEffect` implementation. [Mount Special](../specs/mount-special.md) also records this boundary.

The census subtype distribution by distinct spell ID is 27:76, 14:69, 30:61, 1:15, 8:9, 7:6, 9:6, 13:6, 22:5, 26:5, 17:3, 11:2, 18:1, 23:1; spell counts overlap. Subtype 27 appears in 99 distinct spell/kit/effect cases, 98 of which have `Value_0` matching a `WeaponTrail.ID`. Rend 772 / kit 279229 references procedural 376875: 27, 42, 0, -2.14748365e+09, 0; trail 42 names texture 1147502. Cleave 845 / kit 58292 references procedural 8099: 27, 76, 20, 600, -1; trail 76 names texture 1237598. Only 33 of the 99 cases have a `WeaponTrailParam` row for that matching trail ID. This is strong evidence for the weapon-trail identity, not proof of the modern use of `Value_1..3`, default timing, hand selection, or missing parameter rows. The legacy page specifies the parameters for procedure 8 in build 0.5.3, not procedure 27 in modern builds.

Sources: [SpellVisualKitEffect definition](https://github.com/wowdev/WoWDBDefs/blob/master/definitions/SpellVisualKitEffect.dbd), [SpellProceduralEffect definition](https://github.com/wowdev/WoWDBDefs/blob/master/definitions/SpellProceduralEffect.dbd). The input CSV is `data/db2/active/SpellProceduralEffect.csv`; the full census is in `data/diagnostics/spell-visual-census-2026-10-27/ledger.json`.

**Named gap: modern procedural weapon-trail rendering semantics.** Subtype 27 is strongly identified, but its modern parameter/default behavior is not established by the available sources. No implementation, failing behavior assertion, or after-capture is claimed. Other subtypes remain unimplemented.

## Refraction: composition semantics unresolved

The census reports refraction emitters for 156 distinct supplied spell IDs (398 spec memberships). For Battle Shout 6673, model 6194303 contains refraction emitter 1. `godot/core/src/m2_particles.rs` selects refraction for non-multitexture flag `0x100000`; `godot/rust/src/particles.rs` excludes it from the forward pool. WebWowViewerCpp `wowViewerLib/shaders/slang/forwardRendering/m2ParticleShader.frag.slang` discards refraction and contains an unreachable partial height/alpha calculation. Its code does not specify how the refraction pass turns that result into screen displacement, including blend and ordering. The current [particle spec](../specs/m2-particles.md) expressly records this missing pass.

Wowhead's `modelviewer/live/viewer/viewer.min.js` embeds a particle fragment shader whose `uPixelShader == 4` branch also discards; its particle program has no scene sampler. Installed Retail 12.1.5.70575 CASC contains `refraction.bls` FDID 2990911 and `refractionapply.bls` FDID 2990912. They inflate to DXIL bytecode; the installed LLVM 23 bitcode parser rejects their data layout. No scale or pass order was recovered. Extracted inputs and the viewer script are under `data/diagnostics/spell-visual-census-2026-10-27/refraction-probe/`.

**Named gap: refraction buffer composition and displacement.** Source evidence identifies the emitter and partial output, but not the final rendered behavior. No substitute distortion equation, behavioral test, or after-capture is claimed.

## Type 12: edge-glow shader and lifecycle unresolved

The 70448 census identifies type 12 in 100 distinct spell IDs (248 spec memberships). The 118 distinct spell/kit/effect cases all reference an existing `EdgeGlowEffect` row. For Avatar 107574 / kit 190072, effect 5013 has duration 0.4, fade-in 0.1, fade-out 0.3, Fresnel coefficient 0.7, RGB (0.152941182, 0.450980425, 0.603921592), multiplier 2, flags 8 and curve 106. WoWDBDefs `SpellVisualKitEffect.dbd` calls this `EdgeFlowEffectID`; the actual exported table and WoWDBDefs file are named `EdgeGlowEffect`.

Wowhead's viewer accepts type 12 entries but its effect dispatch skips them. It has a Fresnel calculation for *dissolve* edge glow, not a documented mapping for standalone `EdgeGlowEffect`. The local reference clients contain no `EdgeGlowEffect` implementation. The curve, flags, source blend and lifetime/overlap behavior cannot be inferred from field names alone.

**Named gap: standalone edge-glow shader and lifetime rules.** No generic implementation, real-case behavioral assertion or after-capture is claimed.

## Type 3: camera effect semantics unresolved

WoWDBDefs identifies type 3 as `CameraEffectID`; the 70448 census reports 44 distinct spell IDs (80 spec memberships). Slam 1464 / kit 278885 references CameraEffect 699, flags 4. Its CameraEffectEntry rows 694 and 700 each have duration 0.4 seconds, but use different `EffectType`, `DirectionType`, `MovementType`, `AttenuationType`, phases, amplitudes and curves. The DBD and available local clients do not identify these enum values, the camera axes, or attenuation and curve application. A field name or a generic camera shake is not enough to establish the authored motion.

**Named gap: CameraEffectEntry enum and evaluation rules.** No invented shake, behavioral test or after-capture is claimed.

## Type 10: unit voice IDs unresolved

The census reports 37 distinct spells (78 memberships) with unsupported type-10 values. Existing `godot/core/src/spell_visual_voice.rs` handles inferred values 34–40; WoWDBDefs calls type 10 a possible unit sound ID, without mapping its values. The affected spell/kit pairs use value 1 in 16 pairs, 2 in 15, 24 in 7 and 25 in 2. Examples: Disengage 781 / kit 270943 uses 1; Psychic Scream 8122 / kit 43749 uses 2; Incapacitating Roar 99 / kit 42876 uses 24. The available references do not define those values as specific CreatureSoundData columns. Names and spell themes alone do not prove the mapping.

**Named gap: type-10 values 1, 2, 24 and 25.** No invented voice mapping, behavioral test or after-capture is claimed.

