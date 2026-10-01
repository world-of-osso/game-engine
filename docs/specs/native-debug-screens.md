# Native debug screens

Offline `--screen` debug destinations of the Godot client, ported from the Bevy debug scenes (`src/scenes/*_debug`). None contacts a server; load failures stop startup with an error. `particledebug` is specified in [Godot conversion](godot-conversion.md).

## What it must do

### m2debug (`src/scenes/m2_debug/mod.rs`)

- [x] Show wolf reference model 126487 with its preferred creature display skins (126494, 126495), yawed −90°, on a 100-yd grass plane (187126 tiled 20×) under a shadowed directional light rotated XYZ (−0.9, −0.6, 0) rad, clear colour (0.05, 0.06, 0.08).
- [x] Frame it with a 45° orbit camera 6 yd from (0, 1, 0); left drag orbits, the wheel zooms; the model animates.

### selectiondebug (`src/scenes/selection_debug/mod.rs`)

- [x] Mount the original `selection_debug_screen` with the original five candidates.
- [x] Row click selects; Up/Left and Down/Right cycle ("Focused …"); Enter/Space toggle pinned mode; Escape or Back returns to Login.

### debugcharacter (`src/scenes/geoset_debug/mod.rs`)

- [x] Show two copies of the configured character at x = −1.7 and +1.7, yawed −90°: left with the geoset-heavy displays (head 1128, hands 510, waist 109162, legs 159629), right with the runtime-model displays (head 685129, hands 154616, waist 160997, legs 73783); shared shoulder 148865, back 181925, chest 175942, feet 154620.
- [x] `DEBUG_CHARACTER_*` variables override race, class, sex, appearance and every display; 0 empties a slot; an unparsable value is an error.
- [x] 30-yd grass plane tiled 6×, warm shadowed light, orbit camera from (0, 1.8, 6) toward (0, 1, 0), zoom 1.5–12 yd.

### Not converted

- [ ] `skyboxdebug`, `inworldselectiondebug`, `nameplatedebug` fail explicitly as unconverted.

## Tests asserting this spec

- `godot/network/examples/native_debug_screen_fixture.rs` — `native_debug_screen_fixture <screen>` launches the real client with `--screen <screen>` under `godot/tests/<screen>_screen_flow.gd`, runs the public `game-engine-cli` (`ping`, `--json dump-scene`, `dump-tree`, `dump-ui-tree`, `screenshot`) over the own-PID socket, and requires a normal exit with the socket removed.
- `godot/tests/{m2debug,selectiondebug,debugcharacter}_screen_flow.gd` — live node/input assertions, CLI reply contents, and CLI WebP pixels matching the live frame's model/screen pixels.
- `godot/rust/src/selection_debug_tests.rs`, `debug_character_tests.rs` — candidate model, key bindings, configuration defaults/overrides/errors.

Run: build with `python3 scripts/depot-build.py --root "$PWD" --fixture native_debug_screen_fixture`, then `GODOT_BIN=… GAME_ENGINE_CLI=… target/debug/examples/native_debug_screen_fixture <screen>` inside a headless compositor.

## Implementation inventory

- `godot/rust/src/m2_debug.rs` — m2debug scene and the shared debug environment, light, ground and orbit-input helpers.
- `godot/rust/src/selection_debug.rs` — selectiondebug model, keys and standalone registry UI.
- `godot/rust/src/debug_character.rs` — debugcharacter configuration and scene.
- `godot/rust/src/ui/mod.rs` — `RegistryUi::show_standalone_screen`.
- `godot/rust/src/startup.rs` — `--screen` routing.
