# Stormwind loading gate

The tile-wide world-entry gate required distant city scenery, and terrain GPU construction used sorted neighbouring tiles ahead of the player's tile. Spatial entry prerequisites replace tile ownership; all distant work remains queued. Policy is specified once in [world loading](../../specs/world-loading.md).

## Reproduction and baseline

On October 5, 2026, instrumented `6049107a` ran against an isolated copy of the `c8cd38f` server database, account `fb_swload`, UDP 5280, Weston socket `sw7`, and the same Dozen Vulkan recipe used by the original audit. Trade District position: engine `(-8928, 99.295, -606)`, area 1519, map 0. Shared UDP 5000 and the original server database were untouched.

Entry did not reach InWorld in 480 seconds. At the deadline terrain parsing pending was zero, full object pending was 7,370, and the center gate was 2,044/2,891. The count was draining, not permanently stuck: initial ADT/WMO total 1,714 expanded to 2,891 when the first city WMO registered 1,177 active MODD doodads. Additional roots were still waiting. Only 54 of 1,703 center-tile ADT doodad origins are within 100 yards.

Opt-in `GAME_PROFILE_MS=0.1` spans measured worker parse/decode, main-thread texture upload, placement steps and WMO build slices. Recorded placement medians: ADT 0.9 ms, MODD 5.9 ms; p95 4.1/15.5 ms. The first WMO needed 133 build slices totalling 1,244.3 ms, but sparse rendered frames spread them across roughly a minute. Main-thread upload median 0.3 ms, maximum 463.2 ms; WMO worker median 9.3 ms, maximum 235.1 ms. Placement spans include prerequisite lookup and some load dispatch, and values below 0.1 ms are absent. These are diagnostic samples, not unbiased benchmarks.

The center terrain became attached at 119.4 seconds despite parsed terrain pending reaching zero much earlier. `TerrainMaterials::next_tile` selected the BTree-sorted initial neighbours first. Both rendering-resource budgets are per frame; this host's slow rendered frames amplify unrelated queued work. Raising the fixture/product timeout or discarding objects would not correct that readiness boundary.

## Fix

`NearbyProgress` records authored positions/bounds and settlement by placement identity. Nearby WMO children are registered before their root completes, so root attachment cannot produce a false-ready frame. Loading prioritizes individual nearby placements and asset loads rather than all placements in the center tile. Required terrain tiles build center-first. Collision construction remains nearest-first; only nearby group bounds hold the gate. Far objects remain pending and continue spawning after entry.

## After and acceptance

Rendered `7c214055` reached InWorld in **80.744 seconds**, with nearby progress **494/494**, nearby collision pending **0**, full object pending **10,530**, and failures **0**. Distant pending then fell to **9,766** by 172.447 seconds while the screen remained InWorld. The captured first frame is `after-inworld.webp`. Same launch recipe and retained caches were used; the after run is warm, so this pair is not an isolated performance benchmark.

The corrected private-server headless fixture (`da4d06a6`, same Rust code) entered in **2.515 seconds** with **10,622** distant placements and **6** distant center-tile collision groups still pending. It stayed InWorld and reached full object pending **0** in **14.241 seconds**, with failures **0** and 8,059 unique ADT doodads/WMO roots spawned (MODD children are not in that counter). This separately proves retained work drains; headless timing is not rendered timing. Its first attempt used Godot's 64×64 dummy viewport and missed clipped UI controls; the fixture now matches the live 1920×1080 viewport and waits for layout before pointer input.

Two spatial-progress tests, six native loading tests and one real-asset WMO-extent test passed through the locked local helper. Extension/CLI build had no compiler warnings; changed Rust packages passed formatting. Existing spell-attachment errors and ObjectDB/texture/font shutdown leaks remain outside this loading fix; acceptance is not a clean-resource-lifetime claim. All owned PIDs were gone, `agents-swload.slice` stopped, UDP 5280 free, and the shared UDP 5000 server PID unchanged after cleanup.

## Proof

Baseline samples, profile summary, argv, server setup and logs: `data/diagnostics/swload-2026-10-05/`. Automated coverage includes dynamic child discovery, moved-player requirements, inclusive boundaries, duplicate completion, tile-edge coverage, and `godot/tests/stormwind_loading.gd` for real private-server entry with nonzero distant pending and eventual drain. Before/after rendered and headless proof must be interpreted separately.

## Sources

- [World loading contract](../../specs/world-loading.md) — gate policy and rationale.
- [Object streaming](../../../godot/rust/src/terrain/objects.rs) — worker, queue, spawn, WMO-child ordering.
- [Spatial progress](../../../godot/rust/src/terrain/object_progress.rs) — nearby prerequisite accounting.
- [Terrain materials](../../../godot/rust/src/terrain/material.rs) — budgeted tile build order.

## See Also

- [[godot-conversion]] — native loading lifecycle.
- [WMO floor collision](../../specs/wmo-floor-collision.md) — collision readiness.
