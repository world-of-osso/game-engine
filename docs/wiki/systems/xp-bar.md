# XP HUD

One reactive screen for Retail experience state, with Modern Blizzard atlases and Forever's FlareUI numeric palette. [Contract](../../specs/xp-bar.md).

## Data boundary

`shared-protocol/src/protocol/experience_messages.rs:17-21` carries `PlayerXpUpdate { xp, next_level_xp, rested_xp }`; zero next-level XP means level cap (`:14-15`). `godot/network/src/lib.rs:231` receives it and `godot/rust/src/account.rs:125,1079-1081` retains it. The local replica's `shared::components::UnitLevel` supplies level. No fields are missing and no server/protocol change is needed.

The dedicated RegistryUi participates in GameClient's existing skin/scale traversal. Its Screen reads the canvas's ActiveSkin and hud_layout; missing XP/level or leaving InWorld removes the canvas. Hover state comes from the projected button's registry input. The Forever border reuses the registered Blizzard tooltip-border sheet, with its centre transparent and its XP-specific bronze tint applied after each sync.

## Sources

Paths below refer to local source trees, not downloaded art.

- `~/.cache/wow-ui-sim/blizzard-ui/retail/AddOns/Blizzard_ActionBar/Mainline/StatusTrackingBar.xml:3-7,36-46`: 571×17 container, named frame atlas, bottom-centre manager/main container.
- Same tree `Mainline/StatusTrackingManagerOverrides.lua:42-51`: bar = container minus 6, BOTTOMLEFT (1,5), therefore fill top-left (1,1), 565×11. `:30-31`: Retail experience visibility.
- `Blizzard_EditMode/Mainline/EditModePresetLayouts.lua:582-594` and `Standard/EditModePresetLayoutConstants.lua:29`: Modern bar 1 bottom offset 0.
- `Blizzard_ActionBar/Mainline/StatusTrackingBarTemplate.xml:20,37-41`: background atlas and centred hover text, 1 unit up.
- `Blizzard_ActionBar/Mainline/ExpBarOverrides.lua:2-18`: unrested Experience / rested Rested fill atlases; `XP_STATUS_BAR_TEXT` formatting. The previous XP spec records the build's English string `XP: %d/%d`, FRIZQT 10 outline.
- `Blizzard_ActionBar/Mainline/ExpBar.xml:11-16,20-36`: prediction atlas, 10×14 pip and yOffset 2. `Shared/ExpBar.lua:72-75,91-93,135-157`: hover show/hide, prediction endpoint and overflow hiding, pip edge exclusions. Earned XP draws over prediction.
- `/home/osso-test/.worktrees/foreverplan.md`: XP row records 1192×17. The supplied `data/diagnostics/forever-reference/user-flareui-hud-2026-10-03.png` places the thin XP bar at top centre; this placement intentionally overrides FlareUI's unchanged Edit Mode anchor. Width comes from the plan, not screenshot pixel measurement.
- `data/reference/flareui/Modules/XPBar.lua:28-43`: flat fill, 16 edge/outset 4, bronze (0.80,0.60,0.34), dark track (0.15,0.15,0.15,0.9), rested alpha 0.4, fill insets 1/2/1/2, XP (0.58,0,0.55), rested (0,0.39,0.88). `:128-140,145-162`: recoloured prediction, hidden dividers/frame and tooltip border. Lua floats are authoritative; its #94008C/#0063E0/#CC9957 comments are rounded labels. No FlareUI Media is used.

## Scope and proof

CPU tests cover concrete state, authored geometry/palette/atlas names and skin mirror. No live rendering or hover/lifecycle proof. Gain/max-level animations, tooltip, always-on text preference, other status bars and individual edit-mode movers are outside this task. Existing XP chat/message handling remains unchanged; historical Bevy XP tests and September 25 evidence do not prove this Godot screen.

## See Also

- [HUD presets](../../specs/hud-edit-mode.md)
- [[ui-system]] — RegistryUi and SharedContext
- [[godot-replication]] — replicated UnitLevel
