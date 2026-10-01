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
# Build the extension and one owned UDP fixture executable (any godot/network/examples/*.rs stem):
python3 scripts/depot-build.py --root "$PWD" --fixture native_input_fixture
```

`--fixture` accepts every top-level `godot/network/examples/*.rs` file stem (`native_input_fixture`, `native_npc_visual_fixture`, `native_reconnect_fixture`, `native_transfer_fixture`, and any new one); the helper validates the name and the Dockerfile builds it. Fixtures locate their checkout from the installed `target/debug/examples/<name>` path.

The helper runs `depot` with `DEPOT_TOKEN` when set; otherwise with the first `depot/depot.yaml` login found in `$XDG_CONFIG_HOME`, then `~/.config`, and fails before uploading when neither has one. Fixtures that isolate `XDG_CONFIG_HOME` for Godot therefore still reach the user's login through the root launcher. The token is never printed.

Each worktree needs the matching sibling repositories beside it: `asset-resolver`, `ui-toolkit-godot-conversion`, `ui-toolkit-macros`, `shared-protocol`, and `bevy-patches`. `DEPOT_SIBLING_<NAME>` (name upper-cased, `-` as `_`) points one of them elsewhere, for example `DEPOT_SIBLING_SHARED_PROTOCOL=/home/osso/.worktrees/shared-protocol-visage` for a protocol branch. The worktree itself can have any directory name. Source-file symlinks and symlinked `target`/`target/debug` directories fail explicitly; the helper never deletes existing targets. Use a checkout-local artifact directory rather than a shared target symlink.

## Build boundary

The helper uploads a source-only snapshot: tracked inputs, nonignored untracked compile inputs, and matching sibling repositories. It excludes `data/`, secrets, targets, and Git metadata.

Remote registry and Git caches are shared. The Cargo target cache is per checkout (`godot-target-<sha256(checkout path)[:20]>`, `sharing=locked`); Cargo decides freshness by mtime, so checkout-specific target state is never reused by another worktree. Source snapshots retain original file timestamps (`copy2`); after acquiring the checkout target lock, the build refreshes staged compile-input timestamps before Cargo runs. This retains the lock-held freshness protection from the earlier A/B/A stale-artifact diagnosis without sharing target artifacts between diverging checkouts. Stable Cargo cannot use `-Zchecksum-freshness`. Worktrees never receive a target cache. By default, each worktree gets only `target/debug/libgame_engine_godot.so`, installed atomically after a lossless gzip download. `--fixture` also builds exactly one `game-engine-network` example after the lock-held source refresh and installs its decompressed executable at `target/debug/examples/<name>`; no target cache is downloaded. Run that executable from the same checkout. `native_input_fixture` modes `menu`, `sound`, `sound-click`, `sound-outcome`, `merchant-click`, `loot`, `settings-reload`, `footsteps`, `reset-windows`, `portal-particles-enabled`, `portal-particles-disabled`, and `portal-density` launch pinned Godot directly (or `GODOT_BIN`), with client flags after `--`. Menu intentionally routes `--screen charselect` and tests the already-built extension without invoking the root launcher or Depot inside its isolated `XDG_CONFIG_HOME`. Menu seeds owned `canonical()` keybinding defaults so inherited legacy bindings cannot change its W movement assertion; user legacy data remains untouched. Other input modes retain their root-launcher requirement, which needs a separate lightweight root launcher build if absent. The NPC visual fixture launches Godot directly.

`settings-reload` saves authored Options in one authenticated native process and checks the same owned canonical file through camera, TargetSelf and AutoLoot consumers in a fresh process. Both processes deliberately terminate at markers via owned SIGKILL/reap/reader join; not normal-shutdown coverage. See [accepted bounded saved-artifact PASS](wiki/systems/godot-conversion.md#native-settingsreload--bounded-two-process-proof).

The `loot` mode targets loot UI/network behavior and the four AutoLoot/Shift combinations. Existing full four-case bounded proof remains valid for its original fields, including inventory/currency and Options checks; it does not prove the newly prepared reach probes. Test-only `76e5bd5c` / `6b1468e2` add `native_input_fixture/loot_range.rs` and `godot/tests/loot_reach_probe.gd`: fresh authoritative NPCs at 5.1 yards (no `LootUnit`), 4.9 and 5.0 (one manual open and real close-button release each, no awards), and friendly living 5.0 (one `InteractNpc`, no `LootUnit`). They use actual posed-mesh clicks and strict phase counts. Fresh spawn avoids interpolation missing the exact boundary; observed player pose arranges input, not expected reach. The original `target.rs` five-yard constant and `> 5` interaction guard supply the inclusive policy reference; production receiver behavior is unchanged. First Depot build and GPU run of these probes are pending: preparation is not runtime proof. See [loot reach scope](specs/loot-frame.md#prepared-native-reach-probes-not-runtime-proof). Source-hash pinning is not an acceptance gate.

Isolated `merchant-click` assembly must link the authored `rendering/` subtree alongside scenes/shaders/tests/UI. Fixture-only `54eaef69` corrects that required-link omission; fresh Depot fixture build and merchant retry remain pending. The prior exit101 occurred before login, not in vendor behavior; native5a startup actions proof remains valid, combined gate OPEN. See [resource-gap evidence and proof boundary](wiki/systems/godot-conversion.md#isolated-merchant-fixture-rendering-gap--corrected-source-proof-pending).

Local CASC startup initialization requires a complete active-build resolution cache in the fixture's Godot user-data. The observed worker writes that cache; warm setup reused only cached tables, without source pinning. Interrupted cold fixture startups are not loot RED evidence.

On September 29, 2026, `--fixture native_input_fixture` passed remotely in project `003c4ttwqh`, build `5nqxfxrzpt`: 206.04 s total (204.6 s remote; 205.9 s installed). It installed `target/debug/libgame_engine_godot.so` and `target/debug/examples/native_input_fixture` under the originating checkout. That installed fixture then passed local owned-UDP `sound-click` in 30.787 s (`/home/osso/.worktrees/.game-engine-options-depot-20260929/{fixture-build,exported-fixture-runtime}.log`). This is bounded build/export and fixture behavior proof, not a performance guarantee or conversion parity. Default invocation remains library-only. `native_npc_visual_fixture` is allowlisted but was not remotely built or run in this record.

## Remote tests

`python3 scripts/depot-build.py --root "$PWD" --test <cargo test args...>` runs `cargo test --locked` in the `godot/` workspace on Depot with every argument after `--test` (including `--`), prints the log tail and every `Running`/`test result`/`error`/`FAILED` line, saves the full log to `target/depot-test.log`, and exits with cargo's status; it installs nothing else. It uses the same snapshot, checkout lock and target cache as builds. Depot has no GPU and no Godot engine.

Test data comes only from `godot/depot-test-assets.txt`: data/-relative file paths, validated locally (missing or escaping entries fail before upload), reflinked (else hardlinked, else copied) into the snapshot's `test-assets/`, excluded from the image `COPY`, and bind-mounted writable at the remote `data/` (plus the `godot/core/data` symlink). Remote writes, such as the outfit tests' `data/cache/outfit_links.sqlite`, stay in the build container; local `data/` was byte-identical before and after two runs. A test needing an unlisted file fails remotely with its own missing-path error. Add files to the manifest rather than uploading `data/`. Measured September 29, 2026: staging 131 files (605 MiB) takes 0.3 s by reflink, and Depot re-uploads them in full every run (645 MB context, 21.5–23.2 s on consecutive runs, both as a stable named context and inside the main context). On September 30, 2026 the manifest grew to 1,801 files (1,271 MiB) so every `game-engine-godot --lib` test runs: staging took 1.7 s and the 1.36 GB context upload 108 s of a 179 s run. The added entries are the files those tests open, traced with `strace` from a Depot-built test binary run locally; the largest users are the `wmo` (431 MiB), `assets` (124 MiB), `terrain` (97 MiB), `ground` (83 MiB) and `animation` (74 MiB) test modules. `local-listfile-cache.sqlite` answers the resolver's FDID/path lookups; `community-listfile.csv` is read directly by the outfit model-path tests, and its presence makes the resolver build its community cache remotely. A local run of that binary with only the manifest's files and no CASC install passes all 294 tests.

Results on September 29, 2026 (branch `depottest`):

| Crate | Result | Not runnable remotely |
|---|---|---|
| `game-engine-core` (lib + 37 `tests/`) | 569 passed, 0 failed | none |
| `game-engine-ui-model` | 114 passed, 1 failed | `authored_customize_mode_keeps_dropdown_choices_name_and_postsetup`: ui-toolkit loads FrizQuadrata from the absolute host path `/home/osso/Projects/wow/wow-ui-sim/fonts/FRIZQT__.TTF` (fixed September 30: `set_data_root` sets ui-toolkit's font directory to `data/fonts`) |
| `game-engine-session` | 28 passed | none |
| `game-engine-network` | 14 passed (loopback UDP only) | none |
| `game-engine-godot --lib` | 203 passed, 64 failed | the 64 need the asset resolver (local CASC extraction from a WoW install), `community-listfile.csv` resolution, or streamed terrain/WMO data; not listed in the manifest (fixed September 30: 294 passed, 0 failed) |

`game-engine-godot --lib` compiles and runs without the engine: no test hit godot-rust's "engine not available" panic. GDScript tests under `godot/tests/` need Godot and are not covered.

Remote build or download failures fail explicitly. There is no local Cargo fallback for the extension. Godot import and launch stay local and unchanged.

## Cost and performance

Verified September 29, 2026: `local-builds` uses the user-selected **200 GB per-architecture cache cleanup target** (214,748,364,800 bytes) and **14-day stale retention**. The cleanup target covers checkout-specific target caches (about 6 GB each, measured September 29, 2026). Depot evicts old cache above its target; this is not a hard storage or spending cap. Organization billing controls and the existing `default` project remain unchanged.

The willingness to spend up to $100/month is conditional, not an automatic billing cap. This project uses the existing company account; the earlier $20 personal-plan estimate does not describe that account. Monitor attributable project usage and organization billing. The helper itself never changes account settings or cache limits.

On September 29, 2026, one warm constant-change benchmark measured 13.321 s before the source-freshness update; it is not a cross-worktree correctness or post-refresh latency guarantee. The first full refreshed Options build passed in 49.274 s on project `003c4ttwqh`, build `cpw7crx3ww`, installing `target/debug/libgame_engine_godot.so` (272,736,240 bytes; SHA-256 `a7ca8a89798d8024a66be9cc9044f200cedb077329047a1af7d1fc3cdbcd0b0b`). Verifier 954 then ran pinned Godot 4.7.2 headlessly with isolated XDG paths: `GameClient` registered through `ClassDB`, instantiated as `Node3D`, and attached to a scene tree. This is bounded extension-load proof, not full conversion proof. Cleanup reported 98.28 GB allocated cache removed, but immediately available disk space fell by 0.754 GB because of concurrent activity/shared extents; it is not evidence of 98 GB physical space reclaimed. These are observations, not latency, storage, or cost guarantees. Build output prints the target cache's current byte size.

## Agent rule

Build the Godot extension only through the Depot launcher/helper. Do not invoke local extension Cargo or recreate a bulk target cache unless explicitly asked. Lightweight launcher tests remain allowed.

Proof: `/tmp/claude/verify-depot-options-migration.md`.

See [Godot conversion](specs/godot-conversion.md) and the [conversion wiki](wiki/systems/godot-conversion.md).
