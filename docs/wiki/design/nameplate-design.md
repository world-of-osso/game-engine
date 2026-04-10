# Nameplate Design

Nameplates use a target-first system with three display states driven by targeting, hostility, recent damage, and distance. The current target gets full detail; nearby combatants get compact bars; background actors are hidden or faded.

## Display States

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
- Scale reduces at range before alpha fade (avoids pop)

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

- [nameplate-research-2026-03-27.md](../../nameplate-research-2026-03-27.md) — design rules, open-source references, prototype scope

## See Also

- [[ui-addon-system]] — nameplates are rendered through the engine UI layer
- [[character-generation]] — nameplate anchors to the character entity above it
