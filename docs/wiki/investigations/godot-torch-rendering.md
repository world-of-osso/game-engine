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

## Point-light falloff

Retail: `attenuation = 1 - clamp((d - start) / (end - start), 0, 1)`, light =
`(attenuation * color)^2 * N.L` (WWV `pointLight.frag.slang:51-58`): flat to the start,
then a squared linear ramp. Godot's omni window `(1 - (d/r)^4)^2`
(`scene_forward_lights_inc.glsl` `get_omni_attenuation`) has no flat part, and no range or
exponent reproduces the ramp: at the torch's midpoint (3.47 yd) it gave 0.66 against
retail's 0.25.

The lights keep range = end, exponent 0 and no shadow, so `ATTENUATION` depends on
distance alone; `shaders/m2_point_light.gdshaderinc` inverts it to `d / end` and applies
retail's squared ramp. The light carries `start / end` (`PointLight::start_fraction`) in
its specular parameter, which reaches `light()` as `SPECULAR_AMOUNT = 2 * specular`
(Godot `light_storage.cpp`) even under `specular_disabled`. M2, WMO and terrain share the
include. Fixture `godot/tests/m2_point_light_pixels.gd`: before, the M2 floor 0.32 yd from
the light got 0.744 of overhead light (retail 0.718); after, all three shaders are within
0.02 of retail from 0.32 to 0.99 yd (e.g. 0.28 at 0.46 yd, where Godot's window gives
0.47). Log `data/diagnostics/avfix-2026-09-30/torch/`.

## Remaining gaps

- Blending runs in linear space; retail adds in gamma space, so overlapping additive
  flame particles stay more orange here than retail's yellow-white core.
- The particle debug ground is near-black (0.08 albedo), so the light barely shows on it.

## Tests

`godot/tests/m2_point_light_pixels.gd` (retail falloff on M2, WMO and terrain);
`godot/tests/particle_debug_screen.gd` (flame ramps/cells from the MultiMesh buffer, light
placement/colour/range and ground/M2 reach, halo and flame bones facing the camera from two
orbits); `godot/core/tests/m2_billboard.rs`, `m2_lights.rs`.
