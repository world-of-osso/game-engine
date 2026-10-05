# Desktop and local build hosts

The October 3, 2026 trial replaces Depot-only extension builds with explicit desktop/local Docker transports. [Build commands, provisioning status, limits, and runtime/GPU boundaries](../../remote-builds.md) are the operational source of truth; [Godot conversion](../../specs/godot-conversion.md) owns the build contract.

## Architecture

The root launcher consumes host selection and invokes `scripts/depot-build.py`; that filename remains unchanged. `scripts/build_hosts.py` supplies local Docker or desktop SSH/WSL transport to the named `game-engine` Linux amd64 buildx builder. Explicit selection overrides the saved user default; neither configuration errors nor host failures select another host.

The helper retains source-only snapshots, explicit test assets, checkout-specific locked target caches, and artifact export/install rather than exposing a shared Cargo target to worktrees. Each host has its own builder caches. Runtime Godot import/launch remains on the caller; choosing a build host does not move the server or client there.

## Evidence boundary

October 3, 2026: main observed real extension export from both hosts, desktop CLI export and 16 camera CPU tests. The fully staged desktop server fixture passed admin `pong` plus an authenticated disposable-account UDP roster. The unprivileged desktop GPU fixture passed native Forward+ Vulkan login input and IPC capture on RTX-backed test-only Dozen; main inspected the authored login image. These are bounded host-capability proofs, not head-pinned native/sibling snapshot acceptance.

[The build guide](../../remote-builds.md#desktop-runtime-capability-boundary) owns exact commands, resources, prerequisites, logs and exclusions. Independent followup accepted the bounded host trial; full-world/parity, audio and normal-shutdown acceptance remain unproved. Trial-owned runtimes stopped, but unrelated jobs may remain active. Historical Depot results retain their original scope.

## Sources

- [Build guide](../../remote-builds.md) — host workflow, observed capability, rerun prerequisites and open gates.
- [Build contract](../../specs/godot-conversion.md) — requirements and proof status.
- `scripts/depot-build.py`, `scripts/build_hosts.py`, `launcher/` — host transport and launcher implementation.

## See Also

- [[godot-conversion]] — native client and retained historical evidence.

## Builder GC policy (2026-10-05)
The shared `game-engine` buildx builder runs with `scripts/depot/buildkitd.toml`; recreate it with `~/.worktrees/build-lock.sh scripts/setup-builder.sh` (keeps the cache volume). BuildKit's default policy capped cache mounts at ~488 MiB and the builder at ~93 GiB while it held ~102 GiB, so GC evicted the per-slot Cargo target caches between runs: cargo fingerprints reported ENOENT and every gate rebuilt ~484 crates (gates 30-34). The config keeps cache mounts 30 days up to 250 GB, the builder up to 320 GB, and at least 100 GB disk free.
