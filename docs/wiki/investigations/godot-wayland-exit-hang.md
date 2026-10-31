# Godot client hangs after quit() under Wayland

**Symptom.** A Godot client or test script prints its last line (often PASS), calls `quit()`,
and never exits; runners time out with exit 124. Seen across m2render, uiscreens, devtools and
fixture runs; reproduced at 14/190 runs of `res://tests/m2_animation.gd` under headless cage.

**Root cause.** Godot 4.7.2 engine, not the extension. `WaylandThread::destroy()`
(`platform/linuxbsd/wayland/wayland_thread.cpp`) sets `thread_done` and calls
`wl_display_roundtrip()` to wake the "Wayland Events" thread, which only checks `thread_done`
after `poll(display_fd, -1)` returns. Two losing interleavings, both captured:

1. The events thread is not a registered reader when the sync reply arrives, so main reads it
   itself; the events thread then re-enters `poll(-1)` and main blocks forever in
   `events_thread.wait_to_finish()` (main in `__pthread_clockjoin_ex`, futex inside the
   "Wayland Events" pthread; that thread in `__poll`).
2. The events thread reads and dispatches the sync reply; main stays in
   `wl_display_roundtrip_queue` → `wl_display_poll` → `ppoll`, the events thread in `poll`.

Upstream: godotengine/godot#123059 (open), fix PR godotengine/godot#123946 (wake pipe polled by the
events thread, roundtrip removed). Unmerged; master has the same code.

**Proof.** `scripts/agent/quit-hang-loop.sh` (dumps every thread via eu-stack + gdb on a hang).
Official 4.7.2: 14/190. Self-built 4.7.2 unpatched: 2/60 (one of each mode). Self-built 4.7.2 +
PR #123946: 0/120 m2_animation, 0/60 m2_material_pixels. Evidence:
`data/diagnostics/quithang-2026-10-01/`.

**Status.** The launcher and shell helpers pin patched Godot `4.7.2-pr123946-pr123546-pr123391-matlist-bonepose`: official
4.7.2-stable plus the Wayland and [cold import crash](godot-cold-import-crash.md) fixes,
render-thread teardown PR #123391, the glyph-atlas snapshot fix, the
[material update-list fix](godot-material-update-race.md) and the full-weight Skeleton3D
global pose override fix (`skeleton-full-override.patch`: weight 1.0 assigns the override
directly instead of `interpolate_with`, which decomposed sheared/affine overrides;
reproduction `godot/tests/skeleton_full_override.gd`), built by
`scripts/godot/build-patched-godot.sh` with the official release toolchain (buildroot SDK
godot-2023.08.x-4, accesskit-c 0.22.3, SCons 4.10.1, `production=yes`) and SHA-512 pinned in
`scripts/godot/godot-4.7.2-pr123946-pr123546-pr123391-matlist-bonepose.sha512`; `PYTHONHASHSEED=0` makes the build
bit-reproducible. The earlier Wayland-patched binary had 0/120 hangs (`data/diagnostics/godotpatch-2026-10-01/loop-anim-seeded.txt`). No fallback to the official binary. The patch
for the Wayland hang touches only `platform/linuxbsd/wayland/`; the render-thread fixes also apply on Windows.

**Retirement.** Drop the patch, build script and pin, and return to an official release
download, once a Godot release contains #123946.
