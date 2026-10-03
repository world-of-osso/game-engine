# XP Bar

> Root `src/` paths below name files deleted with the [retired Bevy client](godot-conversion.md#retired-bevy-client-user-decision-2026-10-02).

The Retail experience bar in `MainStatusTrackingBarContainer`, fed by the server's owner-only experience messages. The contract is shared-protocol `protocol/experience_messages.rs` (`PlayerXpUpdate { xp, next_level_xp, rested_xp }`, `LogXpGain`); the server sends `PlayerXpUpdate` on enter world, after each gain and on a level change, with `next_level_xp` 0 at the level cap.

References:
- STB.xml = `Blizzard_ActionBar/Mainline/StatusTrackingBar.xml`; STBT.xml = `Mainline/StatusTrackingBarTemplate.xml`; STMO.lua = `Mainline/StatusTrackingManagerOverrides.lua`
- EB.xml = `Mainline/ExpBar.xml`; EB.lua = `Shared/ExpBar.lua`; EBO.lua = `Mainline/ExpBarOverrides.lua`
- EMPL.lua = `Blizzard_EditMode/Mainline/EditModePresetLayouts.lua`; EMPLC.lua = `Blizzard_EditMode/Standard/EditModePresetLayoutConstants.lua`
- all under `~/.cache/wow-ui-sim/blizzard-ui/retail/AddOns/`; strings from GlobalStrings (build 12.1); atlases from UiTextureAtlasMember (the `-2x` members of atlas 1988, `interface/hud/uiexperiencebar2x.blp`, FDID 4615784)

## What it must do

- [x] `PlayerXpUpdate` sets the bar; `OnExit(InWorld)` clears it.
- [x] Container 571×17 (STB.xml:3-4) at the screen bottom centre: Retail Modern puts status bar 1 at BOTTOM offset 0 of `StatusTrackingBarManager` (EMPL.lua:582-594, `STATUS_BAR_1_ANCHOR_OFFSET_Y` 0, EMPLC.lua:29), below the main action bar at `MAIN_ACTION_BAR_DEFAULT_OFFSET_Y` 45 (EMPLC.lua:2).
- [x] Bar 565×11 at BOTTOMLEFT (1, 5) of the container (STMO.lua:43-51), layered background (`UI-HUD-ExperienceBar-Background`, STBT.xml:20), rested overlay, fill, then the `UI-HUD-ExperienceBar-Frame` art over the whole container (STB.xml:7).
- [x] Fill = `xp / next_level_xp`, the left part of `UI-HUD-ExperienceBar-Fill-Experience`; with a rested pool the fill uses `UI-HUD-ExperienceBar-Fill-Rested` (EBO.lua:2-18, EB.lua:161-170).
- [x] Rested overlay (`ExhaustionLevelFillBar`, `UI-HUD-ExperienceBar-Fill-Prediction`, EB.xml:11-16): width = `(xp + rested_xp) / next_level_xp` × bar width, texcoords (0, ratio). Hidden without a rested pool or when the pool ends past the bar's right edge (EB.lua:135-157).
- [x] Exhaustion tick (`UI-HUD-ExperienceBar-Frame-Pip`, 10×14, EB.xml:20-36): centred on the rested end, 2 above the bar centre (`yOffset`); hidden without a pool and within 1% of either edge (`hideAtBarEdge`, EB.lua:141-145).
- [x] Hover shows the bar text `XP_STATUS_BAR_TEXT` "XP: %d/%d" (EBO.lua:10-12, EB.lua:72-75) in `TextStatusBarText` (FRIZQT 10 outline, white), centred 1 up (STBT.xml:37-41); leaving hides it.
- [x] Hover tooltip (`ExhaustionToolTipText`, EBO.lua:20-35): title `XP_TEXT` "%s / %s  ( %d%% )" with `BreakUpLargeNumbers` and a `ceil` percent, then `EXHAUST_TOOLTIP1`: the rest state name in gold, "%d%% of normal experience" / "gained from monsters." in white.
- [x] Hidden at the level cap (`next_level_xp` 0, Retail `GameRulesUtil.CanShowExperienceBar`, STMO.lua:30-31) and before the first update.
- [x] Edit mode: registered as "Status Bar 1" (`HUD_EDIT_MODE_STATUS_TRACKING_BAR_LABEL` "Status Bar %d", `EditModeSystemTemplates.xml:266-270`), anchor Bottom; the container is shown while edit mode is on even without a bar (`UpdateShownState`, `StatusTrackingManager.lua:233-236`).
- [x] `LogXpGain` prints its chat line: `COMBATLOG_XPGAIN_FIRSTPERSON` "%s dies, you gain %d experience.", with a rested bonus `COMBATLOG_XPGAIN_EXHAUSTION1` "... (+%d exp Rested bonus)", no victim `COMBATLOG_XPGAIN_FIRSTPERSON_UNNAMED` "You gain %d experience.". Quest XP prints nothing here: the quest-complete notice already prints `ERR_QUEST_REWARD_EXP_I` "Experience gained: %d.".
- [ ] Not built: the fade in/out and max-level fade animations (STB.xml:10-29), the gain flare and level-up flipbooks (EBO.lua:3-8), the `xpBarText` always-show option, the other status bars (reputation, honor, artifact, azerite, house favor) and `SecondaryStatusTrackingBarContainer`, the group/raid variants of the XP chat line, Retail's tooltip anchor (`GameTooltip_SetDefaultAnchor`; the shared tooltip follows the cursor).
- [ ] XP chat lines use the System chat type; Retail's `COMBAT_XP_GAIN` chat type colour is not modelled.

## Assumptions

- The rest state name and multiplier come from the C-side `GetRestState()`, not the Lua tree. This uses "Rested" 200% with a pool and "Normal" 100% without one; rested = `rested_xp > 0`.
- Draw order: Retail puts `ExhaustionLevelFillBar` in the bar's ARTWORK layer and the status bar (with its background) in a child frame; this draws background, overlay, fill so the overlay shows past the fill.

## Tests asserting this spec

- `src/game/experience_data.rs` tests: fill fraction, level cap, rested end, hover/tooltip strings, XP chat lines.
- `src/ui/screens/status_tracking_bar_component_tests.rs`: container size and placement, fill width from a `PlayerXpUpdate`, rested overlay and tick extent, overlay/tick hidden past the level and at the edge, hidden at the cap, edit-mode container, hover text.
- `src/scenes/status_tracking_bar/tests.rs`: the in-world plugin follows updates (fill advances, hides at the cap) and hover.
- `src/game/networking/experience_tests.rs`: a kill updates the state and prints the gain with the victim name.
- `src/scenes/tooltip_frame/mod.rs` test `xp_tooltip_uses_xp_text_and_the_rest_state`.
- `tests/unit/edit_mode_tests.rs`: registration, selection box label, drag.
- `src/rendering/hud_layout_tests.rs`: on screen, no overlap with any HUD element (action bars, chat, micro menu, bags), bottom-centre below the main bar.
- Live evidence (2026-09-25, headless, shared :5000 server): `data/diagnostics/xpbar-20260925/` — t01 new level-1 Mage "Xpbar" (`xp_ui`), bar shown with "XP: 0/250"; t04 after killing a Kobold Vermin: fill 45 px of 565 (20/250) and the chat line "Kobold Vermin dies, you gain 20 experience." (`t04-after-kill-ui-tree.txt`). Not shown live: rested overlay/tick (a new character has no rested pool), hover text and tooltip (no cursor in the headless run), level cap.
