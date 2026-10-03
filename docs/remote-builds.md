# Desktop and local Godot extension builds

Root `cargo run` and `rd` compile only the tiny std-only launcher locally. The launcher runs `python3 scripts/depot-build.py --root <checkout>` on the selected `desktop` or `local` Docker build host, then launches/imports Godot locally as usual. The helper retains its filename; Depot is no longer required. This October 3, 2026 trial replaces the Depot-only contract; historical Depot evidence below remains evidence for its original runs, not proof of either new host.

`--build-host desktop|local` selects a host for one helper or launcher invocation; the launcher consumes it rather than forwarding it to Godot. Without an explicit host, the helper reads `~/.config/game-engine/build-host`, independent of runtime `XDG_CONFIG_HOME` isolation. Missing or invalid defaults fail explicitly; no host fallback.

Client options `--screen`, `--state`, `--server`, `--char`, and `--run-js-ui-script <path>` are routed after Godot's `--` separator; native Godot options stay before it. Direct Godot invocation must place client options after `--`. The script uses the shared synchronous JS compiler and native frame-driven consumer; see the [native UI automation contract](specs/native-ui-automation.md) and [bounded Login evidence and open gates](wiki/systems/godot-conversion.md#native-js-automation--bounded-login-green-overall-gate-fail). Routing is not all-action or full-feature acceptance.

## Requirements

- Linux x86_64 caller with Python 3 and Git.
- `local`: Docker/buildx access on the caller; use this host when the desktop is occupied by gaming.
- `desktop`: SSH alias `desktop`, reaching the `OssoBuild` Ubuntu 24.04 WSL distribution as root, with Docker/buildx available there.
- Both hosts use the named `game-engine` buildx builder for Linux amd64. Builder limits: 8 CPUs/16 GB; desktop WSL limits: 8 processors/20 GB. Builders have been created; provisioning and real build/test acceptance are still pending.

## Usage

```sh
# Save the default (choose desktop, or local while gaming):
python3 scripts/depot-build.py --save-build-host desktop
python3 scripts/depot-build.py --save-build-host local
# One-shot override; does not change the saved default:
cargo run -- --build-host desktop --screen charselect
python3 scripts/depot-build.py --root "$PWD" --build-host local
# Build/install only, without launching:
python3 scripts/depot-build.py --root "$PWD"
# Build the extension and one owned UDP fixture executable (any godot/network/examples/*.rs stem):
python3 scripts/depot-build.py --root "$PWD" --fixture native_input_fixture
# Optimized library only, into target/release/ (used by ./deploy.sh; not combinable with --fixture/--cli/--test):
python3 scripts/depot-build.py --root "$PWD" --release
# Also export the IPC client to target/debug/game-engine-cli (with or without --fixture):
python3 scripts/depot-build.py --root "$PWD" --cli
```

`--fixture` accepts every top-level `godot/network/examples/*.rs` file stem (`native_input_fixture`, `native_npc_visual_fixture`, `native_reconnect_fixture`, `native_transfer_fixture`, and any new one); the helper validates the name and the Dockerfile builds it. Fixtures locate their checkout from the installed `target/debug/examples/<name>` path.

`--fixture`, `--cli`, `--release`, and `--test` retain their arguments and source/assets behavior on both hosts. For CPU tests, use `python3 scripts/depot-build.py --root "$PWD" --build-host desktop --test -p game-engine-core` (substitute `local` as needed). Host selection does not make GDScript or GPU tests part of `--test`.

## Desktop runtime capability boundary

Approved scope includes desktop server execution, tests, and GPU-backed manual client testing, but build-host selection is not runtime orchestration. Desktop server startup/network access, runtime test execution, and manual client workflows remain pending; no working desktop runtime command is asserted here. The newly provisioned desktop has an i7-13700 (16 cores/24 threads), 32 GB RAM, and RTX 4070 Ti. Current WSL GPU probes report software `llvmpipe`, not RTX-backed rendering; hardware presence is not GPU acceptance. No real desktop/local build, test, or GPU pass is claimed by this trial documentation.

Each worktree needs the matching sibling repositories beside it: `asset-resolver`, `ui-toolkit-godot-conversion`, `ui-toolkit-macros`, `shared-protocol`, and `bevy-patches`. `DEPOT_SIBLING_<NAME>` (name upper-cased, `-` as `_`) points one of them elsewhere, for example `DEPOT_SIBLING_SHARED_PROTOCOL=/home/osso/.worktrees/shared-protocol-visage` for a protocol branch. The worktree itself can have any directory name. Source-file symlinks and symlinked `target`/`target/debug` directories fail explicitly; the helper never deletes existing targets. Use a checkout-local artifact directory rather than a shared target symlink.

## Build boundary

The helper uploads a source-only snapshot: tracked inputs, nonignored untracked compile inputs under the checkout's `godot/` (the only checkout path any godot crate reads), and matching sibling repositories. It excludes `data/`, secrets, targets, and Git metadata.

Registry and Git caches are shared within each selected builder; caches are not transferred between hosts. The Cargo target cache is per checkout (`godot-target-<sha256(checkout path)[:20]>`, `sharing=locked`); Cargo decides freshness by mtime, so checkout-specific target state is never reused by another worktree. Source snapshots retain original file timestamps (`copy2`); after acquiring the checkout target lock, the build refreshes staged compile-input timestamps before Cargo runs. This retains the lock-held freshness protection from the earlier A/B/A stale-artifact diagnosis without sharing target artifacts between diverging checkouts. Stable Cargo cannot use `-Zchecksum-freshness`. Worktrees never receive a target cache. By default, each worktree gets only `target/debug/libgame_engine_godot.so`, installed atomically after a lossless gzip download. `--fixture` also builds exactly one `game-engine-network` example after the lock-held source refresh and installs its decompressed executable at `target/debug/examples/<name>`; no target cache is downloaded. Run that executable from the same checkout. `native_input_fixture` modes `menu`, `sound`, `sound-click`, `sound-outcome`, `merchant-click`, `merchant-cursor`, `merchant-services`, `loot`, `settings-reload`, `footsteps`, `reset-windows`, `portal-particles-enabled`, `portal-particles-disabled`, and `portal-density` launch pinned Godot directly (or `GODOT_BIN`), with client flags after `--`. Menu intentionally routes `--screen charselect` and tests the already-built extension without invoking the root launcher or Depot inside its isolated `XDG_CONFIG_HOME`. Menu seeds owned `canonical()` keybinding defaults so inherited legacy bindings cannot change its W movement assertion; user legacy data remains untouched. Other input modes retain their root-launcher requirement, which needs a separate lightweight root launcher build if absent. The NPC visual fixture launches Godot directly.

`settings-reload` saves authored Options in one authenticated native process and checks the same owned canonical file through camera, TargetSelf and AutoLoot consumers in a fresh process. Both processes deliberately terminate at markers via owned SIGKILL/reap/reader join; not normal-shutdown coverage. See [accepted bounded saved-artifact PASS](wiki/systems/godot-conversion.md#native-settingsreload--bounded-two-process-proof).

The `loot` mode targets loot UI/network behavior and the four AutoLoot/Shift combinations. Existing full four-case bounded proof remains valid for its original fields, including inventory/currency and Options checks; it does not prove the newly prepared reach probes. Test-only `76e5bd5c` / `6b1468e2` add `native_input_fixture/loot_range.rs` and `godot/tests/loot_reach_probe.gd`: fresh authoritative NPCs at 5.1 yards (no `LootUnit`), 4.9 and 5.0 (one manual open and real close-button release each, no awards), and friendly living 5.0 (one `InteractNpc`, no `LootUnit`). They use actual posed-mesh clicks and strict phase counts. Fresh spawn avoids interpolation missing the exact boundary; observed player pose arranges input, not expected reach. The original `target.rs` five-yard constant and `> 5` interaction guard supply the inclusive policy reference; production receiver behavior is unchanged. First Depot build and GPU run of these probes are pending: preparation is not runtime proof. See [loot reach scope](specs/loot-frame.md#prepared-native-reach-probes-not-runtime-proof). Source-hash pinning is not an acceptance gate.

Isolated `merchant-click` assembly must link the authored `rendering/` subtree alongside scenes/shaders/tests/UI. Fixture-only `54eaef69` corrects that omission; Depot `vjh7m1nggb` exits0 and retry removes TAA errors but hits an unexplained pre-auth15-second timeout. Diagnostic merchant run passes at Loading4162ms; `51a3e85f` restores15 seconds with timing retained. MAIN accepts independent1429 bounded startup/rendering/merchant PASS; native5a proofs retained. Auth timeout remains unexplained; one ObjectDB leak warning prevents clean-resource claims. Main goal remains OPEN. See [resource-gap evidence and proof boundary](wiki/systems/godot-conversion.md#isolated-merchant-fixture-rendering-gap--corrected-source-proof-pending).

Local CASC startup initialization requires a complete active-build resolution cache in the fixture's Godot user-data. The observed worker writes that cache; warm setup reused only cached tables, without source pinning. Interrupted cold fixture startups are not loot RED evidence.

On September 29, 2026, `--fixture native_input_fixture` passed remotely in project `003c4ttwqh`, build `5nqxfxrzpt`: 206.04 s total (204.6 s remote; 205.9 s installed). It installed `target/debug/libgame_engine_godot.so` and `target/debug/examples/native_input_fixture` under the originating checkout. That installed fixture then passed local owned-UDP `sound-click` in 30.787 s (`/home/osso/.worktrees/.game-engine-options-depot-20260929/{fixture-build,exported-fixture-runtime}.log`). This is bounded build/export and fixture behavior proof, not a performance guarantee or conversion parity. Default invocation remains library-only. `native_npc_visual_fixture` is allowlisted but was not remotely built or run in this record.

## Native merchant cursor fixture

After building/installing the matching native extension and `native_input_fixture` through the helper above, invoke from this checkout:

```sh
target/debug/examples/native_input_fixture merchant-cursor
```

MAIN `3cb53ab4` registers independent fixture dispatch and direct pinned Godot launch (or `GODOT_BIN`), client flags after `--`, `world_merchant_cursor_flow.gd`, and the existing owned vendor spawn. `merchant-cursor` is not a launcher `--screen` destination. Test-only `d825108c`/`2c00a751` prepares physical vendor pickup → own embedded bag0/slot0 with exact decoded Buy and client COMMIT before peer inventory/gold updates. MAIN-accepted independent gate1446 bounded PASS at `837e2c1e`/`b073e4dd`: scoped functional/source/format/readability and matching build/five-flow evidence accepted. Historical authenticated pickup RED retained. Invocation documentation is not GREEN evidence. Cleanup deliberately kills/reaps the owned child, not normal shutdown. Existing merchant-click and bags modes are unchanged. [Exact oracle and exclusions](wiki/systems/godot-conversion.md#native-merchant-cursor-buy--main-accepted-bounded-pass).

## Native merchant services fixture — MAIN-accepted bounded PASS

Existing `--fixture native_input_fixture` builds/installs this mode with the matching extension. Invoke from this checkout:

```sh
target/debug/examples/native_input_fixture merchant-services
```

`merchant-services` launches pinned Godot directly (or `GODOT_BIN`) with client flags after `--`; no root launcher required. It is an internal fixture mode, not a user startup destination.

MAIN independently accepted1513 **bounded PASS**, recorded in `158e523d`: direct Repair All and Sell All Junk without confirmation, exact decoded requests, client-before-authority barriers, authoritative results and quiet disabled repeats. See [authoritative acceptance, historical REDs, retained warnings and exclusions](wiki/systems/godot-conversion.md#native-direct-services--main-accepted-bounded-pass); that section owns the proof ledger. Cleanup deliberately kills/reaps the owned child: forced cleanup is not normal shutdown proof. Full conversion remains **OPEN**.

## CPU tests and historical Depot evidence

`python3 scripts/depot-build.py --root "$PWD" --test <cargo test args...>` runs `cargo test --locked` in the `godot/` workspace on the selected desktop/local host with every argument after `--test` (including `--`), prints the log tail and every `Running`/`test result`/`error`/`FAILED` line, saves the full log to `target/depot-test.log`, and exits with cargo's status; it installs nothing else. It uses the same snapshot, checkout lock and target cache as builds. This mode runs CPU Rust tests, not Godot/GPU runtime tests. The measurements and pass/failure records below are historical Depot evidence; they do not assert replacement-host passes.

Test data comes only from `godot/depot-test-assets.txt`: data/-relative file paths, validated locally (missing or escaping entries fail before upload), reflinked (else hardlinked, else copied) into the snapshot's `test-assets/`, excluded from the image `COPY`, bind-mounted read-only at `/test-assets`, and copied to the remote `data/` (plus the `godot/core/data` symlink) for the test run. Remote writes, such as the outfit tests' `data/cache/outfit_links.sqlite`, stay in the build container; local `data/` was byte-identical before and after two runs. A test needing an unlisted file fails remotely with its own missing-path error. Add files to the manifest rather than uploading `data/`. Measured September 29, 2026: staging 131 files (605 MiB) takes 0.3 s by reflink, and Depot re-uploads them in full every run (645 MB context, 21.5–23.2 s on consecutive runs, both as a stable named context and inside the main context). On September 30, 2026 the manifest grew to 1,801 files (1,271 MiB) so every `game-engine-godot --lib` test runs: staging took 1.7 s and the 1.36 GB context upload 108 s of a 179 s run. The added entries are the files those tests open, traced with `strace` from a Depot-built test binary run locally; the largest users are the `wmo` (431 MiB), `assets` (124 MiB), `terrain` (97 MiB), `ground` (83 MiB) and `animation` (74 MiB) test modules. `local-listfile-cache.sqlite` answers the resolver's FDID/path lookups; `community-listfile.csv` is read directly by the outfit model-path tests, and its presence makes the resolver build its community cache remotely. A local run of that binary with only the manifest's files and no CASC install passes all 294 tests. Since September 30, 2026 repeat uploads are incremental: the context is rebuilt at a fixed per-checkout path (`~/.cache/game-engine/depot-build/context-{build,test}-<checkout key>`), and the assets are no longer bind-mounted writable. BuildKit re-sent the whole context whenever its path changed (three runs from fresh temporary paths: 1.33 GB each) or after a writable bind of it (three runs of an unchanged fixed context: 1.36 GB each). Three consecutive `--test -p game-engine-godot --lib` runs then sent 1.36 GB (97.6 s), 16.3 MB (2.4 s) and 267 kB (0.3 s), finishing in 137 s, 48 s and 45 s.

Results on September 29, 2026 (branch `depottest`):

| Crate | Result | Not runnable remotely |
|---|---|---|
| `game-engine-core` (lib + 37 `tests/`) | 569 passed, 0 failed | none |
| `game-engine-ui-model` | 114 passed, 1 failed | `authored_customize_mode_keeps_dropdown_choices_name_and_postsetup`: ui-toolkit loads FrizQuadrata from the absolute host path `/home/osso/Projects/wow/wow-ui-sim/fonts/FRIZQT__.TTF` (fixed September 30: `set_data_root` sets ui-toolkit's font directory to `data/fonts`) |
| `game-engine-session` | 28 passed | none |
| `game-engine-network` | 14 passed (loopback UDP only) | none |
| `game-engine-godot --lib` | 203 passed, 64 failed | the 64 need the asset resolver (local CASC extraction from a WoW install), `community-listfile.csv` resolution, or streamed terrain/WMO data; not listed in the manifest (fixed September 30: 294 passed, 0 failed) |

`game-engine-godot --lib` compiles and runs without the engine: no test hit godot-rust's "engine not available" panic. GDScript tests under `godot/tests/` need Godot and are not covered.

Selected-host build or artifact-transfer failures fail explicitly. `local` is an explicit build host, not a Cargo fallback. Godot import and launch stay local and unchanged.

## Historical Depot cost and performance

Verified September 29, 2026: `local-builds` uses the user-selected **200 GB per-architecture cache cleanup target** (214,748,364,800 bytes) and **14-day stale retention**. The cleanup target covers checkout-specific target caches (about 6 GB each, measured September 29, 2026). Depot evicts old cache above its target; this is not a hard storage or spending cap. Organization billing controls and the existing `default` project remain unchanged.

The willingness to spend up to $100/month is conditional, not an automatic billing cap. This project uses the existing company account; the earlier $20 personal-plan estimate does not describe that account. Monitor attributable project usage and organization billing. The helper itself never changes account settings or cache limits.

On September 29, 2026, one warm constant-change benchmark measured 13.321 s before the source-freshness update; it is not a cross-worktree correctness or post-refresh latency guarantee. The first full refreshed Options build passed in 49.274 s on project `003c4ttwqh`, build `cpw7crx3ww`, installing `target/debug/libgame_engine_godot.so` (272,736,240 bytes; SHA-256 `a7ca8a89798d8024a66be9cc9044f200cedb077329047a1af7d1fc3cdbcd0b0b`). Verifier 954 then ran pinned Godot 4.7.2 headlessly with isolated XDG paths: `GameClient` registered through `ClassDB`, instantiated as `Node3D`, and attached to a scene tree. This is bounded extension-load proof, not full conversion proof. Cleanup reported 98.28 GB allocated cache removed, but immediately available disk space fell by 0.754 GB because of concurrent activity/shared extents; it is not evidence of 98 GB physical space reclaimed. These are observations, not latency, storage, or cost guarantees. Build output prints the target cache's current byte size.

## Agent rule

Build the Godot extension through the launcher/helper with `desktop` or `local`. Do not bypass it with direct extension Cargo or recreate a bulk target cache unless explicitly asked. Every agent-started local build/server/client/extraction still requires `agent-run`; existing resource and data-safety rules remain unchanged. Lightweight launcher tests remain allowed.

Historical Depot migration proof: `/tmp/claude/verify-depot-options-migration.md`.

See [Godot conversion](specs/godot-conversion.md) and the [conversion wiki](wiki/systems/godot-conversion.md).
