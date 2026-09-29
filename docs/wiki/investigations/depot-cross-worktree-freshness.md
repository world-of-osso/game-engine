# Depot cross-worktree freshness

A shared locked Cargo target cache could reuse dependency artifacts from a newer worktree for an older but correctly snapshotted source tree. Refreshing staged compile-input timestamps after acquiring the target lock forces Cargo to rebuild those warm dependencies and restores source/artifact consistency.

## Root cause and proof

The failure sequence was worktree A, then older worktree B, then A again. The final A build failed with `E0425` despite its exact source being present in the uploaded snapshot. The shared target cache's artifact freshness, not snapshot collection, was stale.

`e4b213a8` refreshes staged compile-input timestamps under the shared target lock. Repeating A/B/A then recompiled dependencies for every build and passed. The first full refreshed Options build passed in 49.274 s on Depot project `003c4ttwqh`, build `cpw7crx3ww`, installing `target/debug/libgame_engine_godot.so` (272,736,240 bytes; SHA-256 prefix `a7ca…`). Local extension-load verification is pending.

The earlier 13.321 s warm constant-change result predates the refresh and does not establish cross-worktree correctness or post-refresh performance. Cleanup reported 98.28 GB allocated cache removed; a concurrent/shared-extent measurement showed 0.754 GB less immediately free space, so it does not establish physical-space reclamation.

## Sources

- [Remote Godot builds](../../remote-builds.md) — current helper boundary and recorded result.
- `data/diagnostics/depot-freshness/` — A/B/A reproduction results and logs.
- `/home/osso/.worktrees/.game-engine-options-depot-20260929/fixed-build.log` — successful refreshed build output.

## See Also

- [[godot-conversion]] — remote native extension build boundary.
