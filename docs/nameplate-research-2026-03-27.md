# Nameplate Research

Date: 2026-03-27

## Goal

Improve in-world nameplates so they read cleanly during play, preserve the WoW-inspired feel of the project, and avoid turning crowded scenes into UI noise.

## Recommended Direction

The strongest default direction is a target-first nameplate system:

- Current target gets the full plate: name, health, cast/progress, and high-value state.
- Nearby combatants get reduced plates: short name plus a thin health bar.
- Non-hostile or background actors show little or nothing until hovered, targeted, or recently damaged.
- Plates fade with distance and hide when occluded by world geometry.
- Colors should communicate relationship and urgency, not decoration.

This keeps the player's attention on the active interaction while preserving situational awareness in the periphery.

## Practical Design Rules

### 1. Prioritize information hierarchy

Not every unit should carry the same amount of UI.

- Targeted enemy: full detail.
- Non-target hostile in combat: compact detail.
- Friendly player or NPC: name only, or name plus small bar when relevant.
- Ambient NPCs / distant actors: hidden or strongly faded.

### 2. Reduce clutter aggressively

Crowded scenes fail when every actor has a full-width health bar.

- Hide full-health bars after a short timeout.
- Collapse long names to a shorter label where possible.
- Avoid permanent status icon rows except for the current target or special units.
- Use alpha fade and scale reduction before hard popping plates off.

### 3. Make states legible at a glance

Players should identify threat and status without reading text.

- Hostile, neutral, and friendly relationships need distinct color families.
- Low health needs a strong change in fill color or brightness.
- Casting should introduce a clear secondary bar or progress strip.
- Elite / boss / quest-important units should use restrained markers, not ornate framing on every unit.

### 4. Fit the project's visual language

The project is rebuilding the WoW client, so the UI should feel authored rather than generic.

- Favor restrained fantasy framing for special units only.
- Keep ordinary bars simple and readable.
- Use texture framing sparingly so plates still work in crowded combat.
- Prefer a strong silhouette and spacing system over adding more ornament.

## Open-Source Inspirations

### Veloren

Links:

- https://docs.veloren.net/veloren_voxygen/hud/overhead/index.html
- https://gitlab.com/veloren/veloren

Why it is useful:

- Veloren has a coherent overhead UI system rather than ad hoc widgets.
- It combines multiple kinds of overhead information in one place: names, bars, and speech/state overlays.
- It is a good reference if `game-engine` wants one reusable overhead component with multiple display modes.

Good ideas to borrow:

- Unified overhead widget architecture.
- Different display states depending on context.
- Consistent vertical stacking above the actor.

### Flare Engine

Links:

- https://github.com/flareteam/flare-engine
- https://github.com/flareteam/flare-engine/wiki/Attribute-Reference

Why it is useful:

- Flare is a strong example of UI restraint in an action-RPG context.
- Its UI docs explicitly support hiding bars when values are full after a timeout, which is exactly the kind of anti-clutter rule nameplates benefit from.

Good ideas to borrow:

- Bars that disappear when not conveying new information.
- Simple, readable health presentation.
- Prioritizing gameplay readability over constant UI persistence.

### Luanti / Minetest ecosystem

Links:

- https://github.com/luanti-org/luanti
- https://content.luanti.org/packages/Wuzzy/hudbars/

Why it is useful:

- Luanti and its mod ecosystem are useful references for straightforward HUD language.
- The presentation is often plain, but legible, which is useful for testing which information density actually works before styling it heavily.

Good ideas to borrow:

- Numeric readability where bars alone are ambiguous.
- Minimal shapes and strong contrast.
- Testing utility-first presentations before final art direction.

### Ryzom Core

Link:

- https://github.com/kaetemi/ryzomclassic

Why it is useful:

- Ryzom is relevant because it comes from the MMO space and deals with persistent in-world unit readability rather than purely moment-to-moment action combat.
- It is best treated as a reference point for density and MMO legibility, not as an art target.

Good ideas to borrow:

- Conservative information presentation for MMO scenarios.
- Strong separation between "important target" UI and "background population" UI.

## Design Directions Worth Prototyping

### Option A: Minimal modern

- Thin bars.
- Small text.
- Strong distance fade.
- Full detail only for the current target.

Best if:

- The goal is maximum readability with minimal screen noise.

Risk:

- Can feel too generic unless the typography and color system are deliberate.

### Option B: MMO readable

- Clear relation colors.
- Full target frame style overhead plate for selected enemy.
- Cast bars and elite markers.
- Compact combat plates for nearby units.

Best if:

- The goal is classic MMO combat readability with familiar affordances.

Risk:

- Can get noisy quickly if compact-state rules are not strict.

### Option C: Diegetic fantasy

- More stylized framing.
- Decorative markers or rune-like accents.
- Less text, more symbolic signaling.

Best if:

- The goal is strong identity and atmosphere.

Risk:

- Readability can regress if styling outruns information hierarchy.

## Recommendation For `game-engine`

Start with Option B implemented conservatively:

- Full plate only for current target.
- Compact health bars for nearby hostile units in combat.
- Friendly and neutral units default to name-only or hidden.
- Add cast bars only for the current target first.
- Add elite / quest markers only after the base system proves readable.

That direction is closest to the project's WoW-like goals while still improving clutter and clarity.

## Suggested First Prototype Scope

1. Define three display states: hidden, compact, and full.
2. Drive state from targeting, hostility, recent damage, and distance.
3. Implement one compact bar style and one full target style.
4. Add timed hiding for full-health non-target bars.
5. Test with sparse scenes and crowded scenes before adding decorative framing.

## Notes

- This research was produced before inspecting the actual in-world nameplate rendering code.
- Follow-up work should inspect the current UI and rendering path in `src/` before implementation decisions are locked.
