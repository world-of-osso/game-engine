# Nameplate Design

This page records the intended target-first nameplate design. Current work pixel-matches the supplied reference while preserving existing visibility/distance settings, independent health/spellbar thickness choices, and replicated cast presentation. The target, hostility, combat, and occlusion state machine below is not implemented.

## Current implementation boundary

- `HudOptions` persists independent `Thin`/`Thick` choices. The explicit defaults are Thick health and Thin spellbar.
- `NAMEPLATE_SCALE = 0.5`: effective health width is 188px from a 376px raw interior; health heights are 20px Thick / 10px Thin and cast heights are 10px Thick / 6px Thin. Frame bitmap outer bounds vary. It is not pixel verified.
- Health uses UI-overlay sprites, not world-space PBR. The shared art cache loads frame PNGs derived from the supplied reference, with transparent interiors for live bar content; it loads a glyph-free health gradient crop and authored `4505182` cast fill/background. No generic pip is rendered because none appears in the reference.
- Non-glyph pixels must pixel-match `data/diagnostics/nameplate-style/reference.png`: frames, fill, geometry, colors, endpoints, gaps, and placement. Glyph rasterization may differ only; names use white Friz at 13px and cast labels use white Friz at 10px.
- `shared::casting::CastState` is replicated from the server and mirrored from the client worker to the render world, including additions, elapsed progress changes, and removal. Normal casts fill; channels drain.
- Server cast presentation accepts player cast intents, validates available spell data, exposes timed cast state, and removes it on stop, movement cancellation, or expiry. It does not apply spell effects or supply NPC casts.
- Pixel-match verification is pending: the GPU fixture must produce aligned half-size captures for all four thickness combinations and mask only glyph regions. Frame alpha reconstruction from the composited reference is ambiguous despite reproducible linear unmatting, so no perfect-match claim is valid before GPU comparison. Connected server-to-client replication remains unproven. Do not infer the planned display states below from this implementation.

## Intended display states

| State | When | Content |
|-------|------|---------|
| **Full** | Current target | Name, health bar, cast bar, status markers |
| **Compact** | Non-target hostile in combat | Short name + thin health bar |
| **Hidden** | Friendly/neutral ambient, out of range, occluded | Nothing, or brief fade-in on hover/damage |

Full-health bars on non-target units hide after a short timeout.

## Information Hierarchy

- **Color communicates relationship**: distinct families for hostile, neutral, friendly — not decoration
- **Low health**: strong fill-color shift, not just brightness
- **Casting**: secondary bar or progress strip on target only
- **Elite/boss/quest markers**: restrained — applied only after base system proves readable

## Distance and Occlusion

- Plates fade with distance using alpha reduction before hard removal
- Plates hide when occluded by world geometry
- Name and health-bar dimensions remain fixed in logical screen pixels across zoom; distance fade/hide manages range clutter

## Clutter Rules

- No permanent icon rows except on current target or special units
- Collapse long names where possible
- Full framing (elite borders, ornament) only for special units — keeps crowded scenes readable

## Prototype Scope

1. Define three display states (hidden / compact / full)
2. Drive state from targeting, hostility, recent damage, distance
3. Implement one compact bar style and one full target style
4. Add timed hiding for full-health non-target bars
5. Test with sparse and crowded scenes before adding decorative framing

Cast bars and elite/quest markers come after the base system validates.

## Reference Projects

- **Veloren**: unified overhead widget with multiple display modes; vertical stacking above actor
- **Flare Engine**: bars disappear when at full value after timeout — explicit anti-clutter rule
- **Ryzom Core**: MMO-space separation between "important target" UI and "background population" UI

## Sources

- [nameplate-research-2026-03-27.md](../../nameplate-research-2026-03-27.md) — intended design rules, references, prototype scope
- [`src/game/state/client_options.rs`](../../../src/game/state/client_options.rs) — persisted health/spellbar thickness values
- [`src/rendering/ui/nameplate_art.rs`](../../../src/rendering/ui/nameplate_art.rs) — shared reference-derived frame and live-content art cache
- [`debug/make_nameplate_skins.py`](../../../debug/make_nameplate_skins.py) — reproducible frame extraction, linear unmatting, and provenance
- [`debug/compare_nameplates.py`](../../../debug/compare_nameplates.py) — half-size reference/GPU diagnostic comparison
- [`src/rendering/ui/health_bar.rs`](../../../src/rendering/ui/health_bar.rs) — UI-overlay health sprites
- [`src/rendering/ui/nameplate_cast_bar.rs`](../../../src/rendering/ui/nameplate_cast_bar.rs) — authored cast presentation and visibility gates
- [`src/network_runtime/replication.rs`](../../../src/network_runtime/replication.rs) — worker-to-render-world cast snapshots
- [`../../../game-server/crates/server/src/cast_presentation.rs`](../../../../game-server/crates/server/src/cast_presentation.rs) — authoritative player cast presentation lifecycle

## See Also

- [[ui-addon-system]] — nameplates are rendered through the engine UI layer
- [[character-generation]] — nameplate anchors to the character entity above it
- [[networking]] — replicated entity boundary for cast state
