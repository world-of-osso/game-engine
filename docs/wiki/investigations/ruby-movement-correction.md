# Ruby movement correction

## Original stop: an automation route error

VERIFIED on engine `78a0b50f2`: walking from Ruby Life Pools' entrance `(1386.20,169.719)` at yaw87.4734° stops at `(1436.7565,171.9504)`. WMO4391189, placement35566826, group1/FDID4391460, face2941 has MOPY0x60/material5 and normalY0.456748: a62.82° rock face, not a walkable floor. It has no DETAIL0x04 exclusion. A centered tunnel line to `(1445,161)` walks successfully with697 recorded frames and0 collision rejections. Do not remove this wall or raise the slope limit to admit the incorrect straight-line route.

## Separate server correction

VERIFIED in the same private run: a valid ground-following segment later snapped backward despite0 client collision rejections. A bounded1.2-second pause of the private server reproduced the correction with661 client trace frames and no client collision rejection. No server wall or slope clamp changed XZ. The movement bank left authority3.87yd behind the latest report; projecting its interpolated height129.87059 onto the floor129.89124 set `constrained=true`. The existing1-second sustained-disagreement timer then changed the movement epoch.

The error is correction classification, not collision geometry: a floor-following intermediate height is not evidence that the client's endpoint was rejected. The movement bank and authoritative floor projection still apply.

## Decided rule and acceptance

The lead approved treating ordinary floor-following projection within `STEP_UP_HEIGHT` of the requested Y as unconstrained, everywhere. Larger height rejection and actual wall, slope or step rejection retain the existing correction policy. No collision, speed, step, slope, floor-selection or timing constants change.

Acceptance pairs the native predictor walking the real entrance segment with the server's real Lightyear receive/fixed-update path, including queued60Hz inputs during a1.2-second server pause, unchanged speed limits, eventual catch-up and a large-height rejection control. At this design commit the server pause acceptance is expected to fail; implementation and final live proof are not yet complete.

## Sources

- `data/diagnostics/ruby-collision-stuck-2026-11-09/`: `normal-route-before.json`, `geometry-columns.json`, `center-tunnel-before.json`, `server-stall-client-trace-before.json`, `server-correction-reason.txt`.
- [Native acceptance](../../../godot/rust/src/ground.rs): `ruby_entrance_floor_following_walks_the_recorded_normal_segment`.
- Server `crates/server-combat/src/networking_tests/ruby_movement.rs` and `docs/specs/player-height.md`.
- Local solarityclient `docs/architecture/movement-collision.md:99-110`: historical player support normal threshold≈0.642788; not a current-Retail route measurement.

## See Also

- [[player-ground]] — authored floor and wall support.
- [WMO floor contract](../../specs/wmo-floor-collision.md).
