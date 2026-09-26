# Stockade Floor Fall

`game-server-admin teleport <char> 34 x y z` to (97-101, 76-78) or (74, -16) in The Stockade dropped the player through the floor; after the fall the client drew sky instead of the dungeon. Teleports to x 103 and to creature spawn points held. Evidence: `../../../data/diagnostics/stockadefixes-20260926/`.

## Root cause: the points are outside the WMO

- The Stockade is one global WMO (108631, WDT 791060, placed at the origin; WoW = (-x, -y, z) of the file coordinates). A vertical ray through every triangle of its 27 groups, with no MOPY filter, crosses **no face at all** in the columns (97, 76), (99, 77), (101, 78) and (74, -16). They are inside wall and rock between the cells. The Jail18 cell floor begins at x ≈ 102 for y 76-78. At y -16 the Jail02 cell floor spans x 77-92.
- Floor collision itself holds: over a 2 yd grid of the whole dungeon, the server `GroundMap` finds every walkable collidable face that a brute-force scan finds (7,650 of 7,650). The client uses the same `shared::ground` rule. All 93 TDB1210 Stockade spawns stand on a floor.
- Live (`03`, `04`): teleported to (103, 76, -34.5) the player stays at -34.9. Teleported to (99, 77, -34.5) the player falls to -1255 within 12 s.

## Sky instead of the dungeon

Below the dungeon the camera is in no interior group, so portal culling (WebWowViewerCpp and Retail) draws only the exterior group `ext` and whatever its portals reach. Every Stockade group except `ext` is interior (MOGP 0x2000), so the view is sky. This is correct for a camera outside the WMO.

## Open

- The server lets an unsupported player fall forever. TrinityCore kills a player below `Map::GetMinHeight` with `DAMAGE_FALL_TO_VOID` (MovementHandler.cpp:594-612). That height is -500 without grid data (TerrainMgr.cpp:671, GridMap.cpp:549); on ADT tiles it comes from the MFBO planes. Not implemented.

## See Also

- [[stockade-entrance]]
- [[player-ground]]
