# Keybindings

The keybinding system covers rebindable in-world actions. UI navigation outside those actions, shell, and debug controls stay fixed. This split is intentional.

## Bindable Actions

Persisted with client options; configurable via Options → Keybindings.

**Movement**: forward, backward, strafe left, strafe right, jump, run toggle, autorun

**Camera**: turn left/right, pitch up/down, zoom in/out

**Targeting**: target nearest

**Action bars**: main buttons 1–12 plus Action Bars 2–5 (`MULTIACTIONBAR1..4BUTTON1..12`), unbound by default on extras. Current native contract: [key bindings](../../specs/key-bindings.md); assignment/input flow: [[spellbook-action-bar]].

**Audio**: toggle mute (Ctrl+S)

**Interface**: existing actions include character C, professions K, achievements Y, talents N, adventure guide Shift-J, Group Finder I, loot rules L and world map M. VERIFIED, CATALOG: the Retail audit adds Reputation U, Statistics Shift-Y, Guild J, Dungeons and Raids Ctrl-I, and World State Scores Shift-Space; Currency, cooldown settings, Housing Dashboard, map size, minimap, PvP, mount journal, pet journal and Toy Box stay unbound because no verified default was available.

VERIFIED, CODE: new candidate defaults use only keys already recorded in this spec; other new actions stay unbound. Actions fire in World input mode without a focused edit box or game menu; see `src/ui_input_mode.rs`.

## Retail Interface audit

VERIFIED, SOURCE: the audit covers 26 action IDs in the local Retail `Bindings_Standard.xml`. VERIFIED, CODE: 15 IDs use existing native behavior: `TOGGLEREAGENTBAG1`, `TOGGLECHARACTER2`, `TOGGLECURRENCY`, `TOGGLECOOLDOWNVIEWERSETTINGS`, `TOGGLESTATISTICS`, `TOGGLEHOUSINGDASHBOARD`, `TOGGLEWORLDMAPSIZE`, `TOGGLEMINIMAP`, `TOGGLEWORLDSTATESCORES`, `TOGGLEGUILDTAB`, `TOGGLEDUNGEONSANDRAIDS`, `TOGGLECHARACTER4`, and the Mount Journal, Pet Journal, and Toy Box actions. VERIFIED, TEST CODE: the catalog test asserts the new rows in Retail source order.

VERIFIED, PRIOR CAPTURE: the five defaults already recorded in the spec are U (Reputation), Shift-Y (Statistics), Shift-Space (World State Scores), J (Guild), and Ctrl-I (Dungeons and Raids); the earlier spec cites `~/Repos/worldofwhatever/DefaultBindings.wtf` as their source. VERIFIED, CODE: the other 10 supported candidates are unbound. VERIFIED, SOURCE: the binding XML gives no default keys, and account cache files do not establish defaults.

VERIFIED, CODE: 11 candidates stay out of the catalog: `TOGGLEPETBOOK` (no Pet category in Player Spells), `TOGGLEBATTLEFIELDMINIMAP` (no native Battlefield Map), `TOGGLEFRIENDSTAB`, `TOGGLEQUICKJOINTAB`, `TOGGLEWHOTAB`, `TOGGLECHATTAB`, and `TOGGLERAIDTAB` (no matching native tabs), `TOGGLETEXTTOSPEECH` (no native panel), `TOGGLECOLLECTIONSHEIRLOOM` (no Heirlooms journal), `TOGGLECOLLECTIONSWARDROBE` (the native Transmog service is not the Collections Journal's Appearances tab), and `TOGGLEGARRISONLANDINGPAGE` (no native landing page).

INFERRED, BEHAVIOR: `TOGGLEWORLDMAPSIZE` toggles the existing normal/maximized map state; Retail's handler is a TODO, so the native mapping uses the action name and available map state.

## Fixed Inputs (Intentional)

| Input | Reason |
|-------|--------|
| `LMB + RMB` move-forward chord | Multi-button chord, not a single bindable action |
| Login screen keys | Screen-local text/focus/submit behavior |
| Char select navigation | Fixed to screen flow |
| Menu/options overlay | Navigation and modal dismissal must stay stable even if gameplay bindings break |
| Action-bar edit/debug controls | Editor affordances outside the player-facing binding set |
| In-world `Escape` | Single chain: clear focus → cursor/spell cancel (hook) → top popup (hook) → close all panels → clear target → game menu |
| `F9` World Builder toggle | Fixed diagnostic UI control; active only when launched with `--world-builder` |

## Non-Goals of the Current System

The current implementation explicitly does not solve:

- Full client-wide rebinding
- Bindable menu/login/char-select UI navigation
- Bindable debug/editor controls
- Multi-input chords as first-class bindable actions

If scope expands, the source document should be updated before implementation so future fixed inputs remain clearly intentional.

## Sources

- [key-bindings spec](../../specs/key-bindings.md) — Retail action names, keys and behavior contract
- `godot/core/src/input_bindings_data.rs` — persisted actions, sections and default keys
- `godot/ui-model/src/player_spells_data.rs`, `godot/ui-collections/src/lib.rs`, `godot/ui-collections/src/transmog_frame.rs` — spellbook categories and implemented collection/transmog panels
- `godot/rust/src/follower_dungeons.rs`, `character_frame.rs`, `achievements.rs`, `battleground.rs`, `guild_ranks.rs`, `housing.rs`, `minimap.rs`, `mount_journal.rs`, `spells/cooldown_viewer.rs`, `world_map.rs`, `bags.rs` — native dispatch for supported actions
- `godot/tests/world_keybinds_flow.gd` — Interface row and physical input behavior
- [keybindings-scope.md](../../keybindings-scope.md) — scope definition and rationale

## See Also

- [[ui-addon-system]] — menu/overlay inputs that remain fixed live in the UI layer
- [[world-builder]] — opt-in diagnostic sidebar controlled by F9
