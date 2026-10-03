# Deploying the client

`./deploy.sh` builds the Linux x86_64 Godot client bundle and publishes it to the world-of-osso file server. `./deploy.sh --dry-run` builds and assembles the same bundle, then prints the publish commands without running them.

## Build

1. `python3 scripts/depot-build.py --root <checkout> --release` builds the optimized extension on Depot into `target/release/libgame_engine_godot.so` (see [remote builds](remote-builds.md)). Release builds default to the live realm, `game.worldofosso.com:5000`.
2. The runtime is the patched pinned Godot `4.7.2-pr123946-pr123546` from `scripts/godot/pinned.sh`, checksum-verified; build it with `scripts/godot/build-patched-godot.sh` if missing.
3. The bundle is assembled in `target/deploy/linux-x86_64/` (`BUNDLE_DIR` overrides):

| Path | Content |
|---|---|
| `game-engine-linux-x86_64` | `sh` entry: marks `runtime/godot` executable, runs it on `godot/`; every argument is a client option after Godot's `--` |
| `runtime/godot` | patched Godot editor build, used as the runtime |
| `lib/libgame_engine_godot.so` | release extension (needs only glibc/libstdc++; built on Debian bookworm) |
| `godot/` | tracked project files without Rust crates or `tests/`, a bundle `game_engine.gdextension` pointing at `lib/`, and the pre-imported `.godot/` cache (without `.godot/editor/`) |
| `data/` | runtime data allowlist, below |

The import runs once at assembly with `.godot/extension_list.cfg` already written. Without it, Godot hot-loads the extension during its first scan and aborts at exit (134); this happens with official and patched 4.7.2 and with both debug and release extensions (`data/diagnostics/deploygodot-2026-10-03/import-crash/`). The dev launcher's `import_project_once` still runs a cold import.

### Data

`deploy.sh` copies an allowlist from `data/`: the directories `cache campsite-ui db2 dbfilesclient fogs fonts glues item-models los minimap models music reference sounds tactkeys terrain textures ui`, top-level `*.csv`, `*.ron` and `local-listfile-cache.sqlite`. It drops `*.missing` extraction markers, `*.lock`, `cache/casc` and `cache/pre-retail-*`. `data/` also holds `auth_token*` files, diagnostics and screenshots, so it is never synced wholesale. On 2026-10-03 the bundle was 38 GB and 93,328 files. Of that, CASC extraction caches (`models`, `textures`, `terrain`, `music`) make up about 29 GB, and `los` 6.3 GB.

Players still need a retail WoW install for local CASC, found by `WOW_INSTALL_PATH`, `WOW_DATA_PATH` or the asset resolver's standard locations. The client extracts assets it lacks into `data/`.

## Proof

On 2026-10-03 at `574ad01f`, `./deploy.sh --dry-run` exited 0, with the Depot release build and an import that also exited 0 (`data/diagnostics/deploygodot-2026-10-03/dry-run.log`). A reflinked copy of the bundle was made outside the checkout, with the runtime's execute bit cleared to match a game-launcher download. Its entry was run under headless cage with `--server 127.0.0.1:5190 --screen inworld --char Fbdeploy`. It reached Northshire in-world, with player, terrain, NPCs and HUD (`inworld.png`, `dump-scene.txt`, `client-run1.log`), and the server logged `Character 30 entered world`.

The run used a saved-token login, the builder's WoW install for CASC, and its resolver cache in `~/.cache/asset-resolver`. It does not prove a cold resolver cache, a machine without the repo, or a game-launcher download.

## Publish

This is the path the live file server uses. As of 2026-10-03, sakuin's `osso-file-server` runs `file-server serve /var/lib/file-server --storage s3`, serving the manifest from `/var/lib/private/file-server/manifest.json` and files from DigitalOcean Spaces. The older `/docker-volumes/file-server` and `/repos/sakuin/ops` compose path no longer exists.

1. `cargo run --release --manifest-path ../file-server/Cargo.toml -- manifest <bundle>` writes `<bundle>/manifest.json`. `FILE_SERVER_DIR` overrides the checkout.
2. `rclone sync <bundle>/ woo:<bucket>[/<prefix>]` with the `FILE_SERVER_S3_*` credentials decrypted from `../sakuin/secrets/sakuin-env.age`.
3. `scp` the manifest to `sakuin:/var/lib/private/file-server/manifest.json`, then `chmod 0644` it and check that `osso-file-server` is active.

`file-server/sync-files.sh` follows the same route but still builds the retired Bevy binary.

## game-launcher contract

game-launcher (`../game-launcher`) downloads every manifest path into `~/.local/share/WorldOfOsso/`. It skips top-level `game-engine-*` paths other than its own platform's, gives mode 755 only to paths starting with `game-engine`, and spawns `game-engine-<platform>` with no arguments and the game directory as the working directory. The bundle therefore:

- names its entry `game-engine-linux-x86_64`;
- keeps every other path from starting with `game-engine-`;
- marks the runtime executable from the entry script.

With no `--server`, the release client connects to the live realm.

## Other platforms

Only `linux-x86_64` exists. `deploy.sh` refuses to run on other hosts.

- **Windows x86_64:** the Bevy-era native build (`scripts/windows-dev.ps1`, `docs/windows-development.md`) was removed with the Bevy client. A Godot build would need:
  - a Depot (or cross) build of `game_engine_godot.dll`, plus a `windows.x86_64` entry in the bundle `.gdextension`;
  - a patched Godot 4.7.2 Windows runtime;
  - a `game-engine-windows-x86_64` entry executable, because game-launcher spawns that path directly and cannot run a script.

  The 2026-09-11 finding still applies: MSVC failed on bundled QuickJS (`libquickjs-sys 0.9.0`) and the GNU target compiled it. None of this exists yet.
- **macOS:** no extension build, Godot runtime, signing or entry exists.
