# WoWee Collision Detection

Reference documentation for collision detection in [WoWee](https://github.com/AridTag/WoWee) (`~/Repos/WoWee/`), a C++ WoW 3.3.5a client. Source files referenced below are relative to that repo.

## Architecture Overview

Collision is split across three geometry types, each with its own renderer:

| Layer | Renderer | Geometry | Purpose |
|-------|----------|----------|---------|
| **Terrain** | `TerrainManager` | Heightmap grid | `getHeightAt(x, y)` — bilinear interpolation |
| **WMO** | `WMORenderer` | Collision triangles per group | Floors, walls, interior detection |
| **M2** | `M2Renderer` | AABB + optional collision mesh | Doodad floors and wall blocking |

The `CameraController` (`src/rendering/camera_controller.cpp`) orchestrates all three: it moves the player, resolves ground height, runs wall collision sweeps, and positions the camera.

## Player Movement Pipeline

Each frame in `CameraController::update()`:

1. **Apply movement input** → candidate `targetPos`
2. **Swimming**: floor clamp + horizontal wall sweep (WMO + M2)
3. **Refresh WMO interior state** (`isInsideWMO`, `isInsideInteriorWMO`)
4. **Sweep collision** — sub-step wall checks to prevent tunneling
5. **Ground resolution** — multi-source floor height with priority logic
6. **Camera orbit** — collision-clipped distance from pivot
7. **Void fall detection** — auto-unstuck after prolonged freefall

## Sweep Collision (Anti-Tunneling)

`camera_controller.cpp:603-643`

Movement is split into small sub-steps. Step size adapts to context:

```
stepSize = insideWMO ? 0.20f : 0.35f
sweepSteps = clamp(ceil(moveDist / stepSize), 1, 8)
```

Each step:
1. `wmoRenderer->checkWallCollision(stepPos, candidate, adjusted)` — XY push, accept upward Z (ramps)
2. `m2Renderer->checkCollision(stepPos, candidate, adjusted)` — XY push only

The candidate position accumulates pushback across steps.

## WMO Collision

### Floor Height — `WMORenderer::getFloorHeight()`

`wmo_renderer.cpp:2636-2775`

- Vertical ray cast downward from `(x, y, z+500)` through collision triangles
- **Spatial grid**: `group.getTrianglesInRange()` narrows candidate triangles by XY range
- **Two-sided** ray-triangle (Möller–Trumbore), tries both windings
- Returns highest walkable floor ≤ query Z + margin
- Outputs surface normal Z component for slope rejection
- **Multi-level culling**: world AABB → world group bounds → local AABB → spatial grid → ray test

### Wall Collision — `WMORenderer::checkWallCollision()`

`wmo_renderer.cpp:2777-2967`

Player modeled as a **horizontal cylinder**:

```cpp
PLAYER_RADIUS = insideWMO ? 0.45f : 0.50f
PLAYER_HEIGHT = 2.0f
MAX_STEP_HEIGHT = 1.0f
```

Two collision responses per triangle:

1. **Swept test** — detects plane crossing between `from` and `to` (prevents tunneling through thin walls). Pushes player back to safe side with capped force (`MAX_SWEPT_PUSH = 0.15f`).

2. **Closest-point push** — for penetrating geometry without crossing. Uses `closestPointOnTriangle()` (Ericson, Real-Time Collision Detection §5.1.5). Horizontal-only push with skin separation (`0.005f`).

Filters:
- Skip floor-like surfaces (`|normal.z| >= 0.35`)
- Skip low geometry below step-up height
- Skip short stair risers (`triHeight < 1.0 && maxZ <= feetZ + 1.2`)

### Interior Detection

- `isInsideWMO()` — point-in-group-AABB test across all WMO instances
- `isInsideInteriorWMO()` — same but checks group flags for interior marking
- `updateActiveGroup()` — tracks which group the player occupies, uses **portal refs** for neighbor traversal (avoids full scan each frame)

## M2 Collision

### Collision Shape Categories

`m2_renderer.cpp:72-114` — `getTightCollisionBounds()`

M2 models use fitted AABBs with per-category scaling:

| Category | XY Scale | Z Scale | Notes |
|----------|----------|---------|-------|
| **Tree trunk** | `clamp(horiz*0.05, 0.5, 5.0)` | `min(trunkHalf*2.5, 3.5)` | Cylinder at base, center shifted down |
| **Narrow vertical prop** | 0.30× | 0.96× | Lamps, posts — keep passable gaps |
| **Small solid prop** | 1.00× | 1.00× | Crates, chests — full bounds |
| **Stepped low platform** | 0.98× | 0.52× | Tree curbs, planters |
| **Default** | 0.66× | 0.76× | Tighter fit to avoid oversized blockers |

Categories are boolean flags on `M2ModelGPU` (e.g. `collisionTreeTrunk`, `collisionSmallSolidProp`).

### Stepped Surfaces

`m2_renderer.cpp:116-151` — `getEffectiveCollisionTopLocal()`

Some M2s have **radial height profiles** instead of flat tops:

**Fountains** (`collisionSteppedFountain`):
```
r > 0.85 → 18% height (outer lip)
r > 0.65 → 36% (mid step)
r > 0.45 → 54% (inner step)
r > 0.28 → 70% (center platform)
r > 0.14 → 84% (statue body)
else     → 96% (top)
```

**Low platforms** (`collisionSteppedLowPlatform`): use edge distance (max of |nx|, |ny|) instead of radial — prevents diagonal corner clip-through.

### Floor Height — `M2Renderer::getFloorHeight()`

`m2_renderer.cpp:3480-3623`

Two parallel approaches, highest wins:

1. **Mesh-based**: if `model.collision.valid()`, cast vertical ray against collision triangles via spatial grid (`getFloorTrisInRange`). Two-sided Möller–Trumbore. Rejects slopes steeper than ~70° (`|worldN.z| < 0.35`).

2. **AABB-based**: `getTightCollisionBounds()` + `getEffectiveCollisionTopLocal()`. Transforms local-space top to world space.

Filters: `collisionNoBlock`, `isInvisibleTrap`, `isSpellEffect` are skipped. Bridges get extended Z margin (25 units vs 2).

### Wall Collision — `M2Renderer::checkCollision()`

`m2_renderer.cpp:3625-3680+`

If `model.collision.valid()`, uses mesh-based closest-point push against wall triangles (same approach as WMO). Spatial grid via `getWallTrisInRange()`. Caps total push per instance (`MAX_TOTAL_PUSH = 0.02f`).

Falls back to AABB-based segment intersection (`segmentIntersectsAABB`) for models without collision mesh.

## Ground Resolution Priority

`camera_controller.cpp:646-910`

The camera controller combines terrain, WMO, and M2 floor heights with extensive priority logic:

1. **Terrain + WMO center sample** with slope rejection:
   - Terrain: `MIN_WALKABLE_NORMAL = 0.7` (~45°)
   - WMO: `MIN_WALKABLE_NORMAL = 0.45` (allows tunnel ramps)
2. **Inside-WMO seam handling**: prefers WMO floor when descending into tunnels
3. **Seam stability**: caps downward floor step per frame (`0.60f` normal, `2.0f` fast fall)
4. **Multi-sample WMO**: 5-point footprint (`±0.35f`) to catch narrow planks/boards
5. **Multi-sample M2**: 5-point footprint (`±0.4f`) for ships, platforms, bridges
6. **Snap conditions**: near-ground OR air-falling OR slope-grace
7. **No-ground grace**: `0.06s` micro-gap tolerance for seam misses

Collision results are cached (`COLLISION_CACHE_DISTANCE = 0.15f`) and invalidated when the player moves or descends.

## Camera Collision

`camera_controller.cpp:1004-1087`

1. **WMO raycast**: `wmoRenderer->raycastBoundingBoxes(pivot, camDir, maxDist)` — if hit, pull camera in by `CAM_RADIUS = 0.3f`
2. **M2 raycast**: same approach, takes minimum of WMO and M2 hit distances
3. **Terrain floor clamp**: ensures camera stays above terrain with `MIN_FLOOR_CLEARANCE = 0.35f`
4. **WMO floor clamp**: inside tunnels, probes near player height to avoid latching to ceiling
5. **Smooth interpolation**: `camLerp = 1 - exp(-CAM_SMOOTH_SPEED * dt)`
6. **Pivot lift**: when terrain behind camera is high, lifts pivot point to prevent terrain clipping (`clamp(0, 1.4)`)

## Spatial Acceleration

Both M2Renderer and WMORenderer use:

- **World-space AABB broadphase** per instance (cached `worldBoundsMin`/`worldBoundsMax`)
- **`gatherCandidates(queryMin, queryMax, scratch)`** — spatial query returning instance indices
- **Per-group world bounds** for WMO (cached `worldGroupBounds`)
- **Spatial grids** per collision mesh group: `getTrianglesInRange()`, `getWallTrianglesInRange()`, `getFloorTrisInRange()`
- **Pre-computed triangle Z bounds** (`triBounds[].minZ`, `maxZ`) for fast vertical rejection
- **Collision focus radius** — optionally limits queries to a radius around the player

## Key Algorithms

- **Möller–Trumbore** ray-triangle intersection (`m2_renderer.cpp:229-248`)
- **Closest point on triangle** (Ericson §5.1.5) (`m2_renderer.cpp:251-281`)
- **Segment-AABB intersection** slab method (`m2_renderer.cpp:153-180`)
- **AABB transform** via 8-corner projection (`m2_renderer.cpp:182-205`)

## Void Fall Recovery

`camera_controller.cpp:916-936`

- Safe position saved every `SAFE_POS_SAVE_INTERVAL` when grounded on real geometry
- After `AUTO_UNSTUCK_FALL_TIME` of continuous freefall, fires `autoUnstuckCallback_`
- Only fires once per fall sequence

## Visual Melee Guard

`m2_renderer.cpp` (application.cpp reference)

Client-side only: hostile melee units in attack range are nudged away from the player's position to prevent their model from rendering inside the player. No server position change.
