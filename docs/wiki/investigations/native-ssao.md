# Native SSAO

Verified 2026-10-07. SSAO requires the pinned Forward+ renderer's depth/normal prepass. Disabling it caused black terrain; Dozen also corrupts SSAO with the prepass enabled. Depth of Field was removed, not migrated.

## Reproduction and root cause

`godot/tests/ssao_renderer_probe.gd` renders the same floor/box with StandardMaterial3D and the production terrain shader. It logs native Environment defaults: radius 1, intensity 2, power 1.5, light affect 0, AO-channel affect 0. Authored terrain requires AO-channel affect 1: its custom light() adds lighting without reading LIGHT_COLOR, so direct-light affect alone has no effect. `ambient_light_disabled` bypasses SSAO; the zero IRRADIANCE/RADIANCE overrides preserve authored Off output while allowing AO sampling.

The project disables `rendering/driver/depth_prepass/enable`. Pinned Godot `ed1daf0bf001b61586d9930840f2f1394092c079`, `render_forward_clustered.cpp:2113-2118`, gates `using_ssao` on `depth_pre_pass`. With the prepass disabled, sampling AO without generating it makes the terrain black on both Dozen and Lavapipe. The earlier branch globally enabled the prepass. User decision (2026-10-07) replaces that with restart-required SSAO: `project.godot` restores prepass disabled; saved On enables it only before renderer initialization.

A runtime-toggle implementation failed saved-On → Off: 430,777 visible terrain pixels disappeared. Pinned `scene_shader_forward_clustered.cpp:408,452-458` builds opaque pipelines with EQUAL depth comparison/no depth writes when the prepass is enabled. Pipeline caching makes the setting unsuitable for live toggling. Restoring the setting alone cannot restore pixels. Keeping the startup choice fixed avoids invalidating cached pipeline assumptions. Off image parity, not a state-only check, gates this fix. The new default preserves a41c02c1's deliberate prepass-disabled CPU-bound submission configuration; no new performance benchmark claim.

## Restart-required startup mechanism

Investigation order:

1. `application/config/project_settings_override="user://..."` is supported: pinned `core/config/project_settings.cpp:803-815` loads it during ProjectSettings setup. However, `user://` resolves through `core/os/os.cpp:318-333` and `drivers/unix/os_unix.cpp:1064-1066` to the user **data** directory, not the existing config directory. It would require a second generated state file and leave existing saved-On users without an override on their first launch. Rejected for the required config location and single saved source.
2. Launcher-generated override: pinned Godot's `--help` has no external project-override-file flag. Its existing launcher is std-only, and the deployed wrapper starts Godot directly. This would require another RON reader and a separate bundled launch path, or moving/mutating project files. Rejected rather than adding duplicate startup policy.
3. Chosen documented GDExtension initialization path: set `ExtensionLibrary::min_level()` to Core so core bindings are available, then read existing options and set `rendering/driver/depth_prepass/enable` only in `on_stage_init(Servers)`. Pinned `main/main.cpp:3014-3015` calls Servers extensions **before** constructing/initializing RenderingServer at `3320-3326`. Scene/main-loop/Options code never changes the prepass. This works for launcher, direct Godot, and deployed bundles without generated files, new dependencies or runtime project.godot edits.

`display_options.rs::initialize_ssao_prepass` uses the same production config-first/legacy load and validation as GameClient. Saved Off sets false; saved On sets true. The world controller uses that startup setting for every Options application and newly attached/replacement world, never the pending saved boolean. Graphics adds `SSAO (Requires Restart)` using the existing toggle-row presentation; no existing restart-label convention was found in the Options UI. Existing SSAO/MSAA validation remains explicit (None or Taa required); no implicit AA changes.

Sources at pinned Godot `ed1daf0bf001b61586d9930840f2f1394092c079`:
- [ProjectSettings override loading](https://github.com/godotengine/godot/blob/ed1daf0bf001b61586d9930840f2f1394092c079/core/config/project_settings.cpp#L803-L815)
- [user directory resolution](https://github.com/godotengine/godot/blob/ed1daf0bf001b61586d9930840f2f1394092c079/core/os/os.cpp#L318-L333)
- [Servers extension callback](https://github.com/godotengine/godot/blob/ed1daf0bf001b61586d9930840f2f1394092c079/main/main.cpp#L3014-L3015) and [RenderingServer construction](https://github.com/godotengine/godot/blob/ed1daf0bf001b61586d9930840f2f1394092c079/main/main.cpp#L3320-L3326)
- [gdext 0.5.5 initialization API](https://github.com/godot-rust/gdext/blob/v0.5.5/godot-core/src/init/mod.rs)

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

`ssao_options.gd` checks saved startup SSAO/prepass, real restart-labeled UI toggle persistence with unchanged live Environment identity/values/prepass, camera attributes, portrait exclusion, and replacement worlds. `ssao_options_pixels.gd` rejects flat-area dimming; contact darkening alone cannot pass. Dozen remains blocked and must fail the oracle. Lavapipe is a reference proof, not a hardware performance claim. Full-world/multi-vendor parity, transparency, emissive AO and Retail kernel equivalence are not established.

## Earlier live-toggle bounded proof (superseded by restart-required behavior)

At `7571569d`, matching Rust extension/CLI built successfully (Rust source unchanged from `d9e6847a`). Six unique `client_options` tests pass, including production file load/save with both removed keys; focused Cargo formatting passes. Consumer fixture with saved Off and On exits 0. Lavapipe pixel fixture with saved Off and On exits 0: 7,877 crease pixels darkened by >0.025, mean darkening 0.0050009334, isolated flat region byte-unchanged, exact UI and exact restored Off. Complete decoded Off fixture image matches the `ad9e3797` terrain-shader reference scene. This is controlled-scene master-material parity, not a rebuilt full-master client or full-world parity claim.

Final Dozen fixture exits 1 with checkerboard-like darkening across the whole floor and box; isolated flat mean darkening 0.2225490175. Its Off/master and UI checks pass before the strict rejection. No hardware acceptance claim.

- Captures: `data/diagnostics/ssao/stable-ssao_options_pixels-lvp-{off,on}/{startup-off,startup-on,off,ssao-on,off-restored}.png` (startup filename follows saved mode); Dozen `stable-ssao_options_pixels-dzn-off/{off,ssao-on}.png`.
- Logs: `/tmp/claude/stable-ssao_options{,_pixels}-{headless,lvp,dzn}-{off,on}.out` (only executed combinations exist), `/tmp/claude/ssao-{tests,build}.out`.
- Master reference: `git -C <slot> show ad9e3797:godot/shaders/terrain.gdshader`; captured under `data/diagnostics/ssao/master-{lvp,dzn}/startup-off.png` with the same scene/camera/UI, original prepass disabled and SSAO Off.
- RID/ObjectDB teardown warnings/errors remain, already present in the previous agent's fixtures. Exit-0 functional evidence is not clean-shutdown proof. No new suppression or unrelated cleanup fix.

## Sources

- `godot/tests/ssao_renderer_probe.gd` — concrete renderer experiment.
- `godot/rendering/world_effects_controller.gd` — runtime mapping/restoration.
- Pinned Godot source: `servers/rendering/renderer_rd/forward_clustered/render_forward_clustered.cpp`, `core/config/project_settings.h`, `doc/classes/Environment.xml`; read at exact commit above.
- [Graphics requirements](../../specs/graphics-effects.md#native-godot-ssao-mapping).

## See Also

- [Rendering pipeline](../systems/rendering-pipeline.md).
