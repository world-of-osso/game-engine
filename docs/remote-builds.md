# Desktop and local Godot extension builds

Root `cargo run` and `rd` compile only the tiny std-only launcher locally. The launcher runs `python3 scripts/depot-build.py --root <checkout>` on the selected `desktop` or `local` Docker build host, then launches/imports Godot locally as usual. The helper retains its filename; Depot is no longer required. This October 3, 2026 trial replaces the Depot-only contract; historical Depot evidence below remains evidence for its original runs, not proof of either new host.

`--build-host desktop|local|native` selects a host for one helper or launcher invocation; the launcher consumes it rather than forwarding it to Godot. Without an explicit host, the helper reads `~/.config/game-engine/build-host`, independent of runtime `XDG_CONFIG_HOME` isolation. Missing or invalid defaults fail explicitly; no host fallback.

Client options `--screen`, `--state`, `--server`, `--char`, and `--run-js-ui-script <path>` are routed after Godot's `--` separator; native Godot options stay before it. Direct Godot invocation must place client options after `--`. The script uses the shared synchronous JS compiler and native frame-driven consumer; see the [native UI automation contract](specs/native-ui-automation.md) and [bounded Login evidence and open gates](wiki/systems/godot-conversion.md#native-js-automation--bounded-login-green-overall-gate-fail). Routing is not all-action or full-feature acceptance.

## Requirements

- Linux x86_64 caller with Python 3 and Git.
- `local`: Docker/buildx access on the caller; use this host when the desktop is occupied by gaming.
- `desktop`: SSH alias `desktop`, reaching the `OssoBuild` Ubuntu 24.04 WSL distribution as root, with Docker/buildx available there.
- Both hosts use the named `game-engine` buildx builder for Linux amd64. Builder limits: 8 CPUs/16 GB; desktop WSL limits: 8 processors/20 GB. Both builders have exported real native extensions; desktop also exported the IPC CLI and ran 16 camera-related core tests on October 3, 2026.

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

`--fixture` accepts every top-level `godot/network/examples/*.rs` file stem (`native_input_fixture`, `native_npc_visual_fixture`, `native_reconnect_fixture`, `native_transfer_fixture`, and any new one); the helper validates the name and the Dockerfile builds it. Fixtures locate their checkout from the installed `target/debug/examples/<name>` path. `native_input_fixture` and `native_npc_visual_fixture` stage authored root CSVs while preserving generated fixture catalogs; SQL setup and WAL-safe listfile snapshots use bundled rusqlite, not a host `sqlite3` executable. Their private data trees live under canonical `data/`.

`--fixture`, `--cli`, `--release`, and `--test` retain their arguments and source/assets behavior on both hosts. For CPU tests, use `python3 scripts/depot-build.py --root "$PWD" --build-host desktop --test -p game-engine-core` (substitute `local` as needed). Host selection does not make GDScript or GPU tests part of `--test`.

## UI-model integration tests

UI-model suites share `tests/integration.rs` instead of separate Cargo binaries. Add new suites to that harness. Select one suite with `python3 scripts/depot-build.py --root "$PWD" --test -p game-engine-ui-model --test integration <suite>::`; the old `--test <suite>` target names no longer exist. [Measured rebuild impact and unchanged test inventory](wiki/investigations/native-dev-build-timings.md).

## UI icons prepared before shipping

Release builds run `scripts/prepare_ui_icons.py` on the originating developer checkout before native compilation. It enumerates all positive spell/trait/class/spec and source-local item icon metadata, writes `data/cache/required-ui-icons.json`, and uses the existing sibling `asset-resolver/target/debug/casc-local` to fill missing BLPs from local archives only. Source-table hashes and actual file hashes/sizes are recorded in `data/cache/ui-icon-provenance.json`; these cache files and textures are included by `deploy.sh`.

The shipped UI only reads these files. Missing/invalid required images stop release preparation with a full FDID list and extraction receipts under `data/diagnostics/ui-icon-preparation/`; no runtime CASC or question-mark substitution. `python3 scripts/tests/test_ui_icon_manifest.py` validates the declared extracted set. Debug builds and CPU tests remain available to diagnose incomplete source data without publishing it. This does not remove the existing runtime M2/CASC dependency; that separate model-isolation work remains open.

## Warm-slot rule

Verified source review: 2026-10-06, master `2f5f3e79`. Agents must reuse their assigned fixed worktree slots and switch branches in those slots. Never create new client/server worktrees, delete `target/`, prune builder caches, or recreate bulk target caches. Temporary **shared-protocol** worktrees are allowed: they supply source only and have no checkout target cache of their own.

The path is the cache identity, not the branch: `scripts/depot-build.py:261–269` hashes `os.fsencode(root)` with SHA-256 and keeps the first 20 hex characters; `scripts/build_hosts.py:32–46` passes `godot-target-<hash>` to BuildKit. The server helper adds `server-` to that checkout key and passes `server-target-<key>`, giving **`server-target-server-<hash>`** (`scripts/desktop-server-build.py:70–88,154–170`). A new slot path therefore starts another cache; switching branches at the same path preserves the warm slot. [Build boundary](#build-boundary) owns snapshot freshness, per-checkout locking and artifact export details.

## Native host (`native`, agent-server default since 2026-10-09)

Server agents use `scripts/desktop-server-build.py --root <server-slot> --build-host native --test --workspace --no-fail-fast -- --skip query_300k_listings_under_200ms`, or the server's `scripts/build-host.py --build-host native --test ...`. Both run host Cargo in the original server checkout with `CARGO_TARGET_DIR=<server-slot>/target` and the same native slots as the engine. Native tests read real checkout `data/`, not `desktop-server/test-data.txt` staging; server slots need a valid `data/world.db` link and the other fixtures they consume. Use `BUILD_HOST_SCRIPTS=<engine-slot>/scripts` for the server adapter when the engine checkout is not its sibling. Both server helpers reuse the engine's `native_source_root()` for `DEPOT_SIBLING_SHARED_PROTOCOL`: a checkout-keyed symlink root supplies overridden path dependencies through Cargo's manifest path, while working directory, data and target remain in the actual checkout.

Server release exports stay bookworm-only: `desktop-server-build.py --build-host local|desktop --release` (also used by server `deploy.sh`). `build-host.py --release` delegates exports to that helper; test/check/features/run are not release-export options. `native --release` fails before Cargo: NixOS glibc 2.42 cannot load Arch-linked glibc 2.43 artifacts.

`--build-host native` runs host `cargo build|test --locked --manifest-path godot/Cargo.toml --target-dir <checkout>/target` (without a checkout-specific `CARGO_TARGET_DIR` in the compiler environment), so the extension, fixtures and CLI land where `godot/game_engine.gdextension` and fixtures expect them, and tests read the checkout's real `data/` (no staged test-asset subset). Engine native builds set `CARGO_BUILD_JOBS` to the host CPU affinity count, overriding the root config's two-job cap (explicit Cargo `-j` still wins). They set clang/mold defaults independently of caller cwd; explicit `RUSTFLAGS`/`CARGO_ENCODED_RUSTFLAGS` or target linker environment remains authoritative. Root Cargo/release settings are unchanged. `scripts/native_cargo.py` supplies the shared `GAME_ENGINE_NATIVE_SLOTS` (default 3) host-wide slots to engine and server helpers, so agents run it under `scripts/agent/agent-run` without `build-lock.sh`. `--release` refuses `native`: host (Arch) links need the host's newest glibc, so shipped artifacts use the bookworm container (`--build-host local`).

Wrap every agent build/test using the shared container builder with `~/.worktrees/build-lock.sh`; this global lock serialises builds across slots, unlike the helper's per-checkout lock. The wrapper's implementation is `~/.worktrees/build-lock.sh:5–9` (host-local, not tracked here). On this Ubuntu WSL host select `--build-host local` explicitly:

```sh
scripts/agent/agent-run <run_name> ~/.worktrees/build-lock.sh \
  python3 scripts/depot-build.py --root "$PWD" --build-host local --cli
```

Use the corresponding server helper for an assigned server slot. BuildKit GC limits live only in [`scripts/depot/buildkitd.toml`](../scripts/depot/buildkitd.toml); [builder GC policy](wiki/systems/build-hosts.md#builder-gc-policy) owns the eviction diagnosis and provisioning guidance. Do not change GC policy or rebuild the builder as part of ordinary slot use.

The [builder GC policy](wiki/systems/build-hosts.md#builder-gc-policy) records the user-approved aggregate budget superseding the former warm-every-slot policy; the configuration is applied on OssoBuild, with effective policy recorded there. [Desktop disk exhaustion](wiki/investigations/desktop-disk-exhaustion.md) records observed cache pruning and completed offline VHD compaction, distinguishing Windows host capacity from WSL guest free space.

### Shared native KTX cache

`vendor/ktx2-rw` stores the pinned 212 MB KTX 4.4.0 source archive, libktx and
bindings in `${XDG_CACHE_HOME:-~/.cache}/game-engine/ktx`. New Cargo `OUT_DIR`s
reuse the same compiler/target/options-keyed native artifacts under file locks;
only initial cache population fetches/builds source. `KTX_CACHE_DIR` overrides
the root; `KTX_SOFTWARE_ARCHIVE` is an explicit offline archive input. Failures
are loud, with no alternate library/download path. Bookworm has a separate
BuildKit cache; native Arch artifacts never enter releases. See
[patch provenance](../vendor/README.md).

The Godot workspace pins `glob` debug info because it is both a CLI normal
dependency and a bindgen build dependency; extension and workspace tests now
share its host profile rather than rebuilding clang-sys/bindgen/KTX.

### Shared worktree data

Run `python3 scripts/agent/link-worktree-data.py <canonical-repo> <slot>` when preparing an assigned slot. The linker reads the slot's Git index: directories without tracked descendants become whole-directory links; only tracked subtrees are traversed. Tracked art stays branch-owned. Root auth tokens and SQLite sidecars stay slot-owned; sidecars inside a shared database directory stay with that database.

For existing real asset directories, stop the slot's clients, extractors and importers before running:

```sh
python3 scripts/agent/link-worktree-data.py --repair \
  /home/osso-test/Projects/world-of-osso/game-engine /absolute/path/to/idle-slot
```

Repair checks the slot owner's `/proc/*/fd` before changing anything and refuses while any of them holds a path open; unreadable non-dumpable services (systemd --user) are named and skipped. Keep the slot idle until it finishes. Slot-only files move into canonical using exclusive creation, never overwriting. Identical duplicates are removed locally. Differing files are reported and retained under `~/.worktrees/repair-conflicts/<slot>-data-repair-conflicts-*/` (beside the slots, never inside one) before the real directory is replaced by its canonical link. Root-file conflicts remain in place and are reported. Resolve preserved conflicts manually; do not blindly copy them over canonical. No repair is implicit in normal linking.

Cache importers use `data/cache`, atlas imports use `data/db2` and `data/textures`, and runtime/local-CASC extraction uses the same linked directories. No writer-specific destination override is needed. [Live runs](headless-live-run.md#1-reserve-inputs-and-owned-paths) own the runtime environment.

## Private headless live runs

Follow [Private headless live client](headless-live-run.md) for an owned Weston/Dozen client, copied private server database, disposable accounts, explicit IPC sockets and exact-PID cleanup. That page owns the reusable runtime recipe; the [capability boundary](#desktop-runtime-capability-boundary) below records historical proof and limits.

## Build cancellation

Local Docker clients inherit the helper's cgroup instead of entering a separate
`agents-build_host.slice`. Start agent builds through `scripts/agent/agent-run <name>`
so stopping `agents-<name>.slice` reaches both the helper and its buildx client.
The shared transport used by engine and server builds also starts a Linux guardian:
parent death (including SIGKILL), SIGTERM, or SIGINT requests SIGINT cancellation
of the owned client process group. The guardian waits up to 20 seconds before
killing an unresponsive client group; it never restarts or kills the shared builder.
The uploaded desktop worker uses the same guardian, but this does not establish
that an SSH disconnect terminates the remote worker.

Verified October 5, 2026 with buildx 0.30.1 and BuildKit 0.33.1, using the
`buildorphan` slot (`TARGET_CACHE=godot-target-f0047e41ce8fae69428c`):

- During network-test compilation, clients 866594/866614 belonged to
  `agents-buildorphan.slice`; container cargo 786962 and rustc 786975 were active.
  Stopping that slice left neither client nor any container cargo/rustc within
  0.30 seconds. The log reported `#11 CANCELED` and `Canceled: context canceled`;
  build ref `rriz5sbucra7w3hc2pnzlb0xl` ended without killing container processes
  manually. Graceful client cancellation was sufficient for this installed builder.
- The default extension build compiled ui-model/GDExtension, exported and installed
  `target/debug/libgame_engine_godot.so` in 40.3 seconds (completed ref
  `ideq6k0ta7tfzzbibzb5d0xqe`; installed ELF x86-64, SHA-256
  `d921b85b630d5c00ab2c52c7392eacf9af7949981e66a6f5d1906cca4263d053`).
  Session-test compile/export also completed with exit 0 in 106 seconds
  (ref `u972aous6a4lqhh5tylpmem05`); core-test compile/export completed too.
- The cancellation build waited on the unchanged shared lock from 13:22 to 13:41
  CDT. History records bracket its solve: preceding locked build ended at
  18:41:19.986 UTC, this solve ran 18:41:21.294–18:41:36.635 UTC, and the next
  locked server solve began 18:41:37.640 UTC. No overlap between these solves.

Proof logs/process listings: the slot's `target/buildorphan-proof/`. Regression
fixtures in `scripts/tests/test_build_hosts.py` reproduce helper SIGTERM and
SIGKILL with real child processes (both failed before the fix).

## Native server/simulator acceptance boundary

The server and simulator normal development paths use the shared native runner, not the Docker extension builder described above; see the [server guide](../../game-server/docs/remote-builds.md) and [simulator guide](../../../wow/wow-ui-sim/docs/remote-builds.md).

Main-observed evidence on October 3, 2026: actual native desktop builds passed for both projects; simulator headless CLI startup returned `lua-errors []`, exit 0; isolated desktop server admin `pong` and disposable-account authenticated UDP login passed, with the owned process cleaned up. Simulator GUI failed before its first frame: `target/native-desktop-gui-long.log` ends with connection reset (101) while creating the Wayland event loop. Engine `target/native-wslg-boundary.log` records the correct `/run/user/1000/wayland-0` symlink to `/mnt/wslg/runtime-dir/wayland-0`, repeated Weston SIG11 in `stderr.log`, and repeated approximately 102-second RDP peer disconnect/restart cycles. Windows msrdc ran in Session 0 while the active console was Session 1. Host compositor crash is observed; crash root cause and session causality are not established. This is not a simulator Lua/CASC/font stall. No WSL, service, or process restart was performed.

Local uses installed Arch Cargo/rustc 1.98.1 without rustup; desktop uses pinned Rust 1.98.1 through rustup. This is host-specific tool selection, not fallback. The actual local simulator `wow-cli` helper built and printed `--help` in 0.35 seconds; this is bounded CLI proof, not GUI acceptance. User rejected local server build attempts: actual local server proof is deferred and the no-local-compile guard remains. Local capability is preserved, but actual local runtime acceptance is deferred, not required now for the current gate. Corrected source gate passed: two repaired fixtures, zero Ruff diagnostics across 14 files, and pycompile for nine changed files. The prior 94 passing fixtures retain revision-scoped credit; this is not a fresh 96-test run. Evidence: `target/native-migration-corrected-verify.md`. Normal desktop GUI remains unresolved; source-gate success would not complete the whole workflow. These native observations do not supersede the older Docker trial evidence below.

## Server builds and tests

`scripts/desktop-server-build.py` selects its host like the extension helper: `--build-host desktop|local`, else the saved `~/.config/game-engine/build-host`. `--test` uses the same selection.

```sh
# Export target/debug/{game-server,game-server-admin,game-cli} into the server checkout (--release/--bin as needed):
python3 scripts/desktop-server-build.py --root ../game-server
# Same, against a shared-protocol branch worktree instead of ../shared-protocol:
DEPOT_SIBLING_SHARED_PROTOCOL=/path/to/shared-protocol-branch python3 scripts/desktop-server-build.py --root ../game-server
# `cargo test --locked` for the server workspace; every argument after --test goes to cargo:
python3 scripts/desktop-server-build.py --root ../game-server --test -- --skip query_300k_listings_under_200ms
```

`--test` cannot combine with `--release`, `--bin` or `--save-build-host`. It prints the log tail and summary, saves the full log to `<root>/target/server-test.log`, and exits with cargo's status. Tests read their data from the paths they use locally: `scripts/desktop-server/test-data.txt` lists files, directories and globs under the server checkout's `data/` and its sibling game-engine's `data/` (`DEPOT_SIBLING_GAME_ENGINE` overrides it). An entry matching nothing fails before the host is used; tests needing an unlisted file fail on the host with their own missing-path error. `rsync` mirrors the list (size+mtime quick check, unlisted files deleted) to one `server-test-data` directory per host (`/root/.cache/game-engine/` in OssoBuild, `~/.cache/game-engine/depot-build/` locally), which the Dockerfile `test` stage binds read-only as the `test-data` build context: the game-engine part sits at `<checkout parent>/game-engine/data`, beside the checkout (the image keeps the host checkout paths) where the server's ground and LOS load by default, and the server part is copied to `data/` because SQLite writes beside `world.db`. Every checkout shares that mirror, so server test runs serialize on its lock. The first sync of the ~12 GB ground/LOS set over the ~17 MB/s desktop link takes about 12 minutes; the desktop mirror was seeded from the trial ground copy instead.

On October 3, 2026, `--test --no-fail-fast -- --skip query_300k_listings_under_200ms` on desktop against game-server `a8d9c58` passed 1394 tests (53 ignored, exit 0). Against `04e65e0` (before the checkout-path image) it ran 1392 passed and failed `creature_aggro::tests::neutral_boar_attacked_by_a_players_pet_fights_the_pet` twice on different assertions. After the initial `world.db` upload, a rerun's sync took about 1-3 s.

## Desktop runtime capability boundary

Build-host selection does not start runtime processes. `python3 scripts/desktop-server-build.py --root ../game-server` builds and exports compatible Linux server, admin, and game-cli executables from current sources; an old Arch-built server requires newer glibc than Ubuntu 24.04. Stage those executables under `bin/`, a consistent SQLite backup at `data/world.db`, `data/gametables/`, and `data/economy/config.json` in an owned WSL runtime directory. Set `GAME_SERVER_GROUND_DIR` to an owned copy of the ground assets selected by the server's `scripts/ground_files.py`, including the complete `los/` bake. Never copy live `game.redb`. The bounded fixture `scripts/tests/desktop_server_smoke.py <staged-root>` starts a loopback UDP server on port 15001 with fresh player storage, requires admin `pong` and a disposable-account authenticated roster, then stops its owned process. On October 3, 2026, fixture `8ef0928c` passed admin `pong` and an authenticated disposable-account UDP roster after this complete staging (`target/desktop-server-smoke-complete-data.log`). Earlier missing economy/ground/LOS failures were staging defects; existing world-data warnings remain. This is server capability proof, not gameplay acceptance.

Desktop has an i7-13700 (16 cores/24 threads), 32 GB RAM, and RTX 4070 Ti. Ubuntu's stock Vulkan ICDs do not expose this GPU. A test-only Mesa 25.2.8 Dozen build under `/opt/game-engine/mesa-dzn` enumerates `Microsoft Direct3D12 (NVIDIA GeForce RTX 4070 Ti)` with Vulkan 1.2; it is explicitly nonconformant. [The private live-run recipe](headless-live-run.md#4-start-rendering-and-drive-the-real-client) owns the Dozen environment and display-variable isolation; inherited WSLg surface probing can hang without an interactive Windows display. `scripts/tests/desktop_gpu_smoke.py <staged-client-root>` runs as the unprivileged `osso-test` user inside OssoBuild. It owns a headless Weston/pixman compositor, uses RTX-backed Vulkan for Godot, types into the login username box through native JS UI automation, checks the resulting UI registry, captures IPC performance/screenshot evidence, and stops both processes. Stage `godot/depot-test-assets.txt` assets plus authored `data/glues/`, `data/ui/`, `data/fonts/`, `data/sounds/`, and sound/music indexes. Preserve a complete active-build CASC resolution cache under the test user's cache; the trial reads the existing `/mnt/c/World of Warcraft` install without CDN downloads. Broader world scenes need their additional models, textures, terrain, and sounds. The smoke uses a Dummy audio driver to avoid playing through the user's Windows speakers; it does not prove audio. On October 3, 2026, fixture `055d82ab` passed native Forward+ rendering, JS click/type and IPC capture (`target/desktop-gpu-smoke-authored.log`). Main inspected `data/build-host/gpu-evidence/login.webp`: authored castle background, logo, input borders and bronze controls are visible; `desktop-probe` was appended to the initial `admin` username. One telemetry sample reported 50 fps/7.98 ms; this is not a benchmark. This proves bounded login input/render/capture capability for desktop testing, not full-world, visual parity, audible output, or normal-shutdown acceptance. Dozen is a workaround for missing packaged WSL Vulkan support, restricted to testing; retire the custom build when the distro supplies a proven suitable driver.

### Rerun the staged desktop fixtures

These commands reuse the existing owned staging, not a fresh source-pinned deployment:

```sh
ssh desktop wsl.exe -d OssoBuild -u root --exec env GAME_SERVER_GROUND_DIR=/root/data/game-engine-trial/ground python3 /mnt/c/Users/adeia/data/build-host/desktop_server_smoke.py /root/data/game-engine-trial/server
ssh desktop wsl.exe -d OssoBuild -u osso-test --exec python3 /mnt/c/Users/adeia/data/build-host/desktop_gpu_smoke.py /home/osso-test/game-engine-trial/client
```

The GPU fixture requires the warm complete active-build CASC cache, authored UI/font/glue and sound resources/indexes described above; an assetless image is not visual acceptance. The existing WoW install is `/mnt/c/World of Warcraft`; the successful authored run log is `target/desktop-gpu-smoke-authored.log`. Server staging requires the consistent world database, gametables, economy config and selected ground plus full LOS bake; executable export alone is insufficient.

### Gaming and proof limits

On agent-server the saved default is `native` (2026-10-09). The desktop history below is retained for the OssoBuild host. All trial-owned runtimes were stopped. Other-session Cargo jobs may still be active; this does not establish an idle desktop. If WSL must release memory, first check running jobs and their owners and confirm no other session needs OssoBuild; only then optionally run `ssh desktop wsl.exe --terminate OssoBuild`. Never terminate the distro merely because this trial finished.

Both extension exports, desktop CLI export and the 16-test camera subset are host-capability observations, not head-pinned native/sibling snapshot acceptance. The camera subset retained an unused-import warning; it is not an all-core or warning-free pass. Main's ledger is `target/build-host-proof.md`. Independent followup at `0c8f7275` accepted the bounded trial: launcher format/check/readability and 25 process tests pass after `f8d3a3c9`; the unchanged 42-test Python proof remains valid. Both runtime fixtures and the authored image were independently inspected. Full-game parity remains outside this acceptance.

Each worktree needs the matching sibling repositories beside it: `asset-resolver`, `ui-toolkit`, `ui-toolkit-macros`, and `shared-protocol`. Both `godot/rust/Cargo.toml` and `godot/ui-model/Cargo.toml` depend on `ui-toolkit-core` at `../../../ui-toolkit/core` (canonical `/syncthing/Sync/Projects/world-of-osso/ui-toolkit/core`). The helper's toolkit sibling override is `DEPOT_SIBLING_UI_TOOLKIT`; its internal `ROOT_NAME` is unrelated to this sibling rename. Toolkit merge `96dbda4` integrated `godot-conversion` and its ancestor `testinfra-godot`; older worktree paths in proof records remain historical attribution, not active setup guidance. `DEPOT_SIBLING_<NAME>` (name upper-cased, `-` as `_`) points one of them elsewhere, for example `DEPOT_SIBLING_SHARED_PROTOCOL=/home/osso/.worktrees/shared-protocol-visage` for a protocol branch. The worktree itself can have any directory name. Source-file symlinks and symlinked `target`/`target/debug` directories fail explicitly; the helper never deletes existing targets. Use a checkout-local artifact directory rather than a shared target symlink.

## Build boundary

The helper uploads a source-only snapshot: tracked inputs, nonignored untracked compile inputs under the checkout's `godot/` and `vendor/`, and matching sibling repositories. It excludes `data/`, secrets, targets, and Git metadata.

Registry and Git caches are shared within each selected builder; caches are not transferred between hosts. The Cargo target cache uses the checkout identity defined by the [warm-slot rule](#warm-slot-rule) and `sharing=locked`; Cargo decides freshness by mtime, so checkout-specific target state is never reused by another worktree. Source snapshots retain original file timestamps (`copy2`). After acquiring the checkout target lock, `refresh-source-mtimes.py` hashes staged compile inputs and records their content hashes and nanosecond timestamps in the target cache's `source-mtimes.json`. Unchanged content restores its recorded timestamp; changed, new, or reintroduced content receives a newer timestamp, even when its checkout mtime moved backwards. The first run initializes this state and refreshes every source once. This preserves the earlier A/B/A stale-artifact protection without recompiling unchanged workspace, sibling, and vendored sources on every gate. Registry/Git sources and test assets are outside this refresh. Stable Cargo cannot use `-Zchecksum-freshness`. Worktrees never receive a target cache. By default, each worktree gets only `target/debug/libgame_engine_godot.so`, installed atomically after a lossless gzip download. `--fixture` also builds exactly one `game-engine-network` example after the lock-held source refresh and installs its decompressed executable at `target/debug/examples/<name>`; no target cache is downloaded. Run that executable from the same checkout. `native_input_fixture` modes `menu`, `sound`, `sound-click`, `sound-outcome`, `merchant-click`, `merchant-cursor`, `merchant-services`, `loot`, `settings-reload`, `footsteps`, `reset-windows`, `portal-particles-enabled`, `portal-particles-disabled`, and `portal-density` launch pinned Godot directly (or `GODOT_BIN`), with client flags after `--`. Menu intentionally routes `--screen charselect` and tests the already-built extension without invoking the root launcher or Depot inside its isolated `XDG_CONFIG_HOME`. Menu seeds owned `canonical()` keybinding defaults so inherited legacy bindings cannot change its W movement assertion; user legacy data remains untouched. Other input modes retain their root-launcher requirement, which needs a separate lightweight root launcher build if absent. The NPC visual fixture launches Godot directly.

`settings-reload` saves authored Options in one authenticated native process and checks the same owned canonical file through camera, TargetSelf and AutoLoot consumers in a fresh process. Both processes deliberately terminate at markers via owned SIGKILL/reap/reader join; not normal-shutdown coverage. See [accepted bounded saved-artifact PASS](wiki/systems/godot-conversion.md#native-settingsreload--bounded-two-process-proof).

The `loot` mode targets loot UI/network behavior and the four AutoLoot/Shift combinations. Existing full four-case bounded proof remains valid for its original fields, including inventory/currency and Options checks; it does not prove the newly prepared reach probes. Test-only `76e5bd5c` / `6b1468e2` add `native_input_fixture/loot_range.rs` and `godot/tests/loot_reach_probe.gd`: fresh authoritative NPCs at 5.1 yards (no `LootUnit`), 4.9 and 5.0 (one manual open and real close-button release each, no awards), and friendly living 5.0 (one `InteractNpc`, no `LootUnit`). They use actual posed-mesh clicks and strict phase counts. Fresh spawn avoids interpolation missing the exact boundary; observed player pose arranges input, not expected reach. The original `target.rs` five-yard constant and `> 5` interaction guard supply the inclusive policy reference; production receiver behavior is unchanged. First Depot build and GPU run of these probes are pending: preparation is not runtime proof. See [loot reach scope](specs/loot-frame.md#prepared-native-reach-probes-not-runtime-proof). Source-hash pinning is not an acceptance gate.

Isolated `merchant-click` assembly must link the authored `rendering/` subtree alongside scenes/shaders/tests/UI. Fixture-only `54eaef69` corrects that omission; Depot `vjh7m1nggb` exits0 and retry removes TAA errors but hits an unexplained pre-auth15-second timeout. Diagnostic merchant run passes at Loading4162ms; `51a3e85f` restores15 seconds with timing retained. MAIN accepts independent1429 bounded startup/rendering/merchant PASS; native5a proofs retained. Auth timeout remains unexplained; one ObjectDB leak warning prevents clean-resource claims. Main goal remains OPEN. See [resource-gap evidence and proof boundary](wiki/systems/godot-conversion.md#isolated-merchant-fixture-rendering-gap--corrected-source-proof-pending).

Local CASC startup initialization requires a complete active-build resolution cache in the fixture's Godot user-data. The observed worker writes that cache; warm setup reused only cached tables, without source pinning. Interrupted cold fixture startups are not loot RED evidence.

On September 29, 2026, `--fixture native_input_fixture` passed remotely in project `003c4ttwqh`, build `5nqxfxrzpt`: 206.04 s total (204.6 s remote; 205.9 s installed). It installed `target/debug/libgame_engine_godot.so` and `target/debug/examples/native_input_fixture` under the originating checkout. That installed fixture then passed local owned-UDP `sound-click` in 30.787 s (`data/diagnostics/options-depot-20260929/{fixture-build,exported-fixture-runtime}.log`). This is bounded build/export and fixture behavior proof, not a performance guarantee or conversion parity. Default invocation remains library-only. `native_npc_visual_fixture` is allowlisted but was not remotely built or run in this record.

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

### Units-derived world flows

`world_units_flow.gd` and its inherited terrain, lighting, height, collision, camera and objects flows require `GODOT_TEST_SERVER` (host:UDP-port), `GODOT_TEST_ACCOUNT` and `GODOT_TEST_PASSWORD`. Missing inputs fail before connecting; initial authentication and reconnect use the same supplied values. The account needs at least two roster characters, with card 1 positioned in the world region required by the flow's assertions. Build/import the client first, then run, for example:

```sh
GODOT_TEST_SERVER=127.0.0.1:5203 GODOT_TEST_ACCOUNT=fb_unitsflow GODOT_TEST_PASSWORD=fbtest \
  scripts/agent/agent-run unitsflow "$GODOT_BIN" --path godot -s res://tests/world_lighting_flow.gd
```

Use an owned private server and account. Substitute any of the inherited flow script names above; they share these required inputs, with no endpoint or credential defaults.

Selected-host build or artifact-transfer failures fail explicitly. `local` is an explicit build host, not a Cargo fallback. Godot import and launch stay local and unchanged.

## Historical Depot cost and performance

Verified September 29, 2026: `local-builds` uses the user-selected **200 GB per-architecture cache cleanup target** (214,748,364,800 bytes) and **14-day stale retention**. The cleanup target covers checkout-specific target caches (about 6 GB each, measured September 29, 2026). Depot evicts old cache above its target; this is not a hard storage or spending cap. Organization billing controls and the existing `default` project remain unchanged.

The willingness to spend up to $100/month is conditional, not an automatic billing cap. This project uses the existing company account; the earlier $20 personal-plan estimate does not describe that account. Monitor attributable project usage and organization billing. The helper itself never changes account settings or cache limits.

On September 29, 2026, one warm constant-change benchmark measured 13.321 s before the source-freshness update; it is not a cross-worktree correctness or post-refresh latency guarantee. The first full refreshed Options build passed in 49.274 s on project `003c4ttwqh`, build `cpw7crx3ww`, installing `target/debug/libgame_engine_godot.so` (272,736,240 bytes; SHA-256 `a7ca8a89798d8024a66be9cc9044f200cedb077329047a1af7d1fc3cdbcd0b0b`). Verifier 954 then ran pinned Godot 4.7.2 headlessly with isolated XDG paths: `GameClient` registered through `ClassDB`, instantiated as `Node3D`, and attached to a scene tree. This is bounded extension-load proof, not full conversion proof. Cleanup reported 98.28 GB allocated cache removed, but immediately available disk space fell by 0.754 GB because of concurrent activity/shared extents; it is not evidence of 98 GB physical space reclaimed. These are observations, not latency, storage, or cost guarantees. Build output prints the target cache's current byte size.

## Agent rule

Build the Godot extension through the launcher/helper with `desktop` or `local`. Do not bypass it with direct extension Cargo or recreate a bulk target cache unless explicitly asked. Every agent-started local build/server/client/extraction still requires `agent-run`; existing resource and data-safety rules remain unchanged. Lightweight launcher tests remain allowed.

Historical Depot migration proof: `/tmp/claude/verify-depot-options-migration.md`.

See [Godot conversion](specs/godot-conversion.md) and the [conversion wiki](wiki/systems/godot-conversion.md).
