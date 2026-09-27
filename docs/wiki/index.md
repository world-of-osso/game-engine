# Wiki Index

Knowledge base for the game-engine project, organized across five categories.
Last updated: 2026-09-27.

## Systems

Engine subsystems and how they work.

- [rendering-pipeline](systems/rendering-pipeline.md) — M2 model rendering, live InWorld camera-direction CLI, optional-distance-fog shader specialization, authored alpha-tested foliage depth coverage, camera collision independent of view culling, startup-only Empty PBR/light/debug/material/target-visual registration boundaries, blend modes, terrain/particle/skybox pipelines, known Bevy bugs, and open UI/render-resource investigation; native fog verification and original-video pixel equivalence remain unproven
- [godot-conversion](systems/godot-conversion.md) — Godot 4.7.2/Rust GDExtension experiment: independently gated shared physics/motion helpers, portable input-binding/camera-input/movement-input/M2 batch-decision data with Bevy-only adapters, unwired native physical input mapping 4/4 and retained-state 3/3 proof, bounded core camera-input and movement-input 6/6 proof, standalone M2 shader GPU pixels, bounded owned-loopback UDP wire proof, native host-state motion, local successful-auth CharacterSelectUI/roster fixture, UI/parser proof, authorized headless Bevy transport worker, limited asset/account boundaries, bounded real-UDP NPC visual inspection and independently gated authored Stand default; Death animation has bounded development GREEN while final-gate/pixel/InWorld/parity proof remains open, M2 material binding/render parity remains pending, and a [detailed parity matrix](../specs/godot-parity-matrix.md); all feature parity remains open
- [animation](systems/animation.md) — Bevy-backed M2 bone playback, raw-TRS pivot semantics, crossfade rules, landing completion, HD skeleton loading, replicated NPC authored-idle orientation and distance/visibility sampling LOD
- [networking](systems/networking.md) — Lightyear UDP, dedicated 60 Hz transport worker over unchanged 20 Hz simulation, centralized application dispatch, entity replication, reconnect lifecycle, and event/dirty-driven application boundaries; CPU/FPS proof remains open
- [scripted-movement](systems/scripted-movement.md) — bounded forward routes through normal player movement; connected displacement and loaded-tile measurement demonstrated
- [auction-house-ui](systems/auction-house-ui.md) — Retail AuctionHouseFrame opened by the auctioneer interaction: Buy/Sell/Auctions on the server auction protocol, Item.csv icons and categories
- [trade-and-mail](systems/trade-and-mail.md) — Retail TradeFrame (unit/group menu Trade, TRADE popup, 7 slots, accept highlights) and MailFrame/OpenMailFrame at Mailbox game objects (inbox pages, open mail, Send Mail with attachments/C.O.D., confirmations), minimap mail indicator
- [banks](systems/banks.md) — Retail BankFrame (character + Warband bank) at bankers and GuildBankFrame at replicated Guild Vault objects; right-click deposit/withdraw, money entry, tab purchase, guild logs
- [group-frames](systems/group-frames.md) — raid-style party frame and raid frames from server `GroupMemberStates`, invite popup, member menus, ready check
- [loot-and-flight](systems/loot-and-flight.md) — corpses, Retail LootFrame, auto-loot and cursor, Retail FlightMapFrame on UiMap art, server-driven flights and `MovementControl` repositioning
- [cursor-item](systems/cursor-item.md) — cursor pickup/drop/swap/equip/destroy through server requests, StackSplitFrame, merchant drag buy/sell, item catalog (ItemSparse) names and tooltips
- [merchant-frame](systems/merchant-frame.md) — Retail MerchantFrame on the server vendor, bag contents from InventorySnapshot/Delta, right-click buy/sell/buyback, Repair All
- [professions-ui](systems/professions-ui.md) — Retail trainer frame, ProfessionsBook (K) and ProfessionsFrame: DB2 recipe catalog, ProfessionSnapshot, CraftRecipe through the spell pipeline
- [quest-ui](systems/quest-ui.md) — client quest runtime, objective tracker, quest log (L), quest giver frame and talktome markers on the server quest/interaction protocol
- [spell-catalog](systems/spell-catalog.md) — background-loaded 12.1.0.69933 spell DB2 catalog, bincode cache under `data/cache/`, static description token rendering and its limits
- [talents-ui](systems/talents-ui.md) — Retail trait-tree window `PlayerSpellsFrame`: CSV tree load, mirrored server rules, local pending config, Apply/Reset/spec, authored `talents-*` atlas art
- [ui-system](systems/ui-system.md) — rsx!/Screen/SharedContext, native Bevy projection, authored UI FileDataID resolution, layout, widgets, input, automation, unit frames, and World Builder sidebar
- [world-builder](systems/world-builder.md) — opt-in InWorld scene inventory, subtree render/processing isolation, bounded live property editing
- [unit-tooltip](systems/unit-tooltip.md) — Retail unit GameTooltip (hover by unit frame/nameplate/ray, default anchor), NPC drops/vendor sections with appearance-collection check/cross marks from server CreatureTooltip data
- [player-ground](systems/player-ground.md) — shared terrain + WMO floor rule (MOPY/BSP, 1.6 yd step reach) on client and server, lazy server tile loading, and fall tracking on repositions
- [terrain](systems/terrain.md) — ADT loading, split files, authored MCVT axes, tile ordering, object placement rotation, doodad collision, map switches and WMO-only maps (WDT global WMO)
- [asset-pipeline](systems/asset-pipeline.md) — CASC lookup chain, casc-local tool, community listfile, FDID resolution for runtime/UI consumers, TACT keys
- [character-rendering](systems/character-rendering.md) — HD skeletons, player model-completion appearance boundary, authored NPC compositing, geosets, helmet hiding, target circles
- [character-creation](systems/character-creation.md) — local-Retail reference contract, catalog-driven core/additional selections, persistence boundary, FileDataID UI/backdrop artwork, authored framing/scale/camera-distance application, scoped lighting/material evidence, and explicit current limits
- [skybox](systems/skybox.md) — explicit procedural-vs-authored InWorld sky selection, authored lookup chain, and environmental sun/camera-IBL ownership boundary
- [retail-lighting](systems/retail-lighting.md) — one RetailSceneLight from the LightParams blend; terrain, M2 and M2 effect shaders use WebWowViewerCpp calcLight and fog in authored space; native creature producer wiring covers nine common light/fog uniforms, not terrain's cube map; no tonemapping, no Bevy IBL; SkySun only casts shadows
- [sound](systems/sound.md) — Footsteps, music catalog, zone music, and sound-flag-aware Bevy backend registration; no-sound Empty has no audio threads
- [lore-knowledge-graph](systems/lore-knowledge-graph.md) — Graph schema for NPC AI, quest generation, faction relations

## Formats

WoW file format specifications as used by the engine.

- [m2-format](formats/m2-format.md) — MD21 chunks, bones, animations, geosets, skin files, 156-byte modern light records, first-key authored camera snapshots, particles, texture types
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
- [nameplate-design](design/nameplate-design.md) — `NameplateStyle` sizes/colours (Options > Nameplates, Thin/Thick presets), FactionTemplate reaction tints, name centred above the bar in overlay units, half-scale reference calibration, local-owner exclusion, shared name/health/cast distance policy, registry-first plate-owner selection, and offline cast/channel preview; rendered/test verification remains open
- [collision-system](design/collision-system.md) — collision layers: terrain and WMO floors (see player-ground), horizontal WMO/M2 blocking; no M2 floors

## Investigations

- [ui-rounding-seams](investigations/ui-rounding-seams.md) — 1 px seams between abutting UI textures at UI scale 2/3 (auction house tabs): taffy 0.10.1 parent-relative location rounding; patched in bevy-patches.
- [bevy-godot-shadow-comparison](investigations/bevy-godot-shadow-comparison.md) — Source-only Bevy 0.19/Godot 4.7.2 directional-shadow comparison: CPU caster scans, Godot silhouette-plane culling, batching, geometry substitution/LOD, redraw, and conditional Bevy GPU preprocessing; no runtime winner claimed.
- [bevy-godot-bone-comparison](investigations/bevy-godot-bone-comparison.md) — Source-only Bevy 0.19, Godot 4.7.2, and solarityclient skeletal-animation comparison: CPU bone representation, dirty/equality boundaries, palette identity/upload, and deformation pass boundaries; no runtime winner or array-conversion recommendation.
- [movement-performance](investigations/movement-performance.md) — Release in-world FPS profile (2026-09-24): GPU-bound under firmware-limited clocks, shadows ~44% of GPU, unused MSAA prepasses removed (−22% GPU/frame), `sync_registry` top CPU item. Current exact-name timed callback-removal interface and historical CPU-isolation evidence: upload removals showed no bulk reduction; disabling pipelined rendering lowered CPU with an FPS trade-off. Includes repaired MSAA glyph corruption, firmware-limit evidence, and unresolved original tile hitch.
- [solarityclient-performance-comparison](investigations/solarityclient-performance-comparison.md) — What solarityclient does for frame cost vs our entity-heavy M2/terrain spawning; ranked candidates, none measured yet; placement recomposition is before the inspected final frustum rejection.
- [empty-window-baseline](investigations/empty-window-baseline.md) — Native/core/reactive blank-renderer stages remain low-cost; continuous blank rendering reaches 216.31478% process CPU before project services. Independent runtime audit passes the bounded attribution.
- [abbey-interior-black-world](investigations/abbey-interior-black-world.md) — MOCV lighting alpha used as vertex opacity discarded interior WMO color while the depth prepass occluded the world
- [stormwind-dark-render](investigations/stormwind-dark-render.md) — unified MapObj (MOHD 0x02) district WMOs drawn unlit as texture×MOCV turned black; MOCV is now added to daylight/MOHD ambient
- [wmo-retail-lighting](investigations/wmo-retail-lighting.md) — Retail WMO light model (ambient + 2×MOCV + sun, interior/exterior blend by MOCV alpha), fixup, two-layer MOCV2 shaders, alpha test, metal and uniqueId placement dedup, per WebWowViewerCpp
- [stormwind-hilly-plaza](investigations/stormwind-hilly-plaza.md) — Trade District drawn as bare hilly terrain: antiportal AABB occlusion and bbox-only camera group hid every `sw_tradedistrict` group; portal culling now follows the Retail interior/exterior traversal; MOBA large material ids and MOMT texture_2 offset fixed (one wall texture everywhere, no roofs)
- [npc-motion-validation](investigations/npc-motion-validation.md) — Revision-pinned Northshire NPC idle/facing, landing, fog, picking, WMO basis, and terrain-streaming evidence with explicit remaining boundaries.

Root cause analyses and debug findings.

- [Inactive event-only NPCs](../../../game-server/docs/wiki/systems/world-data.md#event-dependent-creature-spawns) — Northshire floating Arena Tournament NPCs traced to omitted server event membership, not client height/skin transforms

- [terrain-blend-steps](investigations/terrain-blend-steps.md) — stair-stepped texture edges: sRGB alpha map and height blend driven by the diffuse specular mask; WDT MPHD-driven MCAL decode and Retail weighted/height-weighted blends
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
- [authored-skybox-black-output](investigations/authored-skybox-black-output.md) — `skyboxdebug` authored M2 black output remains separate from ordinary InWorld procedural sky
- [charselect-ground-patch-dark-terrain](investigations/charselect-ground-patch-dark-terrain.md) — corrected terrain normals and removed campsite workaround plane
- [character-select-waterfall-loading](investigations/character-select-waterfall-loading.md) — split shadows, primary backdrop filtering, UV/timing, and terrain-attached mist emitter forwarding; waterfall visibility is accepted, while scene brightness remains separate
- [character-select-lighting-overwrite](investigations/character-select-lighting-overwrite.md) — sky overwrite root cause, M2 light-record and attachment-clock correction, environmental-sun ownership; no Retail brightness match claimed
- [procedural-sky-dome-visibility](investigations/procedural-sky-dome-visibility.md) — restored raw-zero dome was backface-culled; late-created materials also missed settled sky colors
- [washed-out-sky](investigations/washed-out-sky.md) — near-white noon sky: swapped/linear LightData colours, horizon bands spread over the dome, raw FogEnd units and Smog fog colour; sky and fog now blend the player's Light zones
- [procedural-cloud-regeneration](investigations/procedural-cloud-regeneration.md) — Procedural cloud hotspot, Empty scheduling boundaries, capped-measurement retirement, and September 5 replicated-NPC M2 cache reuse; uncapped Green remains pending
- [replicated-unit-noops](investigations/replicated-unit-noops.md) — Empty-stage replicated-unit semantic NOOP boundaries, event-driven NPC visibility, fixes, tests, and connected relaunch proof; prior paced values are historical
- [compile-latency](investigations/compile-latency.md) — Bevy dynamic-link feature wiring, measured edit-build comparison, and remaining under-three-second gap

## Reference

External resources and asset lists.

- [open-source-wow-clients](reference/open-source-wow-clients.md) — Clients, renderers, viewers, editors, format libraries
- [test-assets](reference/test-assets.md) — Available local test files with paths and use cases
- [keybindings](reference/keybindings.md) — Bindable actions vs fixed inputs, scope boundaries
- [audio-libraries](reference/audio-libraries.md) — Audio engines and spatial audio tools (AudioNimbus, etc.)
