# wow-engine

Bevy 0.18 3D engine rebuilding the WoW client. Renders M2 models, terrain, and eventually the full game world. Sibling to wow-ui-sim (iced-based UI overlay).

## Structure

```
src/
├── main.rs          # Bevy App: camera, lights, ground plane, M2 model loading
├── lib.rs           # Re-exports dump + ipc
├── asset/
│   ├── mod.rs       # Re-exports blp + m2 modules
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

- `cargo run --bin wow-engine -- [model.m2]` — Launch 3D scene
- `cargo run --bin wow-engine -- screenshot output.webp model.m2` — Capture screenshot and exit
- `cargo run --bin wow-engine -- model.m2 --dump-tree` — Dump entity hierarchy
- `./run-tests.sh` — cargo test + clippy
- Edition 2024, rust-version 1.89
- `[profile.dev.package."*"] opt-level = 2` — deps optimized in debug builds (Bevy needs this)
- Textures loaded from `data/textures/{fdid}.blp` (named by FileDataID)

## Test Assets

- M2: `data/models/club_1h_torch_a_01.m2` — **textured** item model (FDID 145513 + 198077)
- BLP: `data/textures/145513.blp` + `198077.blp` — torch flame/glow textures
- M2: `data/models/humanmale.m2` + `humanmale00.skin` — character model (skin textures are runtime-resolved, no hardcoded BLPs available)
- M2: `data/models/boar.m2` — creature model (runtime creature skin, no hardcoded BLPs)
- M2: `/syncthing/Sync/Projects/wow/reference-addons.new/TomTom/Images/Arrow.m2` (2.9KB, legacy format, no TXID)
- BLP: `~/Projects/wow/Interface/` — 137K UI textures from WoW client (not model textures)

## Related

- casc-extract: `../casc-extract/` — CLI to download WoW M2/BLP assets from Blizzard CASC CDN (cascette-rs)
- wow-ui-sim: `../wow-ui-sim/` — WoW addon UI simulator (iced + custom wgpu)
- WMVx: `~/Repos/WMVx` — WoW Model Viewer X (C++ reference for M2/BLP loading)
- WoWee: https://github.com/gtker/wow_messages — Rust WoW protocol/format crates
- cascette-rs: `~/Repos/cascette-rs` — Rust CASC/NGDP protocol implementation (used by casc-extract)
- CASCLib: https://github.com/ladislav-zezula/CascLib — C library for reading CASC storage (WoW asset extraction)
- Future: wow-engine 3D scene + wow-ui-sim UI overlay
