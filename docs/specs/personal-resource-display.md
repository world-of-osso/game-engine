# Personal Resource Display

Retail `PersonalResourceDisplayFrame` in the Godot client: the player's health, power, alternate power and class resource bars in one HUD block, enabled by an option.

References (`~/.cache/wow-ui-sim/blizzard-ui/retail/AddOns/`): `Blizzard_SettingsDefinitions_Frame/Combat.lua:14-17`; `Blizzard_PersonalResourceDisplay/Blizzard_PersonalResourceDisplay.lua`, `.xml`, `AlternatePowerBars/ManaAlternatePower.lua`; `Blizzard_EditMode/Mainline/EditModePresetLayouts.lua:756-780`, `Shared/EditModeSettingDisplayInfo.lua:950-1060`; `/syncthing/Sync/Projects/wow/wow-ui-sim/src/cvars.yaml:1013` (`nameplateShowSelf: '0'`); GlobalColor 391.

## What it must do

- [x] Option "Personal Resource Display" (`nameplateShowSelf`), off by default, persisted in `options_settings.ron` as `nameplateShowSelf`. It sits in Options → HUD: the client's Options menu has no Combat category, and a fourteenth tab overflows the 454 px tab column.
- [x] While on, `PersonalResourceDisplayFrame` shows (Visible Setting Always). The client has no Edit Mode, so the frame keeps the Modern preset: 200 wide, BOTTOM at UIParent BOTTOM + (−410, 380), health and power bars 200×15, padding 4, opacity 100%, no bar text, no class colour.
- [x] Health bar in `PERSONAL_RESOURCE_DISPLAY_DEFAULT_HEALTH_COLOR` (0, 0.8, 0); power bar 4 below in `MANA_BAR_COLOR` for mana, else `PowerBarColor`; both `UI-HUD-CoolDownManager-Bar` over `-Bar-BG` (TOPLEFT −2,3 / BOTTOMRIGHT 6,−7).
- [x] Alternate mana bar 4 below the power bar for Shadow priests (258) and Balance druids (102), in `PowerBarColor.MANA`.
- [x] `ClassFrameContainer` 200×15 for classes in `CLASS_FRAME_INFO_MAP`, TOP at the last bar's BOTTOM + (0, yOffset − 4) (Paladin −14, Rogue/DK −10, Evoker −12, others −8), with the display's own copy of the class bar (same bar objects as the PlayerFrame's) centred on it. The container counts toward the frame height unless the druid is out of cat form (`HasClassInfo`), so a Frost mage's frame is 61 tall with no points.
- [ ] Not built: Edit Mode settings (size, width, heights, padding, opacity, Visible Setting In Combat/Hidden, hide bars, class colour, bar text, hide class bar on the PlayerFrame); heal prediction, absorbs and temp max-health loss; mana cost prediction, builder/spender feedback and full-power pulse; Brewmaster stagger, Devourer soul fragment and Augmentation Ebon Might alternate bars.

## Tests asserting this spec

- `godot/ui-model/tests/personal_resource_display.rs`: rogue health/energy/5 combo points, paladin mana/Holy Power, Frost mage mana only, Shadow priest alternate mana, disabled draws nothing.
- `godot/ui-model/tests/options_policy.rs`: `personal_resource_display_toggle_persists`.
- Live: `godot/tests/personal_resource_display_live.gd` (rogue enables the option through Options, then fights a Blackrock Worg with Sinister Strike); evidence `data/diagnostics/prd-2026-10-03/run2/`.
