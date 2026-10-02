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

**Status.** The fix needs a patched Godot binary; the launcher pins the official release.
