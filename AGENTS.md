# game-engine

> **CLAUDE.md is a symlink to AGENTS.md.** Edit AGENTS.md directly; git tracks AGENTS.md.

Bevy 0.18 3D engine rebuilding the WoW client. Renders M2 models, terrain, and eventually the full game world. Standalone renderer with its own Bevy UI/debug tooling.

## Structure

```
src/
├── main.rs              # Bevy App entry point, plugin registration
├── lib.rs               # Re-exports dump + ipc + scene_tree
├── asset/
│   ├── mod.rs           # Re-exports format parsers + asset cache
│   ├── blp.rs           # BLP texture → Bevy Image (image-blp)
│   ├── m2.rs            # M2 Bevy mesh building (render batches)
│   ├── m2_format/       # Pure M2 parser (no Bevy deps)
│   │   ├── mod.rs       # MD21 chunk parser, read utils, vertex/material parsing
│   │   ├── m2_anim.rs   # Bone, animation sequence, track evaluation
│   │   ├── m2_particle.rs # Particle emitter parser (FakeAnimBlock)
│   │   ├── m2_attach.rs # Attachment point parser
│   │   ├── m2_light.rs  # M2 light parser
│   │   └── m2_bone_names.rs # Bone name lookup
│   ├── adt_format/      # Pure ADT parser (no Bevy deps)
│   │   ├── mod.rs       # MCNK heightmaps, normals
│   │   ├── adt_tex.rs   # Texture layer compositing
│   │   └── adt_obj.rs   # Doodad/WMO placement (MDDF/MODF)
│   ├── wmo_format/      # Pure WMO parser (no Bevy deps)
│   └── asset_cache.rs   # FDID → disk cache via AssetResolver trait
├── rendering/
│   ├── model/           # M2 spawning, materials, animation
│   ├── particles/       # GPU particles via bevy_hanabi
│   ├── terrain/         # ADT terrain rendering, LOD, materials
│   ├── skybox/          # Sky rendering, light data, sky materials
│   ├── character/       # Character models, customization, texture compositing
│   ├── camera/          # Camera, orbit camera, culling
│   ├── lighting/        # Light volume lookup
│   └── ui/              # Nameplates, health bars, minimap, action bar
├── scenes/
│   ├── login/           # Login screen + helpers
│   ├── char_select/     # Character select (UI + 3D scene + warband + campsite)
│   ├── char_create/     # Character creation
│   ├── game_menu/       # In-game menu
│   ├── loading/         # Loading screen
│   ├── particle_debug/  # Particle debug scene
│   ├── skybox_debug/    # Skybox debug scene
│   ├── geoset_debug/    # Geoset debug scene
│   └── selection_debug/ # Selection debug screens
├── game/
│   ├── networking/      # Auth, player/NPC sync, reconnect
│   ├── equipment/       # Equipment, transmog, outfit data
│   ├── creatures/       # Creature display info, named models
│   ├── world_db/        # SQLite world data (outfits, zones)
│   └── state/           # Game state, client options
├── sound/               # Footsteps, music catalog, zone music
├── ipc/                 # Unix socket IPC server + Bevy plugin
└── ui/                  # UI toolkit (rsx!, screens, widgets)
```

## Dev

- `cargo run --bin game-engine -- [model.m2]` — Launch 3D scene with M2 model
- `cargo run --bin game-engine -- [terrain.adt]` — Launch 3D scene with ADT terrain
- `cargo run --bin game-engine -- screenshot output.webp model.m2` — Capture screenshot and exit
- `cargo run --bin game-engine -- model.m2 --dump-tree` — Dump entity hierarchy (named bones, meshes)
- `cargo run --bin game-engine -- --screen charselect --dump-scene --server 127.0.0.1:5000` — Dump semantic scene tree (Character, Background, Camera, Lights, equipment slots)
- `cargo run --bin game-engine -- --screen inworld` — Auto-login (admin/admin), pick first char, enter world (defaults to 127.0.0.1:5000). Use `--char Name` to pick a specific character.
- `LOGIN_USER=alice LOGIN_PASS=secret cargo run --bin game-engine -- --server 127.0.0.1:5000 --state login --run-js-ui-script debug/login.js` — Drive the real login UI path via JS automation, wait for `CharSelect`, then dump the entity tree
- `cargo run --bin game-engine-cli -- --socket /tmp/game-engine-<pid>.sock <command>` — IPC CLI for running instance
  - `dump-scene` — Dump semantic scene tree (high-level: character, background, camera, lights)
  - `dump-ui-tree` — Dump UI frame registry (names, anchors, positions, widget data)
  - `dump-tree` — Dump Bevy entity hierarchy
  - `ping` — Check if instance is alive
  - Socket auto-discovered via `/tmp/game-engine-*.sock` glob
- `cargo run --bin png_to_ktx2 -- input.png output.ktx2` — Convert PNG to KTX2 (RGBA8 sRGB, no mipmaps)
- `./run-tests.sh` — cargo test + clippy
- `cd ../game-server && ./run-dev.sh` — Auto-restart server on code changes (for testing `--screen inworld`)
- Game server uses **UDP** (lightyear/netcode) — check with `ss -ulnp | grep 5000`, NOT `ss -tlnp`
- `[profile.dev.package."*"] opt-level = 2` — deps optimized in debug builds (Bevy needs this)
- Textures loaded from `data/textures/{fdid}.blp` (named by FileDataID)
- **NEVER download files to /tmp/** — always save to `data/` for persistence. /tmp is ephemeral.

## Editing Workflow

- `data/` is effectively a different repo/cache tree for this project. Do not stage or commit files under `data/` from this repo unless the user explicitly asks for that exact path.
- After `cargo fmt`, immediately check `git status --short`.
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
- **Gotcha: item material textures** — some item-driven textures come from `ItemDisplayInfo.ModelMaterialResourcesID_*` via `TextureFileData`, not from the same path as attached runtime M2 textures. Auto-extraction is not fully reliable for every such path yet. If an item geoset/model shows untextured, verify the resolved texture FDID exists under `data/textures/` and extract it manually with `cargo run --bin casc-local -- <fdid> -o data/textures` before assuming the render path is wrong.

## ADT Terrain

- Split files: root `.adt` (heights/normals), `_tex0.adt` (texture layers), `_obj0.adt` (doodads/WMOs)
- The engine renders root terrain + `_tex0` texture compositing and has implemented doodad/WMO spawning from `_obj*` companions
- For low-level ADT/MDDF/MODF/WMO format details and debugging workflow, use `./.codex/skills/wow-adt-terrain-objects/SKILL.md`

## Animation

- Animation transitions must always crossfade smoothly — never snap between poses. Use `blend_time` from M2 sequence data with a minimum of 150ms for movement transitions.
- When re-transitioning mid-blend (e.g. quick direction changes), preserve blend progress so the outgoing pose weight is continuous. Resetting to 0 causes visible pops.
- WoW animation IDs: see `ANIM_*` constants in `src/rendering/model/animation.rs`

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

- asset_resolver: `src/asset/asset_resolver.rs` — AssetResolver trait for CASC extraction via cascette-rs. Resolution tables at `data/casc/root.bin` + `encoding.bin`.
- wow-ui-sim: `/syncthing/Sync/Projects/wow/wow-ui-sim/` — WoW addon UI simulator (iced + custom wgpu)
- game-server: `../game-server/` — Bevy 0.18 headless game server (lightyear networking, redb persistence, SQLite world data from AzerothCore)
- Open source WoW clients, renderers, and format libraries: `docs/open-source-wow-clients.md`
