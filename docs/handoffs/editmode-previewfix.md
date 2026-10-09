# Edit Mode preview/selection audit — 2026-10-09

## Proof boundary

- Baseline revision `4850c6652`: rebuilt extension/CLI exit0; native 1920×1080 preview rectangles captured through Godot `Control.get_global_rect`, not inferred from pixels. Source JSON: `data/diagnostics/editmode-previewfix-2026-10-09/baseline-rects.json`.
- Live private UDP5600, fresh account `fb_previewfix`, server HEAD/master `5b04aa931720d441aaea1f99d3c11dc2a69bba40`: Forever default layout entered through real F10 and visually inspected. Capture succeeded; subsequent fixture failed at an incorrectly named Previous control before writing numeric inventory. No numeric live rows or Modern live comparison are claimed.
- Fix `03f6955e4`: RED showed chat canvas `(-26,875,469,235)` versus rendered skin `(-2,868,450,214)`; GREEN targeted `hud_edit` 22/22 includes selection footprint, saved drag and preserved canvas/skin inset. Final extension build/captures blocked by lead-reported NVIDIA userspace615.78 versus loaded module615.71.09; user reboot required. Queued capture-only build cancelled before acquisition; existing images explicitly renamed `before-*`.

Rectangles are `(x,y,width,height)` at UI scale1, 1920×1080. Tables contain baseline native selection/root rects. “Forever preset” is the **same production component’s authored footprint under FOREVER**, for the preview’s concrete fixture state; it is not a new layout proposal. Live `pending` means no trustworthy numerical observation exists. Missing live party/raid/target/buff states must remain absent unless mounted in that live run.

## Placement path and findings

`hud_edit_preview::preview_screen` (hud_edit_preview.rs:199-274) calls the real player/action-bar/chat/minimap/tracker/buff/XP/group/micro components. Each reads `hud_layout(ctx)` (`godot/ui-model/src/ui/hud_layout.rs:523-528`), choosing the same `MODERN`/`FOREVER` constants as live canvases. `GameClient::apply_ui_layout` (`godot/rust/src/ui_layout.rs:80-90`) publishes skin/settings/saved offsets and resyncs those canvases. No preview-specific positioning code or incorrect Forever preset was found in the named paths; no preset anchors changed.

Raid now starts `(22,145)`, matching the Mainline top-left anchor already landed as `e793a64b8`; original bottom-centre raid report does not reproduce on this base. Preview deliberately mounts both five-member party and eight one-member raid groups, so their top-left preset boxes overlap each other; that synthetic simultaneous inventory is not a reason to relocate either live preset.

Forever party root reserves14 units above its members while its title is hidden (`group_frames_component.rs:144-150,176-184,222`): selection equals the native container, not the painted members alone. This is a reported visual gap, not an inferred live placement bug; preset stays compact/top-left.

Forever chat was the proven selection-footprint defect: the authored canvas intentionally starts offscreen to put its inset border flush bottom-left (`hud_layout.rs:409-415`). Both selection collection and placement projection now use `ChatFrame1FlareSkin`, while moving the entire canvas/descendant tree. The frame position itself is unchanged. Modern chat retains its native container, including tab/input allocation.

Tracker selection intentionally differs from its content-height root: Retail `Blizzard_ObjectiveTracker/Blizzard_ObjectiveTrackerContainer.lua:203-209` sets container height from parent height plus offsetY. Existing native contract/test preserves the full default container selection (Modern805 / Forever703), including centre-label clearance; not silently changed to header-only29/32. No tracker preset relocation.

## Retail labels

Retail does **not** show every system name continuously. `Shared/EditModeSystemTemplates.xml:35-39` declares the label `hidden="true"`; Lua `3193-3195` returns `self:IsSelected() or self:IsShowingEditInstructions()`. Lua `3248-3254` returns `self.system:GetSystemName()` when selected, otherwise `HUD_EDIT_MODE_INSTRUCTIONS_CLICK_TO_EDIT`; `3259-3261` applies text and visibility. Therefore empty error/buff/debuff boxes at rest are expected; hover shows “Click to edit”, selected shows the name. Existing shared-label behavior passed in the 22-test GREEN run. No label rule changed.

## Forever baseline

| System | Preview selection | Preview native root | Live rect | Forever preset root/skin | Mismatch |
|---|---|---|---|---|---|
| Player Frame (`PlayerFrame`) | (510, 780, 240, 60) | (510, 780, 240, 60) | pending | (510, 780, 240, 60) | none against native root |
| Target Frame (`TargetFrame`) | (1170, 780, 240, 60) | (1170, 780, 240, 60) | pending | (1170, 780, 240, 60) | none against native root |
| Target of Target (`TargetOfTargetFrame`) | (1170, 868, 120, 28) | (1170, 868, 120, 28) | pending | (1170, 868, 120, 28) | none against native root |
| Focus Frame (`FocusFrame`) | (1420, 780, 160, 36) | (1420, 780, 160, 36) | pending | (1420, 780, 160, 36) | none against native root |
| Cast Bar (`PlayerCastingBarFrame`) | (797, 778, 326, 34) | (797, 778, 326, 34) | pending | (797, 778, 326, 34) | none against native root |
| Action Bar 1 (`MainActionBar`) | (737, 1030, 446, 48) | (737, 1030, 446, 48) | pending | (737, 1030, 446, 48) | none against native root |
| Action Bar 2 (`MultiBarBottomLeft`) | (737, 980, 446, 48) | (737, 980, 446, 48) | pending | (737, 980, 446, 48) | none against native root |
| Action Bar 3 (`MultiBarBottomRight`) | (767, 941, 386, 37) | (767, 941, 386, 37) | pending | (767, 941, 386, 37) | none against native root |
| Action Bar 4 (`MultiBarRight`) | not mounted | not mounted | pending | not mounted | disabled side bar; no selection |
| Action Bar 5 (`MultiBarLeft`) | not mounted | not mounted | pending | not mounted | disabled side bar; no selection |
| Status Bar 1 (`ExperienceBar`) | (662, 6, 596, 17) | (662, 6, 596, 17) | pending | (662, 6, 596, 17) | none against native root |
| Minimap (`MinimapCluster`) | (1660, 0, 260, 260) | (1660, 0, 260, 260) | pending | (1660, 0, 260, 260) | none against native root |
| Objective Tracker (`ObjectiveTrackerFrame`) | (1673, 272, 234, 703) | (1673, 272, 234, 29) | pending | (1673, 272, 234, 29) | intentional Retail full-height container vs content root |
| Buffs (`BuffFrame`) | (1265, 10, 400, 135) | (1265, 10, 400, 135) | pending | (1265, 10, 400, 135) | none against native root |
| Debuffs (`DebuffFrame`) | (1370, 155, 280, 90) | (1370, 155, 280, 90) | pending | (1370, 155, 280, 90) | none against native root |
| Party Frames (`CompactPartyFrame`) | (22, 147, 98, 234) | (22, 147, 98, 234) | pending | (22, 147, 98, 234) | root matches; hidden title reserves14px above painted members under the Forever skin |
| Raid Frames (`CompactRaidFrameContainer`) | (22, 145, 576, 50) | (22, 145, 576, 50) | pending | (22, 145, 576, 50) | top-left correct; synthetic preview party overlap only |
| Error Text (`UIErrorsFrame`) | (704, 122, 512, 60) | (704, 122, 512, 60) | pending | (704, 122, 512, 60) | none against native root |
| Chat Frame (`ChatFrame1`) | (-26, 875, 469, 235) | (-26, 875, 469, 235) | pending | (-26, 875, 469, 235); skin (-2,868,450,214) | FIXED in CPU proof: selection becomes (-2,868,450,214); native recapture pending |
| Micro Menu (`MicroMenuContainer`) | (1585, 1034, 329, 40) | (1585, 1034, 329, 40) | pending | (1585, 1034, 329, 40) | none against native root |
| Bags Bar (`BagsBar`) | (1706, 984, 208, 47) | (1706, 984, 208, 47) | pending | (1706, 984, 208, 47) | none against native root |

## Modern baseline

| System | Preview selection | Preview native root | Live rect | Forever preset root/skin | Mismatch |
|---|---|---|---|---|---|
| Player Frame (`PlayerFrame`) | (428, 730, 232, 100) | (428, 730, 232, 100) | pending | (510, 780, 240, 60) | none against native root |
| Target Frame (`TargetFrame`) | (1260, 730, 232, 100) | (1260, 730, 232, 100) | pending | (1170, 780, 240, 60) | none against native root |
| Target of Target (`TargetOfTargetFrame`) | (1500, 747, 100, 38) | (1500, 747, 100, 38) | pending | (1170, 868, 120, 28) | none against native root |
| Focus Frame (`FocusFrame`) | (1608, 747, 100, 38) | (1608, 747, 100, 38) | pending | (1420, 780, 160, 36) | none against native root |
| Cast Bar (`PlayerCastingBarFrame`) | (828, 900, 264, 28) | (828, 900, 264, 28) | pending | (797, 778, 326, 34) | none against native root |
| Action Bar 1 (`MainActionBar`) | (679, 990, 562, 45) | (679, 990, 562, 45) | pending | (737, 1030, 446, 48) | none against native root |
| Action Bar 2 (`MultiBarBottomLeft`) | (679, 943, 562, 45) | (679, 943, 562, 45) | pending | (737, 980, 446, 48) | none against native root |
| Action Bar 3 (`MultiBarBottomRight`) | (679, 896, 562, 45) | (679, 896, 562, 45) | pending | (767, 941, 386, 37) | none against native root |
| Action Bar 4 (`MultiBarRight`) | not mounted | not mounted | pending | not mounted | disabled side bar; no selection |
| Action Bar 5 (`MultiBarLeft`) | not mounted | not mounted | pending | not mounted | disabled side bar; no selection |
| Status Bar 1 (`ExperienceBar`) | (675, 1063, 571, 17) | (675, 1063, 571, 17) | pending | (662, 6, 596, 17) | none against native root |
| Minimap (`MinimapCluster`) | (1664, 0, 256, 256) | (1664, 0, 256, 256) | pending | (1660, 0, 260, 260) | none against native root |
| Objective Tracker (`ObjectiveTrackerFrame`) | (1660, 275, 260, 805) | (1660, 275, 260, 32) | pending | (1673, 272, 234, 29) | intentional Retail full-height container vs content root |
| Buffs (`BuffFrame`) | (1265, 10, 400, 135) | (1265, 10, 400, 135) | pending | (1265, 10, 400, 135) | none against native root |
| Debuffs (`DebuffFrame`) | (1370, 155, 280, 90) | (1370, 155, 280, 90) | pending | (1370, 155, 280, 90) | none against native root |
| Party Frames (`CompactPartyFrame`) | (22, 147, 98, 234) | (22, 147, 98, 234) | pending | (22, 147, 98, 234) | root matches; hidden title reserves14px above painted members under the Forever skin |
| Raid Frames (`CompactRaidFrameContainer`) | (22, 145, 576, 50) | (22, 145, 576, 50) | pending | (22, 145, 576, 50) | top-left correct; synthetic preview party overlap only |
| Error Text (`UIErrorsFrame`) | (704, 122, 512, 60) | (704, 122, 512, 60) | pending | (704, 122, 512, 60) | none against native root |
| Chat Frame (`ChatFrame1`) | (0, 760, 500, 280) | (0, 760, 500, 280) | pending | (-26, 875, 469, 235); skin (-2,868,450,214) | none against native root |
| Micro Menu (`MicroMenuContainer`) | (1585, 1034, 329, 40) | (1585, 1034, 329, 40) | pending | (1585, 1034, 329, 40) | none against native root |
| Bags Bar (`BagsBar`) | (1706, 984, 208, 47) | (1706, 984, 208, 47) | pending | (1706, 984, 208, 47) | none against native root |

## Pending after reboot

1. Locked helper extension/CLI build at `03f6955e4`; CPU proof already valid.
2. Run prepared `data/diagnostics/editmode-previewfix-2026-10-09/capture.gd`; produce fresh `modern.png`/`forever.png`, final native rectangles and inspect downscales.
3. Start owned copied private server on UDP5600, then prepared `live.gd` with GODOT_HUDEDIT_* environment; validates Forever/Modern manager preset labels, real F10, chat drag to (80,640), reset to unchanged preset; captures live Forever and writes both-skin numeric inventory. Corrected control name is `EditModeManagerFramePrev`.
4. Complete live table cells only from recorded numeric inventory; absent live systems are N/A, not invented rectangles. DamageMeter is rendered top-left in preview/live but has no registered selection in EDIT_MODE_ELEMENTS; no extra mover added in this bounded fix.
5. Inspect final downscales, stop exact owned PIDs and `agents-editmode-previewfix.slice`; keep PNGs only in requested AgentShared directory.
