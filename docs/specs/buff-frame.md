# Buff frame

This spec defines the player BuffFrame and DebuffFrame at their Retail defaults. Citations are to `~/.cache/wow-ui-sim/blizzard-ui/retail/AddOns/` (`Blizzard_BuffFrame`, `Blizzard_EditMode`, `Blizzard_FrameXMLUtil`, `Blizzard_SharedXML`, `Blizzard_Fonts_Shared`).

## What it must do

- [x] `BuffFrame` anchors TOPRIGHT -255,-10 and `DebuffFrame` TOPRIGHT -270,-155 on the screen, independently (EditModePresetLayouts.lua:425-431, 443-449). Both are edit-mode elements.
- [x] Buttons are 30×40 with a 30×30 icon at the top (BuffFrame.lua:139-140, BuffFrameTemplates.xml:5-11), 5 px padding (EditModePresetLayouts.lua:423, 440), 11 buffs / 8 debuffs per row, growing left and wrapping down (EditModePresetLayouts.lua:419-421, 436-438).
- [x] Buff icons start 15 px left of BuffFrame's right edge, where the collapse button is anchored (BuffFrame.xml:14-17, BuffFrame.lua:531-534). The frames are sized for every slot: BuffFrame 400×135, DebuffFrame 280×90 (BuffFrame.lua:251-273).
- [x] Up to 32 buffs and 16 debuffs (BuffFrame.lua:2-3), in replicated order.
- [x] Duration text sits under the icon in `GameFontNormalSmall` (FRIZQT 10, 1,-1 shadow), formatted by `SecondsToTimeAbbrev`: a unit is used from 1.5 of it and rounds up ("89 s", "2 m", "60 m", "2 h"), seconds truncate (TimeUtil.lua:463-483). Permanent auras show none. Text is gold, and white below 90 s (`BUFF_DURATION_WARNING_TIME`, AuraUtil.lua:2; `AuraButtonMixin:UpdateDuration`).
- [x] Timed auras under 31 s (`BUFF_WARNING_TIME`, BuffFrame.lua:1) flash: button alpha bounces 0.3 to 1.0 over 1.5 s (BuffFrameTemplates.xml:62-71, BuffFrame.lua:37-62).
- [x] Stack count above 1 at the icon's bottom right, -2,2, in `NumberFontNormal` (ARIALN 14 outline, white) (BuffFrameTemplates.xml:37-41).
- [x] Buffs have no border (`UpdateAuraType`). Debuffs draw a 40×40 border centred on the icon (BuffFrameTemplates.xml:20-25) from the `ui-debuff-border-*` atlas (`interface/hud/uidebuffframes.blp`, FDID 7553349). DebuffFrame shows dispel types (`ShowDispelType` 1, EditModePresetLayouts.lua:441), so Magic/Curse/Disease/Poison use their `-icon` member and untyped debuffs `default-noicon` (Mainline/AuraUtil.lua:20-23).
- [x] Colorblind mode keeps the border and writes the dispel abbreviation ("Ma", "Cu", "Di", "Po"; enUS `DEBUFF_SYMBOL_*`) at the button's top left (BuffFrameTemplates.xml:32-36, AuraUtil.lua `SetAuraSymbol`).
- [x] Hidden and passive auras are not shown.
- [x] Hovering a button shows the aura tooltip. Right-click on a buff sends `CancelAura`; right-click on a debuff does nothing (`AuraButtonMixin:OnClick`).
- [ ] Collapse/expand arrow. Retail shows it only when the `collapseExpandBuffs` CVar is on and a buff lasts over 90 s; the CVar default is not in the Lua tree, so it is not built.
- [ ] Retail tooltip content (`GameTooltip:SetUnitAura`: name, dispel type, description, time remaining). The shared tooltip shows name, description, duration, stacks and source instead.
- [ ] Temporary weapon enchants, consolidated buffs, private aura anchors and the deadly-debuff warning.
- [ ] Bleed borders: the protocol has no Bleed dispel type.
- [ ] Live debuff proof: creatures never cast and no Northshire creature aura reaches the player, so debuff layout, borders and symbols are proven by screen tests only.

## Live proof

`data/diagnostics/buffs-20260924/` (2026-09-24, level-10 Warrior InviteTarget): Battle Shout (6673) shows "60 m" under its icon at BuffButton0 x=1620 y=10, the hover tooltip shows it, and `ui.rightClick("BuffButton0")` removes it (`buff-proof.js`).

## Implementation inventory

- `src/ui/screens/buff_frame_component.rs`: layout, duration text, borders, flash curve.
- `src/scenes/buff_frame/mod.rs`: mounting, flash system, right-click cancel.
- `src/game/buff_data.rs`: `AuraInstance::timer_text`, hidden/passive filter.
- `src/scenes/tooltip_frame/mod.rs`: aura tooltip on hover.
