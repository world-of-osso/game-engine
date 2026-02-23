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

## Related

- wow-ui-sim: `../wow-ui-sim/` — WoW addon UI simulator (iced + custom wgpu)
- Future: wow-engine 3D scene + wow-ui-sim UI overlay
