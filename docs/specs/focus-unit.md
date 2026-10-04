# Focus Unit

Retail's focus: a client-local unit set from the unit menu and shown in FocusFrame. No server state. Code: `godot/rust/src/unit_menu.rs` (menu entries and actions), `godot/rust/src/targeting.rs` (`Targeting::focus`, `focus_frame_state`), `godot/ui-model/src/ui/screens/inworld_unit_frames_component.rs` (`focus_menu_item`, `SmallFrameSpec::FOCUS`), `inworld_unit_frames_flare.rs` (`FLARE_FOCUS`).

References:
- `Blizzard_UnitPopupShared/UnitPopupSharedButtonMixins.lua:2169-2186` (`FocusUnit(unit)`, `ClearFocus()`), `UnitPopupSharedMenus.lua:316,346-352` (TARGET lists Set Focus; FOCUS lists Clear Focus)
- `Blizzard_UnitFrame/Mainline/TargetFrame.lua:239-247` (`PLAYER_FOCUS_CHANGED`, shown while `UnitExists("focus")`), `:1131-1138` (`FocusFrame_OpenMenu`), `:1148-1176` (`SetSmallSize` keeps level and mana bar)
- FlareUI 1.3 `Core.lua:295` (focus 160×36, `powerHeight = 0`, level shown)
- `Bindings_Standard.xml:1186-1191` (`FOCUSTARGET`, `TARGETFOCUS`, no default key)

## What it must do

- [x] The focus is client-local and cleared on leaving the world.
- [x] TargetFrame menu leads with Set Focus: it makes the target the focus (NPC or player).
- [x] Right-clicking FocusFrame opens the menu for the focus unit with Clear Focus (and the raid target icons); Clear Focus clears the focus. Set Focus is not offered there, Clear Focus nowhere else.
- [x] FocusFrame shows the focus unit's name, level and health, updated live, while the unit is replicated; hidden with no focus or when the unit leaves replication (it returns if the unit does).
- [x] Modern: portrait-off art at 0.75 with level text and mana bar. Forever: FlareUI frame, level shown, no power bar (FlareUI `powerHeight = 0`).
- [ ] `FOCUSTARGET` / `TARGETFOCUS` bindings: not in `input_bindings_data` (Retail has no default key either).
- [ ] Focus auras, cast bar, target-of-focus and `/focus` are not converted.

## Proof

- `godot/rust/src/focus_frame_tests.rs`: name, level, health and power of the focus; health change propagates; despawn (`Replica::clear`) hides it.
- `godot/rust/src/unit_menu.rs` tests: TargetFrame menu offers Set Focus, FocusFrame menu Clear Focus; Set/Clear change the focus.
- `godot/ui-model/tests/focus_frame.rs`: both presets show FocusFrame with name, level and health fill, hidden without a focus; Modern shows the mana bar.
