# Depot cross-worktree freshness

A shared locked Cargo target cache could reuse dependency artifacts from a newer worktree for an older but correctly snapshotted source tree. Refreshing staged compile-input timestamps after acquiring the target lock forces Cargo to rebuild those warm dependencies and restores source/artifact consistency.

## Root cause and proof

The failure sequence was worktree A, then older worktree B, then A again. The final A build failed with `E0425` despite its exact source being present in the uploaded snapshot. The shared target cache's artifact freshness, not snapshot collection, was stale.

`e4b213a8` refreshes staged compile-input timestamps under the shared target lock. Repeating A/B/A then recompiled dependencies for every build and passed. The first full refreshed Options build passed in 49.274 s on Depot project `003c4ttwqh`, build `cpw7crx3ww`, installing `target/debug/libgame_engine_godot.so` (272,736,240 bytes; SHA-256 `a7ca8a89798d8024a66be9cc9044f200cedb077329047a1af7d1fc3cdbcd0b0b`). Verifier 954 then headlessly loaded pinned Godot 4.7.2 with isolated XDG paths, registered and instantiated `GameClient` as `Node3D`, and attached it to a scene tree. This proves class load and attachment only.

The earlier 13.321 s warm constant-change result predates the refresh and does not establish cross-worktree correctness or post-refresh performance. Cleanup reported 98.28 GB allocated cache removed; a concurrent/shared-extent measurement showed 0.754 GB less immediately free space, so it does not establish physical-space reclamation.

## Optional fixture export proof

At `be6aeedb`, Depot project `003c4ttwqh` build `5nqxfxrzpt` ran `scripts/depot-build.py --root <checkout> --fixture native_input_fixture` successfully in 206.042 s. It installed the default library plus `target/debug/examples/native_input_fixture` in the originating checkout; no target cache was installed. The directly launched installed fixture passed local owned-UDP `sound-click` in 30.787 s. This is one optional export and fixture-path result only. Default builds remain library-only; `native_npc_visual_fixture` is allowlisted but was not built or run. Verifier 963 remains pending.

## Sources

- [Remote Godot builds](../../remote-builds.md) — current helper boundary and recorded result.
- `data/diagnostics/depot-freshness/` — A/B/A reproduction results and logs.
- `/home/osso/.worktrees/.game-engine-options-depot-20260929/fixed-build.log` — successful refreshed build output.
- `/tmp/claude/verify-depot-options-migration.md` — verifier 954 installed-artifact and isolated Godot smoke proof.
- `/home/osso/.worktrees/.game-engine-options-depot-20260929/fixture-build.log` — build `5nqxfxrzpt`, exported artifacts, and timing.
- `/home/osso/.worktrees/.game-engine-options-depot-20260929/exported-fixture-runtime.log` — direct installed-fixture owned-UDP `sound-click` result.

## See Also

- [[godot-conversion]] — remote native extension build boundary.
