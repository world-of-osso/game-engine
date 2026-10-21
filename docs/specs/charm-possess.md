# Charm and possession

Native control for server aura2/6 uses replicated `UnitCharm`, existing movement input and pet action bar. Authority/lifetime belong to the [server contract](../../../game-server/docs/specs/charm-possess.md); client [runtime](../wiki/systems/charm-possess.md) follows TC a352b1fa Unit/CharmInfo and Retail's existing pet widgets.

## What it must do

- [x] Replicate the charmer server identity, charm/possess mode and component removal over the actual native UDP bridge; preserve the NPC entity on release.
- [x] Aura2 owned by the selected player switches movement, camera, input epoch/position and local animation to that unit. Selected-player identity remains the character, not the NPC.
- [x] The selected player's charmed body accepts no local movement/casts; its motion follows server snapshots. Aura6 owned by that player does not switch the mover/camera.
- [x] Control transfer clears old jump/autorun/network-report state, retaining the user's walk/run toggle and reporting the new position/epoch. Native body-control restoration proof remains below.
- [x] Existing pet bar/portrait/context dismiss refer to the active charm before a real pet. Dismiss sends existing command3; no creature deletion or hunter stable mutation.
- [x] Both Modern and Forever native1920×1080 exercise actual605 possession, NPC movement with parked priest/camera following, cancel/body control restored, and205364 body remaining active.

## How it works

- [Client control selection and animation](../wiki/systems/charm-possess.md).

## Implementation inventory

- `godot/network/src/replica/codec.rs`: UnitCharm native replication codec.
- `godot/rust/src/world.rs`: charmer/mode selection, effective mover and motion/animation source.
- `godot/rust/src/{gameplay,transport_gameplay}.rs`: mover-owned prediction, epoch and input frame.
- `godot/rust/src/{replicated,unit_menu}.rs`: existing controlled-unit HUD and dismiss route.

## Tests asserting this spec

- `godot/network/src/wire_charm_tests.rs`: real UDP identity/mode/removal. IgnoredRED375c7eda7:0PASS/1FAIL, exact missing UnitCharm codec. At14d61bb20:actualUDP1GREEN0.90s +3 client state/report regressions GREEN; wire test enabled.
- `godot/rust/src/{world,gameplay,replicated}.rs`: ownership/mode selection, state transfer and HUD release tests.
- `godot/tests/charm_possess_live.gd`: actual private native605/205364 at1920×1080 in each selected skin.

## Native proof and bounds

- At267edf0ca, actual native605/205364 pass in Modern and Forever on private UDP5971: mover/camera transfer, parked priest, authoritative NPC movement, channel cancellation, priest control restoration and ordinary-charm body movement. Both processes exit0 with `CHARM_POSSESS_DONE`; eight original PNGs are1920×1080, inspected via1280px previews.
- Originals: `/syncthing/AgentShared/charm-possess-2026-10-21/`. Diagnostic logs and image hashes: canonical `data/diagnostics/charm-possess-2026-10-21/{client-modern.log,client-forever.log,screenshots.json}`.
- Setup uses the native CharacterSelect Down/Enter route after its CanvasLayer attaches, a published Blackrock Spy49874, and online-only admin spell learning. Existing native IPC `spell stop` sends StopSpellCast; Escape clears the target first and is not claimed as channel-cancel proof.
- Context Dismiss is source-wired and server command3 release is tested; an actual native menu click and an NPC-controlled player rendered case remain unqualified.

## Out of scope

- New widget layouts/art, ability rotations, possession-specific spell visuals and broader spell-class completeness certification.
