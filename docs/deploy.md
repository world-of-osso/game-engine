# Deploying the client

`./deploy.sh` builds one combined Linux/Windows x86_64 Godot player bundle and publishes it to the public R2 bucket behind `files.worldofosso.com`. `./deploy.sh --dry-run` builds and assembles the same bundle, then prints the publish commands without running them.

## Build

1. `python3 scripts/depot-build.py --root <checkout> --release` builds the optimized extension on Depot into `target/release/libgame_engine_godot.so` (see [remote builds](remote-builds.md)). Release builds default to the live realm, `game.worldofosso.com:5000`.
2. The runtime is the patched pinned Godot `4.7.2-pr123946-pr123546` from `scripts/godot/pinned.sh`, checksum-verified; build it with `scripts/godot/build-patched-godot.sh` if missing.
3. `scripts/windows-release.py --phase build` builds the Windows extension, patched runtime and native entry on desktop and exports them into `target/windows-release/artifacts/`.
4. The combined bundle is assembled in `target/deploy/player/` (`BUNDLE_DIR` overrides); the shared project/data assembler runs only once:

| Path | Content |
|---|---|
| `game-engine-linux-x86_64` | `sh` entry: marks `runtime/godot` executable, runs it on `godot/`; every argument is a client option after Godot's `--` |
| `runtime/godot` | patched Linux Godot editor build, used as the runtime |
| `game-engine-windows-x86_64` | extensionless native PE entry; resolves its own directory and launches `runtime/godot.exe` on `godot/` |
| `runtime/godot.exe` | patched Windows Win32/Vulkan Godot editor runtime |
| `lib/game_engine_godot.dll` | native GNU Windows release extension |
| `lib/libgame_engine_godot.so` | release extension (needs only glibc/libstdc++; built on Debian bookworm) |
| `godot/` | tracked project files without Rust crates or `tests/`, a bundle `game_engine.gdextension` pointing at `lib/`, and the pre-imported `.godot/` cache (without `.godot/editor/`) |
| `data/` | runtime data allowlist, below |

The import runs once at assembly with `.godot/extension_list.cfg` already written. Without it, Godot hot-loads the extension during its first scan and aborts at exit (134); this happens with official and patched 4.7.2 and with both debug and release extensions (`data/diagnostics/deploygodot-2026-10-03/import-crash/`). The dev launcher's `import_project_once` still runs a cold import.

### Data

`deploy.sh` uses the shared Linux/Windows assembler `scripts/player_bundle.py`. Its `data/` allowlist includes `cache campsite-ui db2 dbfilesclient fogs fonts forever-1.60.1.70205 gametables glues item-models los minimap models music products reference sounds terrain textures ui`, top-level `*.csv`, `*.ron` and `local-listfile-cache.sqlite`. TACT keys, credentials and diagnostics are excluded. It drops `*.missing` extraction markers, `*.lock`, `cache/casc` and `cache/pre-retail-*`. Both player entries set `GAME_ENGINE_ASSET_MODE=extracted-only`. `data/` also holds `auth_token*` files, diagnostics and screenshots, so it is never synced wholesale. On 2026-10-03 the bundle was 38 GB and 93,328 files. Of that, CASC extraction caches (`models`, `textures`, `terrain`, `music`) make up about 29 GB, and `los` 6.3 GB.

Mutable catalogs are copied independently: `cache`, `db2`, `dbfilesclient`, `gametables`, `reference`, and SQLite/CSV/RON/JSON/text files. Immutable extracted payloads retain cheap hard-link staging. The manifest generator hashes the staged bundle; `player_bundle.py --verify-manifest` checks its members' size and SHA256 before the first upload and aborts on drift. [Staging contract](specs/player-bundle.md) owns this boundary.

The player-bundle direction is extracted-only: players must not need a retail install or runtime CASC extraction. [Shipped assets](specs/shipped-assets.md) owns the runtime policy landed in `b4094d2b2`/`f6b32b79e`; P1 requires `GAME_ENGINE_ASSET_MODE=extracted-only`, while unset still selects development local-CASC mode. P1 did not change deployment; the combined Linux/Windows bundle now selects extracted-only from both entries. Manifest-qualified packaging and pristine no-install/download acceptance remain later phases; [offline closure](wiki/systems/offline-asset-closure.md) records the incomplete audit, not a release-ready asset set.

## Proof

On 2026-10-03 at `574ad01f`, `./deploy.sh --dry-run` exited 0, with the Depot release build and an import that also exited 0 (`data/diagnostics/deploygodot-2026-10-03/dry-run.log`). A reflinked copy of the bundle was made outside the checkout, with the runtime's execute bit cleared to match a game-launcher download. Its entry was run under headless cage with `--server 127.0.0.1:5190 --screen inworld --char Fbdeploy`. It reached Northshire in-world, with player, terrain, NPCs and HUD (`inworld.png`, `dump-scene.txt`, `client-run1.log`), and the server logged `Character 30 entered world`.

This historical run used a saved-token login, the builder's WoW install for CASC, and its resolver cache in `~/.cache/asset-resolver`. It does not prove the extracted-only player contract, a cold resolver cache, a machine without the repo, or a game-launcher download.

### 2026-10-11 Windows LAN and combined dry-run proof

`data/diagnostics/windows-release-2026-10-11/combined-deploy-dry-run-release4.log` records the actual combined dry-run at `a6bc8215e` / shared-protocol `1101804`: Linux release/import and native Windows build succeeded; the assembled player contained 1,265,245 files (rounded 165 GiB). Manifest/publication/deletion commands were printed only; nothing was published or tagged.

Native Windows `99256a4f9` / frozen shared-protocol `58e8ac9` ran through the actual launcher at the default `%APPDATA%\WorldOfOsso` root. The LAN manifest/download/verification and native entry reached login, roster and authoritative private InWorld on UDP54392: connected, one local player, nine initial terrain tiles loaded, no pending/failed tiles. The isolated server used historical master `0fd9d401` with the same frozen protocol; newer PvP server source requires newer protocol types. Patch blocks and lock files remained intact. Evidence: `final-frozen58-lan-proof.log`, `lan-release4-proof/world-network.txt`, `terrain-status.txt` and `windows-lan-{login,roster,inworld}.png`. This is a Northshire acceptance subset, not all-map closure or a published/cold full-download certificate. The final performance sample was 6 FPS (earlier 4 FPS); no performance acceptance is claimed. Published player builds must be rebuilt from current master, not these frozen proof artifacts.

The separate launcher now has a per-user Windows installer and generated bronze app icon. Native desktop proof covers portable first-run Start Menu integration, installation without elevation, visible Start Search, actual shortcut launch to exact PLAY readiness, and clean uninstall while preserving game files. [Launcher installation contract](../../game-launcher/docs/specs/windows-installation.md) owns packaging and icon reproduction. Diagnostics include `installer-generated-icon-final.log` and `installer-proof/`; inspected PNGs are shared in the requested AgentShared folders. Launcher self-update remains disabled.

### 2026-10-11 current-master release5 preparation

Final frozen engine `8aaac41fcb878ff454a9d1910b254dfd5eb1e0de` merges origin/master `d209261b183895a28738a5f87aad995ece523f70`, after master advanced during the first assembly. Launcher `e3d68b7c05763685fc1b47f812ebbdfbb1301a2b` already contains its origin/master. Both rebuilt player platforms and current server master `fece848ac9f0a87e9aec5298713c1157d0512298` use the same frozen current protocol master `6ba2f5cdd23a0d7ed230f6221d9a9e5660879497`. Patch blocks and master dependency versions remain; the existing Windows-required `rmp-serde` extension dependency is the only Godot lock delta.

`data/diagnostics/windows-release5-2026-10-11/proof-ledger.json` records packaging/icon tests (6/6), native launcher check, both formatter checks, Linux bookworm release, native Windows release, current-master private-server build and combined dry-run (all successful). Combined staging is `target/deploy/player/`, 1,265,469 files, rounded 165 GiB; manifest/upload/delete commands were printed only. Source receipts and binary hashes are saved alongside the ledger. Windows runtime reports optional ANGLE/ar warnings; Linux import reports eight ObjectDB leaks and initializes the development CASC resolver. This import is not an extracted-only closure certificate.

A newly packaged current-user installer was installed from desktop staging without elevation. The installed executable's icon resource and actual registered Start Menu shortcut target were verified; that shell link opens the launcher, and actual GUI PLAY opens the downloaded current player at login. The fresh Start Search image was blank and rejected; it does not replace predecessor icon evidence. The installer was uninstalled afterward, preserving game files. Separately, the same installed launcher CLI forwards `--server 192.168.2.149:54396 --screen login` and reaches private LAN login, roster and authoritative InWorld with the same protocol: one connected local player, nine loaded initial terrain tiles, zero pending/failed. Sample performance was 5 FPS, not a performance acceptance. Only the historical Northshire asset subset was downloaded, refreshed with current project/native files; this is not full cold-download/all-map closure.

**Unmet literal boundary:** GUI PLAY passes no client arguments (`game-launcher/src/main.rs::PlayButton` calls `launch_game(..., &[])`). The release client defaults to the live realm; existing realm choices also use UDP5000. Therefore the actual GUI PLAY login screenshot and explicit-private-port CLI network proof are separate, not one GUI-to-private-server chain. No Connect action was made in the default-realm GUI run, and UDP5000 was never used. Supporting private GUI launch requires a user-owned product/fixture decision; no launcher behavior was added for this proof.

Six inspected PNGs (all at most 1280px) are in `/syncthing/AgentShared/2026-10-11/windows-release5/`. `release5-proof/task-utf8.log`, `world-network.txt`, `terrain-status.txt`, installer state JSONs and the private server log retain machine proof. Publication remains lead-owned; `/home/osso/.worktrees/handoff-windows-release.md` records exact deploy/tag/website checks and lease state. Nothing was published or tagged.

## Publish

Since 2026-10-03 players download from the public R2 bucket `worldofosso-client` (Cloudflare account `mh`), served at its custom domain `https://files.worldofosso.com`: the manifest at `/manifest.json`, every manifest path at `/<path>`. No droplet is in the download path; sakuin's `osso-file-server` service is retired.

1. `cargo run --release --manifest-path ../file-server/Cargo.toml -- manifest <bundle>` writes `<bundle>/manifest.json`. `FILE_SERVER_DIR` overrides the checkout.
2. `rclone copy` uploads the bundle without the manifest, `rclone copyto` then publishes `manifest.json`, and a final `rclone sync` deletes what the bundle no longer has. The live manifest therefore never names a missing file.
3. Every object is uploaded with `Cache-Control: no-cache`, because paths keep their names across deploys and the launcher does not re-hash downloads; Cloudflare's edge never serves a superseded copy.

The destination is the rclone remote `woo-r2` (`DESTINATION` overrides). It needs an R2 S3 credential with *Workers R2 Storage Bucket Item Write* on `worldofosso-client` only: the access key id is the API token id, the secret is the SHA-256 hex of the token value, the endpoint `https://b4e0993d34a0fae9030e79751cc337fe.r2.cloudflarestorage.com`, region `auto`, `no_check_bucket = true`. `deploy.sh` stops if it cannot list the remote.

## game-launcher contract

game-launcher (`../game-launcher`) fetches `https://files.worldofosso.com/manifest.json` and downloads every manifest path into `~/.local/share/WorldOfOsso/`. It skips top-level `game-engine-*` paths other than its own platform's, gives mode 755 only to paths starting with `game-engine`, and spawns `game-engine-<platform>`. GUI Play supplies no client arguments; CLI `play` forwards following arguments unchanged for private-server and startup-screen proof. Linux retains the game-directory working directory; Windows uses the volume root because `CreateProcessW` rejects an overlong current directory. Both Windows entry and Godot project/runtime resolve through absolute executable-relative paths; external script-file arguments should be absolute. The bundle therefore:

- names its entries `game-engine-linux-x86_64` and `game-engine-windows-x86_64`;
- keeps every other path from starting with `game-engine-`;
- marks the runtime executable from the entry script.

With no `--server`, the release client connects to the live realm.

## Other platforms

### Windows x86_64 (native GNU)

`python3 scripts/windows-release.py --root <checkout>` drives native Windows builds through `ssh desktop`, assembles `target/deploy/windows-x86_64/`, transfers it to the desktop's new `C:/Users/adeia/Projects/woo-windows-build/player/` workspace and pre-imports there. It never publishes. `--phase build|assemble` resumes either boundary without rebuilding the other. See [Windows toolchain](remote-builds.md#windows-player-release).

The bundle uses the shared `scripts/player_bundle.py` project/data allowlist and `.gdextension` containing both Linux and Windows selectors. Windows artifacts are `runtime/godot.exe`, `lib/game_engine_godot.dll` and an extensionless PE entry `game-engine-windows-x86_64`. The entry resolves paths relative to itself, forwards client arguments after Godot's `--`, waits for its child and selects extracted-only assets. The actual launcher's exact `Command::new` call is exercised by `scripts/windows-entry-test.py`; Windows does launch this extensionless PE. The launcher and entry keep child CWD at the volume root, rather than changing registry settings. This does not make an overlong project/install root launchable: Godot still changes its CWD to the project directory and fails at the Windows limit (approximately 260 characters). The requirement is the normal `%APPDATA%/WorldOfOsso` install root; long nested assets may transfer and verify, but overlong install roots are unsupported. No Godot CWD patch is planned.

Godot is the same pinned 4.7.2 source commit with PR123546 applied. PR123946 touches only `platform/linuxbsd/wayland` and is not built or needed on Windows. The Windows runtime's SHA-512 receipt is exported with build artifacts, separate from Linux's checksum.

For a combined manifest, the launcher filters only exact top-level entry names, not platform-prefixed directories. Thus both platforms download the other platform's runtime/library; the lead must review those measured extra bytes before publishing. Windows-only staging is not a combined-manifest publication or a full offline/world certification. No Windows bundle has been published by this workflow. [Windows unload and packaging investigation](wiki/investigations/windows-extension-unload.md) records the teardown barrier and product-directory omission found by native private login/world proof; include build-qualified assets and their receipts, not only legacy loose paths.

- **macOS:** no extension build, Godot runtime, signing or entry exists.
