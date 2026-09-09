# Collision System

Collision is split across three geometry layers — terrain heightmap, WMO (world map objects), and M2 (doodads/props) — each with its own floor-height and wall-collision implementation. The `CameraController` orchestrates all three each frame.

## Geometry Layers

| Layer | Floor method | Wall method |
|-------|-------------|-------------|
| **Terrain** | Bilinear heightmap interpolation | — |
| **WMO** | Downward ray through collision triangles | Swept + closest-point push against cylinder |
| **M2** | Authored collision-triangle ray test | Authored collision-triangle ray test after AABB broadphase |

## Player Movement Pipeline (per frame)

1. Apply input → candidate position
2. Sweep collision — sub-step wall checks (WMO + M2) prevent tunneling
3. Ground resolution — multi-source floor height with priority logic
4. Camera orbit — collision-clipped distance from pivot
5. Void fall detection — auto-unstuck after prolonged freefall

## Sweep Collision (Anti-Tunneling)

Movement is split into sub-steps: `stepSize = insideWMO ? 0.20f : 0.35f`, capped at 8 steps. Each step runs WMO wall check first (allows upward Z for ramps), then M2 wall check (XY-only push). Pushback accumulates across steps.

## WMO Collision

Player modeled as a horizontal cylinder (`radius = 0.45–0.50f`, `height = 2.0f`, `maxStep = 1.0f`).

**Wall response**: swept test detects plane crossing (tunneling prevention); closest-point push handles penetration. Both reject floor-like surfaces (`|normal.z| >= 0.35`) and short stair risers.

**Interior tracking**: portal-ref neighbor traversal keeps the active group current without a full scan each frame. Interior WMO reduces step size and adjusts floor preference.

## M2 Collision

Doodad placements use parsed authored M2 collision indices and vertices. Their world-space AABB is broadphase only; a candidate continues to a backface-inclusive triangle ray test through the placement affine transform. An inside-AABB ray does not block without a triangle hit.

A model with no authored collision indices has no solid doodad collider. Render/visual bounds are retained separately for portal/contact interactions and never become a solidity fallback. This current implementation does not establish the broader sweep, closest-point, floor, or spatial-grid design described elsewhere on this page.

## Ground Resolution Priority

Combined in `CameraController` with slope rejection: terrain (`min walkable normal = 0.70`) + WMO (`0.45`, allows ramps) + M2. Seam stability: downward floor step is capped per frame. 5-point footprint sampling on both WMO and M2 catches narrow planks and bridges. Results are cached until the player moves.

## Camera Collision

WMO and M2 raycasts from pivot toward camera; minimum hit distance sets orbit length with `CAM_RADIUS = 0.3f` pull-in. Terrain floor clamp prevents clipping below ground. Smooth interpolation via `1 - exp(-speed * dt)`.

Collision uses `RayCastVisibility::Visible`, not Bevy's `VisibleInView` default. A wall clipped behind the camera after collision is no longer in that camera's frustum but still has inherited visibility and must continue blocking recovery. Hierarchically hidden walls remain excluded, so hidden scene geometry does not create invisible camera collision. Commit `7d1d8a86` adds a regression with actual transform propagation, visibility propagation, and frustum updates: the old policy recovers through the view-culled wall; the corrected policy remains clipped, then recovers after the parent becomes hidden. Original-video pixel equivalence during camera motion remains unproven.

## Spatial Acceleration

Both renderers use: world AABB broadphase per instance → per-group bounds → per-mesh spatial grid (triangles filtered by XY range and Z bounds). Triangle Z bounds enable fast vertical rejection before ray tests.

## Sources

- [wowee-collision.md](../../wowee-collision.md) — WoWee source analysis with file/line references
- `../../data/diagnostics/cpu-goal-resumed/doodad-authored-collision/report.md` — implementation and 45 scoped tests
- `../../../data/diagnostics/world-objects-20260909/camera-red.log` and `camera-green.log` — actual transform/visibility/frustum regression RED/GREEN output
- `game-engine` commit `7d1d8a86` — camera collision independent of view culling

## See Also

- [[character-generation]] — characters are the moving entities that drive collision queries
- [[open-source-wow-clients]] — WoWee is the reference client analyzed here
- [[terrain]] — ADT object placement and doodad collision
- [[rendering-pipeline]] — rendered foliage depth coverage and camera collision rendering context
