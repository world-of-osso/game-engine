# Godot Vulkan present thread

On Linux the Stormwind frame was the sum of main-thread and render-thread time, not
the larger of the two as on Windows. The render thread blocked inside
`vkQueuePresentKHR` for about one GPU frame, and the main thread then waited in
`RenderingServer::sync()` for the render thread.

## Cause

An instrumented `lto=none` Godot timed the waits per frame. Indoor pose, landed
extension, Linux cage, RTX 5060 Ti (`data/diagnostics/fps-120-sync-waits/`,
`fps-120-vsync-waits/`):

| Wait | ms/frame |
|---|---|
| main thread in `RenderingServer::sync()` (one call per frame) | 8–14 |
| render thread `vkQueuePresentKHR` | 5.5–6.4 |
| render thread idle for commands | 1.4–4.2 |
| frame fence wait / swap chain acquire | ≤ 0.02 |

Present blocked for the same time in FIFO and in MAILBOX, which the driver accepted
without fallback. Godot's Wayland frame-callback throttle (`emulate_vsync`) is off when
VSync is disabled. Cage 0.3.1 (wlroots 0.20) does not advertise
`wp_linux_drm_syncobj_manager_v1`; without explicit sync, NVIDIA's Wayland WSI waits in
present for the frame's GPU work
([NVIDIA forum](https://forums.developer.nvidia.com/t/vulkan-wayland-vkqueuepresentkhr-waits-for-gpu-to-finish/302203)).
niri 26.04 also lacks explicit sync (issue #843, PR #3956 unmerged). No upstream Godot
change moves present off the render thread to pin instead.

## Fix

`scripts/godot/vulkan-present-thread.patch` (in `drivers/vulkan/` only) moves
`vkQueuePresentKHR` to a "Vulkan present" thread with at most one present in flight. The
render thread waits for the previous present before the next present, before acquiring
a swap chain image (present and acquire must be externally synchronized on a swap
chain), before releasing a swap chain and before freeing a queue. The driver destructor
joins the thread. Out-of-date results still mark the surface for resize. Godot acquires
only when it blits to the screen, so culling and recording of the next frame overlap
the blocking present. The patch applies on Windows too.

## Evidence

Same server session, alternating instrumented builds, indoor pose, host shared
(`data/diagnostics/fps-120-present-waits/`):

| Run | Load | Frame mean ms (3 segments) | Present on render thread ms | Main sync wait ms |
|---|---|---|---|---|
| base | 31–33 | 35.8 / 25.1 / 21.9 | 6.42 | 9.4 |
| present thread | 23–24 | 13.8 / 14.0 / 13.4 | 0.07 | 2.6 |
| base | 32–37 | 36.9 / 41.3 / 45.5 | 6.1 | 13.0 |
| present thread | 35–36 | 13.9 / 15.4 / 14.7 | 0.06 | 2.4 |

The swap chain acquire then waits about 0.6 ms for the previous present. No run logged a
present error.

## Retirement

Drop the patch once Godot presents off the render thread, or once the supported Linux
compositors and NVIDIA driver implement explicit sync and present no longer waits for
GPU work.
