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
- [x] Forever puts player, target, target of target, focus, pet and the player cast bar at FlareUI's positions (`Modules/UnitFrames.lua:1888-1894`), and the micro menu, main action bar and bags bar at Forever's Camelot constants (`Blizzard_EditMode/Camelot/EditModePresetLayoutConstants.lua:6-10,38-49`). Its minimap, buffs, debuffs, party, raid, damage meter and objective tracker keep Modern's anchors, as Forever's own preset does.
- Test: `godot/rust/src/ui/hud_layout_tests.rs`.

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
