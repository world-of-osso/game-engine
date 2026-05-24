# Open Source WoW Clients & Renderers

Reference list of open source projects that reimplement or render WoW client data.

See also: [awesome-wow-rust](https://github.com/arlyon/awesome-wow-rust) — curated Rust WoW ecosystem list (servers, libraries, format crates).

## Client Reimplementations

| Project | Language | Target | Status | Notes |
|---|---|---|---|---|
| [Whoa](https://github.com/whoahq/whoa) | C++ | 3.3.5a | **Active** (1,300+ commits, 2026) | Login screen, animations, character selection. Most serious active effort. |
| [wow_client](https://github.com/) (~/Repos/wow_client) | C++ | — | Active | Client reimplementation/reference |
| [WoWee](https://github.com/Kelsidavis/WoWee) | C++ (OpenGL) | Vanilla/TBC/WotLK | Stalled | MIT. Reportedly AI-generated. |
| [Thunderbrew](https://github.com/openwow-org/thunderbrew) | C++ | — | Stalled | Clean-room reimplementation |
| [OpenWow](https://github.com/World0fWarcraft/OpenWow) | C++ | 1.12 | Abandoned | — |
| [Wowser](https://github.com/wowserhq/client) | TypeScript/WebGL 2 | WotLK 3.3.5a | Stalled | Browser-based, MIT |
| [Warcraft-Arena-Unity](https://github.com/Reinisch/Warcraft-Arena-Unity) | C# (Unity) | — | Abandoned (2019) | Arena combat sim: 30+ spells, aura system, networking via Photon Bolt |
| [idewave-cli](https://github.com/idewave/idewave-cli) | Rust | — | — | CLI-based WoW client |

## Renderers & Map Viewers

| Project | Language | Status | Notes |
|---|---|---|---|
| [WebWowViewerCpp](https://github.com/Deamon87/WebWowViewerCpp) | C++/Vulkan | Active | Powers wow.tools map viewer |
| [wowmapview](https://sourceforge.net/projects/wowmapview/) | C++ | Legacy | ADT/WMO/M2 rendering reference |
| [jsWoWModelViewer](https://github.com/vjeux/jsWoWModelViewer) | JavaScript/WebGL | Abandoned | Browser M2 viewer |
| [forge](https://github.com/bigglesss/forge) | Rust/Bevy | — | Pure-Rust WoW renderer (WDT/ADT/BLP via wow_chunky) |
| [worgen-rs](https://github.com/nocthir/worgen-rs) | Rust/Bevy | — | Desktop 3D asset viewer (M2/maps via warcraft-rs) |

## Model Viewers

| Project | Language | Status | Notes |
|---|---|---|---|
| [WMVx](https://github.com/) (~/Repos/WMVx) | C++ | Active | WoW Model Viewer X — M2/BLP reference |
| [wowmodelviewer](https://github.com/wowmodelviewer/wowmodelviewer) | C++ | Active (2023) | Long-standing desktop model/character viewer, 2,000+ commits |
| [Scenemachine](https://github.com/CucFlavius/scenemachine) | C# | Active | Reference for loading M2 scene/model data |
| [Everlook](https://github.com/WowDevTools/Everlook) | C# | Stalled (2022) | Cross-platform viewer built on libwarcraft |

## Map Editors

| Project | Language | Status | Notes |
|---|---|---|---|
| [noggit3](https://github.com/wowdev/noggit3) | C++ | Active | Open-source WoW map editor |
| [Neo](https://github.com/WowDevTools/Neo) | C# | Abandoned (2016) | WoW map editor for WotLK/WoD |

## Rust Servers

| Project | Target | Notes |
|---|---|---|
| [wrath-rs](https://github.com/Victov/wrath-rs) | 3.3.5a | Most complete Rust server — movement in world |
| [azerust](https://github.com/arlyon/azerust) | — | Modular server with GraphQL + tokio-console |
| [wow_vanilla_server](https://github.com/gtker/wow_vanilla_server) | 1.12 | Zero external deps, just `cargo run` |

## Rust Libraries

| Project | Notes |
|---|---|
| [wow-srp](https://github.com/gtker/wow_srp) | WoW SRP-6 authentication |
| [namigator-rs](https://github.com/gtker/namigator-rs) | Rust bindings for WoW pathfinding (namigator) |
| [divert](https://github.com/0xFounders/divert) | Rust bindings for Recast pathfinding |
| [dev_wow_auth_server](https://github.com/gtker/dev_wow_auth_server) | Dev-only auth server (any matching user/pass) |

## Format Libraries

| Project | Language | Notes |
|---|---|---|
| [warcraft-rs](https://github.com/wowemulation-dev/warcraft-rs) | Rust | Crate collection for WoW formats (MPQ, ADT, M2, WMO, DBC) |
| [wow_messages](https://github.com/gtker/wow_messages) | Rust | WoW protocol/format crates |
| [wow_dbc](https://github.com/gtker/wow_dbc) | Rust | DBC reader for 1.12, 2.4.3, 3.3.5 |
| [wow_chunky](https://github.com/bigglesss/wow_chunky) | Rust | ADT, WDT, BLP, BLS parser (1.12–3.3.5) |
| [wowdbdefs-rs](https://github.com/gtker/wowdbdefs-rs) | Rust | Rust reader for WoWDBDefs definitions |
| [image-blp](https://github.com/zloy-tulen/image-blp) | Rust | BLP texture reader |
| [WoWDBDefs](https://github.com/wowdev/WoWDBDefs) | — | DB2/DBC schema definitions, layout hashes |

## UI Simulators

| Project | Language | Notes |
|---|---|---|
| [Wowless](https://github.com/ferronn-dev/wowless) | Lua/C | WoW addon UI simulator |
| wow-ui-sim (ours) | Rust/iced | `/syncthing/Sync/Projects/wow/wow-ui-sim/` |
