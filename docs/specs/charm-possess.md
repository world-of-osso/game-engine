# Charm and possession

Native control for server aura2/6 uses replicated `UnitCharm`, existing movement input and pet action bar. Authority/lifetime belong to the [server contract](../../../game-server/docs/specs/charm-possess.md); client [runtime](../wiki/systems/charm-possess.md) follows TC a352b1fa Unit/CharmInfo and Retail's existing pet widgets.

## What it must do

- [ ] Replicate the charmer server identity, charm/possess mode and component removal over the actual native UDP bridge; preserve the NPC entity on release.
- [ ] Aura2 owned by the selected player switches movement, camera, input epoch/position and local animation to that unit. Selected-player identity remains the character, not the NPC.
- [ ] The selected player's charmed body accepts no local movement/casts; its motion follows server snapshots. Aura6 owned by that player does not switch the mover/camera.
- [ ] Control transfer clears old jump/autorun/network-report state, retaining the user's walk/run toggle; release restores body control.
- [ ] Existing pet bar/portrait/context dismiss refer to the active charm before a real pet. Dismiss sends existing command3; no creature deletion or hunter stable mutation.
- [ ] Both Modern and Forever native1920×1080 exercise actual605 possession, NPC movement with parked priest/camera following, cancel/body control restored, and205364 body remaining active.

## How it works

- [Client control selection and animation](../wiki/systems/charm-possess.md).

## Implementation inventory

- `godot/network/src/replica/codec.rs`: UnitCharm native replication codec.
- `godot/rust/src/world.rs`: charmer/mode selection, effective mover and motion/animation source.
- `godot/rust/src/{gameplay,transport_gameplay}.rs`: mover-owned prediction, epoch and input frame.
- `godot/rust/src/{replicated,unit_menu}.rs`: existing controlled-unit HUD and dismiss route.

## Tests asserting this spec

- `godot/network/src/wire_charm_tests.rs`: real UDP identity/mode/removal. IgnoredRED375c7eda7:0PASS/1FAIL, exact missing UnitCharm codec.
- `godot/rust/src/{world,gameplay,replicated}.rs`: ownership/mode selection, state transfer and HUD release tests.
- `godot/tests/charm_possess_live.gd`: actual private native605/205364 at1920×1080 in each selected skin.

## Known gaps (current cycle)

- [ ] Targeted GREEN and rendered native proof pending.

## Out of scope

- New widget layouts/art, ability rotations, possession-specific spell visuals and broader spell-class completeness certification.
