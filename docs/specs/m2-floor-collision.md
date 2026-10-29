# M2 doodad collision floors

Players stand and walk on authored M2 collision geometry on both authoritative server and native client. This includes ADT MDDF placements and active WMO doodad sets, with position, rotation and scale composed exactly as their models are placed.

- Use MD20 bounding triangles/vertices/normals (0xD8/0xE0/0xE8), never the render mesh. Empty collision arrays mean no collision. Bake floor admission from the authored face normal rotated into world space: upward and within the shared slope limit; a downward-facing underside is not a floor.
- The first solid face within the shared step-up/search interval supports the player, including steep faces. The directional 50° ascent limit remains enforced; roofs beyond step reach are not support.
- `data/los/<map>/m2/<row>_<col>.los` contains authored-normal walkable faces; `data/los/<map>/m2-solid/<row>_<col>.los` contains the remaining nonwalkable hull faces. Client and server query their union. Missing solid companions for an existing walkable tile fail explicitly; no render geometry substitutes them.
- `los-bake` writes both M2 companions beside visible LOS. `--m2-only` updates both companions, including global-WMO doodad sets. Collision faces have no material or minimum-thickness exclusion. Existing maps require both companions to be baked before these consumers are deployed.
- Legacy global-WMO WDTs retain their authored MWMO paths and MODF name indices. The native loader and baker resolve those paths through the local community listfile; direct FileDataID MODF entries stay direct. Assets are never rewritten.
- Existing M2 camera bodies/rays remain unchanged. Existing LOS render-geometry policy and navigation obstacle classification remain unchanged.

Acceptance: real EotS model193193 at placement995848 supports Alliance start2523.76,1596.13,1247.7; Horde start is also supported. A second real continent doodad floor supports walking. Targeted behavioral tests, terrain/WMO regressions, private Modern/Forever live proof, and before/after server tick measurements are required. Proof bakes must not write canonical data or world.db.
