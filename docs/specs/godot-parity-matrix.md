# Godot feature parity matrix

Tracking matrix for the required single-deliverable client replacement. Each source specification remains the contract source of truth; this file records only conversion status. **Handled** requires a Godot-owned implementation plus current behavioral/runtime proof. Parser/core data, a preview helper, portable UI models, and the headless Bevy transport worker do not establish feature parity.

**Current result: 0 handled.** All rows remain required. Bevy is authorized only as the headless networking transport worker; it is not a UI, scene, rendering, audio, input, gameplay, diagnostic, or CLI fallback.

Status vocabulary: **Missing** — no Godot implementation and proof; **Blocked** — a specific known dependency prevents a Godot implementation/proof; **Handled** — implementation and current proof. `None` means no Godot parity proof, not that the Bevy behavior lacks proof.

| Source contract | Required capability | Status | Godot proof / boundary |
| --- | --- | --- | --- |
| [Auction house](auction-house-ui.md) | Auctioneer/gossip open-close lifecycle, interaction closure and errors. | Missing | None; transport worker does not project auction state. |
| [Auction house](auction-house-ui.md) | Buy search/categories/results/item bids/buyouts, money and Retail window/tab layout. | Missing | None. |
| [Auction house](auction-house-ui.md) | Sell stack selection, pricing/deposit/duration/create validation and owned-auction/bid actions. | Missing | None. |
| [Authored skybox depth](authored-skybox-depth.md) | Background-depth sky occlusion, foliage/transparent ordering, and unchanged authored material composition. | Missing | None; preview directional light is not sky rendering. |
| [Bank frame](bank-frame.md) | Banker lifecycle, character/Warband tabs, slot grids, money and purchase presentation. | Missing | None. |
| [Bank frame](bank-frame.md) | Withdraw/deposit, money transfer, tab settings, errors and Warband IPC status. | Missing | None. |
| [Buff frame](buff-frame.md) | Buff/debuff anchors, slot geometry, replicated ordering/caps, timers, flash and stacks. | Missing | None. |
| [Buff frame](buff-frame.md) | Dispel borders/symbols, hover tooltip and buff cancellation behavior. | Missing | None. |
| [Camera direction](camera-direction.md) | CLI camera-angle validation, atomic update, in-world camera selection and normal-input continuity. | Missing | None; fixed preview camera is not an in-world camera. |
| [Character creation icon masks](character-creation-icon-masks.md) | Local-CASC authored mask composition, RGB/alpha preservation, cache/error semantics. | Missing | Core BLP decode does not expose this Godot UI behavior. |
| [Character creation](character-creation.md) | Source-referenced race/class/body/category controls, authored assets, exact geometry, native input and navigation art. | Missing | Portable UI model/startup wiring has no Godot projection or visual proof. |
| [Character creation](character-creation.md) | Authored Alliance/Horde/Pandaren scenes: model, texture, animation, point/ambient lighting, camera and lifecycle. | Missing | M2 preview is not authored creation-scene rendering. |
| [Character creation](character-creation.md) | Catalog-driven customization, compatibility, partial-support disclosure, name generation and complete persisted creation request. | Missing | Native login create-account action explicitly reports unsupported; no creation scene/workflow exists. |
| [Character-selection visibility](character-selection-visibility.md) | Campsite scene fog, sky-owned light/IBL ownership and correct depth ordering. | Missing | None. |
| [Character-selection visibility](character-selection-visibility.md) | Authored attachment point lights, track durations and item-root lifetime. | Missing | Skeleton attachment is not light attachment parity. |
| [Chat frame](chat-frame.md) | Tabbed Chattynator frame, message routing/formatting/flash/scroll/copy and authored layout. | Missing | None. |
| [Chat frame](chat-frame.md) | Edit focus/input history, slash commands, popup precedence and combat-log spell links/tooltips. | Missing | None. |
| [Cursor item](cursor-item.md) | Pickup/drop/drag/swap/equip/clear lifecycle and cursor visual over all frames. | Missing | None. |
| [Cursor item](cursor-item.md) | Destroy confirmation, merchant cursor actions, stack split keyboard flow and item catalog tooltips. | Missing | None. |
| [Death/resurrection](death-resurrection-ui.md) | Death/release/ghost/corpse-resurrection popups, range updates, grading and world-exit reset. | Missing | None. |
| [Empty-window baseline](empty-window-baseline.md) | `--empty-window` exclusive native event-loop diagnostic with no game initialization or continuous work. | Missing | No Godot CLI diagnostic equivalent. |
| [Event-driven application updates](event-driven-application-updates.md) | Worker-owned 60 Hz transport, ordered main-thread application dispatch, lifecycle/reconnect and queued auth semantics. | Missing | `character_select_flow.gd` at `d1641bc9` proves successful `admin`/`admin` auth against explicitly local UDP `127.0.0.1:5000`, native roster projection, and LoginUI→CharacterSelectUI replacement. Current `1296be4b` native-build and auth→Back-flow logs are GREEN with empty stderr; Back restores LoginUI. SelectChar is not separately asserted: original cards are Frame `onclick`, but native projection currently supports only Button callbacks, so functional roster selection is unproven. EnterWorld/create/delete/campsite/menu remain unsupported; no creation/loading/world routing or complete workflow proof. |
| [Event-driven application updates](event-driven-application-updates.md) | Change-driven equipment/application work and M2 animation dirty/crossfade behavior. | Missing | Preview skeleton attachment lacks current playback, interrupted-blend and gameplay proof. |
| [Flight master](flight-master.md) | Flight-map lifecycle, DB2 tiled map, route/pin/tooltip presentation and taxi requests. | Missing | None. |
| [Flight master](flight-master.md) | Controlled replicated flight, mount/camera behavior and MovementControl epoch snapping. | Missing | Transport bridge does not apply world/player state. |
| [Graphics effects](graphics-effects.md) | Persisted independent particle/DOF/bloom/AA/SSAO configuration and validation. | Missing | None. |
| [Graphics effects](graphics-effects.md) | Runtime effect/plugin gating while preserving unrelated scene behavior. | Missing | None. |
| [Group frames](group-frames.md) | Group state/invites/declines/cancellation, party/raid roster and live member-state application. | Missing | None. |
| [Group frames](group-frames.md) | Compact party/raid layouts, menus, targeting, leadership/conversion and ready checks. | Missing | None. |
| [Guild bank](guild-bank-frame.md) | Replicated/pickable vault lifecycle and GuildBank window/tabs/permissions/layout. | Missing | None. |
| [Guild bank](guild-bank-frame.md) | Guild item/money actions, purchase, logs, info text, errors and IPC status. | Missing | None. |
| [HUD edit mode](hud-edit-mode.md) | F10/Escape mode, all registered HUD movers, snap/clamp and authored-position restoration. | Missing | None. |
| [HUD edit mode](hud-edit-mode.md) | Account layouts/per-character active layout, manager actions and persistence. | Missing | None. |
| [Instances](instances.md) | NewWorld loading lifecycle, old-world teardown, player placement and exactly-one WorldPortAck. | Missing | No Godot world scene/state application. |
| [Instances](instances.md) | Transfer errors/summon continuity and WDT global-WMO map loading/ground/camera behavior. | Missing | Core WDT/WMO parsing has no runtime map rendering/collision/streaming proof. |
| [InWorld scene isolation](inworld-scene-isolation.md) | Cumulative scene stages and all diagnostic CLI controls, preserving specified renderer/UI/network behavior. | Missing | No Godot diagnostic-stage/CLI parity. |
| [InWorld scene isolation](inworld-scene-isolation.md) | Exact-name timed callback removal, CPU span attribution and valid frozen-scene diagnostic rules. | Missing | None. |
| [Loot frame](loot-frame.md) | Corpse pose/lootability/cursor/right-click policy, Auto Loot option and server lifecycle/errors. | Missing | None. |
| [Loot frame](loot-frame.md) | Under-cursor Retail LootFrame/cards/actions/bag-money updates and stated interactions. | Missing | None. |
| [M2 loop variations](m2-loop-variations.md) | Authored weighted variation parsing/selection, deterministic entity streams and loop-boundary rules. | Missing | Native animation attachment has no demonstrated variation runtime. |
| [M2 loop variations](m2-loop-variations.md) | Overflow/crossfade continuity and explicit invalid-metadata failure. | Missing | None. |
| [Mail frame](mail-frame.md) | Pickable mailbox lifecycle, inbox/open-mail views, paging/read/open-all and C.O.D. flow. | Missing | None. |
| [Mail frame](mail-frame.md) | Send/reply/return/delete/attachments/money, minimap indicator and IPC commands. | Missing | None. |
| [Meeting stones](meeting-stones.md) | Replicated meeting-stone use, target retention and Retail refusal errors. | Missing | None. |
| [Meeting stones](meeting-stones.md) | Summon popup replacement/countdown/combat gate/response and IPC interaction. | Missing | None. |
| [Merchant frame](merchant-frame.md) | Vendor lifecycle, merchant/buyback layout, paging, pricing/stock/repair/junk and bag state. | Missing | None. |
| [Merchant frame](merchant-frame.md) | Buy/sell/buyback/cursor/stack-split actions, errors and item tooltips. | Missing | None. |
| [Nameplate debug](nameplate-debug.md) | Offline CLI scene, looping cast/channel preview, pause, plate selection and cleanup. | Missing | No Godot debug scene. |
| [Nameplate style](nameplate-style.md) | Exact non-glyph visual parity, authored half-scale geometry/colors and name/cast placement. | Missing | None. |
| [Nameplate style](nameplate-style.md) | Persisted style controls, cast replication presentation, local exclusion/distance and registry-first picking. | Missing | None. |
| [NPC animation LOD](npc-animation-lod.md) | Replicated visual attachment/facing/skins and distance/frustum sampling tiers. | Missing | Godot M2 conversion is a preview path, not replicated NPC runtime. |
| [NPC animation LOD](npc-animation-lod.md) | First authored pose, skipped-write cleanliness and InWorld/local/debug exclusions. | Missing | None. |
| [NPC appearance importer](npc-appearance-importer.md) | Local WDC5 importer, material resolution, deterministic SQLite coverage and explicit failures. | Missing | No Godot conversion integration; retained Rust/Python tooling still must remain usable. |
| [NPC appearance](npc-appearance.md) | Replicated NPC full-choice material/geoset/baked-body/hair isolation semantics. | Missing | None. |
| [NPC appearance](npc-appearance.md) | Visible real-spawn clothing/hair/colour correctness, including all declared texture types. | Missing | None. |
| [Professions frame](professions-frame.md) | DB2 catalog/snapshot/bag data and trainer lifecycle, list/requirements/rank/training confirmation. | Missing | None. |
| [Professions frame](professions-frame.md) | Professions Book and Recipes views: search/tree/schematic/reagents/crafting/cast updates. | Missing | None. |
| [Quest UI](quest-ui.md) | NPC quest interaction/giver detail-progress-reward lifecycle, token substitution and chat results. | Missing | None. |
| [Quest UI](quest-ui.md) | Tracker/log/watch/abandon, world markers, edit mode and quest state application. | Missing | None. |
| [Registry-backed UI](registry-bevy-ui.md) | Registry/RSX/addon-compatible UI authority, IDs/lifecycle/layout/input/editing semantics. | Missing | `ui_projection.gd` at `3de5946b` proves selected native login fixture behaviors: viewport Unicode/Ctrl-A/backspace/focus editing, gold/resize/removal updates, disabled callback suppression, assets/font/insets. It is not addon or complete registry parity. |
| [Registry-backed UI](registry-bevy-ui.md) | Native-equivalent visual projection, authored texture/slice/text/caret/focus/order behavior and real addon operations. | Missing | RealForward+ screenshot `04aa3610` predates final UI colour/focus changes; no exact current visual baseline is verified. |
| [Render-set isolation](render-set-isolation.md) | Timed render-set removal diagnostic, argument errors, counters and valid cutoff evidence. | Missing | No Godot equivalent. |
| [Scripted movement](scripted-movement.md) | CLI bounded movement, validation/cancellation and normal movement/network/collision semantics. | Missing | None. |
| [Scripted movement](scripted-movement.md) | Authored M2 doodad collision and waypoint interaction. | Missing | No Godot collision/world runtime. |
| [Service-window baseline](service-window-baseline.md) | `--service-window` core/render/continuous startup modes and exclusive CLI routing. | Missing | No Godot equivalent. |
| [Service-window baseline](service-window-baseline.md) | Event-policy-specific blank renderer measurement/profiler boundaries and native proof. | Missing | None. |
| [Shared material clock](shared-material-clock.md) | Terrain/water shared-clock UV animation with unchanged material-write and environment semantics. | Missing | Godot raw terrain geometry has no materials/water/shader runtime. |
| [Shared skin palettes](shared-skin-palettes.md) | Correct shared skin palette identity, extraction/lifetime/history and GPU evidence. | Missing | Godot skin conversion does not establish allocation/extraction parity. |
| [Split ADT shadows](split-adt-shadows.md) | Split-root/tex companion shadows and strict terrain failure behavior. | Missing | `3de5946b`/`18666116` add ADT authored-field, tile/LOD, fixture-coordinate, and companion-coverage parser evidence. Reported `fb54637f` pure-core `--lib`: 233/233 GREEN/no warnings; Bevy adapter tests were relocated by `1a5c9045`/`aadb3597` and not executed. No Godot terrain runtime parity. |
| [Split ADT shadows](split-adt-shadows.md) | Waterfall/ripple placement, shader UV/material timing, selected particles and authored heights. | Missing | None. |
| [Task-submission batching](task-submission-batching.md) | Retired experiment's absence: do not restore an unsupported batching path or claim its performance benefit. | Missing | Conversion has no corresponding diagnostic/inventory proof. |
| [Trade frame](trade-frame.md) | Player trade request/popup/lifecycle/window/accept-cancel behavior. | Missing | None. |
| [Trade frame](trade-frame.md) | Offer slots/money/IPC and bag interactions. | Missing | None. |
| [UI frame order](ui-frame-order.md) | Shared strata/frame-level/raise/ID ordering across all renderer families and scheduling gates. | Missing | Portable model does not prove Godot rendering order/lifecycle. |
| [UI layout invalidation](ui-layout-invalidation.md) | Dirty-only layout/dependency invalidation and settled projection synchronization. | Missing | None. |
| [Unit tooltip](unit-tooltip.md) | World/unit/nameplate hover selection, Retail anchors, NPC/player lines and async cache. | Missing | None. |
| [Unit tooltip](unit-tooltip.md) | User-required drops/vendor sections, collection marks/order, ID lines and IPC hover proof. | Missing | None. |
| [Update schedule isolation](update-schedule-isolation.md) | Whole-update/main-work diagnostic cutoffs, validation/counters and blank-client compatibility. | Missing | No Godot equivalent. |
| [Window manager](window-manager.md) | Central window classes/session lifetime, Escape/world-exit policy and panel/wide/bag placement. | Missing | None. |
| [Window manager](window-manager.md) | Raise ordering, drag/clamp and per-character persisted positions/reset. | Missing | None. |
| [WMO floor collision](wmo-floor-collision.md) | Shared terrain/WMO floor selection, flags/BSP/slope/placement semantics and client prediction. | Missing | Core WMO parsing does not provide runtime collision. |
| [WMO floor collision](wmo-floor-collision.md) | Server-cache-compatible map ground, gravity/fall/reposition behavior and WMO movement cases. | Missing | No Godot world state/collision proof. |
| [World Builder](world-builder.md) | Opt-in F9 lifecycle, stable scene forest/search/pagination/component inspection. | Missing | No Godot debug sidebar. |
| [World Builder](world-builder.md) | Reversible subtree render/processing isolation and validated transform/light/shadow editing. | Missing | None. |
| [XP bar](xp-bar.md) | Owner XP state, Retail bar/rested/tick/hover/tooltip/level-cap/edit-mode behavior. | Missing | None. |
| [XP bar](xp-bar.md) | XP gain chat lines and HUD layout/non-overlap behavior. | Missing | None. |

## Cross-cutting contracts still required

| Contract | Required capability | Status | Godot proof / boundary |
| --- | --- | --- | --- |
| [Godot conversion](godot-conversion.md) | Local CASC asset resolution; M2/BLP/ADT/WMO runtime semantics, characters, equipment, streaming, lighting, sky, shadows, particles and audio. | Missing | `ac02b9a0` runtime proof covers raw 256-chunk ADT geometry only; agent39 reports M2/torch-BLP asset assertions GREEN. Neither establishes rendered materials, world assets, characters, streaming, lighting, sky, shadows, particles, or audio. |
| [Godot conversion](godot-conversion.md) | Login, registration, selection/creation/deletion, loading, world entry/reconnect and all server workflows. | Missing | `d1641bc9`/`9249929e` prove successful local `admin`/`admin` auth at UDP `127.0.0.1:5000` creates native `CharacterSelectUI`, maps protocol roster data through the original `CharacterSelectModel`, and hides LoginUI. Empty authored UI/Back fixture is GREEN; `744f3e1b3` mapping tests are pure. Current `1296be4b` native-build and auth→Back-flow logs are GREEN with empty stderr; Back restores LoginUI. `b7070542` adds Frame left-press routing, but the real viewport card fixture is RED: clicking `Elara` selected `Theron`; agent68 build/fixture GREEN remains pending. `94c4e6ae` reuses the authored `LoadingModel`, `1be11d6c` exposes `RegistryUi.show_loading`, and `e8845a5e` centralizes original shell construction/panel styles/Godot model registration; its targeted loading state/style test is GREEN for status/zone/tip, 25%/80% progress, and shell style. Native `loading_ui.gd` remains RED after `63de89d7`: PNG decoding reaches `LoadingBarBackground` decoration rejection. Three-slice projection is pending agent74. No GameClient Loading routing or world-readiness logic exists. No positive EnterWorld/delete/create, character/world scene, appearance/background, reconnect, or matched visual proof. |
| [Godot conversion](godot-conversion.md) | Exact authored UI/layout/text/focus/editing/scroll/layer/input and named-frame automation. | Missing | `ui_projection.gd` at `3de5946b` is behavioral editing/layout proof; verifier67 reran it and `login_flow.gd` PASS on an artifact timing-qualified to `d1641bc9` or `12a23693` (identical host files; tint-only difference). `character_select_ui.gd` is GREEN for empty authored UI and Back action. `b7070542` adds Frame left-press routing, but its real viewport fixture is RED: clicking `Elara` selected `Theron`; agent68 build/fixture GREEN remains pending. `a89ab4d4` is an input readability refactor with no runtime proof yet. Native loading projection remains RED after `63de89d7`: PNG decoding reaches `LoadingBarBackground` decoration rejection; three-slice projection is pending agent74. No matched visual proof exists. |
| [Godot conversion](godot-conversion.md) | CLI, IPC, screenshot, scene/UI-tree diagnostics, JavaScript automation and all debug scenes. | Missing | None. |
| [Godot conversion](godot-conversion.md) | Integrated behavioral, visual, loading, frame-time and memory comparison evidence. | Missing | No full runtime GREEN; no performance or visual parity evidence. The native loading fixture remains RED after `63de89d7`: PNG decoding reaches `LoadingBarBackground` decoration rejection; three-slice projection is pending agent74. Real viewport roster-card selection is RED (`Elara` click selects `Theron`). |

## Sources

- [Godot conversion specification](godot-conversion.md) — conversion boundary and current Godot inventory.
- All linked feature contracts above — requirements remain owned by their individual specifications.

## Maintenance

Update a row only after the Godot implementation and proof cover the stated capability at the current integrated revision. Link proof in the row; do not change the source contract or use a partial/parser/transport result to close runtime parity.
