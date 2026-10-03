# Damage meter

Retail's built-in damage meter window (`Blizzard_DamageMeter`, 12.x) at the top left. The server computes the sessions (game-server `docs/specs/damage-meter.md`) and sends `DamageMeterSnapshot`; the client only shows them.

Client: `godot/ui-model/src/damage_meter_data.rs` (session selection, rows, number formats), `godot/ui-model/src/ui/screens/damage_meter_component.rs` (window), `godot/rust/src/damage_meter.rs` (host, actions, fixture state). Fixture: `godot/tests/damage_meter.gd`.

## What it must do

- [x] The window is `DamageMeter` anchored TOPLEFT of UIParent at 0,0 (DamageMeter.xml:11-15; Edit Mode preset `anchorInfo`, EditModePresetLayouts.lua:857-863), 400x140 with 16 px bars 4 px apart: the preset raw values FrameWidth 100, FrameHeight 20, BarHeight 1, Padding 2 over the slider minimums (EditModeSettingDisplayInfo.lua:1127-1175, `ConvertValueDiffFromMin`/`ConvertValueDefault`). Shown in the world only.
- [x] Header (`DamageMeterSessionWindowTemplate`, DamageMeterSessionWindow.xml): `ui-damagemeters-header-bar`; the session timer `"[MM:SS] "` at +15,-9 only while the player is in combat (`ShouldDisplaySessionTimer`, `SetSessionDuration`); the type dropdown arrow and "Damage Done" (the default type, DamageMeter.lua:92-97) after it; on the right the session dropdown ("O"/"C"), settings and minimize button art.
- [x] The window starts on `Overall` (DamageMeter.lua:80). Clicking the session dropdown opens a menu with `Current Segment` and `Overall` radios (DamageMeterSessionWindow.lua:433-437); choosing one switches the session and closes it.
- [x] Rows (`DamageMeterSourceEntryTemplate`, Default style): the server's rank order, "N. Name" (`DAMAGE_METER_SOURCE_NAME`) left, Compact numbers "damage (dps)" (`DAMAGE_METER_ENTRY_FORMAT_COMPACT`, the preset `Numbers`) right in NumberFontNormal (ARIALN 14 outline); the StatusBar fill is `UI-HUD-CoolDownManager-Bar` tinted by `RAID_CLASS_COLORS`, filled by damage over the session's highest, over `ui-damagemeters-bar-shadowbg`/`-shadowedge`. Numbers use `AbbreviateLargeNumbers` as last shipped in Lua (UIParent.lua:774-785).
- [x] `damagemeters-background` behind everything at 50% (preset BackgroundTransparency 50).
- [ ] The dropdown menu's art is a plain dark panel, not `MenuStyle2`; past sessions ("Combat N"), the meter type menu, the settings menu, minimize, resize and moving are not implemented.
- [ ] Spec icons (`ShowSpecIcon` 1) are not drawn: the snapshot has no spec. The rows start at the bar's left edge as with icons off.
- [ ] Retail's C `AbbreviateLargeNumbers` may format differently from the old Lua one.
- [ ] Only the first 5 rows fit; the scroll box, scroll bar and pinned local-player row (`AlwaysShowsLocalPlayer`) are not implemented.
- [ ] The per-spell breakdown window (`DamageMeterSourceWindow`, opened by clicking a row) is not implemented; the snapshot already carries each source's spells.

## Tests

- `godot/ui-model/src/damage_meter_data.rs` tests: number abbreviation and clock format; Overall default with ranked class-coloured compact rows and hidden timer out of combat; Current with its timer in combat and empty before the first combat.
- `godot/tests/damage_meter.gd` (live, private server UDP 5090, level-10 Human mage `Fbdps` at `-8960 -138 81.6`, 10 yd from a Northshire Training Dummy): three Frostbolts at the dummy per combat; the window shows the player's damage equal to the summed combat log damage in Current and Overall; a second combat is a new session, Overall sums both; the session menu switches the view. Screenshots in `data/diagnostics/dpsmeter-2026-09-29/`. Exit 0 at game-engine `10f0ce87` + fixture `026070d8`: combat 1 logged 307 = Current 307 = Overall 307; combat 2 logged 308 = Current 308 (session 2), Overall 615.
