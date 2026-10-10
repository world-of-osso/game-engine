# Passenger mounts

Party/raid members can ride available passenger seats on a player's summoned mount. Engine seating is native Godot M2 presentation; `shared-protocol` carries ownership and occupancy; `game-server` owns all decisions. See [mount systems](../wiki/systems/mounts.md).

## What it must do

### Data and wire
- [x] Export readable Vehicle and VehicleSeat records from pinned local Retail CASC; preserve sparse seat indices, attachment enums, offsets, flags and passenger animation IDs.
- [x] Replicate VehicleID and eight indexed server-entity occupancy slots on Mounted, plus each passenger's driver/seat identity; transport board/exit requests.
- [ ] Resolve mount spells/displays through authentic creature VehicleIDs, including requested mammoth, yak, drake and chopper; record available seat counts.

### Server
- [x] Board an empty enter/exit non-control seat for a living party/raid member; reject non-group and full-mount requests without mutating passenger state.
- [x] Exit frees occupancy and restores movement control.
- [x] Passenger translation follows the driver's rotation/translation; eject on driver dismount, death, despawn, map or zone transfer.
- [ ] Ignore actual passenger movement packets; clear disconnected passenger occupancy; verify passenger death and mount replacement.
- [ ] Verify local catalog and actual M2 seat bind offsets, not just synthetic transform fixtures.

### Client and live acceptance
- [ ] Resolve VehicleSeat attachment enums into the mount M2 lookup; apply offsets and seat animation to passenger visuals.
- [ ] Passenger camera follows the vehicle; passenger input cannot translate the character.
- [ ] Both skins expose the same Retail vehicle leave control and group-member Ride interaction.
- [ ] Two fb_* characters on a private UDP port demonstrate mammoth/yak seating in both skins; inspect captures before publication.

## How it works
- [Mounts](../wiki/systems/mounts.md).

## Implementation inventory
- `scripts/export_db2_csv.py`: pinned local Vehicle/VehicleSeat export layouts.
- Sibling `shared-protocol/src/components.rs`: Mounted seats and VehiclePassenger.
- Sibling `shared-protocol/src/protocol/{gameplay_messages,registration}.rs`: board/exit messages.
- Sibling `game-server/crates/server/src/vehicle.rs`: content joins, authority, occupancy, movement control and ejection.
- `godot/network/src/replica/codec.rs`: passenger component decoding.

## Tests asserting this spec
- `scripts/tests/test_vehicle_export.py`.
- Sibling `shared-protocol/tests/vehicle_wire.rs`.
- Sibling `game-server/crates/server/src/vehicle_tests.rs`.
- Sibling `game-server/crates/server/src/networking_tests/receiver_rate.rs` passenger input test (pending).
- `godot/core/src/vehicle_seat.rs` seat-transform test (pending).

## Known gaps (current cycle)
- [ ] Yak and drake creature entries are absent from the current local world.db link source; no guessed VehicleIDs.
- [ ] Client rendering/UI and live proof remain in progress, not accepted.
- [ ] Independent verification backend is blocked by expired Claude OAuth; no substitute paid backend authorized.

## Out of scope
- Mount journal and collection UI, explicitly excluded.
- Full vehicle action/override bar, player-frame vehicle art and seat indicators; only exit-seat control requested.
- NPC passenger vendors/services.
