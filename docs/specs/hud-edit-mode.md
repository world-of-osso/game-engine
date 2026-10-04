# HUD edit mode

> Root `src/` paths below name files deleted with the [retired Bevy client](godot-conversion.md#retired-bevy-client-user-decision-2026-10-02).

Implements the accepted decision "HUD edit mode can move every HUD element, as in WoW
Edit Mode" and open decision 1 (layouts account-wide, active layout per character) of the
[in-game UI plan](../plans/2026-09-23-ingame-ui.md). Window movement is separate:
[window manager](window-manager.md).

## What it must do

- [x] F10 toggles edit mode (World input mode only). Escape exits it, as a step after "close the top popup" and before "close all windows".
- [x] Registered elements (by root frame name; absent or hidden frames are skipped): player frame, target frame, target of target, focus frame, cast bar (`PlayerCastingBarFrame`), action bars 1–5, status bar 1 (`MainStatusTrackingBarContainer`, [XP bar](xp-bar.md)), minimap, objective tracker, buffs, debuffs, party frames, raid frames, error text (`UIErrorsFrame`), micro menu, bags bar. Each has a default anchor (screen edge, corner or center).
- [x] In edit mode each mounted element shows a selection box with its name; the selected one uses the selected art.
- [x] Dragging an element snaps its top-left to an 8-unit grid, then to a screen edge within 8 units, and clamps it to the screen.
- [x] A placement is stored as (anchor, offset from the screen anchor point), so it keeps its distance from that edge when the screen size changes. Elements without a placement keep their authored position; removing a placement restores it.
- [x] Layouts: "Modern" is the built-in preset (authored positions) and cannot be modified, renamed or deleted; saving while it is active creates "Layout N". User layouts are stored account-wide; the active layout is stored per character (server character id).
- [x] Manager panel (`EditModeManagerFrame`): previous/next layout, name box, New (named by the box when unused, else "Layout N"), Rename, Delete (character returns to the preset), Revert, Save, Exit. Exiting (F10, Escape, Exit) discards unsaved edits.
- [x] Persistence: `ui_layout.ron` beside `options_settings.ron`, key `edit_mode: (layouts: {name: (elements: {key: (anchor, offset)})}, active_layout: {character: name})`, RON. Survives restart.
- [x] The action-bar preview (bars 2–5 and mover labels) follows edit mode instead of owning F10.

## Godot client presets

- [x] Two system presets, Modern and Forever, set every HUD frame's anchor (`godot/ui-model/src/ui/hud_layout.rs`); the selected character's preset applies in the world and a switch moves the frames at once.
- [x] Modern keeps the Retail Modern Edit Mode anchors.
- [x] Forever puts player, target, focus and the player cast bar at FlareUI's positions (`Modules/UnitFrames.lua:1888-1894`). The supplied October 3 unit-frame reference overrides pet/ToT positions: pet below the player's right edge (measured 6-unit gap), ToT right of target and top-aligned (measured 8-unit gap), all relative to viewport centre. These replace the source's bottom anchors that overlap at 1080p. The main action bar is centred at UIParent BOTTOM (0, 2), overriding Camelot's combined main/micro/bags strip for the supplied FlareUI reference. Both Camelot gryphons flank the main bar itself (154×95, 30-unit overlap, 5-unit lift); the right cap no longer follows BagsBar. Micro menu and bag bar are hidden under Forever; their underlying Camelot rects remain authored. This selects FlareUI's supported permanent-hiding behavior (`Modules/Visibility.lua:317-344`) to match the reference, not its defaults: `Core.lua:82-99,317-324` defaults hiding and fading off. The screenshot cannot establish whether the player's actual profile hides, fades or relocates them. FlareUI styles/scales buttons but does not choose the shown bars, rows, counts or anchors; these belong to the player's Blizzard Edit Mode layout (`Blizzard_ActionBar/Shared/ActionBar.lua:98-103`). Its minimap, buffs, debuffs, party, raid, damage meter and objective tracker keep Modern's anchors, as Forever's own preset does.
- [x] Forever draws those five unit frames in FlareUI 1.3's shape (`Core.lua:281-302`, `Modules/UnitFrames.lua` `LayoutBars`/`LayoutFrame`): player and target 240×60, target of target 120×28, focus 160×36, pet 160×28; a 4-px inset health bar (the player's 14-high power bar below it), reaction-coloured, filling right to left on target, ToT and focus; health as a percentage on player, target, ToT and focus. ToT's mirrored level/name/percentage override `Core.lua:293` per the reference; small-unit state retains level and colour from the existing full unit state. Player level/name stay left, percentage right; target/ToT/focus are mirrored; no portrait; `Interface\Tooltips\UI-Tooltip-Border` (16-px edge) tinted FlareUI's #A67D45 over `DialogFrame\UI-DialogBox-Background-Dark`. Boss and party frames keep their Retail shape.
- [x] Forever's player cast bar is FlareUI's standalone bar: 292×26 (`Core.lua:281`) in a 326×34 bronze-bordered holder (`UnitFrames.lua:2232-2234`) at BOTTOM (0, 268), filled #5C8FC7 (channels #80BFE0, uninterruptible grey). Spell icon 26×26 sits flush left, no extra gap (`UnitFrames.lua:53,568-569`); its FDID comes from the existing spell catalog, with no substitute art. Name is left-aligned at 4, timer right-aligned at -4 with a 4-unit name/timer gap (`:49,588-596`).
- [x] Existing selected target aura groups appear above its right edge, growing left then up. Selection, ordering, stacks, cooldowns and dispel art remain unchanged. Icons 20 (`Core.lua:292`), outer gap 4 (`UnitFrames.lua:48`), spacing 2 (`:812`), wrap before the middle (`:962-990`). The reference's right-edge placement overrides the default left-corner buff lane.
- Inherited client text allocations (Lua sizes these intrinsically): level width 20, percentage/timer width 40. Pet/ToT sizes are source-cited 160×28/120×28 (`Core.lua:300,293`), not the previous screenshot measurements 120×24/160×32.
- [x] A unit-frame atlas name the active skin has no member for is logged and not drawn.
- [x] Forever's chat frame, damage meter and tooltips use the `flare_bronze` panel style (`godot/ui-model/src/ui/flare_panel.rs`): a `Backdrop` of `UI-DialogBox-Background-Dark` inside a 16-px `UI-Tooltip-Border` edge, insets 3. Chat: #A67D45 at background alpha 0.6, 10 around the messages plus a 24 header (Chat.lua:1515-1520,1583-1594), replacing the Chattynator background. Meter: #A67D45 at 0.6, 2 outside the window, replacing Blizzard's background and header art (DamageMeter.lua:202-232). Tooltips: centre 0.05/0.05/0.06 at 0.9, border by class (players), reaction (other units), muted item quality (×0.85) or #CC9957 (Tooltips.lua:37-48,117-181); default-anchored tooltips go to `ANCHOR_CURSOR_RIGHT` (Tooltips.lua:410-423,459).
- [x] The pet action bar takes its position from the active `hud_layout` preset. Modern keeps its 318×30 bar at UIParent BOTTOM + (-281, 95). Forever anchors its BOTTOMLEFT to the main bar's BOTTOMLEFT + (30, main height + 4), using the Camelot bottom stack (`Mainline/EditModePresetLayouts.lua:183-198`, `Camelot/EditModePresetLayoutConstants.lua:3,31,35`, `Shared/EditModeManager.lua:664-672,703-718`). FlareUI scales pet buttons by 1.06 (`Core.lua:102-107,218`, `Modules/ActionBars.lua:43,179-190`); chrome uses skin-resolved atlas names. At 1920×1080, Forever's pet rect is (692.14, 994.5, 337.08, 31.8), following the centred main bar. `godot/ui-model/tests/forever_pet_bar.rs` pins Modern's geometry and resolved art to its pre-fix dump and tests preset switching.
- Test: `godot/rust/src/ui/hud_layout_tests.rs`, `godot/ui-model/tests/forever_unit_reference.rs`, `godot/ui-model/tests/forever_flare_frames.rs`, `godot/ui-model/tests/forever_pet_bar.rs` (Modern frame trees pinned to their pre-Forever dump).
- Test: `godot/rust/src/ui/flare_panel_tests.rs`, `godot/rust/src/ui/modern_panel_snapshot_tests.rs` (Modern chat/meter/tooltip canvases pinned to their forever6 hashes).
- Not drawn under Forever yet: class colour on the player health bar (state carries no class), FlareUI's power text, health/power separator, player/ToT aura population, indicators and target/ToT cast bars (#CC995C); class bars and status icons are not shown on the Forever player frame. The objective tracker is not scaled to the minimap width yet (FlareUI `matchTrackerWidth`, Minimap.lua:366-405: scale = minimap frame outer width 260 / 288): the Godot projection has no per-frame scale. The tooltip centre multiplies the dark dialog background, so it shows black at 0.78 alpha where FlareUI's recoloured Blizzard centre is 0.05/0.05/0.06 at 0.9. `ANCHOR_CURSOR_RIGHT` is taken as the tooltip's BOTTOMLEFT on the cursor: no local Blizzard source defines it.

### Bounded action-bar reference correction

- [x] Forever keeps the existing 12-button single row: 45×45, gap 2, FlareUI scale 1.06 (`Core.lua:218`, `Modules/ActionBars.lua:179-190`), giving 595.72×47.7. At 1920×1080 its rect is (662.14, 1030.3, 595.72, 47.7); left/right cap rects are (538.14, 1001.65, 154, 95) and (1227.86, 1001.65, 154, 95). Hidden micro/bags keep rects (912, 1034, 329, 40)/(1248, 1031, 368, 47). Modern tree fixtures and geometry stay unchanged.
- Reference measurements, not FlareUI defaults: two larger rows of 9 slots, one smaller row of 10 above; on the supplied 2000×1126 whole-HUD image, larger slots approximately 38×38 with 2-px gaps, smaller approximately 29×29 with 2-px gaps, block centred around x=1000 and bottom-aligned. Pixel sizes reflect screenshot/UI scaling, not authored UI units. Exact bar identities/profile settings are not recoverable from this image.
- Missing/out of scope: additional player action bars (`MultiBarBottomLeft`, `MultiBarBottomRight`, etc.), variable shown-icon counts and 9-per-row wrapping. FlareUI enumerates bars1–8 and pet/stance for styling (`Modules/ActionBars.lua:34-44`), not visibility. No new bars, fade engine, Classic-only bag mechanics or FlareUI Media art.
- Tests: `godot/rust/src/ui/hud_layout_tests.rs` pins concrete main/caps/micro/bags/pet rects and utility visibility on a 1366×768 canvas; `godot/ui-model/tests/forever_action_bars.rs`, `forever_pet_bar.rs` pin art, centred placement, preset switching and unchanged Modern dumps. No live-client proof.

### Bounded reference-correction boundary

The unit-frame reference fixtures prove geometry/text with populated ToT/focus states. Runtime `targeting.rs::unit_frames_state` still supplies `None` for both (pre-existing); this pass does not add unit relationships. Target/ToT cast bars remain deferred: `nameplates.rs:974-983` reads replicated `CastState`, but the HUD snapshot has no cast projection and ToT has no resolved unit identity. Reuse that state in a separate bounded binding pass; no new network plumbing was started. No live client proof is claimed.

## Art

- Selection boxes: `Interface/EditMode/EditModeUIHighlightBackground.blp` (FDID 4554383) and `EditModeUISelectedBackground.blp` (4554386), stretched, alpha 0.7. Retail slices these as nine-slices; this uses them stretched.
- Panel: the static-popup dialog background (FDID 6839810) and `static_popup` border panel style; buttons use the `defaultbutton-nineslice-*` atlases. Retail's EditModeManagerFrame dialog border was not identified in the listfile.

## Implementation inventory

- `src/edit_mode/mod.rs` — `EditMode`, F10 toggle, drag/snap, `PostUpdate` override writer (captures and restores authored layout).
- `src/edit_mode/elements.rs` — element registry.
- `src/edit_mode/layouts.rs` — layout data and preset rules.
- `src/edit_mode/panel.rs` — screens, panel click/name input, actions and persistence.
- `src/ui/screens/edit_mode_component.rs` — `rsx!` selection boxes and panel.
- `src/scenes/game_menu/mod.rs` — Escape step.

## Tests asserting this spec

- `tests/unit/edit_mode_tests.rs` — anchor math, snapping, preset rules, element registry, selection boxes, drag + Save click + restart, per-character active layout, discard on exit, rename/delete persistence, Escape order.

## Known gaps

- No chat frame exists, so chat is not registered. The objective-tracker frame is registered but not mounted in world yet; it becomes movable once a frame with that name is mounted. `BuffFrame` and `DebuffFrame` are mounted ([buff frame](buff-frame.md)).
- Exiting with unsaved edits discards them without a confirmation prompt.
- No per-element settings (size, orientation, padding) as in Retail.
