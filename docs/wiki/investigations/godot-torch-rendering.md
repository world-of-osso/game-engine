# Godot torch rendering (club_1h_torch_a_01)

`--screen particledebug`'s torch (FDID 145304) compared against solarityclient (Wrath) and
WebWowViewerCpp (retail) on 2026-09-30. Captures: `data/diagnostics/torch-2026-09-30/`
(`before/`, `after/`, `before-after.png`, `*-strip.png`).

## Asset

- Batch 0 opaque handle; batch 1 is a 0.6 yd golden halo quad (texture 198077, material
  flags 0x11 unlit/no-zwrite, blend 2) in the YZ plane of bone 1 (flags 0x208, spherical
  billboard, scale pulses on a global sequence).
- One emitter (flags 0x820010: model space, head quads, compressed gravity) on bone 10,
  child of spherical billboard bone 2; 20/s, 0.8 s, 4x4 flipbook, blend 4.
- One point light on bone 9: diffuse (119, 74, 34)/255 x 1.1, authored attenuation
  1.389-2.222 yd. Global flags 0x80, so retail forces 1.6666-5.2667 yd
  (WebWowViewerCpp `animationManager.cpp:1238-1241`); solarity ignores the tracks too.

## Differences found

| Aspect | Godot before | Reference | Now |
|---|---|---|---|
| Billboard bones 0x8/0x10/0x20/0x40 | not implemented; halo edge-on to the default view, flame frame fixed | both clients rebuild bone axes in view space (WWV `calcBoneMatrix`, solarity `pose.rs:531-610`) | `m2_billboard` + `animation/billboard.rs` |
| M2 point lights | not rendered | WWV deferred point pass (`pointLight.frag.slang:43-58`); solarity per-receiver lights with fixed attenuation | `OmniLight3D` per light (`assets/m2_lights.rs`), M2/terrain/WMO shaders add it to opaque surfaces |
| Spawn, lifetime, colour/alpha/scale ramps, flipbook, twinkle, spin, gravity/drag, emission area, head-quad billboarding, blend/alpha test/depth | already WWV-ordered (`m2_particles`) | same | unchanged |
| ParticleColor overrides | none | indices 11-13 only | torch index 0: not applicable, not implemented |
| Ribbons, tail quads, EXP2 alpha cutoff, DETL light multiplier | none | present | torch has none; not implemented |
| Soft particles | none | none in either | — |

## Remaining gaps

- Godot's omni falloff `(1 - (d/r)^4)^2` stands in for retail's linear start-to-end ramp
  (squared); range is the retail end.
- Blending runs in linear space; retail adds in gamma space, so overlapping additive
  flame particles stay more orange here than retail's yellow-white core.
- The particle debug ground is near-black (0.08 albedo), so the light barely shows on it.

## Tests

`godot/tests/particle_debug_screen.gd` (flame ramps/cells from the MultiMesh buffer, light
placement/colour/range and ground/M2 reach, halo and flame bones facing the camera from two
orbits); `godot/core/tests/m2_billboard.rs`, `m2_lights.rs`.
