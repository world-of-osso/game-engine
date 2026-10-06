# Private headless live client

Run the real Godot client against an owned private game-server, rendering through an owned Weston compositor rather than the user's WSLg display. This is a live run, not a mocked UDP fixture or Godot's displayless `--headless` mode.

Verified source/record review: 2026-10-06, engine master `2f5f3e79`. Commands below generalise saved October 5 runs; this documentation pass did not launch them. Runtime prerequisites and limits remain in [desktop capability](remote-builds.md#desktop-runtime-capability-boundary); builds follow [warm slots](remote-builds.md#warm-slot-rule).

## 1. Reserve inputs and owned paths

Set these variables to approved, existing inputs before executing examples. Use a fixed warm client slot; do not create a worktree for a run.

| Variable | Meaning |
|---|---|
| `W`, `AGENT` | Fixed engine slot and unique run/slice name using letters, digits or underscores |
| `RUN`, `RUNTIME`, `WAYLAND` | Owned evidence directory, `$W/target/<run>-runtime`, unique private socket basename |
| `CONFIG`, `USERDATA`, `RESOLVER` | `$RUN/config`, `$RUN/userdata`, `$RUN/resolver-cache` |
| `GODOT`, `CLI` | Pinned patched Godot executable and matching `$W/target/debug/game-engine-cli` |
| `SERVER_SOURCE`, `ADMIN_SOURCE` | Protocol-compatible server and admin executables from approved staging |
| `REDB_SNAPSHOT`, `SERVER_DATA`, `GROUND` | Consistent offline player snapshot, complete immutable server data, ground bake |
| `PORT`, `ACCOUNT`, `PASSWORD`, `CHARACTER` | Reserved private UDP port, disposable `fb_*` account, credentials and own character |
| `RACE`, `CLASS`, `LEVEL`, `X`, `Y`, `Z` | Explicit scenario setup inputs for that character |
| `RESOLVER_SOURCE`, `DOZEN`, `WSL_LIB` | Warm resolver cache, `/opt/game-engine/mesa-dzn`, `/usr/lib/wsl/lib` |

**Never touch UDP `:5000` or `/home/osso-test/data/osso-5000`.** Do not start, stop, restart, administer, replace binaries, copy a live redb, or alter files there. Obtain inputs from separately approved staging; this recipe grants no access to that protected instance. Use only `fb_*` accounts and their own characters, never `admin` or `Theron`, even if a copied database contains them. Do not restart WSL, WSLg or other sessions' services.

Check `ss -ulnp` for the chosen port before launch and record the baseline. Reserve a different free port if occupied; do not kill its owner. Record engine/server/protocol revisions, executable hashes, exact argv/environment, working directories and intended proof in `$RUN/proof-ledger.txt`. A protocol fingerprint mismatch requires matching binaries, not repeated login attempts.

Create owned directories; `$RUNTIME` must be owned by the invoking user and mode **0700**. Copy the resolver cache, not a symlink to the shared writable cache:

```sh
mkdir -p "$RUN" "$RUNTIME" "$CONFIG/world-of-osso" "$USERDATA"
chmod 0700 "$RUNTIME" "$CONFIG" "$USERDATA"
cp -a --reflink=auto "$RESOLVER_SOURCE" "$RESOLVER"
```

Use a fresh destination for that copy. Reusing an existing owned warm cache is fine; do not nest another cache copy inside it. Set `ASSET_RESOLVER_CACHE_DIR=$RESOLVER` and `ASSET_RESOLVER_SHARED_DATA_DIR=$W/data` for the client.

Before launch, ensure `$W/data` is an owned writable directory, not a whole-directory link to canonical data. Existing immutable assets may be file symlinks, but directories receiving extraction must be owned, and `casc/` and `cache/` must be copies. Do not write through shared directory links. Preserve existing assets and caches; do not delete `target/`, CASC tables or the resolver cache to obtain a cold run. [Asset pipeline](wiki/systems/asset-pipeline.md) owns extraction details.

## 2. Stage and start the private server

Make `$RUN/server` the server's working directory. Copy matching executables to its `bin/`, make its `data/` writable for private runtime files, and stage complete server inputs there. The server reads `data/world.db` relative to its working directory: use a consistent snapshot, copied into staging and made read-only, not the protected live database. Include gametables, economy configuration and the complete LOS/ground inputs described in [server staging requirements](remote-builds.md#desktop-runtime-capability-boundary).

```sh
mkdir -p "$RUN/server/bin" "$RUN/server/data"
cp --reflink=auto "$SERVER_SOURCE" "$RUN/server/bin/game-server"
cp --reflink=auto "$ADMIN_SOURCE" "$RUN/server/bin/game-server-admin"
cp --reflink=auto "$REDB_SNAPSHOT" "$RUN/private.redb"
cp -a --reflink=auto "$SERVER_DATA/." "$RUN/server/data/"
chmod a-w "$RUN/server/data/world.db"
```

`REDB_SNAPSHOT` must already be a consistent **offline backup**. Never hardlink it or point `GAME_SERVER_DB_PATH` at the source. Do not overwrite an existing private database when resuming a run. Static inputs may be read-only; the private redb must be writable.

Run from `$RUN/server`, with logs retained:

```sh
"$W/scripts/agent/agent-run" "$AGENT" env \
  GAME_SERVER_GROUND_DIR="$GROUND" \
  GAME_SERVER_DB_PATH="$RUN/private.redb" \
  GAME_SERVER_ADMIN_SOCKET="$RUNTIME/admin.sock" \
  GAME_SERVER_PORT="$PORT" \
  "$RUN/server/bin/game-server"
```

This is a long-running command. Launch server, compositor and client concurrently in owned managed terminals or a detached runner that records argv, cwd, logs and actual child PIDs. `agent-run` runs in the foreground and creates a systemd scope; a detached wrapper's PID is **not necessarily the server/Godot PID**. Record the process tree and each child's command/start identity before cleanup. Never probe `game-server --help` as a harmless command; use the admin client's documented commands instead.

Require admin `ping` → `pong` and a UDP listener at the chosen private port before client launch:

```sh
"$W/scripts/agent/agent-run" "$AGENT" env \
  GAME_SERVER_ADMIN_SOCKET="$RUNTIME/admin.sock" \
  "$RUN/server/bin/game-server-admin" ping
```

## 3. Create the disposable account and character

Use the same private admin socket for every operation. Direct account creation is the observed setup path:

```sh
"$W/scripts/agent/agent-run" "$AGENT" env GAME_SERVER_ADMIN_SOCKET="$RUNTIME/admin.sock" \
  "$RUN/server/bin/game-server-admin" create-account "$ACCOUNT" "$PASSWORD"
"$W/scripts/agent/agent-run" "$AGENT" env GAME_SERVER_ADMIN_SOCKET="$RUNTIME/admin.sock" \
  "$RUN/server/bin/game-server-admin" create-character "$ACCOUNT" "$CHARACTER" "$RACE" "$CLASS"
"$W/scripts/agent/agent-run" "$AGENT" env GAME_SERVER_ADMIN_SOCKET="$RUNTIME/admin.sock" \
  "$RUN/server/bin/game-server-admin" list-characters "$ACCOUNT"
```

If exercising registration instead, submit it only to `127.0.0.1:$PORT`, then use `game-server-admin approve-registration "$ACCOUNT"` through that socket. Do not both create and register the same account. Apply scenario setup with `set-level "$CHARACTER" "$LEVEL"` and `set-position "$CHARACTER" "$X" "$Y" "$Z"` through the same `agent-run`/environment prefix. No direct redb edits.

Write `(username: "<fb_account>", password: "<password>")` to `$CONFIG/world-of-osso/credentials.ron` (mode 0600), substituting actual values; do not commit it or record tokens/passwords in a public ledger. Capture the **returned character ID**, not an ID from another run. Optional preset selection uses the [Forever persistence schema](wiki/systems/forever-preset.md#edit-mode-and-character-selection) in that same isolated config directory.

### Login-token gotcha

`$W/data/auth_token.127.0.0.1_<port>` belongs to the account/database in which it was issued. Reusing a port with a different redb copy can yield `Account not found` or pending authentication even when credentials exist. Remove only that private endpoint's token from the **owned** data directory and authenticate with the disposable credentials, or create the account in the private database first. Never remove the `:5000` token or shared tokens. Isolated XDG directories alone do not isolate this data-directory token.

## 4. Start rendering and drive the real client

Start Weston in its own managed process; its pixman renderer is independent of Godot's Vulkan renderer:

```sh
"$W/scripts/agent/agent-run" "$AGENT" env -u DISPLAY \
  XDG_RUNTIME_DIR="$RUNTIME" WAYLAND_DISPLAY="$WAYLAND" \
  weston --backend=headless --renderer=pixman --no-config \
  --socket="$WAYLAND" --width=1920 --height=1080
```

Wait for `$RUNTIME/$WAYLAND` to exist and Weston to report readiness. Do not use the WSLg socket. For the client, Dozen's Vulkan environment is:

```sh
VK_DRIVER_FILES="$DOZEN/share/vulkan/icd.d/dzn_icd.x86_64.json"
LD_LIBRARY_PATH="$WSL_LIB"
```

Pass those assignments to `env` as below; setting shell variables alone does not export them. Dozen is test-only/nonconformant; [capability limits and retirement condition](remote-builds.md#desktop-runtime-capability-boundary) remain applicable. Unset `DISPLAY`; use only the private `WAYLAND_DISPLAY` for rendered runs. For displayless driver enumeration unset both display variables.

Use the already built/imported matching extension and IPC CLI. If `godot/.godot/extension_list.cfg` is missing, perform one owned `--headless --audio-driver Dummy --path "$W/godot" --editor --import --quit` under the isolated config/data/resolver environment first. Import is not rendered gameplay evidence. Build through the [warm-slot workflow](remote-builds.md#warm-slot-rule), explicitly on `local` on this host; never bypass it with extension Cargo.

Normal scene startup plus JS UI automation:

```sh
"$W/scripts/agent/agent-run" "$AGENT" env -u DISPLAY \
  XDG_RUNTIME_DIR="$RUNTIME" WAYLAND_DISPLAY="$WAYLAND" \
  XDG_CONFIG_HOME="$CONFIG" XDG_DATA_HOME="$USERDATA" \
  ASSET_RESOLVER_CACHE_DIR="$RESOLVER" ASSET_RESOLVER_SHARED_DATA_DIR="$W/data" \
  VK_DRIVER_FILES="$DOZEN/share/vulkan/icd.d/dzn_icd.x86_64.json" LD_LIBRARY_PATH="$WSL_LIB" \
  "$GODOT" --path "$W/godot" --display-driver wayland --rendering-driver vulkan \
  --audio-driver Dummy --resolution 1920x1080 -- \
  --server "127.0.0.1:$PORT" --screen inworld --char "$CHARACTER" \
  --run-js-ui-script "$UI_SCRIPT"
```

Set `UI_SCRIPT` to an owned scenario script, or omit its flag for manual IPC observation. JS scripts use the normal startup path and native UI consumer; [automation contract](specs/native-ui-automation.md) owns available actions. Prefer state/observable readiness over fixed sleeps. The saved multi-client audit had parallel startup timeouts but sequential launches succeeded; that is an observed load boundary, not permission to change product timeouts.

**`--script "$CAPTURE_SCRIPT"` is different.** Replace the normal client-options tail with native `--script` before any `--` separator, retaining the same display/environment prefix. An owned `live-capture.gd` extending `SceneTree` must instantiate `res://scenes/client.tscn`, attach it to root, wait for asset startup, call `connect_account` with the explicit private endpoint/credentials, and drive character selection. It does not automatically perform the normal `--screen inworld` startup. Parameterise the endpoint/account/character and output directory; do not copy a scratch script's hardcoded values. Wait for `RenderingServer.frame_post_draw` before pixel capture and record live state beside the image. This method supports precise mid-cast captures; it is not a JS UI script. Neither rendered route uses Godot `--headless`; Dummy audio deliberately excludes audible-output proof.

## 5. Collect evidence and clean up

Find and record the actual Godot PID (`CLIENT_PID`), then always select its IPC socket explicitly. Auto-discovery may reach another agent's client.

```sh
"$W/scripts/agent/agent-run" "$AGENT" "$CLI" \
  --socket "/tmp/game-engine-$CLIENT_PID.sock" dump-ui-tree
"$W/scripts/agent/agent-run" "$AGENT" "$CLI" \
  --socket "/tmp/game-engine-$CLIENT_PID.sock" screenshot "$RUN/inworld.webp"
```

Retain dump output, client/server logs and actual captures in `$RUN`. A loading image, partial HUD, successful IPC capture or process still running is not proof of the requested in-world behavior. Record exact revisions, observed assertions, failures and exclusions in the proof ledger; do not promote old runs to current-head acceptance.

Terminate only recorded, still-owned exact PIDs after checking command/start identity: client first, then private server and Weston, including their recorded descendants. Request TERM, wait/reap, and use KILL only for owned survivors. Then stop only this run's slice:

```sh
systemctl --user stop "agents-$AGENT.slice"
```

Do not use `pkill godot`, stop `agents.slice`, or touch another run's slice. Verify every owned PID exited, the private UDP port is free, and the slice is inactive. Retain evidence, private data and warm caches. Forced cleanup proves isolation/termination, not normal Godot shutdown. The protected shared instance must remain untouched throughout.

## Sources

- `data/diagnostics/castrow-2026-10-05/attempt4/{proof-ledger.txt,setup.json,*-argv.json,live-capture.gd,admin-setup.log}` — private account/admin setup, Weston, Vulkan environment, script-driven live capture and exact-PID cleanup.
- `data/diagnostics/playaudit2-2026-10-05/proof-ledger.txt` — real private-server multi-account gameplay, sequential startup observation and proof exclusions.
- `data/diagnostics/swload-2026-10-05/{after.log.argv.json,server.log.argv.json}` — reusable rendered client/server environment and script entry point.
- `/home/osso-test/.worktrees/handoff-shot.md` — cache isolation, normal JS startup and endpoint-token failure diagnosis. Historical examples there access the protected shared staging; do not replay them verbatim.
- [agent-run](../scripts/agent/agent-run) — foreground scope execution and per-name slice ownership (`scripts/agent/agent-run:13–26`).
