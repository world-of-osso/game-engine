# M2 animation transform errors

Two independent animation paths caused repeated transform errors in the Godot client. The fixes preserve authored data instead of filtering invalid transforms.

## Sequence aliases

Karazhan Spectral Chef displays use human model `1011653`. Its skeleton sequence 208, index 45, has alias flag `0x40` and points to index 44, sequence 60. The old `wow-m2` parser treated alias offsets as the alias's own keys; malformed timestamps produced NaN bone translations. `wow-formats` revision `25f75bc` resolves alias chains for bone tracks and events, including external animation data. `godot/core/tests/m2_external_animation.rs` checks the real model and external animation `1013009`.

## Full bone overrides

In Stormwind Trade, effect model `5386951` exposed a separate zero-scale bone. Godot's `Skeleton3D` interpolated a full-weight global override, decomposing the zero basis into a quaternion and emitting errors. The pinned `scripts/godot/skeleton-full-override.patch` assigns the exact affine transform at weight 1; partial weights still interpolate. `godot/tests/skeleton_full_override.gd` checks zero scale, shear, and partial interpolation.

On the `matlist-bonepose` Godot pin with the alias fix, 90-second rendered samples at both locations showed zero non-finite and zero quaternion errors respectively. Chef displays 16524, 16525 and 16527 were requested. The samples did not independently confirm the active chef sequence or instantiate the specific Trade effect, so they do not prove those exact live states.

## Sources

- `godot/core/tests/m2_external_animation.rs` — real alias fixture.
- `scripts/godot/skeleton-full-override.patch` and `godot/tests/skeleton_full_override.gd` — full-weight correction and reproduction.
- `/home/osso/.worktrees/skin-receipt-staging/532-before-1793454345079974790.log` and `0-before-1793454457798529110.log` — sampled rendered runs.

## See also

- [Godot material update-list race](godot-material-update-race.md) — other patch in the same Godot pin.
- [Shared WoW formats](../systems/wow-formats.md) — parser ownership and engine pin.
