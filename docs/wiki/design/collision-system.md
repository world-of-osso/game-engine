# Collision System

Players are held up by terrain and by WMO floors, through the shared ground rule described in [[player-ground]]. Horizontal blocking is done by WMO render meshes and M2 collision triangles. M2 floors are not implemented.

## Geometry Layers

| Layer | Floor method | Wall method |
|-------|-------------|-------------|
| **Terrain** | `TerrainHeightmap::height_at`, a candidate in `shared::ground` | Slope validation (terrain to terrain) |
| **WMO** | MOPY-collidable BSP faces (`WmoFloors`, [[player-ground]]) | Horizontal mesh ray from player height |
| **M2** | Not implemented | Horizontal authored-triangle ray after AABB broadphase |

## Player Movement Pipeline (per frame)

1. Apply input → candidate position
2. Clamp horizontal movement against WMO and authored M2 collision meshes
3. Resolve grounded state, gravity and snap from `WorldGround`: the highest terrain or WMO floor within step reach

## WMO Collision

A horizontal ray begins `0.6` units above the current player position, filters to WMO collision meshes, and clamps movement before a hit with a `0.05` margin. It ignores visibility: portal-culled and off-screen walls block. Antiportal groups (MOGP 0x4000000) carry no `WmoCollisionMesh` and never block. Vertical support comes from the WMO collision faces, not from this ray (see [[player-ground]]).

## M2 Collision

Doodad placements use parsed authored M2 collision indices and vertices. Their world-space AABB is broadphase only; a candidate continues to a backface-inclusive triangle ray test through the placement affine transform. An inside-AABB ray does not block without a triangle hit.

An unannotated model with no authored collision indices has no solid doodad collider. Render/visual bounds are retained separately for portal/contact interactions and never become a solidity fallback. This current implementation does not establish the broader sweep, closest-point, floor, or spatial-grid design described elsewhere on this page.

## Ground Resolution Priority

`update_grounded` and `apply_gravity_and_ground_snap` probe `WorldGround` ([[player-ground]]). Vertical physics freezes only while the tile under the feet is unloaded. With nothing within step reach, the player falls. M2 collision triangles do not support the player vertically.

## Camera Collision

WMO and M2 raycasts from pivot toward camera; minimum hit distance sets orbit length with `CAM_RADIUS = 0.3f` pull-in. Terrain floor clamp prevents clipping below ground, except while the eye stands in a WMO interior group, whose walls alone bound the camera. Smooth interpolation via `1 - exp(-speed * dt)`. The smoothed position is ray-checked again from the eye and pulled in front of the first blocker, because the straight smoothing path can cut through stair noses ([[stockade-entrance]]).

The Godot client casts camera rays against Godot physics: terrain chunk bodies (layer 1) and WMO wall bodies (layer 2) built from the shared WMO collision faces, which are independent of the render nodes and so of culling ([[stockade-entrance]]). Its player wall ray queries the WMO bodies only, as Bevy's queries `WmoCollisionMesh` only. Separately, native flying movement now queries annotated tree capsules through Godot broadphase and an analytic sphere sweep. Annotated ADT trees replace their authored-triangle camera bodies with fixed trunk/currently bent limb capsules on tree and doodad layers; this supplies no M2 floor support. Source integration is present, runtime acceptance pending; see [[elastic-trees]].

Portal culling decides what is drawn, not what is solid: the camera also collides with the batch meshes (`WmoCollisionMesh`) of WMO groups that portal culling hid, except antiportal groups. Before this, a camera classified outside a group lost that group's walls and escaped for good ([[stockade-entrance]]).

Collision uses `RayCastVisibility::Visible`, not Bevy's `VisibleInView` default. A wall clipped behind the camera after collision is no longer in that camera's frustum but still has inherited visibility and must continue blocking recovery. Hierarchically hidden walls remain excluded, so hidden scene geometry does not create invisible camera collision. Commit `7d1d8a86` adds a regression with actual transform propagation, visibility propagation, and frustum updates: the old policy recovers through the view-culled wall; the corrected policy remains clipped, then recovers after the parent becomes hidden. Original-video pixel equivalence during camera motion remains unproven.

## Sources

- `src/collision.rs` — current terrain grounding and horizontal WMO/M2 collision implementation
- `../../data/diagnostics/npc-motion-20260909/wmo-basis-proof.json` — placement proof that does not establish floor support
- [wowee-collision.md](../../wowee-collision.md) — broader collision reference

## See Also

- [[player-ground]]: how the terrain and WMO floor rule works on the client and the server
- [[character-generation]] — characters are the moving entities that drive collision queries
- [[open-source-wow-clients]] — WoWee is the reference client analyzed here
- [[terrain]] — ADT object placement and doodad collision
- [[elastic-trees]] — manually annotated native flying contact and capsule camera geometry
- [[rendering-pipeline]] — rendered foliage depth coverage and camera collision rendering context
