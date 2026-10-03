# Desktop and local build hosts

The October 3, 2026 trial replaces Depot-only extension builds with explicit desktop/local Docker transports. [Build commands, provisioning status, limits, and runtime/GPU boundaries](../../remote-builds.md) are the operational source of truth; [Godot conversion](../../specs/godot-conversion.md) owns the unverified build contract.

## Architecture

The root launcher consumes host selection and invokes `scripts/depot-build.py`; that filename remains unchanged. `scripts/build_hosts.py` supplies local Docker or desktop SSH/WSL transport to the named `game-engine` Linux amd64 buildx builder. Explicit selection overrides the saved user default; neither configuration errors nor host failures select another host.

The helper retains source-only snapshots, explicit test assets, checkout-specific locked target caches, and artifact export/install rather than exposing a shared Cargo target to worktrees. Each host has its own builder caches. Runtime Godot import/launch remains on the caller; choosing a build host does not move the server or client there.

## Evidence boundary

Both builders have been created; setup is in flight. Real replacement-host builds/tests and desktop server/manual runtime acceptance remain pending. WSL software rendering is not hardware-GPU proof. Historical Depot results retain their original scope and do not validate these transports.

## Sources

- [Build guide](../../remote-builds.md) — approved host workflow and pending capability boundary.
- [Build contract](../../specs/godot-conversion.md) — requirements and proof status.
- `scripts/depot-build.py`, `scripts/build_hosts.py`, `launcher/` — implementation surface under active integration.

## See Also

- [[godot-conversion]] — native client and retained historical evidence.
