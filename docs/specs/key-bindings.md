# Key Bindings

The Retail default key bindings in the Godot client. Actions, persisted tokens and defaults are defined in `src/input_bindings_data.rs`. Options › Keybindings lists them by section.

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
- `godot/core/tests/input_bindings_data.rs`: the inventory, sections and defaults, the token grammar (including `NumpadAdd`/`NumpadSubtract`), and Shift shadowing.
- `godot/tests/world_keybinds_flow.gd` (`native_input_fixture keybinds`, owned UDP) presses real keys:
  - B, Shift-B and F8–F11 against bags 0/1/2/4;
  - Tab, Shift-Tab and F against the vendor, wolf and remote player;
  - Num Pad +/− against minimap zoom 0→1→2→1→0.
