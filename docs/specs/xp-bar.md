# Experience bar

The in-world XP HUD shows the player's progress to the next level and the rested pool, with Retail behaviour in both presets. Source: `godot/ui-model/src/ui/screens/xp_bar_component.rs`; [design and sources](../wiki/systems/xp-bar.md).

## What it must do

- [x] Modern: 571×17 container at the bottom centre, 565×11 bar inset (1,1); Blizzard atlases for background, fill, rested overlay, frame and pip.
- [x] Forever: 596×17 container at the top centre, 6 below the screen edge, matching the supplied reference; 593×14 flat bar inset (1,1): XP purple (0.58,0,0.55), rested blue (0,0.39,0.88), rested overlay the same blue at alpha 0.4, dark track (0.15,0.15,0.15,0.9), `UI-Tooltip-Border` edge 16 tinted (0.80,0.60,0.34) 4 outside the bar. No frame art, no pip, no FlareUI media.
- [x] Fill width = XP / next-level XP of the bar. With a rested pool the fill is the rested art/colour.
- [x] Rested overlay runs from the bar's left to (XP + rested) / next-level XP, under the fill; hidden without a pool or when the pool ends past the bar. Modern's pip is centred on that end, 2 above the bar's centre, hidden within 1% of either edge.
- [x] Hidden at the level cap (`next_level_xp == 0`) and absent before the first `PlayerXpUpdate` and outside the world.
- [x] Hovering the bar shows "XP: <xp>/<next>" (`XP_STATUS_BAR_TEXT`), drawn over fill and border; leaving hides it.
- [x] The canvas mirrors the active skin and UI scale like every HUD canvas: a preset switch moves and reskins the mounted bar.
- [ ] Live rendered proof (pixels, hover through Godot input) not captured.

## How it works

- [XP HUD data and source citations](../wiki/systems/xp-bar.md)
- [HUD presets](hud-edit-mode.md)

## Implementation inventory

- `godot/ui-model/src/ui/screens/xp_bar_component.rs` — state and screen.
- `godot/ui-model/src/ui/hud_layout.rs` — `xp_bar` anchor per preset.
- `godot/rust/src/xp_bar.rs` — `PlayerXpUpdate` to state, hover, canvas lifetime.
- `godot/rust/src/ui/mod.rs` — `show_xp_bar` mount.
- `godot/rust/src/lib.rs` — frame step and canvas traversal.

## Tests asserting this spec

- `godot/ui-model/tests/xp_bar.rs` — geometry, fill, rested overlay and pip, cap, hover text, atlas names, Forever colours and border.
- `godot/rust/src/xp_bar.rs` — state mapping.
- `godot/rust/src/ui/hud_layout_tests.rs` — mounted rect per preset through the skin mirror.

## Out of scope

- Reputation, honor and the other status bars; the second container; trial/banked XP; the disabled-XP toggle; gain and level-up animations; the `xpBarText` always-on option; the exhaustion tooltip; Edit Mode moving or resizing of the bar; the overlay's texcoord crop.
