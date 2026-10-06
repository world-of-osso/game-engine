# Player cast feedback

Player-only animation sampling shares the cast reducer with target/nameplate bars without changing their presentation. Both skins use Retail timing; Forever changes FlareUI dimensions and fill colours, not cast mechanics.

## Source contract

Cached source root: `~/.cache/wow-ui-sim/blizzard-ui/retail/AddOns/Blizzard_UIPanels_Game/`.

| Feedback | Source |
|---|---|
| Success fills, hides spark, shows full-type flash, plays fade and finish | `Shared/CastingBarFrame.lua:575-600` (`FinishSpell`); type art/finish selection at `45-76` |
| Failed/Interrupted text and interrupted art | `Shared/CastingBarFrame.lua:540-572`; spark callback fills and hides at `669-678` |
| Channel finish retains ending value rather than filling | `Shared/CastingBarFrame.lua:466-479,575-600` |
| Flash alpha 0→1 in 0.2s; standard translated glow/flakes | `Mainline/CastingBarFrame.xml:130-146` |
| Channel translated wisp mask/sparkles, rotation, glow scale and alpha | `Mainline/CastingBarFrame.xml:147-167` |
| Interrupt glow fades for 1s; shake jumps at 0.15/0.20/0.25/0.30s; red spark remains 0.1s | `Mainline/CastingBarFrame.xml:169-187` |
| Normal fade: 0.2s delay then 0.3s; interruption hold: 1s then 0.3s | `Shared/CastingBarFrameTemplates.xml:5-16` |
| Pip at fill edge, type-specific trailing FX and clipping masks | `Shared/CastingBarFrame.lua:680-719`; `Mainline/CastingBarFrame.xml:326-388` |

This cached Retail source has no `holdTime` member: its hold is `HoldFadeOutAnim`. The classic-style immediate interruption fill (`Shared/CastingBarFrame.lua:564-571`) is not a Forever skin requirement. FlareUI standalone cast bars override dimensions/colours (`data/reference/flareui/Core.lua:281`, `Modules/UnitFrames.lua:75-79,651-661,2225-2234`); their event updater hides a missing cast instead of defining an alternate feedback animation (`630-636`). The requested Retail feedback therefore uses the same clock under both skins.

## Implementation

- `godot/rust/src/nameplate_casts.rs`: exposes player-only `CastFeedback` from the existing fade clock; interrupt fills after the red spark.
- `godot/ui-model/src/ui/screens/casting_bar_frame_component.rs` and `casting_bar_feedback.rs`: cropped atlas fill, flash, translated/scaled/rotated FX, spark and shake; Forever retains its 292×26 track, icon and fills.
- `godot/rust/src/ui/castbar_fx.rs`: native shader samples atlas mask regions in track coordinates, preserving ADD/BLEND.
- `godot/rust/src/ui/castbar_preview.rs`: secondary exported API; fixed production reducer snapshots without a session or socket.
- `godot/tests/capture_ui_screen.gd`: existing native capture route, selected by `GODOT_CAPTURE_SCREEN=castbaranim_preview` and `GODOT_CASTBAR_SKIN`, `GODOT_CASTBAR_PHASE`, `GODOT_CASTBAR_TIME` (event at 100.0).

Previously the player had progress/colours and reducer fades, but no completion flash, finish FX, interrupted full state/glow/shake or Retail pip (Forever had no spark; Modern had a solid rectangle).

Native interrupted capture reproduced a lost vertical shake: changing `margin_top` cannot move a bottom-anchored bar. Sampling the XML translation into `translate_x`/`translate_y` moves both axes independently of HUD anchors. The existing capture fixture asserts the actual track origin at timestamp 100.175 and isolates interrupt glow pixels.

A second timestamp regression reproduced channel failure retaining “Arcane Missiles” when no interrupter name accompanied `SpellFailure`. Retail forces interrupted channel stops through `HandleInterruptOrSpellFailed(true, ...)` (`Shared/CastingBarFrame.lua:450-456`); the reducer now honours the failure before or after replicated removal, without requiring a name.

## Art and evidence

Retail atlas files: FDIDs `4505182`, `4505194`, `4549775`, `4550035`, `4550359`, `4550462`. Extract only from local CASC into canonical `data/textures/`; assigned slots may link those immutable files. No synchronous extraction in the per-frame HUD path.

Timestamp tests are `player_castbaranim_*` in `godot/rust/src/nameplate_casts_tests.rs`. Rendered evidence status remains pending until captures are inspected; CPU model tests do not prove native shader pixels.

## Sources

- [Feature contract](../../specs/player-cast-feedback.md)
- Cached Retail Lua/XML and local FlareUI references cited above.

## See Also

- [[godot-conversion]] — native client and UI projection
- [[asset-pipeline]] — local CASC and texture loading
