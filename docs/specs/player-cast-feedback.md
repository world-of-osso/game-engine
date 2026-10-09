# Player cast feedback

PlayerCastingBarFrame follows the locally cached Retail CastingBarFrame source under Blizzard_UIPanels_Game. Forever retains FlareUI's player bar dimensions and colours. Source-backed animation details belong in [cast feedback](../wiki/systems/player-cast-feedback.md).

## What it must do

- [x] Successful casts fill, hide their spark, show the 0.2-second completion flash and StandardFinish effects; hold 0.2 seconds, then fade over 0.3 seconds.
- [x] Interrupts/failures show Interrupted/Failed and interrupted art, fill after the 0.1-second interrupt spark under both skins, play source glow/shake, hold 1 second and fade over 0.3 seconds.
- [x] Channels retain their ending value, hide the spark, play ChannelFinish and completion flash with the normal 0.2/0.3-second fade.
- [x] Both skins render source spark/feedback; Forever retains FlareUI's 292×26 track, 4-unit inset, icon and cool-toned fills.
- [x] Player spark top/bottom align with the fill's inner edges. Retail `Blizzard_UIPanels_Game/Mainline/CastingBarFrame.xml:326-330` authors the 8×20 spark; Forever's taller track comes from `data/reference/flareui/Core.lua:281` (292×26). Stretch only spark height to 26 in Forever; Modern remains 8×20 on its 20-high fill. Preserve x-position, width, texture/colour, timing and all other bar geometry.
- [x] Offline previews use production cast state transitions. Key frames are captured only through the existing capture test.

## How it works

- [Source and rendering](../wiki/systems/player-cast-feedback.md)

## Implementation inventory

- `godot/rust/src/nameplate_casts.rs`: replicated cast lifecycle and player presentation.
- `godot/rust/src/spells.rs`: player HUD integration.
- `godot/ui-model/src/ui/screens/casting_bar_frame_component.rs`: both authored skin tracks.
- `godot/rust/src/ui/castbar_preview.rs`: deterministic offline snapshots.

## Tests asserting this spec

- `godot/rust/src/nameplate_casts_tests.rs`: timestamp-driven player feedback.
- `godot/tests/capture_ui_screen.gd`: native offline key-frame capture.
- `godot/ui-model/tests/player_cast_spark.rs`: player casting tree/projection rects in both skins; Forever RED at fill `(0,0,219,26)` versus spark `(215,3,8,20)` (75% progress), Modern unchanged at fill `(0,0,192,20)` versus spark `(188,0,8,20)`.

## Known gaps (current cycle)

- Feedback acceptance verified 2026-10-06: 26 scoped Rust/UI tests and ten inspected native snapshots, including six requested key frames. See [evidence](../wiki/systems/player-cast-feedback.md#verification--2026-10-06).
- Spark-height evidence, 2026-10-09, production revision `397c20803`: targeted tree/projection RED 1 failed (Forever), 1 passed (Modern); GREEN 2 passed. Forever local spark changed `(215,3,8,20)` → `(215,0,8,26)` against unchanged fill `(0,0,219,26)` at 75%; Modern remained `(188,0,8,20)` against `(0,0,192,20)`. Tree-only tests emit missing-atlas diagnostics; native captures load the real art.
- Extension build passed; existing native offline `castbaranim_preview` capture passed in both skins at `midcast`, timestamp `100.000`. Inspected 350×62 Forever and 300×62 Modern crops: Forever yellow pip follows the taller blue fill, Modern unchanged. Native fill rects at 50%: Forever `(507,422,146,26)`, Modern `(512,544,128,20)`; spark centres pass the existing fill-edge assertion. PNGs: `/syncthing/AgentShared/2026-10-09/castbar-spark/{forever,modern}-{midcast,crop}.png`. Logs/hash ledger: `data/diagnostics/castbar-spark-2026-10-09/`. Captures are 1280×720 (headless cage mode); Wayland/cage environment warnings remain recorded.
- Offline fixtures still report texture/font RID and ObjectDB allocation leaks at shutdown; clean resource lifetime is not proven.

## Out of scope

- Empowered and crafting casts: not represented by the current player cast protocol.
- Target/nameplate restyling, live server operations and publishing.
