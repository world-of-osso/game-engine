# Status text

This spec defines the Retail Status Text on unit frame health and power bars. Citations are to `~/.cache/wow-ui-sim/blizzard-ui/retail/AddOns/` (`Blizzard_SettingsDefinitions_Frame`, `Blizzard_TextStatusBar`, `Blizzard_UnitFrame/Mainline`).

## What it must do

- [x] Settings → Interface "Status Text" offers Numeric Value, Percentage, Both and None (Interface.lua:57-105). The choice is saved with the client options as `statusTextDisplay` (`NUMERIC`, `PERCENT`, `BOTH`, `NONE`); the default is `NONE` (wow-ui-sim `cvars.yaml:1413`, `defaultValue = 4`).
- [x] Every choice except None keeps the bar text shown (`SetValue` writes `statusText` 1, Interface.lua:77-90; TextStatusBar.lua:115). With None, a bar shows its text only while the pointer is over it, and shows it as Numeric (`OnStatusBarEnter` adds `lockShow`; NONE takes the numeric branch, TextStatusBar.lua:117-118,170,217-220).
- [x] Numeric shows `value / max` with `AbbreviateLargeNumbers`, because unit frame bars set `capNumericDisplay` (UnitFrame.lua:58-64): over 8 digits drop 6 and add " M", over 5 drop 3 and add " K", over 3 group with "," (12,345; 123 K; 1234 K; 123 M). Retail implements this natively; the rule is its last published Lua (wow-ui-source classic UIParent.lua:774-785).
- [x] Percentage shows `math.ceil(value / max * 100)%` in double arithmetic (12345 / 15000 is 83%, 7 / 100 is 8%) (TextStatusBar.lua:196-197).
- [x] Both hides the centred text and shows the percentage in `LeftText` and the current value alone in `RightText`. A power bar shows the percentage only for Mana (TextStatusBar.lua:177-187).
- [x] A bar with no maximum shows no text. TargetFrame bars show "" at 0 (`zeroText`, TargetFrame.lua:760-769).
- [x] PlayerFrame, TargetFrame and PetFrame health and power bars use their Retail anchors: player 0/2/-2 (PlayerFrame.xml:200-214,260-274), target health 0/2/-5 and mana -4/2/-13 (TargetFrame.xml:167-181,220-234), pet health 0/0/0 and mana 2/4/0 (PetFrame.xml:102-116,141-155). The TargetFrame shows the target's power bar.
- [ ] FocusFrame and boss frames: the client never fills them, so their bars have no values to show.
- [ ] Party frames: the client's party frames are raid-style `CompactUnitFrame`s, whose text follows `raidFramesHealthText`, not this setting (CompactUnitFrame.lua:1089-1120).
- [ ] Opening the Character frame forcing player and pet text (CharacterFrame.lua:231-237).

## Proof

- `godot/core/tests/status_text.rs` (abbreviation, percentage, Both, None hover, zero text, saved option).
- `godot/ui-model/tests/status_text_bars.rs`, `options_policy.rs`, `options_views.rs` (bar font strings, Interface row).
- Live `godot/tests/status_text_live.gd` on a private server: None hover, then Percentage and Both chosen through Options on PlayerFrame and the self-targeted TargetFrame.
