# Godot `Parameter "material" is null` on M2 free

Freeing an M2 model logged four Godot errors per batch, which native fixtures treat as failures (`native_input_fixture footsteps` failed every run, `keybinds` intermittently):

```
ERROR: Parameter "material" is null.  at: material_casts_shadows
ERROR: Parameter "material" is null.  at: material_is_animated
ERROR: Parameter "material" is null.  at: material_get_instance_shader_parameters
ERROR: Parameter "material" is null.  at: material_update_dependency
```

Under `--headless` (dummy renderer) only `material_get_instance_shader_parameters` reports.

## Root cause

Godot 4.7.2 engine ordering, upstream [godotengine/godot#85817](https://github.com/godotengine/godot/issues/85817) (open):

- `MeshInstance3D` has no destructor; its `surface_override_materials` member is destroyed before `~VisualInstance3D` frees the RenderingServer instance. When the batch held the material's last reference, the material RID is freed first.
- `RendererSceneCull::free(instance)` calls `update_dirty_instances()` before deleting. A dirty instance whose surface material was never dependency-tracked still holds the freed RID, and `_update_dirty_instance` reads it in the order of the four errors above.
- A material is tracked (and cleared by `dependency_deleted` when freed) only after the instance's first dirty update. Untracked cases: a model freed in the frame it was built, or never drawn (headless never runs `RenderingServer::draw`, so the footsteps player's batches stayed dirty until `FOOTSTEPS_REMOVE` freed them).
- M2 batches share one `ArrayMesh` per submesh, so the mesh outlives the instance and the instance keeps its mesh base; a per-instance mesh freed with the instance clears the base and hides the bug, which is why a minimal BoxMesh reproduction stays quiet.
- `GeometryInstance3D::~GeometryInstance3D` clears `material_override` from the RenderingServer instance before releasing it, so materials bound as the override are safe.

The m2render rewrite (`8dea62e4`) did not introduce the mechanism: a build of its parent logs the same four errors when a freshly loaded model is freed. `M2MaterialAnimation` holds a second reference but is freed first (Godot deletes children last-to-first), so it never protected the batch. Which GPU-path free made `keybinds` fail after the merge was not captured: twelve traced master runs (2026-10-01) logged no material error.

## Fix (`eb619da4`, `c7ed0120`, `bfb73227`)

`assets::build_model_filtered` binds each batch material with `set_material_override`; batch meshes have one surface, so it is the same binding. Readers (`bind_visual_light`, WMO `doodad_light`, scenery fade, unit pick, character preview/create, tests) use `get_active_material(0)`: `bind_visual_light` also walks WMO groups, which keep their surface override, and panicked at `world_models.rs:50` while it read the material override only. Retirement condition: upstream fixes `MeshInstance3D` destruction order; the material override stays correct either way.

WMO groups (`build_batch_mesh` per instance) free their mesh with the instance; terrain, water and character-select sky still use surface overrides and were not observed failing.

## Proof

Evidence under `data/diagnostics/matnull-20261001/`; fixed build is `bfb73227`.

- `godot/tests/m2_free_material.gd`: frees model 1016191 never added, and added then freed in one frame; asserts its batch materials are released by the free and no error is logged. Master build: exit 1 with the four errors (`m2-free-material-red-2.log`); fixed build: PASS (`suite-fix2/m2_free_material.log`).
- `m2_texture_sharing`, `m2_portal_pixels` (both logged the errors on master), `m2_assets`, `m2_skin_pixels`, `m2_scenery_fade_pixels` pass without errors on the fixed build (`suite-fix2/`). `wmo_doodads_flow` Stockade interior doodad light passes (`wmo-global-fix.log`); its Magic District part fails "Lamp 199823 has constant bone tracks but its pose changed" on master and fixed builds alike (`wmo_doodads_flow-master.log`, `suite-fix2/wmo_doodads_flow.log`).
- `native_input_fixture footsteps`: master fails at `FOOTSTEPS_REMOVE` (`footsteps-base-1.log`); fixed build 3/3 PASS (`fix2-footsteps-{1,2,3}.log`). `keybinds`: fixed build 3/3 PASS, no material error (`fix2-keybinds-{1,2,3}.log`).
