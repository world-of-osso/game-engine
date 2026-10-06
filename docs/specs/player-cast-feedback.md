# Player cast feedback

PlayerCastingBarFrame follows the locally cached Retail CastingBarFrame source under Blizzard_UIPanels_Game. Forever retains FlareUI's player bar dimensions and colours. Source-backed animation details belong in [cast feedback](../wiki/systems/player-cast-feedback.md).

## What it must do

- [ ] Successful casts fill, hide their spark, show the 0.2-second completion flash and StandardFinish effects; hold 0.2 seconds, then fade over 0.3 seconds.
- [ ] Interrupts/failures show Interrupted/Failed and interrupted art, fill after the 0.1-second interrupt spark under both skins, play source glow/shake, hold 1 second and fade over 0.3 seconds.
- [ ] Channels retain their ending value, hide the spark, play ChannelFinish and completion flash with the normal 0.2/0.3-second fade.
- [ ] Both skins render source spark/feedback; Forever retains FlareUI's 292×26 track, 4-unit inset, icon and cool-toned fills.
- [ ] Offline previews use production cast state transitions. Key frames are captured only through the existing capture test.

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

## Known gaps (current cycle)

- [ ] Implementation and rendered key-frame proof pending.

## Out of scope

- Empowered and crafting casts: not represented by the current player cast protocol.
- Target/nameplate restyling, live server operations and publishing.
