# game-engine

> **CLAUDE.md is a symlink to AGENTS.md.** Edit AGENTS.md directly; git tracks AGENTS.md.

WoW client: a Godot 4.7.2 project with a Rust GDExtension. Renders models, terrain and the game world, with native UI and debug tooling. The Bevy client is retired (2026-10-02).

## Structure

```
launcher/        # game-engine-launcher: `cargo run` -> desktop/local extension build -> pinned Godot
tools/           # game-engine-tools: png_to_ktx2, *_cache_import (local, Bevy-free, on game-engine-core)
godot/           # Godot project + Rust workspace (built and tested through desktop/local Docker helpers)
├── project.godot, scenes/, shaders/, ui/
├── tests/       # GDScript fixtures
├── core/        # game-engine-core: M2/ADT/WMO/BLP parsers, DB2/SQLite catalogs and cache importers, camera/movement/lighting data
├── ui-model/    # game-engine-ui-model: rsx! screen components and UI state (ui-toolkit-core)
├── network/     # game-engine-network: headless lightyear transport, IPC wire schema, JS automation; examples/ = process fixtures
├── session/     # game-engine-session: account and character-session decisions
├── rust/        # game-engine-godot: the GDExtension (scenes, rendering, input, IPC server)
└── cli/         # game-engine-cli: IPC client for a running client
```

## Dev

- Plain root `cargo run`/`rd` builds the debug std-only `game-engine-launcher`, then uses `python3 scripts/depot-build.py --root <checkout>` to build the Godot native extension on the selected `desktop` or `local` host before normal local Godot import/launch; `bd` is `cargo build`. The launcher uses `GODOT_BIN`, else the pinned patched Godot `4.7.2-pr123946-pr123546` at `${XDG_CACHE_HOME:-~/.cache}/game-engine/godot/4.7.2-pr123946-pr123546/`, checking its SHA-512 on every launch and failing with build instructions when it is missing or differs (build it once with `scripts/godot/build-patched-godot.sh` (2 jobs; `--jobs N` overrides); [why](docs/wiki/investigations/godot-wayland-exit-hang.md)); runs a one-time headless `--import` when `godot/.godot/extension_list.cfg` is missing; forwards user startup flags after Godot's `--` separator. It does not start the server. See `docs/remote-builds.md`.
- `cargo run -- --screen charselect` — Authenticate with configured credentials or a saved token and open character select.
- `cargo run -- --server dev --screen inworld --char Name` — Resolve `dev`/`prod` server aliases, authenticate, select the named roster character, and enter the world. Omit `--char` to select the default character.
- `cargo run -- --screen charcreate` — Open standalone character creation. Add `--server <host>` to authenticate before entering it; `charcreate-customize` opens its Customize mode.
- `cargo run -- --screen login` or `cargo run -- --screen loading` — Open those native screens. `--state connecting` and `--state reconnecting`, plus legacy screen destinations outside `login`, `charselect`, `charcreate`, `charcreate-customize`, `loading`, `inworld`, `particledebug`, `m2debug`, `selectiondebug`, and `debugcharacter`, fail explicitly as unconverted.
- `cargo run -- --screen m2debug|selectiondebug|debugcharacter` — Offline debug scenes ([spec](docs/specs/native-debug-screens.md)). Process fixture: `native_debug_screen_fixture <screen>` (helper `--fixture native_debug_screen_fixture`; needs `GODOT_BIN`, `GAME_ENGINE_CLI`).
- `cargo run -- --screen particledebug` — Offline M2 particle debug scene: torch, portal and Frostbolt missile emitters with the emitter overlay; drag orbits, wheel zooms, Tab cycles models, 1-9 toggle emitters, 0 restores all, R restarts. Fixture: `GODOT_PARTICLE_SCREENSHOTS=<dir> godot --path godot -s res://tests/particle_debug_screen.gd -- --screen particledebug`.
- `godot/project.godot` selects Godot's native Wayland display driver (`display/display_server/driver.linuxbsd`), so direct and launcher runs open a native niri window owned by Godot's PID; without a Wayland socket Godot logs `Display driver wayland failed, falling back to x11`. Override with `--display-driver x11`.
- The launcher routes `--screen`, `--state`, `--server`, and `--char` after Godot's separator; other arguments remain native Godot arguments. Direct Godot invocation must put client flags after `--`.
- `./deploy.sh [--dry-run]` — Build the release extension on the selected host (`depot-build.py --release`), assemble the Linux x86_64 player bundle in `target/deploy/linux-x86_64/`, and publish it to the file server; `--dry-run` prints the publish commands instead. See [deploy](docs/deploy.md).
- `cargo run -- --run-js-ui-script debug/login.js` — Run a JS UI automation script in the client (fixtures: `native_js_automation_fixture`, `native_js_world_fixture`).
- `python3 scripts/depot-build.py --root "$PWD" --cli` builds `target/debug/game-engine-cli` (combinable with `--fixture`; fixtures take it as `GAME_ENGINE_CLI`). `target/debug/game-engine-cli [--socket /tmp/game-engine-<pid>.sock] <command>` — IPC CLI for a running client
  - `dump-scene` — Dump semantic scene tree (high-level: character, background, camera, lights)
  - `dump-ui-tree` — Dump UI frame registry (names, anchors, positions, widget data)
  - `dump-tree` — Dump the client node tree
  - `screenshot [OUTPUT]` — Capture current frame as WebP (defaults to `screenshot.webp`)
  - `performance` — Report `fps`, `frame_time_ms`, and `focused`
  - `ping` — Check if instance is alive
  - Socket auto-discovered via `/tmp/game-engine-*.sock` glob
- `cargo run -p game-engine-tools --bin png_to_ktx2 -- input.png output.ktx2` — Convert PNG to KTX2 (RGBA8 sRGB, no mipmaps)
- `cargo run -p game-engine-tools --bin <customization|char_texture|creature_display|outfit_links>_cache_import` — Rebuild `data/cache/*.sqlite` the client reads from the DB2 CSVs in `data/`.
- `./run-tests.sh` — root workspace (launcher, tools) tests, clippy, and format check; Godot workspace tests use the desktop/local helper (next line).
- `python3 scripts/depot-build.py --root "$PWD" --test -p game-engine-core [cargo test args...]` — run `godot/` workspace `cargo test --locked` on the selected desktop/local host (CPU only, no Godot engine) with the data files listed in `godot/depot-test-assets.txt`; exits with cargo's status. Agents use this instead of local Cargo in `godot/`.
- Parallel-agent tooling in `scripts/agent/`:
  - `link-worktree-data.py <canonical> <worktree>` links untracked `data/` into a worktree.
  - `seed-target.sh <repo> <dir>` reflink-clones a warm `CARGO_TARGET_DIR`; never start an agent on an empty one.
  - `headless-client.sh start-godot <checkout> <xdg> [godot args]` runs the client in a headless cage, off the user's display; `stop <checkout>/target` stops it.
  - `agent-run <agent-name> <cmd...>` is mandatory for every local build, test server, client and extraction an agent starts (cargo, game-server, Godot/cage, casc-local). It runs inside the capped `agents.slice` (12 cores, 26 GB). Stop everything one agent started with `systemctl --user stop agents-<name>.slice`, everything all agents started with `systemctl --user stop agents.slice`.
  - `record-window (--pid PID | --app-id ID) [-o out.mp4] [--fps 30] [--duration S] [--no-audio]` records one niri window plus that process's PipeWire audio stream to MP4 (H.264 VAAPI + AAC) until Ctrl-C, `--duration`, or the window closes. It calls niri's own `org.gnome.Mutter.ScreenCast` `RecordWindow` (no portal dialog); `pw-video-cat/` (built on first use) reads the LINEAR DMA-BUF stream. Audio is post-mixer: a muted stream records silence.
- `cd ../game-server && ./run-dev.sh` — Auto-restart server on code changes (for testing `--screen inworld`)
- Game server uses **UDP** (lightyear/netcode) — check with `ss -ulnp | grep 5000`, NOT `ss -tlnp`
- Dev profile: `[profile.dev] debug = 1, split-debuginfo = "unpacked"`; `godot/Cargo.toml` adds `opt-level = 2` for dependencies and the parser/network/extension crates (unoptimized they stall the main thread).
- Patched crates `taffy` (godot) and `ktx2-rw` (godot, tools) live in sibling repo `../bevy-patches` (worktrees: `/home/osso/.worktrees/bevy-patches` symlink). Their regression tests run in that repo's workspace; commands in `../bevy-patches/README.md`.
- Textures loaded from `data/textures/{fdid}.blp` (named by FileDataID)
- **NEVER download files to /tmp/** — always save to `data/` for persistence. /tmp is ephemeral.

## Editing Workflow

- `data/` is effectively a different repo/cache tree for this project. Do not stage or commit files under `data/` from this repo unless the user explicitly asks for that exact path.
- After `cargo fmt`, immediately check `git status --short`.
- Agents build the Godot extension through the launcher/helper with `--build-host desktop|local` or the saved default (`python3 scripts/depot-build.py --save-build-host desktop|local`). Do not bypass the helper with direct extension Cargo or recreate bulk target caches unless explicitly asked; lightweight launcher tests are allowed. See `docs/remote-builds.md`.
- Formatter changes count as your changes.

## UI Screens (rsx! + Screen pattern)

- Screens use `ui_toolkit::screen::Screen` with `rsx!` macro for declarative UI (see `login_component.rs`, `char_select_component.rs`)
- Dynamic data injected via `SharedContext` with generation-based dependency tracking. Call `shared.insert(state)` then `screen.sync(&shared, registry)` — Screen auto-detects which types its `build_fn` read and only rebuilds when those types' generations advance. No manual `mark_dirty()` needed.
- Multiple Screens can share one `SharedContext`. Changing a value only rebuilds Screens that read that type (partial rebuild).
- The `rsx!` macro expects `FrameName` (has `.0` field) for `name:` attrs. For dynamic names, use a `DynName(String)` wrapper.
- `!bool_expr` doesn't work inside `rsx!` — pre-compute negations as `let hide = !visible;` before the macro call.
- Post-setup (editbox backdrops, nine-slice textures) happens after first `screen.sync()` since RSX attrs don't cover all frame properties yet.

## Data Assets

- `data/community-listfile.csv` — WoW FDID→path mapping (136MB, from wowdev/wow-listfile). **Use this local copy, never re-download.**
- `data/CharComponentTextureSections.csv` — Character texture region coordinates from wago.tools DB2
- `data/textures/` — BLP textures named by FDID (e.g. `120191.blp`)
- `data/models/` — M2 models and .skin files
- `data/terrain/` — ADT terrain files
- `data/casc/root.bin` + `encoding.bin` — CASC resolution tables (~250MB, from `casc-extract init`). **Never delete — expensive to regenerate.**
- WoW install: `/syncthing/World of Warcraft/` — full install synced from Windows (CASC at `Data/`, retail at `_retail_/`)
- **Asset extraction**: Use local CASC storage, never Blizzard CDN. See `docs/casc-extraction.md`.
- **Gotcha: item material textures** — some item-driven textures come from `ItemDisplayInfo.ModelMaterialResourcesID_*` via `TextureFileData`, not from the same path as attached runtime M2 textures. Auto-extraction is not fully reliable for every such path yet. If an item geoset/model shows untextured, verify the resolved texture FDID exists under `data/textures/` and extract it manually with `cargo run --manifest-path ../asset-resolver/Cargo.toml --bin casc-local -- <fdid> -o data/textures` before assuming the render path is wrong.

## ADT Terrain

- Split files: root `.adt` (heights/normals), `_tex0.adt` (texture layers), `_obj0.adt` (doodads/WMOs)
- The engine renders root terrain + `_tex0` texture compositing and has implemented doodad/WMO spawning from `_obj*` companions
- For low-level ADT/MDDF/MODF/WMO format details and debugging workflow, use `./.codex/skills/wow-adt-terrain-objects/SKILL.md`

## Animation

- Animation transitions must always crossfade smoothly — never snap between poses. Use `blend_time` from M2 sequence data with a minimum of 150ms for movement transitions.
- When re-transitioning mid-blend (e.g. quick direction changes), preserve blend progress so the outgoing pose weight is continuous. Resetting to 0 causes visible pops.
- WoW animation IDs: see `ANIM_*` constants in `godot/rust/src/animation/mod.rs`

## Wiki

LLM-maintained knowledge base at `docs/wiki/`. See `docs/wiki/SCHEMA.md` for conventions and workflows. Read `docs/wiki/index.md` first when answering questions about the project. When working on a feature or investigating a bug, check the wiki for existing knowledge before starting from scratch.

### Wiki Maintenance

After completing work that produces knowledge worth preserving, update the wiki:

- **Investigations/debugging**: Create or update a page in `investigations/` with root cause, symptoms, and fix.
- **New systems or major changes**: Create or update a page in `systems/` describing how the system works.
- **Format discoveries**: Update the relevant page in `formats/` with new fields, quirks, or gotchas.
- **Architecture decisions**: Record rationale in `design/`.

Follow the workflow in `SCHEMA.md`: check existing pages first (update > create), maintain cross-references, update `index.md` and `log.md`. Skip wiki updates for routine bug fixes, config tweaks, and cosmetic changes.

## Related

- asset-resolver: `../asset-resolver` — CASC extraction via cascette-rs (`casc-local`, `casc_refresh`). Resolution tables at `data/casc/root.bin` + `encoding.bin`.
- wow-ui-sim: `/syncthing/Sync/Projects/wow/wow-ui-sim/` — WoW addon UI simulator (iced + custom wgpu)
- game-server: `../game-server/` — Bevy 0.18 headless game server (lightyear networking, redb persistence, SQLite world data from AzerothCore)
- When comparing behavior against other WoW clients, renderers, or format libraries, reference `docs/wiki/reference/open-source-wow-clients.md`
