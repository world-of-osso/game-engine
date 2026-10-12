# Collision System

Players stand on terrain, WMO floors and authored M2 doodad collision floors. [[player-ground]] owns the shared height rules and current native/server wiring. Native horizontal wall rays remain WMO-only; M2 camera bodies retain complete authored hulls.

## Geometry Layers

| Layer | Floor method | Wall method |
|-------|-------------|-------------|
| **Terrain** | `TerrainHeightmap::height_at`, a candidate in `shared::ground` | Slope validation (terrain to terrain) |
| **WMO** | MOPY-collidable BSP faces (`WmoFloors`, [[player-ground]]) | Horizontal mesh ray from player height |
| **M2** | Authored-normal floor bake, shared BVH height query ([[player-ground]]) | Native camera: complete authored hull; player wall rays remain WMO-only |

## Player Movement Pipeline (per frame)

1. Apply input → candidate position
2. Clamp native horizontal movement against WMO physics bodies
3. Resolve grounded state, gravity and snap: highest terrain, WMO or baked M2 floor within step reach

## WMO Collision

A horizontal ray begins `0.6` units above the current player position, filters to WMO collision meshes, and clamps movement before a hit with a `0.05` margin. It ignores visibility: portal-culled and off-screen walls block. Antiportal groups (MOGP 0x4000000) carry no `WmoCollisionMesh` and never block. Vertical support comes from the WMO collision faces, not from this ray (see [[player-ground]]).

## M2 Collision

Doodad placements use parsed authored M2 collision indices and vertices. Their world-space AABB is broadphase only; a candidate continues to a backface-inclusive triangle ray test through the placement affine transform. An inside-AABB ray does not block without a triangle hit.

An unannotated model with no authored collision indices has no solid doodad collider. Render geometry never replaces absent collision geometry. Native movement's floor support comes from the dedicated M2 collision bake described in [[player-ground]], independent of render-node spawning or visibility; general player M2 wall sweeps are not added by that change.

## Ground Resolution Priority

`update_grounded` and `apply_gravity_and_ground_snap` probe `WorldGround` ([[player-ground]]). Vertical physics freezes only while the native terrain tile under the feet is unloaded. With nothing within step reach, the player falls. Baked M2 floor faces compete with terrain/WMO support; downward authored normals are excluded at bake time.

## Camera Collision

WMO and M2 raycasts from pivot toward camera; minimum hit distance sets orbit length with `CAM_RADIUS = 0.3f` pull-in. Terrain floor clamp prevents clipping below ground, except while the eye stands in a WMO interior group, whose walls alone bound the camera. Smooth interpolation via `1 - exp(-speed * dt)`. The smoothed position is ray-checked again from the eye and pulled in front of the first blocker, because the straight smoothing path can cut through stair noses ([[stockade-entrance]]).

The Godot client casts camera rays against Godot physics: terrain chunk bodies (layer 1) and WMO wall bodies (layer 2) built from the shared WMO collision faces, which are independent of the render nodes and so of culling ([[stockade-entrance]]). Its player wall ray queries the WMO bodies only, as Bevy's queries `WmoCollisionMesh` only. Separately, native flying movement now queries annotated tree capsules through Godot broadphase and an analytic sphere sweep. Annotated ADT trees replace their authored-triangle camera bodies with fixed trunk/currently bent limb capsules on tree and doodad layers; these capsules are not the M2 floor source. Authored collision floors are baked separately ([[player-ground]]). Source integration is present, runtime acceptance pending; see [[elastic-trees]].

Portal culling decides what is drawn, not what is solid: the camera also collides with the batch meshes (`WmoCollisionMesh`) of WMO groups that portal culling hid, except antiportal groups. Before this, a camera classified outside a group lost that group's walls and escaped for good ([[stockade-entrance]]).

Collision uses `RayCastVisibility::Visible`, not Bevy's `VisibleInView` default. A wall clipped behind the camera after collision is no longer in that camera's frustum but still has inherited visibility and must continue blocking recovery. Hierarchically hidden walls remain excluded, so hidden scene geometry does not create invisible camera collision. Commit `7d1d8a86` adds a regression with actual transform propagation, visibility propagation, and frustum updates: the old policy recovers through the view-culled wall; the corrected policy remains clipped, then recovers after the parent becomes hidden. Original-video pixel equivalence during camera motion remains unproven.

## Sources

- [M2 floor contract](../../specs/m2-floor-collision.md) — native/server support and proof boundary
- `godot/rust/src/ground.rs`, `terrain/assets.rs` — native movement and pre-render floor loading
- `src/collision.rs` — historical retired Bevy collision implementation
- `../../data/diagnostics/npc-motion-20260909/wmo-basis-proof.json` — placement proof that does not establish floor support
- [wowee-collision.md](../../wowee-collision.md) — broader collision reference

## See Also

- [[player-ground]]: how the terrain and WMO floor rule works on the client and the server
- [[character-generation]] — characters are the moving entities that drive collision queries
- [[open-source-wow-clients]] — WoWee is the reference client analyzed here
- [[terrain]] — ADT object placement and doodad collision
- [[elastic-trees]] — manually annotated native flying contact and capsule camera geometry
- [[rendering-pipeline]] — rendered foliage depth coverage and camera collision rendering context
