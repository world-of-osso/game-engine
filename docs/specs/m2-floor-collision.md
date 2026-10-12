# M2 doodad collision floors

Players stand and walk on authored M2 collision geometry on both authoritative server and native client. This includes ADT MDDF placements and active WMO doodad sets, with position, rotation and scale composed exactly as their models are placed.

- Use MD20 bounding triangles/vertices/normals (0xD8/0xE0/0xE8), never the render mesh. Empty collision arrays mean no collision. Bake floor admission from the authored face normal rotated into world space: upward and within the shared slope limit; a downward-facing underside is not a floor.
- The highest walkable face within the shared step-up/search interval competes with terrain and WMO floors. Roofs beyond step reach and steep faces do not support feet. Existing terrain/WMO rules remain.
- Dedicated collision-hull tiles live beside visible LOS tiles under `data/los/<map>/m2/<row>_<col>.los`. Both clients and server consume these same world-space hulls; native ground is available before render nodes spawn and remains independent of visual culling.
- The M2 companion contains only floor-admitted hull faces; camera collision retains the complete hull. `los-bake` writes both visible LOS and M2 ground. `--m2-only` updates only the collision companion, including global-WMO doodad sets. Collision floors have no material or minimum-thickness exclusion.
- Existing M2 camera bodies/rays remain unchanged. Existing LOS render-geometry policy and navigation obstacle classification remain unchanged.

Acceptance: real EotS model193193 at placement995848 supports Alliance start2523.76,1596.13,1247.7; Horde start is also supported. A second real continent doodad floor supports walking. Targeted behavioral tests, terrain/WMO regressions, private Modern/Forever live proof, and before/after server tick measurements are required. Proof bakes must not write canonical data or world.db.
