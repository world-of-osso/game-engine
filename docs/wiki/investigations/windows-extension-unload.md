# Windows extension unload race

Native Windows bundle import completes, then exits with `0xC0000005`. Background catalog workers can execute extension code after Godot unmaps the DLL.

## Evidence — 2026-10-11

`data/diagnostics/windows-release-2026-10-11/lldb-events-full-4.log` captures the first exception: worker20960 executes an unmapped Rust allocation function (DEP violation). Main thread10820 is in `FreeLibrary` / `LdrUnloadDll`. Other stacks show item-catalog warmup and spell-visual catalog parsing still in the DLL. LLDB launch uses `_NO_DEBUG_HEAP=1` to preserve normal heap performance; default debug heap makes CSV parsing exceptionally slow.

## Fix

`game-engine-core::worker_shutdown` retains detached join handles and joins them at the extension's Core deinitialization, after client owners close their request queues. AssetLoader and detached catalog jobs register immediately. BackgroundLoad, AssetStartup and terrain streaming transfer unconsumed handles on Drop. Normal owner drops stay nonblocking; existing IPC shutdown already joins its worker.

The shutdown barrier must wait for thread exit, not a task-completed flag: thread return and result destructors still execute DLL code. Worker panics are reported. Core shutdown repeats draining because a worker can register another worker while being joined.

## Proof boundary

RED: real staged full-asset Windows import and LLDB exception above. GREEN and private login/in-world acceptance remain pending at implementation commit. NativeSound's non-tree node leak is separate and was not changed.

## Sources

- [Extension lifecycle](../../../godot/rust/src/lib.rs)
- [Shutdown barrier](../../../godot/core/src/worker_shutdown.rs)
- [Windows worker](../../../scripts/windows-worker.py)

## See Also

- [Client deployment](../../deploy.md)
- [[shipped-assets]] — extracted-only startup policy
