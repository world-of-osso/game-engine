# Remote Godot extension builds

Root `cargo run` and `rd` compile only the tiny std-only launcher locally. The launcher runs `python3 scripts/depot-build.py --root <checkout>` to compile the Godot native extension remotely, then launches/imports Godot locally as usual.

## Requirements

- Linux x86_64 host.
- Authenticated `depot` CLI, Python 3, and Git.
- Depot project `jnnl97r4s7`, overridden only with `DEPOT_PROJECT_ID`.

## Usage

```sh
cargo run -- --screen charselect
# Build/download only, without launching:
python3 scripts/depot-build.py --root "$PWD"
```

Each worktree needs the matching sibling repositories beside it: `asset-resolver`, `ui-toolkit-godot-conversion`, `ui-toolkit-macros`, `shared-protocol`, and `bevy-patches`. The worktree itself can have any directory name. Source-file symlinks and symlinked `target`/`target/debug` directories fail explicitly; the helper never deletes existing targets. Use a checkout-local artifact directory rather than a shared target symlink.

## Build boundary

The helper uploads a source-only snapshot: tracked inputs, nonignored untracked compile inputs, and matching sibling repositories. It excludes `data/`, secrets, targets, and Git metadata.

Remote registry, Git, and Cargo target caches are shared; the target cache uses `sharing=locked`. Worktrees never receive a target cache. Each worktree gets only `target/debug/libgame_engine_godot.so`, installed atomically after a lossless gzip download.

Remote build or download failures fail explicitly. There is no local Cargo fallback for the extension. Godot import and launch stay local and unchanged.

## Cost and performance

The $100 monthly ceiling is a conditional budget, not an automatic billing cap. No account settings or cache limits are changed by the helper. Check the project's Cache Storage Policy and organization usage in Depot before sustained use; the actual account limit was not verified because dashboard authentication was unavailable. Sharing a target cache avoids duplication per worktree but does not enforce a storage ceiling.

On September 29, 2026, one warm constant-change benchmark measured 13.321 s. The first integrated build took 45.7 s, including machine startup and recompiling several project crates. Its shared Cargo target cache measured 6.44 GB (6.00 GiB); this excludes registry/Git caches and Docker layers. These are observations, not latency or cost guarantees. Build output prints the target cache's current byte size.

## Agent rule

Build the Godot extension only through the Depot launcher/helper. Do not invoke local extension Cargo or recreate a bulk target cache unless explicitly asked. Lightweight launcher tests remain allowed.

See [Godot conversion](specs/godot-conversion.md) and the [conversion wiki](wiki/systems/godot-conversion.md).
