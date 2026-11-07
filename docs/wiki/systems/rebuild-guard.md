# Rebuild guard

Per-frame full-scene rebuilds slipped in more than once: the world light re-sampled and
rebuilt the sky cubemap every frame under the live clock (fixed in 5956248db, 186601b7e,
4f2a03275), nameplates reset their label theme every frame (7f7d77f62), and the profession
list rebuilt every frame (cac81b01c). The rebuild guard counts those operations so a new
one fails a test instead of costing frame time silently.

## Counters

`godot/rust/src/rebuild_guard.rs` keeps one atomic counter per category and the first call
site since the last `RebuildCounters.take()`: the `#[track_caller]` location, an optional
detail (frame or node path) and the nearest extension frames of a backtrace, symbolized
only by `take`.

| category | counted at |
|---|---|
| `sky_cubemap` | `lighting::create_cubemap`, `update_cubemap` |
| `environment` | each world light `WorldLighting::sync` applies |
| `material_rebind` | each material `TerrainLight::bind*`/`clear_model` writes (call site of the caller) |
| `shader_broadcast` | `set_global_shader_parameter` (every client global uniform write) |
| `mesh_build` | `new_array_mesh`, ribbon surface rebuilds (dynamic region uploads are not rebuilds) |
| `shader_material` | `new_shader_material`, particle material duplicates |
| `theme` | `ThemeOverrides::change_theme_*_override` (sets only a changed value), `new_style_box` |
| `ui_tree` | registry projection node creation/removal and part rebuilds, nameplate spawn/free |
| `shadow_atlas` | root viewport/directional atlas settings changing between frames (`tests/rebuild_guard.gd`; no client code sets them) |

`take()` also returns `world_events` (unit visuals attached: the world changed) and
`objects_created`, Godot's gross object count from the ObjectDB validator counter.

## Reports

`godot/tests/rebuild_guard.gd` wraps a measured window: `rebuilds_per_frame`,
`rebuild_counts`, `rebuild_sites`, `world_events` and `godot_per_frame` (pipeline
compilations, texture/buffer/video memory, object/resource/node count deltas). Every
`frame_benchmark.gd` `BENCH_SEGMENT` carries them.

## Invariant

`godot/tests/rebuild_guard_live.gd` enters the world like the frame benchmark, keeps the
live clock, holds the receipt camera, settles 10 s and measures 10 s idle windows. A
window in which a unit visual entered the scene is reported and measured again (up to
three). The steady window fails when a category exceeds its `ALLOWLIST` budget:

- `shader_broadcast`: 1 per frame (`liquid_animation_time_ms`, uv_animation.rs) plus 32
  per lit-time step (bind_scene's light, fog and water globals);
- `environment`: 1 per lit-time step (0.9375 s at the realm's real-time speed).

Run it at both receipt poses (indoor -8785.9 820.7 97.65 yaw 0, outdoor -8855 645 97
yaw 180°) with `BENCH_SCRIPT=rebuild_guard_live.gd`-style launches of the frame benchmark
environment (`GODOT_TEST_SERVER`, `BENCH_ACCOUNT`, `BENCH_CHARACTER`, `BENCH_CAMERA_YAW`).

## Rule

AGENTS.md: a per-frame path that touches materials, environment, meshes or UI theme/tree
is change-driven and counts here; allowlist entries cite their source.
