# World loading screen (Godot client)

World entry and map transfers hide loading once the player's local entry bubble is ready. Distant objects continue streaming; the loading gate must not wait for every placement sharing an ADT tile. Readiness follows `lib.rs` `update_loading_readiness` → `loading::evaluate_native_loading` and the shared terrain rules (`godot/core/src/game/state/loading_readiness.rs`).

## What it must do

- [x] Require the selected local player and their settled model, a successful map WDT, and attached center terrain. Preserve explicit terrain failure and missing-asset reporting.
- [x] Require attached terrain for every tile intersecting the conservative X/Z square extending 100 yards from the player. Queue those tiles' objects before evaluating their readiness; an undiscovered neighbouring tile must not create a false-ready frame.
- [x] Require ADT/MODD doodad origins within a 100-yard sphere of the player, WMO roots whose authored MODF extents intersect that sphere, and WMO collision groups whose transformed bounds intersect it. A large WMO is required by its extent even if its origin is distant. WMO roots still build all their render batches; only their distant doodads and collision groups are nonblocking.
- [x] Register a WMO's active MODD doodads before marking its root complete. Nearby children remain gate prerequisites even when discovered after the initial ADT queue.
- [x] Prioritize nearby queued/ready placements and their asset loads, including newly discovered WMO doodads. Build local terrain tiles center-first. Far placements are retained, not omitted or marked complete to release loading.

## Map asset identity

World entry and transfers use the same directory-based terrain reader. Retail and Forever Map exports are merged explicitly (retail wins conflicts); unnamed Forever WDT/MAID files load from their FileDataID cache. [Zephras world-map contract](zephras-world-map.md) records required provisioning, no-WDL behavior and incomplete local archive coverage. Spatial readiness/failure rules above are unchanged.

## Completion and streaming

- A failed placement is reported through `godot_error!` and `world_objects.failures`, and counts as settled. No new failure bypass or timeout increase.
- Loading displays `Loading objects done/total...` for the nearby subset. `world_objects.pending` remains the full streaming queue; `nearby_done`, `nearby_total`, and `nearby_collision_pending` expose the actual gate subset.
- Distant placements and collision groups keep building after InWorld. Their completion cannot re-enter loading.
- While loading, terrain and object resource construction use larger bounded slices to amortize slow rendered frames. Restore interactive streaming budgets on InWorld; preserve the same local prerequisites and required WMO batches.
- A WMO-only map preserves the global-WMO gate; it does not require nonexistent ADT tiles.
- The 100-yard bubble is client policy, not a reverse-engineered Retail constant. It covers the default 15-yard camera and roughly fourteen seconds of ordinary 7-yard/s movement. M2 membership uses authored origins; WMO/collision membership uses bounds to avoid losing nearby city geometry with a remote root origin. Visual draw distances remain unchanged.

## Reasoning and evidence

Tile ownership is not a local-readiness boundary: a city WMO can reference objects in distant districts, and terrain parsing completion is not terrain GPU attachment completion. The gate tracks spatial prerequisites and discovery ordering while retaining the rest of the streaming workload. [Stormwind investigation](../wiki/investigations/stormwind-loading.md) records root-cause measurements, before/after rendered timing, separate headless full-drain proof, and host/cache limitations. Evidence lives in `data/diagnostics/swload-2026-10-05/` and the rendered resource-scheduling follow-up `data/diagnostics/swload2-2026-10-06/`.

## Implementation and behavioral tests

- `terrain/object_progress.rs`: spatial progress, dynamic WMO-child discovery, inclusive radius, idempotent completion, and moved-player tests.
- `loading.rs`: nearby terrain coverage and existing terrain/character/global-WMO/readiness tests.
- `terrain/objects.rs`: individual-placement priority and retained full stream.
- `terrain/material.rs`: local terrain build priority.
- `wmo/collision.rs`: nearby collision readiness with unchanged nearest-first construction.
- `godot/tests/stormwind_loading.gd`: isolated UDP 5280 integration regression. InWorld requires finished nearby objects/collision while distant pending remains; distant work must then drain without re-entering loading. See [WMO floor collision](wmo-floor-collision.md).

Historical Northshire/Goldshire tile-gate measurements are superseded by this spatial policy; they were not Stormwind acceptance evidence.
