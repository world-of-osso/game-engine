# XP HUD

One screen for the Retail experience bar: Blizzard atlases under Modern, FlareUI's flat reskin under Forever. [Contract](../../specs/xp-bar.md).

## Data

`shared-protocol/src/protocol/experience_messages.rs:17-21` carries the owner-only `PlayerXpUpdate { xp, next_level_xp, rested_xp }`; `next_level_xp` 0 is the level cap (`:14-15`). `godot/network/src/lib.rs:231` receives it, `godot/rust/src/account.rs:125,1079-1081` keeps the newest and `:329` clears it with the session. That is everything the bar draws; the player's level is not needed. Nothing is missing from the protocol.

`godot/rust/src/xp_bar.rs` mounts a dedicated `RegistryUi` while in world with an update present, in `for_each_registry_ui` so it follows skin and UI scale. Hover is `hovered_ui_frame()` landing on this canvas (the root is mouse-enabled).

## Sources

Retail tree: `~/.cache/wow-ui-sim/blizzard-ui/retail/AddOns`. Forever client tree: `~/.cache/wow-ui-sim/blizzard-ui/wowforever/AddOns`.

- `Blizzard_ActionBar/Mainline/StatusTrackingBar.xml:4,7,35-48`: 571×17 container, `UI-HUD-ExperienceBar-Frame`, BOTTOM of the manager.
- `Blizzard_ActionBar/Mainline/StatusTrackingManagerOverrides.lua:43-51`: bar = container − 6 at BOTTOMLEFT (1,5), so 565×11 with its top-left at (1,1). `:30-31`: experience bar visibility.
- `Blizzard_EditMode/Mainline/EditModePresetLayouts.lua:582-594`: status bar 1 at BOTTOM, offset 0.
- `Blizzard_ActionBar/Mainline/StatusTrackingBarTemplate.xml:20,37-41`: background atlas; hover text centred 1 up.
- `Blizzard_ActionBar/Mainline/ExpBarOverrides.lua:2,6,10-18`: `Fill-Experience` / `Fill-Rested`; `XP_STATUS_BAR_TEXT` ("XP: %d/%d", `data/GlobalStrings.csv:16444`).
- `Blizzard_ActionBar/Mainline/ExpBar.xml:11-16,20-36`: `Fill-Prediction` overlay, 10×14 pip `Frame-Pip`, `yOffset` 2, `hideAtBarEdge`.
- `Blizzard_ActionBar/Shared/ExpBar.lua:72-75,90-93,135-157`: hover text, overlay end, overflow and edge hiding.
- `data/reference/flareui/Modules/XPBar.lua:28-43`: flat fill, border edge 16 / outset 4 / (0.80,0.60,0.34), track (0.15,0.15,0.15,0.9), rested alpha 0.4, XP (0.58,0,0.55), rested (0,0.39,0.88). `:84-90`: Friz Quadrata 12 OUTLINE. `:120-129,141-150`: recoloured overlay, hidden frame art and dividers, border around the fill. FlareUI does not move the bar.
- Forever size: `wowforever/.../Blizzard_StatusTrackingBar/Camelot/StatusTrackingBarConstants.lua:1-3,12` — container 1192×17, adjustment 3, gamepad width 596; `Mainline/StatusTrackingManagerOverrides.lua:46-53` — bar = container − 3 at BOTTOMLEFT (1,2).

## Forever geometry from the reference

`data/diagnostics/forever-reference/user-flareui-hud-2026-10-03.png` (2000 px wide): the unit frames' centres are 620 px apart for 660 units, 0.939 px per unit. The bar's track spans x 721–1277 = 557 px = 593 units, the gamepad container's bar (596 − 3), not the 1189 of the keyboard container. The border begins about 3 px and the track about 7 px below the top edge, so the container sits 6 units down (bar at 7, border at 3). This placement is the reference user's Edit Mode position, not a client default.

## Decisions

- Rested state is `rested_xp > 0`; the client receives no separate rest state.
- The pip is Modern only: the Forever skin has no art for it and the reference shows none.
- The overlay hides when the pool overflows the level, as Retail does; the Forever client's `ShouldRestedXpBarDisplayWhenOverflowing` is not followed.
- Atlas fills are scaled to their width, not cropped.

## See Also

- [HUD presets](../../specs/hud-edit-mode.md)
- [[ui-system]] — RegistryUi and SharedContext
