# Object Rotation Transforms (MDDF/MODF)

ADT world-object placement data stores rotation as `[X, Y, Z]` Euler angles, but they cannot be applied directly in Bevy's coordinate space. The correct mapping — derived from Noggit3 source and validated visually against Adventurer's Rest — is `[Z, Y-180, -X]` applied in `YZX` order.

## Finding

Several hypotheses were tested: global `+90°`/`-90°` yaw, `-180°` yaw offset, split doodad/WMO handling, and per-asset overrides. The `Y-180` yaw was clearly better than raw or `±90°` variants, but props still appeared mirrored. Flipping the non-yaw terms (`-X` instead of `X`) corrected the remaining tilt/lean inversion seen on asymmetric props like the campsite sword.

The bug affects both MDDF doodads (M2) and MODF WMOs — it is not scene-specific.

## Root Cause

The shared `placement_rotation()` helper in `src/terrain_objects.rs` was not applying the correct Euler component signs or order. Noggit3 uses `from_model_rotation(v) = (-v.z, v.y - 90, v.x)` with `rotation_yzx`; our renderer's axis conventions required the adjusted form `[Z, Y-180, -X]` in `EulerRot::YZX`.

## Resolution

`placement_rotation()` in `src/terrain_objects.rs` now implements:

```
stored [X, Y, Z] → model [Z, Y-180, -X] → EulerRot::YZX
```

Two tests lock in the formula:
- `placement_rotation_matches_current_model_rotation_formula`
- `placement_rotation_zero_matches_current_yaw_correction`

Per-asset yaw override hooks introduced during debugging were removed.

**Remaining risk:** if objects are still globally wrong, the next suspect is position-space interpretation (coordinate axis swizzle order), not the rotation formula itself.

## Sources

- [world-object-rotation-investigation-2026-03-22.md](../../world-object-rotation-investigation-2026-03-22.md) — full hypothesis log and Noggit3 reference
- Noggit3 reference: `~/Repos/noggit3/src/noggit/moveable_object.cpp`, `trig.hpp`

## See Also

- [[terrain-tile-ordering]] — investigation that exposed the rotation issue
