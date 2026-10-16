# Loot and Flight Masters

How corpses, the LootFrame, the flight map and server-driven flights fit together on the client. The server rules are game-server `docs/specs/loot.md` and `docs/specs/taxi.md`; the client contracts are `docs/specs/loot-frame.md` and `docs/specs/flight-master.md`.

## Loot

- The server keeps a dead NPC replicated as a corpse until its decay; `Health.current == 0` is the client's dead state. `rendering/model/animation/death.rs` marks the NPC's animated models `DeathPose` (Death once, held); `switch_animation` and emotes skip them.
- `CorpseLootable` (per looter, not replicated) inserts or removes `loot_state::Lootable` on the mirrored entity (`networking/loot.rs`). `quest_sparkle.rs` adds a gold sparkle child for `Lootable`; `wow_cursor.rs` shows `LootAll`.
- Right-click (`target.rs::interact_or_loot`) and IPC `quest interact` both use `loot_state::npc_right_click`: loot a `Lootable` corpse (`LootRequest::Open`, auto = Auto Loot option XOR Shift), only target another corpse, interact with a living NPC.
- `LootState` holds the open corpse's remaining slots; `scenes/loot_frame` places the frame under the cursor once per corpse (`LootFrameAnchor`) and turns card clicks into `LootRequest::Take`.
- Chrome: `quest_art::flat_panel_chrome` = `DefaultPanelFlatTemplate` on the `metal_frame_no_portrait` panel style (the portrait layout with `ui-frame-metal-cornertopleft-2x` in the top-left cell).

### ItemButton input ownership — 2026-10-16

`loot_frame_component::item_button` owns the slot action on a transparent 37×37 Button at (5,4), with right hit inset −157: effective hit width194. The enclosing row is passive; existing icon/count/card paint remains separate and unchanged. Left release commits only while still inside the authored target. The [loot-frame spec](../../specs/loot-frame.md) owns this contract; hover/pushed strokes, right/modified clicks, tooltips, animations and server lifecycle are separate gaps.

Native `RegistryButton::_has_point` applies those insets without resizing the Button. Registry pointer lookup alone was insufficient: after adding the authored Item, icon clicks worked but name/extended-area clicks still missed Godot's default rectangle. All projected buttons use the same inset-aware native picking; zero insets retain the native rectangle. Existing button state signals and paint remain unchanged.

The native debug fixture calls production `show_loot_frame`, not generic `show_viewport_screen`: the former registers the metal-border style. Skipping that host setup fails explicitly with `Unconverted native frame decoration: LootFrameNineSlice`; do not suppress it or change production art to make a preview pass.

Bounded proof: model RED→GREEN2 (source input owner plus authoritative removal/rebinding/closure), native money slot5/item slot9 in both skins:32 physical gutter/icon/name/far-right-extension/outside-cancel probes. Baseline records12 gutter early dispatches,8 icon/name early dispatches,4 already-dispatched cancellations and4 unreachable extension targets. The first Item-only change fixes gutters/icon release/cancel but still misses8 name/extension cases; inset-aware native picking resolves those. Supplied state/input only, not server collection or whole-window acceptance.

All68 before/after full-HD pairs have zero changed RGB pixels. Shared-input controls also pass16 StackSplit cases and4 ReadyCheck cases;48 full-HD StackSplit pairs and12 ReadyCheck button crops remain unchanged. Both-skin comparison panels and representative full contexts inspected. Model2 at `cf2244e9d`; inset math1, engine check and build pass at `00a450458`; scoped format/diff and changed-Rust readability pass. No toolkit/server edits. Exact commands, source/asset/binary/capture hashes and retained setup/partial failures: `data/diagnostics/ui-questlog-loot-hitarea-2026-10-16/loot-hitarea-proof-ledger.json`; PNGs: `/syncthing/AgentShared/2026-10-16/ui-questlog-loot-hitarea/`.

### Native Godot evidence and money-card overflow

Native integration and bounded proof are tracked in [[godot-conversion#Native loot — implemented, proof pending]]. Shared exports and targeted tests (8 + 1), ordered relay wire test (1 at `c6ae14bf`) and root compilation have passed. Runtime `af03660f` reaches `LOOT_DONE` after four Auto Loot/Shift cases plus inventory/error/cursor assertions; post-DONE RenderingServer-null shutdown exits 101, unresolved and explicitly deferred. Independent final acceptance remains pending; no source-pinning requirement.

The [rendered overflow investigation](godot-conversion.md#native-options-and-loot-money-overflow--rendered-red-fixes-awaiting-main-rendering) records native money text needing 51 px (`15 × 3 + 3 × 2`) at authored font size 12, versus its 38 px label and 46 px card. `d6f39c45` reduces explicit fixed multiline projection line gaps to fit native glyph metrics and shadow, without smaller fonts, truncation or clipping. Main rendered proof remains pending, not GREEN.

## Flight

- `TaxiMap` fills `TaxiMapState`; `scenes/flight_map` loads the continent art once per `MapID` from the UiMap CSVs (`FlightMapArt::load`) and lays out tiles, pins, route dots and the tooltip; hovering updates `TaxiMapState.hovered`.
- `MovementControl` (replicated on every `Player`, `#[require]`d) drives `networking/server_movement.rs`, which runs between `player_movement` and `camera_follow`: a new epoch snaps the local transform to the replicated `Position`; while `controlled`, the transform follows it like a remote unit and `player_movement` stops input and local physics.
- The server moves the player along the `TaxiPathNode` points; the mount is the replicated `Mounted` display, so a flying player renders as the gryphon without a rider.

## Gotchas

- UI textures load only from `data/textures/<fdid>.blp`; they are not extracted on demand. Extract with `asset-resolver`'s `casc-local <fdid>... -o data/textures`. 23 Eastern Kingdoms map tiles are not in the local CASC install.
- Zone crossings during a flight enter the Loading state; `OnExit(InWorld)` resets loot and taxi state.
- The gossip option frame is `GossipOption<id>Text` (the clickable font string), not `GossipOption<id>`.

## Sources

- [loot-frame spec](../../specs/loot-frame.md), [flight-master spec](../../specs/flight-master.md)
- Live run: `data/diagnostics/npcloot-20260925/`
- Native evidence and overflow sources: [[godot-conversion#Native loot — implemented, proof pending]]; `/tmp/claude/native-ui-overflow-red.log`; `godot/rust/src/ui/projection.rs`.

## See Also

- [[merchant-frame]] — the same Retail-window scene pattern and bag deltas
- [[networking]] — replication mirror that now carries `MovementControl`
- [[animation]] — the `DeathPose` exception to movement animation
- [[unit-tooltip]] — NPC tooltips list the loot table's drops; grey drops arrive as coins in the money slot (game-server `loot.md` junk rule)
