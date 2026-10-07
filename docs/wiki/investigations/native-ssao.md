# Native SSAO

Verified 2026-10-07. SSAO requires the pinned Forward+ renderer's depth/normal prepass. Disabling it caused black terrain; Dozen also corrupts SSAO with the prepass enabled. Depth of Field was removed, not migrated.

## Reproduction and root cause

`godot/tests/ssao_renderer_probe.gd` renders the same floor/box with StandardMaterial3D and the production terrain shader. It logs native Environment defaults: radius 1, intensity 2, power 1.5, light affect 0, AO-channel affect 0. Authored terrain requires AO-channel affect 1: its custom light() adds lighting without reading LIGHT_COLOR, so direct-light affect alone has no effect. `ambient_light_disabled` bypasses SSAO; the zero IRRADIANCE/RADIANCE overrides preserve authored Off output while allowing AO sampling.

The project disables `rendering/driver/depth_prepass/enable`. Pinned Godot `ed1daf0bf001b61586d9930840f2f1394092c079`, `render_forward_clustered.cpp:2113-2118`, gates `using_ssao` on `depth_pre_pass`. With the prepass disabled, sampling AO without generating it makes the terrain black on both Dozen and Lavapipe. The controller enables the prepass only while SSAO is On, restoring the prior setting on Off/disposal; project.godot is unchanged.

## Renderer comparison

Owned Weston ss30, pinned Godot, same shader/geometry/default SSAO values, 1280×720, Vulkan Forward+, Dummy audio:

| Path | Standard flat darkening | Authored flat darkening | Authored contact darkening |
|---|---:|---:|---:|
| Dozen, no prepass | 0.600000 | 0.796078 | 0.729713 |
| Lavapipe, no prepass | 0.600000 | 0.796078 | 0.729713 |
| Dozen, runtime prepass | 0.108765 | 0.142392 | 0.177955 |
| Lavapipe, runtime prepass | 0 | 0 | 0.020286 |

Authored values use light affect 1/AO-channel affect 1. Regions are specified in the probe, not selected after seeing output. Standard defaults already show corruption on Dozen, excluding authored lighting as its cause. Runtime ProjectSettings switching produces the same result as startup prepass-enabled controls. This proves a Dozen-path SSAO problem, not its internal driver defect. No renderer fallback or disabled-effect substitution was added.

Evidence under `data/diagnostics/ssao/renderer-{dzn,lvp}{,-dynamic}/`: `standard-off.png`, `standard-on-light0-ao0.png`, `authored-off.png`, `authored-on-light1-ao1.png`. Logs `/tmp/claude/ssao-probe-{dzn,lvp}{,-dynamic}.out`. The earlier startup-only Dozen prepass probe had an extension-path error; do not use it as full-client acceptance. Dynamic controls loaded the extension correctly.

## Acceptance boundaries

`ssao_options.gd` checks saved/live Options application, concrete Environment values, Off identity/prepass restoration, camera attributes, portrait exclusion, and replacement worlds. `ssao_options_pixels.gd` rejects flat-area dimming; contact darkening alone cannot pass. Dozen remains blocked and must fail the oracle. Lavapipe is a reference proof, not a hardware performance claim. Full-world/multi-vendor parity, transparency, emissive AO and Retail kernel equivalence are not established.

## Sources

- `godot/tests/ssao_renderer_probe.gd` — concrete renderer experiment.
- `godot/rendering/world_effects_controller.gd` — runtime mapping/restoration.
- Pinned Godot source: `servers/rendering/renderer_rd/forward_clustered/render_forward_clustered.cpp`, `core/config/project_settings.h`, `doc/classes/Environment.xml`; read at exact commit above.
- [Graphics requirements](../../specs/graphics-effects.md#native-godot-ssao-mapping).

## See Also

- [Rendering pipeline](../systems/rendering-pipeline.md).
