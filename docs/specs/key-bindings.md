# Key Bindings

The Retail default key bindings in the Godot client. Actions, persisted tokens and defaults are defined in `godot/core/src/input_bindings_data.rs`. Options › Keybindings lists them by section, one binding button per action.

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
- [x] Action Bars 4/5 use `MULTIACTIONBAR3BUTTON1-12` / `MULTIACTIONBAR4BUTTON1-12`, unbound by default, listed under Action Bar 4/5 and persisted like bars 2/3. Retail `~/.cache/wow-ui-sim/blizzard-ui/retail/AddOns/Blizzard_FrameXML/Bindings_Standard.xml:565-732` defines these commands without default keys; `Blizzard_ActionBar/Shared/MultiActionBars.xml:120,149` gives Left page 4 and Right page 3 (one-based slots 37–48 / 25–36, not 61–120). Bound keys and hotkey labels use the shared main-bar path in both skins.
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

### Binding buttons
- [x] Each Key Bindings row is the action's name and one button showing its key (Retail key text, unabbreviated), or gray "Not Bound" at 0.8 alpha when unbound (`NOT_BOUND`, `BindingButtonTemplate_SetupBindingButton`, `Blizzard_SharedXML/BindingUtil.lua:217-232`). The button sits where Retail's first one does: 160 wide, 80 left of the row centre (`Blizzard_Settings_Shared/Blizzard_Keybindings.xml:65-72`).
- [x] One key per action is the user's design choice ("one button displaying the keybind and often right click to unbind"), a departure from Retail's primary and secondary buttons (`Blizzard_Keybindings.xml:65-82`). The data model already holds one key per action; the options file has no second binding to load.
- [x] Left-click listens: the button reads "Press a key…" and is drawn pressed, and the page's output line reads `SETTINGS_BIND_KEY_TO_COMMAND_OR_CANCEL` with `GetBindingText("ESCAPE")`: `Assign Binding for "<action>" or Press Escape to Cancel` (`Blizzard_Keybindings.lua:425-433`, `Blizzard_SettingsPanel.lua:966`). The next key, Shift- or Ctrl-key, or mouse button binds the action and is saved; modifier keys alone keep listening. Escape stops listening and changes nothing (`KeybindListener:ProcessInput`, `Blizzard_Keybindings.lua:89-92`).
- [x] Right-click unbinds the action, saves, and clears the output line (`Blizzard_Keybindings.lua:434-440`).
- [x] A key another action holds moves: that action becomes unbound and the output line names it in red, `KEY_UNBOUND_ERROR` "Action <action> is Now Unbound!" (`KeybindListener:UnbindKey`, `Blizzard_Keybindings.lua:121-139`; `SettingsPanelMixin:OnKeybindUnbindFailed`, `Blizzard_SettingsPanel.lua:971-974`). A free key reads `KEY_BOUND` "Key Bound Successfully" (:980-982). Retail's `PRIMARY_KEY_UNBOUND_ERROR` applies only when the other action keeps a second key, which one key per action cannot have.
- Ours, not Retail's: the output line sits under the section tabs, not at the panel's bottom (`Blizzard_SettingsPanel.xml:18-22`), and another section clears it. Left and right mouse buttons can be bound while listening; Retail binds only keys, the wheel and other mouse buttons there.
- Code: `godot/ui-model/src/ui/screens/options_menu_active_sections_keybindings.rs` (rows), `options_menu_data.rs` (`listen_for_binding`, `capture_binding`, `cancel_binding_capture`, `unbind_action`), `godot/rust/src/ui/options_keybindings.rs` (right-click hit test: Godot buttons press on the left button only), `godot/rust/src/game_menu.rs`.

### Options list scrolls
- [x] Options pages scroll, as Retail's settings list does: a `WowScrollBoxList` with a `MinimalScrollBar` beside it (`Blizzard_Settings_Shared/Blizzard_SettingsList.xml:48-58`), the Key Bindings list included. The content area is as tall as the category list (`Blizzard_SettingsPanel.xml:59-74`), so the Options panel no longer grows with its page and fits a 768-unit canvas with its Done button.
- [x] A page taller than the area shows the rows that fit and a scroll bar; a page that fits has none (`ScrollUtil.AddManagedScrollBarVisibilityBehavior`, `Blizzard_SettingsList.lua:71-76`). Every Action Bar 2/3 button and pet bar row can be scrolled to and bound.
- [x] The mouse wheel over the area moves one row per notch, up to earlier rows (`ScrollControllerMixin:OnMouseWheel`, `Blizzard_SharedXML/Shared/Scroll/ScrollController.lua:93-99`), stopping at the first and last rows; dragging the thumb scrolls in proportion. While a binding listens for a key the wheel is bound, not scrolled (`KeybindListener:OnForwardMouseWheel` first, `Blizzard_SettingsPanel.lua:87-91`).
- [x] Another category or binding section starts at its top (`SetDataProvider` without `retainScrollPosition`, `Blizzard_SettingsList.lua:140`, `ScrollBox.lua:703-713`).
- [x] Shared `MinimalScrollBar` controls (Options, quest detail and Reputation, both presets): track clicks above/below the thumb page by 95% of the visible content extent, clamped at the ends (`Blizzard_SharedXML/Shared/Scroll/ScrollBar.lua:122-132`). Holding an arrow steps immediately, then repeats after 0.5s and at 0.1s intervals; leaving pauses the timer and release stops it (`ScrollBar.lua:307-340`, `ScrollBar.xml:16-17`). End arrows are disabled and retain normal art desaturated (`ScrollBar.lua:237-239`, `MinimalScrollBar.lua:3-25`, `Blizzard_SharedXMLBase/ButtonStateBehavior.lua:111-127`). Hover/pressed art is excluded.
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
- `godot/rust/src/ui/options_keybindings_tests.rs`: left-click then Q binds Action Bar 2 Button 1 and the button reads "Q"; right-click at a button unbinds that action and a click on the label does nothing; Escape cancels listening with the bindings unchanged; S on Move Forward moves it from Move Backward, which reads "Not Bound", with "Action Move Backward is Now Unbound!"; the page's edits survive saving and reloading the options file.
- `godot/rust/src/ui/scroll_lists_tests.rs`: Action Bar 2's last button scrolls into the area by wheel and its first out, the wheel stops at both ends; dragging the thumb to the bottom shows the last row; another page starts at its top; Sound has no scroll bar; on every page and section the shown rows stay inside the area at every position and the panel fits the canvas.
- `godot/core/tests/input_bindings_data.rs`: the inventory, sections and defaults, the token grammar (including `NumpadAdd`/`NumpadSubtract`), Shift shadowing, Action Bar 2/3 actions unbound and persisted through the options file, and Retail hotkey text.
- `godot/ui-model/tests/forever_action_bars.rs` (`extra_bar_bindings_press_their_slot_and_label_their_button`): a key on `MULTIACTIONBAR1BUTTON3` presses slot 63 and labels `MultiBarBottomLeftButton3`; Shift-1 and middle mouse on Action Bar 3; unbound buttons blank; 0 still presses main bar button 10 under Forever.
- `godot/tests/world_keybinds_flow.gd` (`native_input_fixture keybinds`, owned UDP) presses real keys:
  - B, Shift-B and F8–F11 against bags 0/1/2/4;
  - Tab, Shift-Tab and F against the vendor, wolf and remote player;
  - Num Pad +/− against minimap zoom 0→1→2→1→0.
