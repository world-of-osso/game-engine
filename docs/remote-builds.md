# Remote Godot extension builds

Root `cargo run` and `rd` compile only the tiny std-only launcher locally. The launcher runs `python3 scripts/depot-build.py --root <checkout>` to compile the Godot native extension remotely, then launches/imports Godot locally as usual.

## Requirements

- Linux x86_64 host.
- Authenticated `depot` CLI, Python 3, and Git.
- Depot project `jnnl97r4s7`, overridden only with `DEPOT_PROJECT_ID`.

## Build boundary

The helper uploads a source-only snapshot: tracked inputs, nonignored untracked compile inputs, and matching sibling repositories. It excludes `data/`, secrets, targets, and Git metadata.

Remote registry, Git, and Cargo target caches are shared; the target cache uses `sharing=locked`. Worktrees never receive a target cache. Each worktree gets only `target/debug/libgame_engine_godot.so`, installed atomically after a lossless gzip download.

Remote build or download failures fail explicitly. There is no local Cargo fallback for the extension. Godot import and launch stay local and unchanged.

## Cost and performance

The $100 monthly ceiling is a conditional budget, not an automatic billing cap. Main must verify Depot's actual service limit and cache garbage collection; this guide makes no account or provider-cache claim. One warm constant-change benchmark measured 13.321 s; treat it as an observation, not a promise.

## Agent rule

Build the Godot extension only through the Depot launcher/helper. Do not invoke local extension Cargo or recreate a bulk target cache unless explicitly asked. Lightweight launcher tests remain allowed.

See [Godot conversion](specs/godot-conversion.md) and the [conversion wiki](wiki/systems/godot-conversion.md).
