# Collision System

Collision is split across three geometry layers — terrain heightmap, WMO (world map objects), and M2 (doodads/props) — each with its own floor-height and wall-collision implementation. The `CameraController` orchestrates all three each frame.

## Geometry Layers

| Layer | Floor method | Wall method |
|-------|-------------|-------------|
| **Terrain** | Bilinear heightmap interpolation | — |
| **WMO** | Downward ray through collision triangles | Swept + closest-point push against cylinder |
| **M2** | Ray through mesh triangles or AABB top | Closest-point push (mesh) or AABB segment test |

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

AABBs are fitted per category (tree trunks, narrow posts, small solid props, stepped low platforms, default) with per-category XY/Z scale factors. Some M2s (fountains, low platforms) have radial height profiles instead of flat tops.

Models with a collision mesh use ray/closest-point against that mesh. Models without one fall back to AABB segment intersection.

## Ground Resolution Priority

Combined in `CameraController` with slope rejection: terrain (`min walkable normal = 0.70`) + WMO (`0.45`, allows ramps) + M2. Seam stability: downward floor step is capped per frame. 5-point footprint sampling on both WMO and M2 catches narrow planks and bridges. Results are cached until the player moves.

## Camera Collision

WMO and M2 raycasts from pivot toward camera; minimum hit distance sets orbit length with `CAM_RADIUS = 0.3f` pull-in. Terrain floor clamp prevents clipping below ground. Smooth interpolation via `1 - exp(-speed * dt)`.

## Spatial Acceleration

Both renderers use: world AABB broadphase per instance → per-group bounds → per-mesh spatial grid (triangles filtered by XY range and Z bounds). Triangle Z bounds enable fast vertical rejection before ray tests.

## Sources

- [wowee-collision.md](../../wowee-collision.md) — WoWee source analysis with file/line references

## See Also

- [[character-generation]] — characters are the moving entities that drive collision queries
- [[open-source-wow-clients]] — WoWee is the reference client analyzed here
