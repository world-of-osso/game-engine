## The 2959 hillside pond is legacy material-1 water with zero colour inputs
## Healer ground rings drawn on the visible surface

Client-only rings now sit under each party member: a clockwise-shrinking health arc
hidden above 92%, a pulsing outer ring for the dispel schools of the player's known
DB2 spells, screen-edge chevrons for off-screen members and click picking on the drawn
arc. Scope defaults on for healer parties, off for other roles and raids (lowest-N
opt-in), and the HUD page carries the controls in both skins. Forever captures and
pixel diffs live in `/syncthing/AgentShared/2026-11-05/healer-ground-ring/`; the
feature's own cost measures 0.12-0.16 ms per debug frame. [[healer-ground-rings]]

## Object-42 A/B on real release builds; streaming and legacy water colour still open
## Liquid colour was bound from the wrong row, so legacy water drew black

The Forever LightData rows leave all four water columns at zero on every zone slot; the only row that authors them is the Light's fixed record slot (0x001E303E close, 0x00001317 far). `river_*`/`ocean_*` therefore bound RGB zero with alpha only. The zone colour now wins where it is authored and the fixed row fills only the zero channels, so retail is unchanged and Forever water is no longer black. [[northshire-pale-water#Liquid colour now comes from the authored row — 2026-11-05]].

Consecration showed nothing because an area trigger replayed only the cast bursts (59536/189337)
and its golden circle lives in the display kit (59234/187938, `SpellVisualEvent` 9 -> 10). DB2
events 9/10 now decode to `DisplayStart`/`DisplayEnd` (cache format 20), the kits are held for
the trigger's lifetime, and an effect that authors `SpellVisualEffectName.EffectRadius` is fitted
to the trigger radius. Measured against a same-frame no-effect diff: Consecration 65,735 ->
331,089 px, Death and Decay 136,200 px at 9 s with no burst in flight. [[spell-visuals#Area trigger ground visuals (2026-11-05)]].
`bevy_to_tile_coords` returns `(row, col)`, which is also the ADT file-name order and MAIN's `(index % 64, index / 64)` order — proven by `azeroth_32_48`'s own first KNCM position and by Stormwind loading 9 tiles on the untouched streamer. The harness pose inversion had row and col swapped, so every snapped pose asked for the transposed tile, which `queue_tile` correctly refuses in silence. A one-sided ordering swap appeared to fix it and then loaded other regions' ADTs into the slots, rendering floating sheets with sky gaps. Harness fixed, streamer unchanged; `WORLD_CENSUS_PLACEMENT` and `core/tests/adt_tile_seams.rs` (129 shared vertices, 0.0 mismatch) now guard it. [[northshire-pale-water#The census harness inverted the tile axes — 2026-11-05]].
Same failure class as the Forever maps but a different shape: the M2 is published and only its `skin` is not (`model 201781 ... FDID 507146.skin`). The client resolves retail product `wow` build 12.1.5.70783, key `bc493ecb2898766c0f8947a99b4b3d65`, from `/syncthing/World of Warcraft`, not `/mnt/c`. `scripts/publish_forever_census.py` now takes `--product/--build/--build-key`; seeding it with the failing M2s published 53 receipts and both Orgrimmar poses went to 0 failures. [[northshire-pale-water#Retail receipts: skin-only gaps at Orgrimmar — 2026-11-05]].
Four MH2O layers at `y=14.7` (`liquid_type 5`, `liquid_object 17921`, `material 1`, `LVF 0`, `water.gdshader`) cover it. The shader mixes the `river_*`/`ocean_*` globals, and those bind as `[0,0,0,0.5]/[0,0,0,1]` and `[0,0,0,0.75]/[0,0,0,1]` — RGB all zero, alpha only. The LiquidType colour triple is zero too, so the surface is specular and foam over black. This is the legacy material-1 gap (per-view environment records, writer untraced), not a liquid-path bug; the ocean layers on the same map bind the same pair. [[northshire-pale-water#The 2959 hillside pond is legacy material-1 water whose colour inputs are all zero — 2026-11-05]].
Paired Linux-release A/B at the Stormwind indoor pose (same client, Forever, 1920x1080, host load recorded) names the two regressions behind the failed 120 FPS gate: `d5f97ee2a` reads every unit's merged model bounds each frame (6.90 ms of a 10.37 ms Culling step for 53 units) and the 2026-11-05 foot IK solve ran every unit within 45 yd, scanning all 16,691 doodads per foot. Caching the bounds at visual/equipment attach and skipping unsampled poses plus an 8-yard doodad column index took Culling 10.70 -> 1.92 ms and Foot IK 6.26 -> 0.25 ms, with 16,691/16,691 placements keeping identical visibility and shadow modes. [[godot-stormwind-fps#Culling and foot IK regression (2026-11-06)]].
