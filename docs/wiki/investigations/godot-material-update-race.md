# Godot RD material update-list race

In the 4.7.2 client, rendering can stop after corruption of the RD material
update list when the separate rendering thread is enabled. A captured live
failure stopped in `SelfList::List::remove` while the main thread was creating
a shader material. This identifies the failing list operation, not by itself
the original corrupting write.

The pinned source locks `material_update_list_mutex` when enqueueing and when
transferring pending elements, but transfers the same intrusive elements into
a temporary `SelfList` and removes them there without the mutex. Another
thread can see an element as already queued while it belongs to that temporary
list. The `SelfList` destructor in `material_free` can also remove a queued
element without the mutex. `scripts/godot/material-update-list-mutex.patch`
replaces the temporary list with a pointer vector and explicitly removes a
freed material under the mutex. Material updates and frees remain on the
rendering-server thread; the vector does not extend material lifetime.

The texture, mesh and particles RD storage audit found no identical protected
queue transfer pattern. Texture storage has no SelfList; mesh and particles
use different render-side update lists and were not changed. This is not a
proof that those storages have no other concurrency bugs.

## Evidence

`scripts/godot/material-race/` is a standalone separate-render-thread stress
project: four threads create and release 40,000 ShaderMaterials per round, for
100 rounds, while frames advance. It runs on the assertion-instrumented
developer build (`dev_build=yes` plus the fatal linkage assertions from the
Stonevault diagnostic's `instrument-self-list.py`; stock SelfList checks are
nonfatal) under headless cage, because it needs a real RenderingDevice.

| Build | Runs | Result |
|---|---|---|
| Unpatched 4.7.2 dev | 3 | 3/3 exit 134 after 72–138 s: two `remove` (self_list.h:91, via `material_free`) and one `add` (self_list.h:49) linkage assertions |
| Patched dev (same source + this patch) | 3 | 3/3 exit 0, 4,000,000 materials in 146–148 frames, no assertion |
| Previous production pin `4.7.2-pr123946-pr123546-pr123391` | 1 | hang, killed by 300 s timeout (exit 124); 32,381,490 `self_list.h` errors (`p_elem->_root != this`, `p_elem->_root`), the live Stonevault symptom |
| New production editor `…-matlist-bonepose` | 1 | exit 0, 4,000,000 materials in 155 frames, zero `self_list` errors |

The Windows MinGW editor and release template (`scripts/windows-worker.py`) apply the
same patch, which applied cleanly to the Windows source tree; the Windows template is pinned
separately. The stress test was not run on Windows.

The unpatched `add`/`remove` pair matches the live Stonevault log. A single
40,000-material round passed without the patch, so smaller runs are not proof.
Logs: `data/diagnostics/godot-matlist-2026-10-31/`. The live client hang has
not been re-run on the patched build.

## Sources

- `scripts/godot/material-update-list-mutex.patch` — proposed queue correction.
- `scripts/godot/material-race/` — standalone stress reproduction and invocation.
- `/home/osso/.worktrees/logs/defect-selflist-hang.md` — original failure and stack provenance.
- `/home/osso/.worktrees/windrunner-walk-run/client47.stacks.txt` — live thread stacks.

## See also

- [Wayland shutdown hang and patched Godot build](godot-wayland-exit-hang.md)
- [M2 animation transform errors](m2-nonfinite-transforms.md) — full bone override patch in the same pin.
