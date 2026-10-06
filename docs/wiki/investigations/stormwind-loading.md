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

## Rendered-frame scheduling follow-up (October 6, 2026)

The isolated `swload2` account on UDP 5286 and Weston `sw10`, at the same Trade District position and 1920×1080 Dozen Vulkan setup, reached InWorld in **37.788 s**, with first rendered frame at **38.315 s**, before the scheduling change. All **494/494** local placements were ready, collision pending and failures were zero. This is a new warm baseline, not a reproduction of the earlier 80.744-second host/cache state.

Entry frame callbacks totalled **8.826 s** across 197 measured frames; object processing took **2.677 s** and terrain resource processing **2.703 s**. The three WMO builds consumed 130 slices despite only 1.331 s inside their build steps. Both systems allocated only 8 ms of resource work per rendered frame. Sampled viewport GPU median was 27.6 ms (maximum 76.3 ms), while many whole frame intervals were much longer. Repeated tiny work slices amplify rendered-frame latency; worker parse/decode and shader-resource setup are separately instrumented, not inferred from placement time.

Terrain and object resource construction now share a **64-ms loading slice**, with one deadline covering each call and its WMO/chunk continuations. Clearing loading focus/terrain priorities restores the original interactive budgets. No local prerequisites, WMO batches or queued distant placements are removed. The owned-server fixture `godot/tests/stormwind_entry_profile.gd` verifies local readiness, zero failures and continued distant streaming, records first-draw time separately, and collects viewport CPU/GPU time and pipeline compilation counters. At `b1fd59d7`, identical rendered args reached InWorld in **11.715 s**, first draw in **12.048 s**, with **494/494**, zero nearby collision pending and zero placement failures. Distant pending was **10,494** at entry and **10,379** by 15.946 s. Entry frames fell **199 → 29**, WMO slices **130 → 17**. Interactive budgets remain 8 ms; the unchanged distant collision budget remains 2 ms.

### Entry-window profile

All spans are enabled (`GAME_PROFILE_MS=0`). The aggregation window runs from the Loading screen attachment span through the InWorld observation; startup/character-select costs are excluded. Work queued ahead, including distant asset preparation, is included when it occurs in that window.

| Phase (wall seconds) | Before | After |
|---|---:|---:|
| Instrumented M2/WMO/BLP asset IO | 0.670 | 0.294 |
| BLP decode | 0.110 | 0.093 |
| M2 parse | 0.181 | 0.134 |
| WMO parse and collision parse | 0.091 | 0.070 |
| Mesh build (worker and Godot resources) | 2.791 | 2.115 |
| Material creation | 0.928 | 0.781 |
| Shader resource setup | 0.045 | 0.031 |
| Main-thread placements / WMO build steps | 2.105 | 1.389 |
| Texture upload API | 0.736 | 0.676 |
| Entire client frame callbacks | 8.826 | 5.532 |

Nested phase totals overlap and are not additive. Upload and shader setup spans measure API wall time, not isolated deferred GPU/driver execution. Loading viewport GPU frame medians were **27.755 → 25.238 ms**; viewport CPU medians **9.882 → 10.740 ms**. Last-loading pipeline counters were identical: surface **57**, draw **22**, specialization **28**. These counters do not provide separate GPU compilation durations.

The measured remaining floor is substantial synchronous resource work: mesh build 2.115 s, upload API 0.676 s, and total client callbacks 5.532 s. The rest of first-draw wall time contains account/map entry, Godot engine work, rendering/submission and inter-frame waits; those are not fully separated. This is a warm Dozen/WSL run on a shared host, not a cold-cache or hardware-independent Retail benchmark. The previous 80.744-second result is historical, not the paired baseline.

Headless dummy rendering avoids real uploads/rendering and advances the same small streaming budgets without expensive rendered frames; comparing its **full drain** against rendered **entry** conflates two boundaries. The new owned headless fixture entered in **2.224 s** with 8,032 distant placements pending and drained all object work by **9.699 s**, staying InWorld with zero failures. It confirms no objects were discarded. Six native readiness tests and two spatial-progress tests passed through the locked local helper; native build and package formatting passed. Existing spell-attachment errors and shutdown texture/font/ObjectDB leaks remain outside this fix.

Evidence and exact argv: `data/diagnostics/swload2-2026-10-06/` (`before3.log`, `after.log`, `before-profile.json`, `after-profile.json`, `headless2.log`, targeted test logs and proof ledger).

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
