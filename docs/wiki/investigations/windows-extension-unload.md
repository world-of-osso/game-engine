# Windows extension unload race

Native Windows bundle import completes, then exits with `0xC0000005`. Background catalog workers can execute extension code after Godot unmaps the DLL.

## Evidence — 2026-10-11

`data/diagnostics/windows-release-2026-10-11/lldb-events-full-4.log` captures the first exception: worker20960 executes an unmapped Rust allocation function (DEP violation). Main thread10820 is in `FreeLibrary` / `LdrUnloadDll`. Other stacks show item-catalog warmup and spell-visual catalog parsing still in the DLL. LLDB launch uses `_NO_DEBUG_HEAP=1` to preserve normal heap performance; default debug heap makes CSV parsing exceptionally slow.

## Fix

`game-engine-core::worker_shutdown` retains detached join handles and joins them at the extension's Core deinitialization, after client owners close their request queues. AssetLoader and detached catalog jobs register immediately. BackgroundLoad, AssetStartup and terrain streaming transfer unconsumed handles on Drop. Normal owner drops stay nonblocking; existing IPC shutdown already joins its worker.

The shutdown barrier must wait for thread exit, not a task-completed flag: thread return and result destructors still execute DLL code. Worker panics are reported. Core shutdown repeats draining because a worker can register another worker while being joined.

## Proof boundary

RED: real staged Windows import and LLDB exception above. Native DLL build and full player import GREEN at5ea978206. Warm-cache imports also let the old DLL exit0, so `unload-regression-2.log` tests early-quit Login instead: old DLL exits3221225477 in2.11s, fixed DLL exits0 in2.42s. Native shutdown unit test1/1 passes via the worker helper; other core unit fixtures cannot compile on Windows because12 references assume Unix symlinks/inodes. NativeSound's non-tree node leak is separate and was not changed.

## Product-directory packaging omission

Private Windows UDP54392 authenticated fb_winrelease, projected character30 Winrelease, and reached authoritative InWorld (zone12,117 remote entities,1 local player). PNGs show missing character/NPC visuals; errors name `data/products/wow/dcfc90fffd79ba00406ae46f5f657592/models/1011653.m2` and other build-qualified paths. Those files exist in canonical data but `player_bundle.py` omitted the whole `products` directory. Add it to the shared allowlist, preserving byte-identical assets and receipts. Observed addition:10146 files /3166762580 bytes. Package selection RED reproduces omission; visual recheck remains pending. No world.db writes: private server mounts canonical inputs read-only with bwrap.

## Sources

- [Extension lifecycle](../../../godot/rust/src/lib.rs)
- [Shutdown barrier](../../../godot/core/src/worker_shutdown.rs)
- [Windows worker](../../../scripts/windows-worker.py)

## See Also

- [Client deployment](../../deploy.md)
- [[shipped-assets]] — extracted-only startup policy
