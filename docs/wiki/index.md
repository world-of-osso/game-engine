# Wiki Index

Knowledge base for the game-engine project, organized across five categories.
Last updated: 2026-09-10.

## Systems

Engine subsystems and how they work.

- [rendering-pipeline](systems/rendering-pipeline.md) — M2 model rendering, optional-distance-fog shader specialization, authored alpha-tested foliage depth coverage, camera collision independent of view culling, startup-only Empty PBR/light/debug/material/target-visual registration boundaries, blend modes, terrain/particle/skybox pipelines, known Bevy bugs, and open UI/render-resource investigation; native fog verification and original-video pixel equivalence remain unproven
- [animation](systems/animation.md) — Bevy-backed M2 bone playback, raw-TRS pivot semantics, crossfade rules, landing completion, HD skeleton loading, replicated NPC authored-idle orientation and distance/visibility sampling LOD
- [networking](systems/networking.md) — Lightyear UDP, dedicated 60 Hz transport worker over unchanged 20 Hz simulation, centralized application dispatch, entity replication, reconnect lifecycle, and event/dirty-driven application boundaries; CPU/FPS proof remains open
- [scripted-movement](systems/scripted-movement.md) — bounded forward routes through normal player movement; connected displacement and loaded-tile measurement demonstrated
- [ui-system](systems/ui-system.md) — toolkit rsx!/Screen/SharedContext screens plus native Bevy login entities, semantic automation/dumps, anchor layout, conditional visibility/alpha and quad invalidation, nameplates, in-world stage gates, unit frames, keybindings, World Builder sidebar; native-login proof remains open
- [world-builder](systems/world-builder.md) — opt-in InWorld scene inventory, subtree render/processing isolation, bounded live property editing
- [terrain](systems/terrain.md) — ADT loading, split files, authored MCVT axes, tile ordering, object placement rotation, doodad collision
- [asset-pipeline](systems/asset-pipeline.md) — CASC lookup chain, casc-local tool, community listfile, FDID resolution, TACT keys
- [character-rendering](systems/character-rendering.md) — HD skeletons, player model-completion appearance boundary, authored NPC compositing, geosets, helmet hiding, target circles
- [skybox](systems/skybox.md) — authored lookup chain and independent InWorld camera environment-light initialization
- [sound](systems/sound.md) — Footsteps, music catalog, zone music, and sound-flag-aware Bevy backend registration; no-sound Empty has no audio threads
- [lore-knowledge-graph](systems/lore-knowledge-graph.md) — Graph schema for NPC AI, quest generation, faction relations

## Formats

WoW file format specifications as used by the engine.

- [m2-format](formats/m2-format.md) — MD21 chunks, bones, animations, geosets, skin files, particles, texture types
- [adt-format](formats/adt-format.md) — Split files, MCNK MCVT row/column axes and center-fan topology, texture layers, MDDF/MODF placement
- [blp-format](formats/blp-format.md) — BLP textures, DXT1/DXT5, image-blp crate, compositing helpers
- [casc-format](formats/casc-format.md) — Content-addressable storage, FDID lookup chain, archives, TACT encryption
- [wmo-format](formats/wmo-format.md) — World Map Objects, root + group files, GFID/MODI chunks, corrected local-to-world placement basis
- [db2-format](formats/db2-format.md) — DB2 tables, WoWDBDefs schemas, key tables, local-only authored NPC appearance importer

## Design

Architecture decisions and feature designs.

- [character-generation](design/character-generation.md) — Original character creation: glTF format, template skeletons, race scaling
- [ui-addon-system](design/ui-addon-system.md) — WASM-sandboxed addon plugins, game-api crate, hot reload
- [ui-frame-order](design/ui-frame-order.md) — implemented shared plugin ordering; standalone setup preserved, named scheduling sets and revision-scoped verification
- [nameplate-design](design/nameplate-design.md) — Target-first display, three states, information hierarchy, distance fade
- [collision-system](design/collision-system.md) — current terrain vertical support and horizontal WMO/M2 collision; WMO/M2 floor support remains absent

## Investigations

- [movement-performance](investigations/movement-performance.md) — Current exact-name timed callback-removal interface and historical CPU-isolation evidence: upload removals showed no bulk reduction; disabling pipelined rendering lowered CPU with an FPS trade-off. Includes repaired MSAA glyph corruption, firmware-limit evidence, and unresolved original tile hitch.
- [empty-window-baseline](investigations/empty-window-baseline.md) — Native/core/reactive blank-renderer stages remain low-cost; continuous blank rendering reaches 216.31478% process CPU before project services. Independent runtime audit passes the bounded attribution.
- [npc-motion-validation](investigations/npc-motion-validation.md) — Revision-pinned Northshire NPC idle/facing, landing, fog, picking, WMO basis, and terrain-streaming evidence with explicit remaining boundaries.

Root cause analyses and debug findings.

- [Inactive event-only NPCs](../../../game-server/docs/wiki/systems/world-data.md#event-dependent-creature-spawns) — Northshire floating Arena Tournament NPCs traced to omitted server event membership, not client height/skin transforms

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
- [procedural-cloud-regeneration](investigations/procedural-cloud-regeneration.md) — Procedural cloud hotspot, Empty scheduling boundaries, capped-measurement retirement, and September 5 replicated-NPC M2 cache reuse; uncapped Green remains pending
- [replicated-unit-noops](investigations/replicated-unit-noops.md) — Empty-stage replicated-unit semantic NOOP boundaries, event-driven NPC visibility, fixes, tests, and connected relaunch proof; prior paced values are historical
- [compile-latency](investigations/compile-latency.md) — Bevy dynamic-link feature wiring, measured edit-build comparison, and remaining under-three-second gap
- [login-camera-startup-order](investigations/login-camera-startup-order.md) — initial Login transition precedes toolkit camera startup; PostStartup camera alignment is unexecuted and bounded capture failed

## Reference

External resources and asset lists.

- [open-source-wow-clients](reference/open-source-wow-clients.md) — Clients, renderers, viewers, editors, format libraries
- [test-assets](reference/test-assets.md) — Available local test files with paths and use cases
- [keybindings](reference/keybindings.md) — Bindable actions vs fixed inputs, scope boundaries
- [audio-libraries](reference/audio-libraries.md) — Audio engines and spatial audio tools (AudioNimbus, etc.)
