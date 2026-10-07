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
- [BuildKit config](../../../scripts/depot/buildkitd.toml) — current aggregate GC policy.
- [Private live-run recipe](../../headless-live-run.md) — operational isolation and capture workflow.

## See Also

- [[godot-conversion]] — native client and retained historical evidence.
- [[desktop-disk-exhaustion]] — observed host/guest capacity and recovery boundary.
- [Warm-slot rule](../../remote-builds.md#warm-slot-rule) — fixed paths, branch reuse, cache identities and global build lock.
- [Private headless live client](../../headless-live-run.md) — isolated real-client runtime recipe.

## Builder GC policy

Verified source review: 2026-10-06. [`scripts/depot/buildkitd.toml`](../../../scripts/depot/buildkitd.toml) defines one aggregate `all = true` GC policy: `reservedSpace = "10GB"`, `maxUsedSpace = 100000000000` (100 GB in bytes), `minFreeSpace = "80GB"`, with no retention duration. This user-approved budget supersedes the former 400/450 GB warm-every-slot policy. GC is periodic, not a hard instantaneous cap. Persistent configuration deployment and long-term stability are not yet verified.

Historical October 5 diagnosis: default GC evicted per-slot Cargo target caches, fingerprints reported ENOENT, and gates rebuilt about 484 crates. Fixed slot reuse still avoids unnecessary cache identities; preserving every slot's warm cache is no longer the budget objective. [[desktop-disk-exhaustion]] records the subsequent observed host exhaustion and completed recovery.

When explicitly authorised to reprovision, `~/.worktrees/build-lock.sh scripts/setup-builder.sh` keeps the cache volume. Ordinary agents preserve the builder and follow the [warm-slot rule](../../remote-builds.md#warm-slot-rule); this is not an instruction to recreate it during a run.
