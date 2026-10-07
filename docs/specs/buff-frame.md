# Buff frame

> Root `src/` paths below name files deleted with the [retired Bevy client](godot-conversion.md#retired-bevy-client-user-decision-2026-10-02).

This spec defines the player BuffFrame and DebuffFrame and the TargetFrame auras at their Retail defaults. Citations are to `~/.cache/wow-ui-sim/blizzard-ui/retail/AddOns/` (`Blizzard_BuffFrame`, `Blizzard_EditMode`, `Blizzard_FrameXMLUtil`, `Blizzard_SharedXML`, `Blizzard_Fonts_Shared`).

## What it must do

- [x] `BuffFrame` anchors TOPRIGHT -255,-10 and `DebuffFrame` TOPRIGHT -270,-155 on the screen, independently (EditModePresetLayouts.lua:425-431, 443-449). Both are edit-mode elements.
- [x] Buttons are 30×40 with a 30×30 icon at the top (BuffFrame.lua:139-140, BuffFrameTemplates.xml:5-11), 5 px padding (EditModePresetLayouts.lua:423, 440), 11 buffs / 8 debuffs per row, growing left and wrapping down (EditModePresetLayouts.lua:419-421, 436-438).
- [x] Buff icons start 15 px left of BuffFrame's right edge, where the collapse button is anchored (BuffFrame.xml:14-17, BuffFrame.lua:531-534). The frames are sized for every slot: BuffFrame 400×135, DebuffFrame 280×90 (BuffFrame.lua:251-273).
- [x] Up to 32 buffs and 16 debuffs (BuffFrame.lua:2-3), in replicated (server slot) order: `AuraUtil.ForEachAura(PlayerFrame.unit, "HELPFUL"/"HARMFUL")` with no sort (BuffFrame.lua:638,761). Temporary weapon enchants would lead (BuffFrame.lua:703-704); they are not built.
- [x] BuffFrame buttons have no cooldown swipe; they show duration text only (no Cooldown frame in BuffFrameTemplates.xml).
- [x] Duration text sits under the icon in `GameFontNormalSmall` (FRIZQT 10, 1,-1 shadow), formatted by `SecondsToTimeAbbrev`: a unit is used from 1.5 of it and rounds up ("89 s", "2 m", "60 m", "2 h"), seconds truncate (TimeUtil.lua:463-483). Permanent auras show none. Text is gold, and white below 90 s (`BUFF_DURATION_WARNING_TIME`, AuraUtil.lua:2; `AuraButtonMixin:UpdateDuration`).
- [x] Timed auras under 31 s (`BUFF_WARNING_TIME`, BuffFrame.lua:1) flash: button alpha bounces 0.3 to 1.0 over 1.5 s (BuffFrameTemplates.xml:62-71, BuffFrame.lua:37-62).
- [x] Stack count above 1 at the icon's bottom right, -2,2, in `NumberFontNormal` (ARIALN 14 outline, white) (BuffFrameTemplates.xml:37-41).
- [x] Buffs have no border (`UpdateAuraType`). Debuffs draw a 40×40 border centred on the icon (BuffFrameTemplates.xml:20-25) from the `ui-debuff-border-*` atlas (`interface/hud/uidebuffframes.blp`, FDID 7553349). DebuffFrame shows dispel types (`ShowDispelType` 1, EditModePresetLayouts.lua:441), so Magic/Curse/Disease/Poison use their `-icon` member and untyped debuffs `default-noicon` (Mainline/AuraUtil.lua:20-23).
- [x] Colorblind mode keeps the border and writes the dispel abbreviation ("Ma", "Cu", "Di", "Po"; enUS `DEBUFF_SYMBOL_*`) at the button's top left (BuffFrameTemplates.xml:32-36, AuraUtil.lua `SetAuraSymbol`).
- [x] Hidden and passive auras are not shown.
- [x] Hovering a player buff or debuff uses the native `GameTooltipUI` host with `ANCHOR_BOTTOMLEFT`; leaving hides it and remaining time refreshes while hovered (`AuraButtonMixin:OnEnter/OnLeave/OnUpdate`, BuffFrame.lua:888-913,950-970).
- [x] Right-button release on a player buff sends exactly one `CancelAura { spell_id }` on `CombatChannel`; pressing, left-clicking and right-clicking a debuff send nothing (`AuraButtonMixin:OnLoad/OnClick`, BuffFrame.lua:863-884). Removal waits for server replication; the server rejects passive, harmful and `NO_AURA_CANCEL` auras.
- [ ] Collapse/expand arrow. Retail shows it only when the `collapseExpandBuffs` CVar is on and a buff lasts over 90 s; the CVar default is not in the Lua tree, so it is not built.
- [x] Retail aura tooltip name, rendered aura description and remaining time (`GameTooltip:SetUnitAura`); permanent auras omit time. No spell cost/cast details, stack/source rows or total-duration row.
- [ ] Tooltip dispel-type label.
- [ ] Temporary weapon enchants, consolidated buffs, private aura anchors and the deadly-debuff warning.
- [ ] Bleed borders: the protocol has no Bleed dispel type.
- [ ] Live debuff proof: creatures never cast and no Northshire creature aura reaches the player, so debuff layout, borders and symbols are proven by screen tests only.

## TargetFrame auras

Citations are to `Blizzard_UnitFrame` (`Shared/TargetFrameAuraShared.lua`, `Shared/TargetFrameAuraContainer.lua`, `Shared/TargetFrameAuraButton.xml`, `Mainline/TargetFrame.lua/.xml`), `Blizzard_FrameXMLUtil/AuraUtil.lua` and `Blizzard_SharedXMLBase/AnchorUtil.lua`.

- [x] One container, TOPLEFT at the 192×67 `FrameTexture`'s BOTTOMLEFT + (5, 9) (TargetFrame.lua:4-6,547-553; TargetFrame.xml:79-83); the texture is centred in the 232×100 frame. `buffsOnTop` is off by default (EditModePresetLayouts.lua:247,261), so the container hangs below.
- [x] Groups: a friendly target (or the player targeting itself) lists buffs then debuffs, any other target debuffs then buffs; the second group starts a new line 3 px below (TargetFrameAuraContainer.lua:290-329).
- [x] Horizontal flow right then down: 3 px between icons and lines, a line wraps once its width plus the next icon passes 122 (AnchorUtil.lua:700-717); lines are as tall as their largest icon.
- [x] Icons are 17 px, 21 px when cast by the player (`LargeAuraSize`, TargetFrameAuraContainer.lua:3,413-425).
- [x] Up to 32 buffs and 16 debuffs (TargetFrameAuraShared.lua:3-4), sorted by `AuraUtil.DefaultAuraCompare`: the player's first, then aura instance (AuraUtil.lua:140-156).
- [x] Buffs: every helpful aura. Debuffs: the player's own, all of them on the player, and on a hostile NPC not other players' (TargetFrameAuraContainer.lua:358-411).
- [x] Debuffs draw `DispelBorder` (`UI-Debuff-Overlays` at 0.296875-0.5703125, 0-0.515625, 1 px outside the icon) tinted by dispel type (`AuraUtil.SetAuraBorderColor`; TargetFrameAuraButton.xml:57-63). The colours are Classic `DebuffTypeColor`: Retail's `DEBUFF_TYPE_*_COLOR` values are not in the Lua tree.
- [x] Stack count above 1 at BOTTOMRIGHT +1 in `NumberFontNormalSmall` (ARIALN 12 outline) (TargetFrameAuraButton.xml:14-18).
- [x] Timed auras show a reverse cooldown swipe (black at 0.64 alpha) over the elapsed part, clockwise from 12 o'clock, with the `UI-HUD-ActionBar-SecondaryCooldown` edge at its leading side, centred 1 px down. There is no duration text (`CooldownFrameTemplate` `reverse`, `drawEdge`, `hideCountdownNumbers`; TargetFrameAuraButton.xml:21-25; Cooldown.xml:3-11).
- [ ] The target-of-target's narrower first two lines (101 px, TargetFrame.lua:522-523): the Godot client has no target-of-target frame.
- [ ] `isPriorityAura` and `canApplyAura` sort keys, `noBuffDebuffFilterOnTarget`, stealable borders and pet/vehicle ownership: not replicated.
- [ ] Hover tooltips on the Godot client.

The Retail PlayerFrame has no aura icons: `Mainline/PlayerFrame.lua` only updates BuffFrame and DebuffFrame (PlayerFrame.lua:638-639,746-747).

## Live proof

`godot/tests/auras_live.gd` (2026-09-29, Godot client, private UDP 5088, game-server `e827f09`, level-10 Human mage `Fbauras` 12 yd from a Blackrock Spy) exits 0; screenshots in `data/diagnostics/auras-2026-09-29/`:
- `01`: Arcane Intellect 1459 (icon 135932) at BuffButton0, "60 m", right edge 270 UI units from the screen's right, 10 down. The server logs the mage's Intellect 38 → 39.14.
- `02`: F1 self-target: the same buff as a large (21 px) TargetFrame buff.
- `03`: Frostbolt's Chilled 205708 (icon 135846) as a large debuff with its Magic border and swipe.
- `04`: Chilled, then Polymorph 118 (icon 136071), both large debuffs in instance order.
- `05`: Chilled has expired; Polymorph's swipe grew by 6 s of 60.

`data/diagnostics/buffs-20260924/` (2026-09-24, level-10 Warrior InviteTarget): Battle Shout (6673) shows "60 m" under its icon at BuffButton0 x=1620 y=10, the hover tooltip shows it, and `ui.rightClick("BuffButton0")` removes it (`buff-proof.js`).

## Implementation inventory

- `godot/ui-model/src/game/aura_display_data.rs`: replicated views to `AuraInstance`, shared by both clients.
- `godot/ui-model/src/ui/screens/inworld_unit_frames_aura.rs`: TargetFrame aura filters, sort and flow layout.
- `godot/rust/src/auras.rs`: Godot BuffFrame, countdown, buff cancellation consumer, TargetFrame swipes.
- `godot/rust/src/tooltip_sources.rs` and `tooltips.rs`: player aura hover routing and native GameTooltip host.
- `godot/rust/src/ui/projection.rs`: aura-specific right-button-up input.
- `godot/rust/src/account.rs`: `CancelAura` transport dispatch.
- `godot/ui-model/tests/buffcancel.rs`: mounted aura hit/content and buff/debuff request behavior.
- `godot/network/src/wire_tests.rs`: exactly-one `CancelAura` delivery over loopback UDP.

- `godot/ui-model/src/ui/screens/buff_frame_component.rs`: layout, duration text, borders, flash curve.
- `src/scenes/buff_frame/mod.rs`: mounting, flash system, right-click cancel.
- `src/game/buff_data.rs`: `AuraInstance::timer_text`, hidden/passive filter.
- `src/scenes/tooltip_frame/mod.rs`: aura tooltip on hover.
