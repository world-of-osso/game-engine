# Collision System

Current player collision has terrain vertical support plus horizontal WMO/M2 blocking. WMO and M2 floor-height support are not implemented; do not infer it from rendered geometry or camera collision.

## Geometry Layers

| Layer | Floor method | Wall method |
|-------|-------------|-------------|
| **Terrain** | `TerrainHeightmap::height_at` | Slope validation |
| **WMO** | Not implemented | Horizontal mesh ray from player height |
| **M2** | Not implemented | Horizontal authored-triangle ray after AABB broadphase |

## Player Movement Pipeline (per frame)

1. Apply input → candidate position
2. Clamp horizontal movement against WMO and authored M2 collision meshes
3. Resolve grounded state, gravity, and snap from terrain height only

## WMO Collision

A horizontal ray begins `0.6` units above the current player position, filters to WMO collision meshes, and clamps movement before a hit with a `0.05` margin. It does not query WMO triangles for vertical support, portal containment, or an interior floor.

## M2 Collision

Doodad placements use parsed authored M2 collision indices and vertices. Their world-space AABB is broadphase only; a candidate continues to a backface-inclusive triangle ray test through the placement affine transform. An inside-AABB ray does not block without a triangle hit.

A model with no authored collision indices has no solid doodad collider. Render/visual bounds are retained separately for portal/contact interactions and never become a solidity fallback. This current implementation does not establish the broader sweep, closest-point, floor, or spatial-grid design described elsewhere on this page.

## Ground Resolution Priority

`update_grounded` and `apply_gravity_and_ground_snap` read `TerrainHeightmap` only. Missing terrain freezes vertical velocity; loaded terrain supplies the sole ground height. Neither WMO nor M2 collision triangles currently support the player vertically. The corrected WMO placement basis does not establish the reported free-fall cause; exact transformed floor geometry and bounded native validation remain pending.

## Camera Collision

WMO and M2 raycasts from pivot toward camera; minimum hit distance sets orbit length with `CAM_RADIUS = 0.3f` pull-in. Terrain floor clamp prevents clipping below ground. Smooth interpolation via `1 - exp(-speed * dt)`.

Collision uses `RayCastVisibility::Visible`, not Bevy's `VisibleInView` default. A wall clipped behind the camera after collision is no longer in that camera's frustum but still has inherited visibility and must continue blocking recovery. Hierarchically hidden walls remain excluded, so hidden scene geometry does not create invisible camera collision. Commit `7d1d8a86` adds a regression with actual transform propagation, visibility propagation, and frustum updates: the old policy recovers through the view-culled wall; the corrected policy remains clipped, then recovers after the parent becomes hidden. Original-video pixel equivalence during camera motion remains unproven.

## Sources

- `src/collision.rs` — current terrain grounding and horizontal WMO/M2 collision implementation
- `../../data/diagnostics/npc-motion-20260909/wmo-basis-proof.json` — placement proof that does not establish floor support
- [wowee-collision.md](../../wowee-collision.md) — broader collision reference

## See Also

- [[character-generation]] — characters are the moving entities that drive collision queries
- [[open-source-wow-clients]] — WoWee is the reference client analyzed here
- [[terrain]] — ADT object placement and doodad collision
- [[rendering-pipeline]] — rendered foliage depth coverage and camera collision rendering context
