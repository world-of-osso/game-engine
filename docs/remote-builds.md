# Remote Godot extension builds

Root `cargo run` and `rd` compile only the tiny std-only launcher locally. The launcher runs `python3 scripts/depot-build.py --root <checkout>` to compile the Godot native extension remotely, then launches/imports Godot locally as usual.

## Requirements

- Linux x86_64 host.
- Authenticated `depot` CLI, Python 3, and Git.
- Depot project `local-builds` (`003c4ttwqh`) in the existing Globalcomix organization, overridden only with `DEPOT_PROJECT_ID`.

## Usage

```sh
cargo run -- --screen charselect
# Build/download only, without launching:
python3 scripts/depot-build.py --root "$PWD"
# Build the extension and one owned UDP fixture executable:
python3 scripts/depot-build.py --root "$PWD" --fixture native_input_fixture
# Alternatively: --fixture native_npc_visual_fixture
```

Each worktree needs the matching sibling repositories beside it: `asset-resolver`, `ui-toolkit-godot-conversion`, `ui-toolkit-macros`, `shared-protocol`, and `bevy-patches`. The worktree itself can have any directory name. Source-file symlinks and symlinked `target`/`target/debug` directories fail explicitly; the helper never deletes existing targets. Use a checkout-local artifact directory rather than a shared target symlink.

## Build boundary

The helper uploads a source-only snapshot: tracked inputs, nonignored untracked compile inputs, and matching sibling repositories. It excludes `data/`, secrets, targets, and Git metadata.

Remote registry, Git, and Cargo target caches are shared; the target cache uses `sharing=locked`. Source snapshots retain original file timestamps (`copy2`); after acquiring the remote target lock, the build refreshes staged compile-input timestamps before Cargo runs. This prevents an older snapshot from another worktree being treated as unchanged against a newer target. The reproduced failure was a build from worktree A, then older worktree B, then A again: A's correctly snapshotted source hit stale shared dependency artifacts and failed with `E0425`. Lock-held refresh made the same A/B/A sequence recompile dependencies and pass. It does not touch mounted targets or dependency caches. Worktrees never receive a target cache. By default, each worktree gets only `target/debug/libgame_engine_godot.so`, installed atomically after a lossless gzip download. `--fixture` also builds exactly one allowlisted `game-engine-network` example after the lock-held source refresh and installs its decompressed executable at `target/debug/examples/<name>`; no target cache is downloaded. Run that executable from the same checkout. `native_input_fixture sound-click` and `native_input_fixture reset-windows` launch pinned Godot directly (or `GODOT_BIN`); other input modes retain their root-launcher requirement, which needs a separate lightweight root launcher build if absent. The NPC visual fixture launches Godot directly.

Remote build or download failures fail explicitly. There is no local Cargo fallback for the extension. Godot import and launch stay local and unchanged.

## Cost and performance

Verified September 29, 2026: `local-builds` uses the user-selected **200 GB per-architecture cache cleanup target** (214,748,364,800 bytes) and **14-day stale retention**. Cache is shared across worktrees, not allocated per worktree. Depot evicts old cache above its target; this is not a hard storage or spending cap. Organization billing controls and the existing `default` project remain unchanged.

The willingness to spend up to $100/month is conditional, not an automatic billing cap. This project uses the existing company account; the earlier $20 personal-plan estimate does not describe that account. Monitor attributable project usage and organization billing. The helper itself never changes account settings or cache limits.

On September 29, 2026, one warm constant-change benchmark measured 13.321 s before source freshness refresh; it does not predict cross-worktree correctness or post-refresh latency. The first fully refreshed Options build passed in 49.274 s on project `003c4ttwqh`, build `cpw7crx3ww`, installing `target/debug/libgame_engine_godot.so` (272,736,240 bytes; SHA-256 `a7ca8a89798d8024a66be9cc9044f200cedb077329047a1af7d1fc3cdbcd0b0b`). Verifier 954 then ran pinned Godot 4.7.2 headlessly with isolated XDG paths: `GameClient` registered through `ClassDB`, instantiated as `Node3D`, and attached to a scene tree. This is bounded extension-load proof, not full conversion proof. Cleanup reported 98.28 GB allocated cache removed, but immediately available disk space fell by 0.754 GB because of concurrent activity/shared extents; it is not evidence of 98 GB physical space reclaimed. These are observations, not latency, storage, or cost guarantees. Build output prints the target cache's current byte size.

## Agent rule

Build the Godot extension only through the Depot launcher/helper. Do not invoke local extension Cargo or recreate a bulk target cache unless explicitly asked. Lightweight launcher tests remain allowed.

Proof: `/tmp/claude/verify-depot-options-migration.md`.

See [Godot conversion](specs/godot-conversion.md) and the [conversion wiki](wiki/systems/godot-conversion.md).
