# Nameplate Design

This page records the intended target-first nameplate design. Current implementation is earlier: projected names and health bars obey existing visibility/distance settings; independent health/spellbar thickness choices and replicated cast presentation are being integrated. The target, hostility, combat, and occlusion state machine below is not implemented.

## Current implementation boundary

- `HudOptions` persists independent `Thin`/`Thick` choices for health and spell bars. The explicit user defaults are Thick health and Thin spellbar.
- `shared::casting::CastState` is replicated from the server and mirrored from the client worker to the render world, including additions, elapsed progress changes, and removal.
- Server cast presentation accepts player cast intents, validates available spell data, exposes timed cast state, and removes it on stop, movement cancellation, or expiry. It does not apply spell effects or supply NPC casts.
- Reference-matched health/spell bar rendering and focused current-revision engine proof remain in progress. Do not infer the planned display states below from these data-path changes.

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
- [`src/network_runtime/replication.rs`](../../../src/network_runtime/replication.rs) — worker-to-render-world cast snapshots
- [`../../../game-server/crates/server/src/cast_presentation.rs`](../../../../game-server/crates/server/src/cast_presentation.rs) — authoritative player cast presentation lifecycle

## See Also

- [[ui-addon-system]] — nameplates are rendered through the engine UI layer
- [[character-generation]] — nameplate anchors to the character entity above it
- [[networking]] — replicated entity boundary for cast state
