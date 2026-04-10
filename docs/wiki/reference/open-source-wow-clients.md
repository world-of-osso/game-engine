# Open-Source WoW Clients

Reference catalog of open-source projects that reimplement or render WoW client data.

## Client Reimplementations

| Project | Lang | Target | Status |
|---------|------|--------|--------|
| [Whoa](https://github.com/whoahq/whoa) | C++ | 3.3.5a | Active (1,300+ commits, 2026) — login, animations, char select |
| [WoWee](https://github.com/Kelsidavis/WoWee) | C++ / OpenGL | Vanilla–WotLK | Stalled — MIT, reportedly AI-generated; good collision reference |
| [Thunderbrew](https://github.com/openwow-org/thunderbrew) | C++ | — | Stalled — clean-room reimplementation |
| [OpenWow](https://github.com/World0fWarcraft/OpenWow) | C++ | 1.12 | Abandoned |
| [Wowser](https://github.com/wowserhq/client) | TypeScript / WebGL 2 | 3.3.5a | Stalled — browser-based, MIT |
| [Warcraft-Arena-Unity](https://github.com/Reinisch/Warcraft-Arena-Unity) | C# / Unity | — | Abandoned (2019) — 30+ spells, aura system, Photon Bolt networking |

Local: `~/Repos/wow_client` (C++ reference)

## Renderers & Map Viewers

| Project | Lang | Status | Notes |
|---------|------|--------|-------|
| [WebWowViewerCpp](https://github.com/Deamon87/WebWowViewerCpp) | C++ / Vulkan | Active | Powers wow.tools live map viewer |
| [wowmapview](https://sourceforge.net/projects/wowmapview/) | C++ | Legacy | ADT/WMO/M2 rendering reference |
| [jsWoWModelViewer](https://github.com/vjeux/jsWoWModelViewer) | JS / WebGL | Abandoned | Browser M2 viewer |

## Model Viewers

| Project | Lang | Status | Notes |
|---------|------|--------|-------|
| [WMVx](https://github.com/) | C++ | Active | WoW Model Viewer X — M2/BLP reference (`~/Repos/WMVx`) |
| [wowmodelviewer](https://github.com/wowmodelviewer/wowmodelviewer) | C++ | Active (2023) | 2,000+ commits, desktop character viewer |
| [Everlook](https://github.com/WowDevTools/Everlook) | C# | Stalled (2022) | Built on libwarcraft |

## Map Editors

| Project | Lang | Status |
|---------|------|--------|
| [noggit3](https://github.com/wowdev/noggit3) | C++ | Active — open-source WoW map editor |
| [Neo](https://github.com/WowDevTools/Neo) | C# | Abandoned (2016) — WotLK/WoD |

## Format Libraries

| Project | Lang | Notes |
|---------|------|-------|
| [warcraft-rs](https://github.com/wowemulation-dev/warcraft-rs) | Rust | MPQ, ADT, M2, WMO, DBC crates |
| [wow_messages](https://github.com/gtker/wow_messages) | Rust | WoW protocol/format crates |
| [WoWDBDefs](https://github.com/wowdev/WoWDBDefs) | — | DB2/DBC schema definitions and layout hashes |

## UI Simulators

| Project | Lang | Notes |
|---------|------|-------|
| [Wowless](https://github.com/ferronn-dev/wowless) | Lua/C | WoW addon UI simulator |
| wow-ui-sim (ours) | Rust / iced | `/syncthing/Sync/Projects/wow/wow-ui-sim/` |

## Sources

- [open-source-wow-clients.md](../../open-source-wow-clients.md) — original reference list

## See Also

- [[collision-system]] — WoWee is the primary collision reference
- [[ui-addon-system]] — wow-ui-sim is the UI reference implementation
- [[test-assets]] — test models use M2 format documented by these projects
