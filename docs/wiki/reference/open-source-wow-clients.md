# Open-Source WoW Clients

Reference catalog of open-source projects that reimplement or render WoW client data.

See also: [awesome-wow-rust](https://github.com/arlyon/awesome-wow-rust) — curated Rust WoW ecosystem list (servers, libraries, format crates).

## Client Reimplementations

| Project | Lang | Target | Status |
|---------|------|--------|--------|
| [Whoa](https://github.com/whoahq/whoa) | C++ | 3.3.5a | Active (1,300+ commits, 2026) — login, animations, char select |
| [solarityclient](https://github.com/isekaishy-jpg/solarityclient) | Rust / Vulkan (ash) | 3.3.5a | 1,000+ commits — shipyard ECS, SDL3, egui, mlua, warcraft-rs `wow-*` + gtker `wow_*_messages`; [rendering crate](https://github.com/isekaishy-jpg/solarityclient/tree/main/crates/rendering/src) covers terrain, models, particles, liquid, lighting, weather, minimap, world text, UI |
| wow_client (`~/Repos/wow_client`) | C++ | — | Active — client reimplementation/reference |
| [WoWee](https://github.com/Kelsidavis/WoWee) | C++ / OpenGL | Vanilla–WotLK | Stalled — MIT, reportedly AI-generated; good collision reference |
| [Thunderbrew](https://github.com/openwow-org/thunderbrew) | C++ | — | Stalled — clean-room reimplementation |
| [OpenWow](https://github.com/World0fWarcraft/OpenWow) | C++ | 1.12 | Abandoned |
| [Wowser](https://github.com/wowserhq/client) | TypeScript / WebGL 2 | 3.3.5a | Stalled — browser-based, MIT |
| [Warcraft-Arena-Unity](https://github.com/Reinisch/Warcraft-Arena-Unity) | C# / Unity | — | Abandoned (2019) — 30+ spells, aura system, Photon Bolt networking |
| [idewave-cli](https://github.com/idewave/idewave-cli) | Rust | — | — CLI-based WoW client |

## Renderers & Map Viewers

| Project | Lang | Status | Notes |
|---------|------|--------|-------|
| [WebWowViewerCpp](https://github.com/Deamon87/WebWowViewerCpp) (`~/Repos/WebWowViewerCpp`) | C++ / Vulkan | Active | Powers wow.tools live map viewer. **Primary retail WMO shading reference:** MOMT shader 0–23 → vertex/pixel shader table `wowViewerLib/src/engine/objects/iWmoApi.h` (`wmoMaterialShader`), pixel shaders in `wowViewerLib/shaders/slang/common/commonWMOMaterial.slang` |
| [wowmapview](https://sourceforge.net/projects/wowmapview/) | C++ | Legacy | ADT/WMO/M2 rendering reference |
| [jsWoWModelViewer](https://github.com/vjeux/jsWoWModelViewer) | JS / WebGL | Abandoned | Browser M2 viewer |
| [forge](https://github.com/bigglesss/forge) | Rust / Bevy | — | Pure-Rust WoW renderer (WDT/ADT/BLP via wow_chunky) |
| [worgen-rs](https://github.com/nocthir/worgen-rs) | Rust / Bevy | — | Desktop 3D asset viewer (M2/maps via warcraft-rs) |

## Model Viewers

| Project | Lang | Status | Notes |
|---------|------|--------|-------|
| WMVx (`~/Repos/WMVx`) | C++ | Active | WoW Model Viewer X — M2/BLP reference |
| [wowmodelviewer](https://github.com/wowmodelviewer/wowmodelviewer) | C++ | Active (2023) | 2,000+ commits, desktop character viewer |
| [Scenemachine](https://github.com/CucFlavius/scenemachine) | C# | Active | Reference for loading M2 scene/model data |
| [Everlook](https://github.com/WowDevTools/Everlook) | C# | Stalled (2022) | Built on libwarcraft |

## Map Editors

| Project | Lang | Status |
|---------|------|--------|
| [noggit3](https://github.com/wowdev/noggit3) | C++ | Active — open-source WoW map editor |
| [Neo](https://github.com/WowDevTools/Neo) | C# | Abandoned (2016) — WotLK/WoD |

## Rust Servers

| Project | Target | Notes |
|---------|--------|-------|
| [wrath-rs](https://github.com/Victov/wrath-rs) | 3.3.5a | Most complete Rust server — movement in world |
| [azerust](https://github.com/arlyon/azerust) | — | Modular server with GraphQL + tokio-console |
| [wow_vanilla_server](https://github.com/gtker/wow_vanilla_server) | 1.12 | Zero external deps, just `cargo run` |

## Rust Libraries

| Project | Notes |
|---------|-------|
| [wow-srp](https://github.com/gtker/wow_srp) | WoW SRP-6 authentication |
| [namigator-rs](https://github.com/gtker/namigator-rs) | Rust bindings for WoW pathfinding (namigator) |
| [divert](https://github.com/0xFounders/divert) | Rust bindings for Recast pathfinding |
| [dev_wow_auth_server](https://github.com/gtker/dev_wow_auth_server) | Dev-only auth server (any matching user/pass) |

## Format Libraries

| Project | Lang | Notes |
|---------|------|-------|
| [warcraft-rs](https://github.com/wowemulation-dev/warcraft-rs) | Rust | MPQ, ADT, M2, WMO, DBC crates |
| [wow_messages](https://github.com/gtker/wow_messages) | Rust | WoW protocol/format crates |
| [wow_dbc](https://github.com/gtker/wow_dbc) | Rust | DBC reader for 1.12, 2.4.3, 3.3.5 |
| [wow_chunky](https://github.com/bigglesss/wow_chunky) | Rust | ADT, WDT, BLP, BLS parser (1.12–3.3.5) |
| [wowdbdefs-rs](https://github.com/gtker/wowdbdefs-rs) | Rust | Rust reader for WoWDBDefs definitions |
| [image-blp](https://github.com/zloy-tulen/image-blp) | Rust | BLP texture reader |
| [WoWDBDefs](https://github.com/wowdev/WoWDBDefs) | — | DB2/DBC schema definitions and layout hashes |

## UI Simulators

| Project | Lang | Notes |
|---------|------|-------|
| [Wowless](https://github.com/ferronn-dev/wowless) | Lua/C | WoW addon UI simulator |
| wow-ui-sim (ours) | Rust / iced | `/syncthing/Sync/Projects/wow/wow-ui-sim/` |

## See Also

- [[collision-system]] — WoWee is the primary collision reference
- [[ui-addon-system]] — wow-ui-sim is the UI reference implementation
- [[test-assets]] — test models use M2 format documented by these projects
