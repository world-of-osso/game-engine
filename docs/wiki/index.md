# Wiki Index

Knowledge base for the game-engine project. 33 pages across 4 categories.
Last updated: 2026-04-09.

## Systems

Engine subsystems and how they work.

- [rendering-pipeline](systems/rendering-pipeline.md) — M2 model rendering, blend modes, terrain/particle/skybox pipelines, known Bevy bugs
- [animation](systems/animation.md) — M2 bone animation, crossfade rules, blend times, HD skeleton loading
- [networking](systems/networking.md) — Lightyear UDP, auth flow (password + token), entity replication, planned streaming
- [ui-system](systems/ui-system.md) — rsx!/Screen/SharedContext, anchor layout, nine-slice, widgets, nameplates, unit frames, JS automation, keybindings
- [terrain](systems/terrain.md) — ADT loading, split files, tile ordering, object placement rotation, collision reference
- [asset-pipeline](systems/asset-pipeline.md) — CASC lookup chain, casc-local tool, community listfile, FDID resolution, TACT keys
- [character-rendering](systems/character-rendering.md) — HD skeletons, geosets, texture compositing, helmet hiding, target circles
- [skybox](systems/skybox.md) — Light.csv → LightParams → LightSkybox → FDID lookup chain, fallback skybox
- [sound](systems/sound.md) — Footsteps, music catalog, zone music
- [lore-knowledge-graph](systems/lore-knowledge-graph.md) — Graph schema for NPC AI, quest generation, faction relations

## Formats

WoW file format specifications as used by the engine.

- [m2-format](formats/m2-format.md) — MD21 chunks, bones, animations, geosets, skin files, particles, texture types
- [adt-format](formats/adt-format.md) — Split files (root/_tex0/_obj0), MCNK heightmaps, texture layers, MDDF/MODF placement
- [blp-format](formats/blp-format.md) — BLP textures, DXT1/DXT5, image-blp crate, compositing helpers
- [casc-format](formats/casc-format.md) — Content-addressable storage, FDID lookup chain, archives, TACT encryption
- [wmo-format](formats/wmo-format.md) — World Map Objects, root + group files, GFID/MODI chunks
- [db2-format](formats/db2-format.md) — DB2 tables, WoWDBDefs schemas, key tables (Light, ItemDisplayInfo, HelmetGeosetData, etc.)

## Design

Architecture decisions and feature designs.

- [character-generation](design/character-generation.md) — Original character creation: glTF format, template skeletons, race scaling
- [ui-addon-system](design/ui-addon-system.md) — WASM-sandboxed addon plugins, game-api crate, hot reload
- [nameplate-design](design/nameplate-design.md) — Target-first display, three states, information hierarchy, distance fade
- [collision-system](design/collision-system.md) — Terrain/WMO/M2 collision layers, sweep detection, camera collision

## Investigations

Root cause analyses and debug findings.

- [terrain-tile-ordering](investigations/terrain-tile-ordering.md) — Wrong ADT tile loaded as primary in warband scene
- [object-rotation-transforms](investigations/object-rotation-transforms.md) — MDDF/MODF Euler angle order: YZX per Noggit3
- [lightyear-replication-timeout](investigations/lightyear-replication-timeout.md) — Server panic at SingleSender, not network issue
- [hotreload-frame-staleness](investigations/hotreload-frame-staleness.md) — Dioxus hotreload breaks frame IDs; fix: HashMap keying
- [torch-halo-blend-modes](investigations/torch-halo-blend-modes.md) — Incorrect blend mode fallback causing golden halo
- [bevy-pointlight-skinned-mesh](investigations/bevy-pointlight-skinned-mesh.md) — Bevy 0.18: Text + PointLight + SkinnedMesh = black screen
- [character-texture-compositing](investigations/character-texture-compositing.md) — Duplicate texture injection paths in char-select
- [helmet-hide-rules](investigations/helmet-hide-rules.md) — HelmetGeosetData/Vis + ItemDisplayInfo.GeosetGroup for hair hiding
- [editbox-focus-rendering](investigations/editbox-focus-rendering.md) — Nine-slice fill gap preventing clean focus state visuals
- [target-circle-rendering](investigations/target-circle-rendering.md) — Procedural vs BLP-textured selection circle approaches
- [authored-skybox-black-output](investigations/authored-skybox-black-output.md) — `skyboxdebug` resolves authored skyboxes but renders effectively black output

## Reference

External resources and asset lists.

- [open-source-wow-clients](reference/open-source-wow-clients.md) — Clients, renderers, viewers, editors, format libraries
- [test-assets](reference/test-assets.md) — Available local test files with paths and use cases
- [keybindings](reference/keybindings.md) — Bindable actions vs fixed inputs, scope boundaries
