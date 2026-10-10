# Nameplate Design

This page records the intended target-first nameplate design and the current source-level behavior. The approved half-scale reference calibration, independent thickness choices, visibility/distance policy, owner selection, and replicated cast presentation are separate from the unimplemented target, hostility, combat, and occlusion state machine.

## Current implementation boundary

- `HudOptions.nameplate_style` (`NameplateStyle`, persisted as `nameplateStyle`) holds every plate size and colour: health and cast width/height, fill colour per reaction and per cast type (normal, channel, non-interruptible), class colours for players, border visibility, name and cast font sizes. The HUD Thin/Thick selectors apply height presets (health 20/10px, cast 10/6px); the nearest preset picks the frame skin, whose margin follows the edited size. Options > Nameplates edits the rest. Defaults: Thick health, Thin spellbar.
- Health fills are tinted by the owner's reaction to the local player, from `FactionTemplate` rows through `shared::faction_reaction` (the server's AzerothCore template rules; no reputation). Default colours are Retail `UnitSelectionColor` red/yellow/green, class colours for players (`NamePlateFriendlyFrameOptions.useClassColors = true`), and CastingBar (1, 0.7, 0) / (0, 1, 0) / (0.7, 0.7, 0.7) for casts. The fill art is desaturated at load (each pixel's HSV value) so the tint is the colour; a (1, 0, 0) tint reproduces the reference red channel exactly but drops its (43, 41) green/blue.
- Layout: the name's bottom centre sits 2px above the plate's top edge (the frame when shown, else the body), Retail `CenteredAboveHealthBar` (Blizzard_NamePlateUnitFrame.lua:732-736, `HEALTH_BAR_TO_NAME_ABOVE_SPACING = 2`). All plate offsets are UI units applied in overlay space after projecting the body centre. The earlier code added them in viewport pixels while sprites are sized in UI units, so under the scaled in-world UI camera (auto-fit 0.667 at 1280x685) the name sat ~25px left of a 125px-wide bar; that was the misalignment in `data/diagnostics/nameplate-20260925/user-name-overlaps-bar.png`.
- A `LocalPlayer` owner is excluded during projection, so its name, health, and cast parts remain absent even if local identity is assigned after those visual entities were created. Removing the marker permits the remote plate to project again.
- Name, health, and cast parts use the health-body anchor for the configured `HudOptions.nameplate_distance` fade/hide boundary. The setting, not a claimed retail-native cap, defines current behavior.
- Plate visuals carry their actor owner for screen-space picking. After reconnect/modal and registry-frame input checks, a visible non-local plate selects that owner before the normal world-mesh raycast.
- `--screen nameplatedebug` and `--screen nameplate-debug` enter an offline preview using plain owners and the shared renderers. It loops normal casts and channels; Space pauses/resumes progress, and a plate click selects its preview owner.
- `NAMEPLATE_SCALE = 0.5`: effective health width is 188px from a 376px raw interior; health heights are 20px Thick / 10px Thin and cast heights are 15px Thick / 6px Thin. Frame bitmap outer bounds vary. It is not pixel verified.
- Health uses UI-overlay sprites, not world-space PBR. The shared art cache loads frame PNGs derived from the supplied reference, with transparent interiors for live bar content; it loads a glyph-free health gradient crop and authored `4505182` cast fill/background. No generic pip is rendered because none appears in the reference.
- Non-glyph pixels must pixel-match `data/diagnostics/nameplate-style/reference.png`: frames, fill, geometry, colors, endpoints, gaps, and placement. Glyph rasterization may differ only; names use white Friz at 13px and cast labels use white Friz at 10px.
- `shared::casting::CastState` is replicated from the server and mirrored from the client worker to the render world, including additions, elapsed progress changes, and removal. Normal casts fill; channels drain.
- Server cast presentation accepts player cast intents, validates available spell data, exposes timed cast state, and removes it on stop, movement cancellation, or expiry. It does not apply spell effects or supply NPC casts.
- Pixel-match verification is pending: the GPU fixture must produce aligned half-size captures for all four thickness combinations and mask only glyph regions. Frame alpha reconstruction from the composited reference is ambiguous despite reproducible linear unmatting, so no perfect-match claim is valid before GPU comparison. Connected server-to-client replication remains unproven. The local-owner, distance, selection, and preview code is documented from source through `f3dab635`; no current-cycle test execution or rendered runtime verification is claimed. Do not infer the planned display states below from this implementation.

## Godot client (Retail visibility)

October 9: ordinary Forever plates omit levels and reclaim bar width (`37144af3e`); selected badge/skull behavior and rendered acceptance (`05e945881`) live in the [nameplate contract](../../specs/nameplate-style.md). Forever target rare/elite art is separate unit-frame behavior (`ea8a275bf`); the [HUD contract](../../specs/hud-edit-mode.md) owns its scoped proof and limits. Historical Bevy design above is not native acceptance.

The Godot client implements Retail's CVar-driven visibility, not the target-first state machine below. `godot/core/src/rendering/ui/nameplate_visibility_data.rs` holds the rules and CVar defaults (see the [spec](../../specs/nameplate-style.md) for citations); `godot/rust/src/nameplates.rs` feeds them from replicated snapshots and draws plates on a CanvasLayer (layer 0, below the registry UI; mouse ignored so world clicks still pick units).

- Defaults: plates only for the target and units in combat with the player, enemies (attackable, neutral included) only, within 60 yd of the player. Friendly plates are off, the target's included.
- "In combat with the player" = replicated `CombatStatus` plus `UnitTarget` == local player. There is no replicated threat list.
- Occlusion: a camera ray to the pick-box centre against terrain (layer 1) and WMO collision (layer 2) sets alpha 0.4. Doodads have no collision and never occlude.
- Anchor: body centre 2.5 yd above the unit origin in unit space (Bevy `BAR_Y_OFFSET`). The M2 header box was tried first; it reached 3.5 yd on a goblin and 4.4 yd on a lying soldier, so plates floated.
- The fill uses the Bevy skins, desaturated at load and tinted by `FactionTemplate` reaction. Class colours are not applied yet. The native HUD health-bars switch hides frame/fill while retaining a centered label; Accessibility colorblind mode changes player labels to cyan and NPC labels to yellow without changing fill tint. The HUD distance slider now applies alpha 1 through the camera-to-health-body boundary, then `nameplateMinAlpha` beyond it for unselected plates (`72689598a`), not a gradual fade to zero. Selected plates bypass that camera-distance step; CVar viewer-to-unit eligibility remains separate. See [current distance contract](../../specs/nameplate-style.md). Independent verification fresh-runs those authored controls on a replicated enemy NPC, including live health visibility, retained name, NPC label restoration, unchanged fill tint, and independent CVar eligibility (`/tmp/claude/verify-native-nameplate-options.md`). A pure test covers exact player cyan and NPC yellow labels, but live player-label rendering remains unproven: player-vs-player attacks are rejected, friendly-player plates default off, and no authored control enables them (`/tmp/claude/nameplate-player-*.log`).
- Automation: `nameplate_state()` lists shown plates (alpha, occluded, anchor, rects); `nameplate_rules(id)` returns the rule inputs for any unit.

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
- [`src/game/nameplate_style.rs`](../../../src/game/nameplate_style.rs) — `NameplateStyle`, presets, ranges and the Options slider model
- [`src/game/state/client_options.rs`](../../../src/game/state/client_options.rs) — persisted `nameplateStyle`
- [`src/ui/screens/options_menu_active_sections_nameplates.rs`](../../../src/ui/screens/options_menu_active_sections_nameplates.rs) — Options > Nameplates page
- `shared-protocol/src/faction_reaction.rs` — reaction rules shared with the server
- [`src/rendering/ui/nameplate_art.rs`](../../../src/rendering/ui/nameplate_art.rs) — shared reference-derived frame and live-content art cache
- [`debug/make_nameplate_skins.py`](../../../debug/make_nameplate_skins.py) — reproducible frame extraction, linear unmatting, and provenance
- [`debug/compare_nameplates.py`](../../../debug/compare_nameplates.py) — half-size reference/GPU diagnostic comparison
- [`src/rendering/ui/health_bar.rs`](../../../src/rendering/ui/health_bar.rs) — UI-overlay health sprites
- [`src/rendering/ui/nameplate_cast_bar.rs`](../../../src/rendering/ui/nameplate_cast_bar.rs) — authored cast presentation, local-owner exclusion, and shared-distance gates
- [`src/rendering/ui/nameplate_picking.rs`](../../../src/rendering/ui/nameplate_picking.rs) — screen-space owner hit testing for projected plate parts
- [`src/rendering/ui/target.rs`](../../../src/rendering/ui/target.rs) — registry-first click routing before mesh raycasting
- [`src/scenes/nameplate_debug.rs`](../../../src/scenes/nameplate_debug.rs) — offline looping normal/channel preview and Space pause
- [nameplate debug spec](../../specs/nameplate-debug.md) — offline preview contract and open verification
- [`src/network_runtime/replication.rs`](../../../src/network_runtime/replication.rs) — worker-to-render-world cast snapshots
- [`../../../game-server/crates/server/src/cast_presentation.rs`](../../../../game-server/crates/server/src/cast_presentation.rs) — authoritative player cast presentation lifecycle

- [`godot/core/src/rendering/ui/nameplate_visibility_data.rs`](../../../godot/core/src/rendering/ui/nameplate_visibility_data.rs) — Retail CVar visibility and occluded-alpha rules
- [`godot/rust/src/nameplates.rs`](../../../godot/rust/src/nameplates.rs) — Godot plates, occlusion ray and automation

## See Also

- [[ui-addon-system]] — nameplates are rendered through the engine UI layer
- [[character-generation]] — nameplate anchors to the character entity above it
- [[networking]] — replicated entity boundary for cast state
- [nameplate spec](../../specs/nameplate-style.md) — current nameplate contract and open verification
