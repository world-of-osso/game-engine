# wow-engine

Bevy 0.18 3D engine for rendering WoW assets. Sibling to wow-ui-sim (iced-based UI simulator).

## Structure

```
src/
├── main.rs          # Bevy App: camera, lights, ground plane, placeholder cube
└── asset/
    ├── mod.rs        # Re-exports blp + m2 modules
    ├── blp.rs        # BlpTexturePlugin — BLP texture → Bevy Image (image-blp)
    └── m2.rs         # WowModelPlugin — M2 model → Bevy Mesh (wow-m2)
```

## Dependencies

- `bevy = "0.18"` — Engine, ECS, renderer
- `image-blp = "1"` — BLP decoding (same version as wow-ui-sim)
- `wow-m2 = "0.6"` — M2 model parsing (structs only, mesh conversion is ours)

## Dev

- `cargo run` — Launch 3D scene
- `./run-tests.sh` — cargo test + clippy
- Edition 2024, rust-version 1.89

## Test Assets

- M2: `/syncthing/Sync/Projects/wow/reference-addons.new/TomTom/Images/Arrow.m2` (2.9KB, legacy format)
- BLP: `~/Projects/wow/Interface/` — 137K UI textures from WoW client

## Related

- casc-extract: `../casc-extract/` — CLI to download WoW M2/BLP assets from Blizzard CASC CDN (cascette-rs)
- wow-ui-sim: `../wow-ui-sim/` — WoW addon UI simulator (iced + custom wgpu)
- WMVx: `~/Repos/WMVx` — WoW Model Viewer X (C++ reference for M2/BLP loading)
- WoWee: https://github.com/gtker/wow_messages — Rust WoW protocol/format crates
- cascette-rs: `~/Repos/cascette-rs` — Rust CASC/NGDP protocol implementation (used by casc-extract)
- CASCLib: https://github.com/ladislav-zezula/CascLib — C library for reading CASC storage (WoW asset extraction)
- Future: wow-engine 3D scene + wow-ui-sim UI overlay
