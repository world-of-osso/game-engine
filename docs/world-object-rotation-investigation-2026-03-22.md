# World Object Rotation Investigation

Date: 2026-03-22
Updated: 2026-03-23

## Summary

This investigation started as a char select scene mismatch in Adventurer's Rest, but it expanded into a shared world-object transform issue.

The important conclusion is:

- the problem is not limited to char select
- the problem affects general MDDF/MODF world-object placement
- scene-specific asset blocklists or per-scene overrides are not the right long-term fix
- the shared rotation mapping from ADT placement data into Bevy was inconsistent and likely wrong

At the end of this pass, the shared terrain-object rotation path no longer matches the initial Noggit-style trial. The current best-fit shared rule after further visual validation is:

- stored ADT rotation `[X, Y, Z]`
- converted model rotation `[Z, Y - 180, -X]`
- applied in `YZX` order

That change lives in [terrain_objects.rs](/syncthing/Sync/Projects/world-of-osso/game-engine/src/terrain_objects.rs).

## How The Investigation Started

The original report was in the char select Adventurer's Rest campsite:

- two white objects behind the chest
- later identified as the boots prop
- the boots were rendering as white cards because of incomplete material/shader handling

That renderer bug was real, but it was not the whole problem. After comparing against Blizzard reference images of Adventurer's Rest, it became clear that the overall campsite composition also looked wrong:

- prop facing differed from reference
- camera composition differed from reference
- nearby world objects looked rotated incorrectly

At that point the problem stopped being "fix one prop" and became "verify shared world placement math."

## What Was Investigated

### 1. Boots Rendering Bug

The prop:

- `3718225`
- `world/expansion08/doodads/maw/9mw_domination_legendaryarmor_boots01.m2`

Findings:

- the boots do belong in Adventurer's Rest
- textures were not missing from CASC
- the white result came from unsupported or misinterpreted multi-layer batches
- a narrow workaround was added earlier for helper/glow layers

This was a rendering bug, not a placement bug.

Related files touched during that work:

- [m2.rs](/syncthing/Sync/Projects/world-of-osso/game-engine/src/asset/m2.rs)

### 2. Char Select Camera / Framing

The char select scene had been synthesizing a single-character camera from the authored warband camera:

- replacing the authored focus with a character-centered focus
- pulling the eye inward to a fixed solo distance
- rotating the character to face the camera

That drifted the whole campsite composition away from Blizzard's authored shot.

The char select code was temporarily changed to preserve the authored warband scene camera and authored placement facing instead.

Related files:

- [mod.rs](/syncthing/Sync/Projects/world-of-osso/game-engine/src/char_select_scene/mod.rs)
- [tests.rs](/syncthing/Sync/Projects/world-of-osso/game-engine/src/char_select_scene/tests.rs)

That change was later backed out. It was useful as a composition experiment, but it was not tied to the global world-object transform bug and risked masking the real placement problem.

### 3. Shared ADT Placement Rotation

The main shared path is in:

- [terrain_objects.rs](/syncthing/Sync/Projects/world-of-osso/game-engine/src/terrain_objects.rs)

That path is used for:

- MDDF doodads (M2 placements)
- MODF WMOs (world model placements)

Several hypotheses were tested during the investigation:

#### Hypothesis A: A Global `+90°` or `-90°` Yaw Fix

This was tested directly.

Result:

- some campsite props improved
- some other objects became more wrong
- large background assets, especially trees and trunks, exposed that a naive blanket yaw tweak was not stable

Conclusion:

- a blind global quarter-turn fix was not enough by itself

#### Hypothesis B: A `-180°` Yaw Offset

This was tested after the `-90°` path still left many props globally facing the wrong way.

Result:

- many props lined up much more closely with the Blizzard reference
- overall placement felt more coherent
- but several asymmetric props still appeared mirrored
- for example, the sword leaned the correct amount but to the wrong side, and tipped backward instead of forward

Conclusion:

- the shared yaw term was probably closer with `Y - 180`
- but the remaining issue was not just yaw
- the tilt interpretation for M2 doodads was still wrong
#### Hypothesis C: Different Handling For Doodads And WMOs

This was also tested.

Result:

- some cases improved
- other cases still disagreed

Conclusion:

- the split by object class alone was still not trustworthy

#### Hypothesis D: Per-Asset Local Yaw Overrides

This was briefly prototyped when the evidence looked mixed.

Result:

- useful as a debugging tool
- not acceptable as the actual solution for a world-wide placement problem

Conclusion:

- per-asset overrides should not be the primary fix

That hook was removed once it became clear the issue was shared world-object math.

#### Hypothesis E: Keep `Y - 180`, But Flip The Non-Yaw Tilt Signs

This was tested after several props showed mirrored lean direction even when the overall heading looked closer.

Result:

- the sword no longer looked reversed left/right in the same way
- forward/backward tilt also improved
- other asymmetric props, such as the helm, looked closer as well
- compact props like the boots had already looked mostly correct, which fit the idea that the main remaining error was in tilt, not translation

Conclusion:

- the strongest current fit is:
  - yaw: `rot[1] - 180`
  - bank/roll term from `rot[2]`
  - attitude/pitch term from `-rot[0]`

This is the rule now implemented in `placement_rotation(...)`.

## Reference Implementations Checked

### Noggit3

Noggit turned out to be the most useful local reference.

Relevant files:

- [`moveable_object.cpp`](/home/osso/Repos/noggit3/src/noggit/moveable_object.cpp)
- [`trig.hpp`](/home/osso/Repos/noggit3/src/math/trig.hpp)
- [`matrix_4x4.cpp`](/home/osso/Repos/noggit3/src/math/matrix_4x4.cpp)

Important finding:

Noggit does **not** use the raw stored ADT rotation directly. It first converts the stored `[X, Y, Z]` using:

```text
from_model_rotation(v) = (-v.z, v.y - 90, v.x)
```

Then it applies that converted triple with `rotation_yzx`.

This was the strongest concrete reference found during the investigation.

### worldofwhatever

`worldofwhatever` was checked, but it did not provide one clear authoritative placement rule for ADT instance transforms.

It contains WMO-specific adjustments in some paths, for example in:

- [`wmo.cpp`](/home/osso/Repos/worldofwhatever/wmo.cpp)

That made it less useful than Noggit for deriving one shared transform mapping.

### WoWee

WoWee was checked locally, but it did not provide a directly reusable ADT object-placement reference for this issue during this pass.

## Current Shared Rotation Rule

The current implementation in [terrain_objects.rs](/syncthing/Sync/Projects/world-of-osso/game-engine/src/terrain_objects.rs) uses:

```text
stored rotation [X, Y, Z]
-> model rotation [Z, Y - 180, -X]
-> apply with EulerRot::YZX
```

The key helper is `placement_rotation(...)`.

This is no longer the same as the earlier Noggit-style `[-Z, Y - 90, X]` trial. That earlier mapping was an important reference point, but further visual validation against Adventurer's Rest showed it was not the best fit for this renderer's full transform chain.

## Tests Added / Updated

The transform tests in [terrain_objects.rs](/syncthing/Sync/Projects/world-of-osso/game-engine/src/terrain_objects.rs) were updated to lock in the current shared rule:

- `placement_rotation_matches_current_model_rotation_formula`
- `placement_rotation_zero_matches_current_yaw_correction`

Char select tests still cover the solo-character camera/framing behavior, but the temporary authored-camera preservation experiment was reverted.

- [tests.rs](/syncthing/Sync/Projects/world-of-osso/game-engine/src/char_select_scene/tests.rs)

## Debugging Aids Added During Investigation

Temporary ignored tests were used to inspect live Adventurer's Rest placement data:

- `dump_charselect_nearby_doodads`
- `dump_charselect_nearby_wmos`
- `dump_charselect_neighbor_tile_objects`

These were useful for:

- identifying which assets were actually near the campsite
- ruling out several wrong prop guesses
- verifying that some suspected objects were not even on the central campsite tile

One concrete outcome from these dumps:

- the campsite tent is not a WMO and not a doodad nested inside a parent WMO
- the tent is an MDDF doodad on the tile:
  - `4198188 = world/expansion09/doodads/explorersleague/10el_explorersleague_smalltent02.m2`
- the nearby WMOs are larger exterior assets like rocks and trees, not the campsite set dressing

This ruled out the idea that the tent was being rotated through a WMO-internal quaternion path.

## Important Things Ruled Out

- This is not only a char select problem.
- This is not only a boots problem.
- This is not solved by hiding individual props.
- This is not realistically solved by maintaining a growing hand-written per-asset yaw table.
- The older `game-engine-m2-particles` branch is not authoritative and should not be treated as ground truth.

## Current Status

As of this document:

- char select is back on its regular solo-character framing path
- the shared terrain-object placement rotation currently uses `[Z, Y - 180, -X]` in `YZX` order
- ad hoc per-asset yaw overrides introduced during debugging were removed in favor of the shared rule
- the campsite tent has been confirmed to be an MDDF doodad, not a WMO child doodad

What still needs real validation:

- whether the current shared rule fixes both Adventurer's Rest and general in-world object facing well enough to keep
- whether there is still a remaining axis-space issue beyond rotation alone
- whether position and rotation are being interpreted in a mixed coordinate space somewhere else in the world-object pipeline

## Recommended Next Step If Objects Are Still Wrong

If world objects are still globally rotated wrong after the current change, the next investigation should target the full transform chain rather than adding more exceptions:

1. validate position-space interpretation for MDDF/MODF against a known reference viewer
2. compare transformed bounds or corners for a few clearly identifiable objects
3. verify whether Bevy axis swizzle and placement rotation are being combined in the correct order
4. only after that, revisit whether any remaining mismatch is model-local

In other words: if the current `Y - 180` plus flipped-tilt mapping is still insufficient, the next suspect is not "asset-specific yaw" but a deeper coordinate-space mismatch.
