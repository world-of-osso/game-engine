# game-engine

> **CLAUDE.md is a symlink to AGENTS.md.** Edit AGENTS.md directly; git tracks AGENTS.md.

Bevy 0.18 3D engine rebuilding the WoW client. Renders M2 models, terrain, and eventually the full game world. Standalone renderer with its own Bevy UI/debug tooling.

## Structure

```
src/
├── main.rs          # Bevy App: camera, lights, ground plane, M2/ADT dispatch
├── lib.rs           # Re-exports dump + ipc
├── terrain.rs       # ADT terrain spawning (spawn_adt, camera positioning)
├── asset/
│   ├── mod.rs       # Re-exports blp + m2 + adt modules
│   ├── adt.rs       # ADT terrain parser: MCNK heightmaps → Bevy meshes
│   ├── blp.rs       # BLP texture → Bevy Image (image-blp, 1-bit alpha fix)
│   └── m2.rs        # Custom MD21 chunked M2 parser + TXID texture FDIDs (no external crate)
├── ipc/
│   ├── mod.rs       # Unix socket IPC server (peercred-ipc)
│   └── plugin.rs    # Bevy plugin bridging IPC commands to ECS
└── dump.rs          # Entity hierarchy dump for dump-tree IPC command
```

## Dependencies

- `bevy = "0.18"` — Engine, ECS, renderer (with `bevy_dev_tools` for FPS overlay)
- `image-blp = "1"` — BLP decoding (same version as wow-ui-sim)

## Dev

- `cargo run --bin game-engine -- [model.m2]` — Launch 3D scene with M2 model
- `cargo run --bin game-engine -- [terrain.adt]` — Launch 3D scene with ADT terrain
- `cargo run --bin game-engine -- screenshot output.webp model.m2` — Capture screenshot and exit
- `cargo run --bin game-engine -- model.m2 --dump-tree` — Dump entity hierarchy
- `LOGIN_USER=alice LOGIN_PASS=secret cargo run --bin game-engine -- --server 127.0.0.1:5000 --state login --run-js-ui-script debug/login.js` — Drive the real login UI path via JS automation, wait for `CharSelect`, then dump the entity tree
- `cargo run --bin game-engine-cli -- --socket /tmp/game-engine-<pid>.sock <command>` — IPC CLI for running instance
  - `dump-ui-tree` — Dump UI frame registry (names, anchors, positions, widget data)
  - `dump-tree` — Dump Bevy entity hierarchy
  - `ping` — Check if instance is alive
  - Socket auto-discovered via `/tmp/game-engine-*.sock` glob
- `cargo run --bin png_to_ktx2 -- input.png output.ktx2` — Convert PNG to KTX2 (RGBA8 sRGB, no mipmaps)
- `./run-tests.sh` — cargo test + clippy + dx fmt
- `dx fmt` — Auto-format RSX macro blocks (enforced in run-tests.sh via `dx fmt --check`)
- Edition 2024, rust-version 1.89
- `[profile.dev.package."*"] opt-level = 2` — deps optimized in debug builds (Bevy needs this)
- Textures loaded from `data/textures/{fdid}.blp` (named by FileDataID)
- **NEVER download files to /tmp/** — always save to `data/` for persistence. /tmp is ephemeral.

## UI Screens (rsx! + Screen pattern)

- Screens use `ui_toolkit::screen::Screen` with `rsx!` macro for declarative UI (see `login_component.rs`, `char_select_component.rs`)
- Dynamic data injected via `ScreenContext` — call `screen.context_mut().insert(state)` then `screen.mark_dirty()` + `screen.sync(registry)`
- **Known debt: manual sync.** ECS systems must manually call `mark_dirty()` + `sync()` gated by `Res::is_changed()`. This should be automatic — Screen should detect context changes and sync itself via a Bevy system, like React's render cycle. Fix in ui-toolkit, not by documenting the workaround.
- The `rsx!` macro expects `FrameName` (has `.0` field) for `name:` attrs. For dynamic names, use a `DynName(String)` wrapper.
- `!bool_expr` doesn't work inside `rsx!` — pre-compute negations as `let hide = !visible;` before the macro call.
- Post-setup (editbox backdrops, nine-slice textures) happens after first `screen.sync()` since RSX attrs don't cover all frame properties yet.

## Data Assets

- `data/community-listfile.csv` — WoW FDID→path mapping (136MB, from wowdev/wow-listfile). **Use this local copy, never re-download.**
- `data/CharComponentTextureSections.csv` — Character texture region coordinates from wago.tools DB2
- `data/textures/` — BLP textures named by FDID (e.g. `120191.blp`)
- `data/models/` — M2 models and .skin files
- `data/terrain/` — ADT terrain files
- WoW install: `/syncthing/World of Warcraft/` — full install synced from Windows (CASC at `Data/`, retail at `_retail_/`)
- **Asset extraction**: Use local CASC storage, never Blizzard CDN. See `doc/casc-extraction.md`.

## Test Assets

- M2: `data/models/club_1h_torch_a_01.m2` — **textured** item model (FDID 145513 + 198077)
- BLP: `data/textures/145513.blp` + `198077.blp` — torch flame/glow textures
- M2: `data/models/humanmale.m2` + `humanmale00.skin` — legacy character model (minimal hair, 142KB)
- M2: `data/models/humanmale_hd.m2` + `humanmale_hd00.skin` — **HD character model** (FDID 1011653, 11MB, 113 submeshes, full hairstyles)
- M2: `data/models/boar.m2` — creature model (runtime creature skin, no hardcoded BLPs)
- M2: `/syncthing/Sync/Projects/wow/reference-addons.new/TomTom/Images/Arrow.m2` (2.9KB, legacy format, no TXID)
- ADT: `data/terrain/azeroth_32_48.adt` — Elwynn Forest terrain tile (FDID 778027, 350KB, 256 MCNK chunks)
- BLP: `~/Projects/wow/Interface/` — 137K UI textures from WoW client (not model textures)

## ADT Terrain

- ADT chunks use **reversed 4CC** magic: `REVM`=MVER, `RDHM`=MHDR, `KNCM`=MCNK, `TVCM`=MCVT, `RNCM`=MCNR
- MCNK position at offset 0x68 is stored as **[Y, X, Z]** (not [X, Y, Z])
- MCVT: 145 floats (9×9 outer + 8×8 inner ROAM grid), heights relative to MCNK position.Z
- Terrain grows in **negative X/Y** from the chunk corner position
- Each MCNK = 33.33 yards (CHUNK_SIZE = 100/3), 16×16 chunks per ADT tile
- Split files: root .adt (heights/normals), _tex0.adt (texture layers), _obj0.adt (doodads/WMOs)
- Renders root ADT (heights/normals) + _tex0 (texture splatting via CPU compositing)
- _tex0 parsing: MDID (specular FDIDs, diffuse = FDID-1), MCLY (16-byte layer entries), MCAL (RLE alpha maps)
- Ground textures tile 8× per MCNK, composited at 256×256 per chunk, alpha-blended from 64×64 maps

## Animation

- Animation transitions must always crossfade smoothly — never snap between poses. Use `blend_time` from M2 sequence data with a minimum of 150ms for movement transitions.
- When re-transitioning mid-blend (e.g. quick direction changes), preserve blend progress so the outgoing pose weight is continuous. Resetting to 0 causes visible pops.
- WoW animation IDs: Stand=0, Walk=4, Run=5, ShuffleLeft=11, ShuffleRight=12, WalkBackwards=13, JumpStart=37, Jump=38, JumpEnd=39

## Related

- casc-extract: `../casc-extract/` — CLI to extract WoW assets (cascette-rs). Currently CDN-only, needs local CASC mode.
- wow-ui-sim: `/syncthing/Sync/Projects/wow/wow-ui-sim/` — WoW addon UI simulator (iced + custom wgpu)
- WMVx: `~/Repos/WMVx` — WoW Model Viewer X (C++ reference for M2/BLP loading)
- WoWee: https://github.com/gtker/wow_messages — Rust WoW protocol/format crates
- cascette-rs: `~/Repos/cascette-rs` — Rust CASC/NGDP protocol implementation (used by casc-extract)
- CASCLib: https://github.com/ladislav-zezula/CascLib — C library for reading CASC storage (WoW asset extraction)
- wowmapview 0.5: https://sourceforge.net/projects/wowmapview/ — C++ WoW map viewer (ADT/WMO/M2 rendering reference)
- game-server: `../game-server/` — Bevy 0.18 headless game server (lightyear networking, redb persistence, SQLite world data from AzerothCore)
- Future: game-engine as a full standalone client renderer + game-server authoritative backend
