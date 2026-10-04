# Experience bar

The in-world XP HUD shows the player's progress to the next level and rested pool, with Retail gameplay in both presets. Source: `godot/ui-model/src/ui/screens/xp_bar_component.rs`; [design and sources](../wiki/systems/xp-bar.md).

## What it must do

- [ ] Modern: bottom-centred 571×17 container, 565×11 fill inset (1,1), Blizzard atlas background/frame, purple normal XP and blue rested XP.
- [ ] Forever: top-centred 1192×17 container per supplied reference; 1189×14 flat fill inset (1,1), purple (0.58,0,0.55), rested blue (0,0.39,0.88), dark track (0.15,0.15,0.15,0.9), tooltip border (0.80,0.60,0.34), edge 16, outset 4. No FlareUI media.
- [ ] Fill shows XP / next-level XP. Rested prediction ends at (XP + rested) / next-level XP, beneath earned XP; prediction hides when beyond this level. Modern pip hides at the first/last 1% and past the bar; Forever has no Blizzard dividers or pip.
- [ ] Hide at level cap (`next_level_xp == 0`, server's authoritative cap indication).
- [ ] Hover reveals XP text; leaving hides it. Both skins use Retail state.
- [ ] Existing owner-only XP messages and replicated UnitLevel populate state; absent data renders no bar. No protocol or server changes.
- [ ] Preset switch/viewport resize repositions the mounted canvas; the canvas participates in existing skin/scale mirroring. Leaving world frees it.

## How it works

- [XP HUD data and source citations](../wiki/systems/xp-bar.md)
- [HUD presets](hud-edit-mode.md)

## Implementation inventory

- `godot/ui-model/src/ui/screens/xp_bar_component.rs` — state and reactive screen.
- `godot/ui-model/src/ui/hud_layout.rs` — XP anchor only.
- `godot/rust/src/xp_bar.rs` — account/replica mapping and canvas lifecycle.
- `godot/rust/src/ui/mod.rs` — screen mount and border postsetup.
- `godot/rust/src/lib.rs` — frame-step and skin/scale traversal registration.

## Tests asserting this spec

- `godot/ui-model/tests/xp_bar.rs` — geometry, fills, rested/cap/hover and named art/palette.
- `godot/rust/src/xp_bar.rs` — state mapping tests.
- `godot/rust/src/ui/hud_layout_tests.rs` — mounted preset/skin sync.

## Known gaps (current cycle)

- [ ] Live visual/hover proof not requested; targeted CPU fixtures do not prove rendered pixels or Godot lifecycle.

## Out of scope

- Reputation/honor/other status bars, Classic rules, trial/banked XP, disabled XP toggle, gain animations, always-on XP text preference and editable individual bar settings.
