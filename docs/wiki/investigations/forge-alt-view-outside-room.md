# Forge alt-view "seam" and "void" are an above-ceiling camera

The two defects reported on the alt-angle Forge capture (`lava-shore-alt-after.png`, camera
`(-300, 160, 600)`) are not liquid or terrain bugs. The camera sits above the pool room's
ceiling, outside its shell, so it sees the room's back faces culled, the flat `y=0` terrain
around the WMO, and both lava levels at once. A camera at `y <= 132` inside the room shows a
single bounded pool with no seam and no void.

## Evidence

- **Camera is outside the room.** From `y=132` (same x/z) the frame is fully enclosed: no sky,
  no grey plane. From `y=140` and `y=160` the west side opens onto sky, a flat blue-grey plane
  and the horizon. The room's ceiling is therefore between 132 and 140 yd; WMO geometry is
  single-sided, so a camera above the ceiling sees through the roof and the far wall.
- **The grey plane is terrain, not a missing floor.** `client.terrain_height_at` returns `0.0`
  for x -400..-100, z 300..600. Downward rays from `y=500` hit `TerrainCollision` at `y~0` and
  nothing else; no WMO floor exists under the open side. Map 2519's terrain is a flat slab with
  no diffuse layer, which is the flat grey-purple gradient in the capture.
- **The two lava sheets are two real pools, 72 yd apart.** Group-by-group screen isolation at
  the alt pose (hide all other liquid nodes, diff against a no-liquid render) gives
  `Group8_Liquid` on the left and `Group20_Liquid` beyond its tile edge on the right.
  - `Group8_Liquid`: world y = 121.21, x -392..-196, z 376..564.
  - `Group20_Liquid`: world y = 49.50, x -354..-258, z 526..622.
  Groups 0/2/4/12 are behind the camera (`is_position_behind` true) and contribute no pixels.
- **No row or UV mismatch.** All six nodes bind the same shader, `floats_0 = (0.3, -0.1, 0.03,
  0.28)`, `liquid_ints = (1, 1, 50, 25)`, `flow = (0, 0)`, and `uv = wow_position.xy * 0.06`.
  The coarse/bright vs fine/crimson difference is world position (each pool samples a different
  part of the same two layers) plus grazing-angle mip blur on the upper sheet, not a wrong row,
  object or UV origin.
- **Shore is not the cause.** Forcing `liquid_detail = 1` (shore off) leaves both sheets
  unchanged to within 3/255, and a debug dump of `shore_fade` gives mean D 0.977 for Group8
  (90% of pixels above 0.95) and 0.896 for Group20.

## What the seam actually is

The diagonal is the last drawn `Group8_Liquid` tile row. Beyond it the upper pool ends, the
`Group8`/`Group9` rock between the levels is above the camera's sight line through the culled
roof, and `Group20`'s lava 72 yd lower shows through the opening. Nothing overlaps incorrectly:
liquids are opaque in-shader and depth-tested, so G11 draw order and band-cull rules do not
apply to this image.

## Representative in-room pose

`forge-room-b.png`: camera `(-210, 132, 520)` looking at `(-360, 121.2, 470)`. This shows the
pool around the central platform, the three lavafalls and the grate walkway, with no seam, no
sky and no void. Use an in-room pose for future Forge comparisons; a pose above the ceiling is
not evidence about the pool.

## Sources

- Forge liquid groups and AABBs: runtime dump of `Wmo33788514` group meshes and liquid nodes.
- `R-3d-forge-shore-draw.md`, `R-3d-shore-depth-prepass-source.md` in the 70688 evidence set.

## See Also

- [[northshire-pale-water]] — the water family's depth and shore path.
- [liquid rendering spec](../../specs/liquid-rendering.md) — the ProcFel shore contract.
