# Key Bindings

The Retail default key bindings in the Godot client. Actions, persisted tokens and defaults are defined in `godot/core/src/input_bindings_data.rs`. Options › Keybindings lists them by section.

Sources:
- Binding actions: `~/.cache/wow-ui-sim/blizzard-ui/retail/AddOns/Blizzard_FrameXML/Bindings_Standard.xml`.
- Default keys: the client's built-in set, which the UI source does not ship. The reference is `~/Repos/worldofwhatever/DefaultBindings.wtf`, a non-Blizzard capture of the default `bind KEY ACTION` set.

## What it must do

### Bound in the native client
- [x] Movement: W/S forward/back, A/D strafe, Space jump, X sit/descend, NumLock autorun, Z run toggle, arrows turn and pitch.
- [x] Camera: Page Up/Down zoom.
- [x] Targeting:
  - Tab `TARGETNEARESTENEMY`, Shift-Tab `TARGETPREVIOUSENEMY` (`TargetNearestEnemy(true)`, :998);
  - F `ASSISTTARGET` (`AssistUnit("target")`, :1174);
  - F1 `TARGETSELF`.
- [x] Action bar: 1–0, −, = for `ACTIONBUTTON1-12`.
- [x] Action Bar 2 and 3: `MULTIACTIONBAR1BUTTON1-12` ("Action Bar 2 Button n", `MultiActionButtonDown("MultiBarBottomLeft", n)`) and `MULTIACTIONBAR2BUTTON1-12` ("Action Bar 3 Button n", `MultiBarBottomRight`) (:397-563), each in its own section, Retail's `BINDING_HEADER_ACTIONBAR2`/`3` "Action Bar 2"/"Action Bar 3". Unbound by default: the bindings XML carries no keys. A bound key uses that bar's button through the main bar's path (slots 61-72 and 49-60), whether or not the preset shows the button. Persisted in the options file with the other bindings; files saved before these actions load them unbound.
- [x] Button hotkey text is `GetBindingText(key, 1)` (`Shared/ActionButton.lua:488-495`), on every bar from the live bindings; an unbound button shows none. Modifiers abbreviate to `s-`/`c-` (`SHIFT_KEY_TEXT_ABBR`, `CTRL_KEY_TEXT_ABBR`); the key keeps its `KEY_` text because Retail defines `KEY_ABBR_*` only for gamepad buttons (`Blizzard_SharedXML/SharedConstants.lua:56-90`): Space "Spacebar", Backspace "Backspace", middle mouse "Middle Mouse", mouse 4/5 "Mouse Button 4/5". Addon abbreviations such as "Sp", "BS", "B3" or "S" for Shift are not Retail. The options list shows the same names unabbreviated.
- [x] Pet bar: Ctrl-1..Ctrl-0 for `BONUSACTIONBUTTON1-10` (DefaultBindings.wtf:53-62, `BINDING_HEADER_ACTIONBAR`), listed in the Action Bar section; they press pet bar buttons only while it is shown, and Ctrl-N never presses `ACTIONBUTTONn` (Ctrl shadowing). Hotkeys show `c-1`..`c-0` (`CTRL_KEY_TEXT_ABBR`).
- [x] Frames:
  - C character, P spellbook, L quest log, M world map;
  - Ctrl-R `TOGGLEFPS`;
  - Escape game menu and Enter, `/` and R chat. These five keys are fixed, not rebindable.
- [x] Bags (:1203-1223):
  - B `OPENALLBAGS`;
  - Shift-B `TOGGLEBACKPACK`;
  - F8–F11 `TOGGLEBAG1-4`, which call `ToggleBag(4..1)`.
- [x] Minimap: Num Pad + `MINIMAPZOOMIN` and Num Pad − `MINIMAPZOOMOUT` (:1378-1383).

### Options list scrolls
- [x] Options pages scroll, as Retail's settings list does: a `WowScrollBoxList` with a `MinimalScrollBar` beside it (`Blizzard_Settings_Shared/Blizzard_SettingsList.xml:48-58`), the Key Bindings list included. The content area is as tall as the category list (`Blizzard_SettingsPanel.xml:59-74`), so the Options panel no longer grows with its page and fits a 768-unit canvas with its Done button.
- [x] A page taller than the area shows the rows that fit and a scroll bar; a page that fits has none (`ScrollUtil.AddManagedScrollBarVisibilityBehavior`, `Blizzard_SettingsList.lua:71-76`). Every Action Bar 2/3 button and pet bar row can be scrolled to and bound.
- [x] The mouse wheel over the area moves one row per notch, up to earlier rows (`ScrollControllerMixin:OnMouseWheel`, `Blizzard_SharedXML/Shared/Scroll/ScrollController.lua:93-99`), stopping at the first and last rows; dragging the thumb scrolls in proportion. While a binding listens for a key the wheel is bound, not scrolled (`KeybindListener:OnForwardMouseWheel` first, `Blizzard_SettingsPanel.lua:87-91`).
- [x] Another category or binding section starts at its top (`SetDataProvider` without `retainScrollPosition`, `Blizzard_SettingsList.lua:140`, `ScrollBox.lua:703-713`).
- Ours, not Retail's: rows snap (the ui-toolkit scroll list is row-snapped; Retail scrolls by pixels). The track and thumb are the toolkit list's plain colours, not the `MinimalScrollBar` art; it has no step arrows.
- Code: `godot/ui-model/src/ui/screens/options_menu_scroll.rs` (the area as a ui-toolkit scroll list, position in `FrameRegistry::scroll_lists`), `godot/rust/src/ui/scroll_lists.rs` (wheel and thumb input), `RegistryModel::reset_options_scroll_for` (`godot/rust/src/ui/mod.rs`).

### Blocked: no native frame
These defaults are in the table, or Retail has them, but the Godot client has no frame for them yet:
- `TOGGLETALENTS` N, `TOGGLEACHIEVEMENT` Y, `TOGGLESTATISTICS` Shift-Y, `TOGGLEPROFESSIONBOOK` K.
- `TOGGLESOCIAL` O, `TOGGLEGUILDTAB` J, `TOGGLEENCOUNTERJOURNAL` Shift-J.
- `TOGGLEGROUPFINDER` I, `TOGGLEDUNGEONSANDRAIDS` Ctrl-I.
- `TOGGLECOLLECTIONS` Shift-P, `TOGGLEPETBOOK` Shift-I.
- `TOGGLECHARACTER2` U (reputation), `TOGGLECHARACTER4` (pet tab).
- `TOGGLEBATTLEFIELDMINIMAP` Shift-M, `TOGGLECHANNELPULLOUT` Shift-O, `TOGGLEWORLDSTATESCORES` Shift-Space, `TOGGLEGEARSETS` Shift-G.

### Blocked: the native system lacks what the action needs
- `TARGETPARTYMEMBER1-4` F2–F5 and `TARGETPARTYPET1-4`: native group frames have no unit-targeting path.
- `TARGETNEARESTFRIEND` Ctrl-Tab and `TARGETPREVIOUSFRIEND`: native Tab targeting does not check reaction.
- `TARGETPET` Shift-F1, `PETATTACK` Shift-T: not implemented.
- `TARGETLASTHOSTILE` G: no last-hostile history.
- `NAMEPLATES` V, `FRIENDNAMEPLATES` Shift-V, `ALLNAMEPLATES` Ctrl-V: these toggle `nameplateShowEnemies` and `nameplateShowFriendlyPlayers` (:1121-1160). Native options have only one `show_nameplates` flag.
- `TOGGLEUI` Alt-Z: the binding grammar has no Alt modifier, and there is no single native UIParent root.
- `ACTIONPAGE1-6` Shift-1..6, `NEXTACTIONPAGE` and `PREVIOUSACTIONPAGE`: there is one action bar page.
- `SHAPESHIFTBUTTON1-10` Ctrl-F1..F10: no stance bar.
- `TOGGLESOUND` Ctrl-S: it is in the table but only Bevy handles it. It also mutes everything, while Retail's `Sound_ToggleSound` toggles SFX, ambience and dialog. `TOGGLEMUSIC` Ctrl-M and `MASTERVOLUMEUP`/`MASTERVOLUMEDOWN` Ctrl-=/− are absent.
- `SCREENSHOT` PrintScreen, `TOGGLESHEATH` Z, `TOGGLERUN` Num Pad /, `CHATPAGEUP`/`CHATPAGEDOWN`/`CHATBOTTOM`, `REPLY2` Shift-R, and the vehicle keys are not implemented.

### Defaults that differ from Retail
These are not changed here:
- A/D turn and Q/E strafe in Retail. The table has A/D strafe.
- Insert/Delete pitch in Retail. The table uses Up/Down, which Retail binds to forward/back.
- The mouse wheel zooms in Retail. The table zooms with Page Up/Down, which Retail binds to chat paging.
- Z is `TOGGLESHEATH` in Retail. The table makes Z run toggle, which Retail puts on Num Pad /.
- J is `TOGGLEGUILDTAB` in Retail, with the Adventure Guide on Shift-J. The table puts the Adventure Guide on J.

## Tests
- `godot/rust/src/ui/scroll_lists_tests.rs`: Action Bar 2's last button scrolls into the area by wheel and its first out, the wheel stops at both ends; dragging the thumb to the bottom shows the last row; another page starts at its top; Sound has no scroll bar; on every page and section the shown rows stay inside the area at every position and the panel fits the canvas.
- `godot/core/tests/input_bindings_data.rs`: the inventory, sections and defaults, the token grammar (including `NumpadAdd`/`NumpadSubtract`), Shift shadowing, Action Bar 2/3 actions unbound and persisted through the options file, and Retail hotkey text.
- `godot/ui-model/tests/forever_action_bars.rs` (`extra_bar_bindings_press_their_slot_and_label_their_button`): a key on `MULTIACTIONBAR1BUTTON3` presses slot 63 and labels `MultiBarBottomLeftButton3`; Shift-1 and middle mouse on Action Bar 3; unbound buttons blank; 0 still presses main bar button 10 under Forever.
- `godot/tests/world_keybinds_flow.gd` (`native_input_fixture keybinds`, owned UDP) presses real keys:
  - B, Shift-B and F8–F11 against bags 0/1/2/4;
  - Tab, Shift-Tab and F against the vendor, wolf and remote player;
  - Num Pad +/− against minimap zoom 0→1→2→1→0.
