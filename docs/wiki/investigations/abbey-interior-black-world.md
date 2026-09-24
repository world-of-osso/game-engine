# Abbey Interior Black World

**Symptom**: inside Northshire Abbey (Bevy `(-8883, 80.3, 167)`, in WMO 107074 `nsabbey.wmo`) the in-world frame was almost entirely the dark-navy clear color `srgb(0.05, 0.05, 0.12)`: no walls, terrain or sky dome, only a few near models and nameplates. This happened headless and windowed. Outside the abbey, at the spawn point `(-8949, 82.6, 132)` or at `(-9005, 309)`, the world rendered normally.

**Root cause**: the WMO loader applied WoW's MOCV fixup, which forces vertex alpha to `0` for interior groups (other batches keep their raw MOCV alpha), and wrote that alpha into Bevy `ATTRIBUTE_COLOR`. In WoW this alpha blends indoor and outdoor lighting; it is not opacity. `StandardMaterial` multiplies vertex alpha into base alpha, so textured WMO batches with `AlphaMode::Mask(0.5)` threw away their color. The world camera's depth/normal prepass does not use vertex color, so it still wrote their depth. Everything behind the walls (terrain, the sky dome at 900 units, far models) then failed the depth test.

**Isolation**: `--inworld-stage npcs` rendered the scene, with the near walls missing. `--inworld-stage lighting` went dark. That stage adds the WoW camera render bundle (`DepthPrepass`/`NormalPrepass`). Removing only the prepasses brought the scene back. The sky dome ECS state was visible, and its bounds and near/far `0.1/1000` were normal. There was no fog.

**Fix** (`94c0baa3`): `make_vertex_colors_opaque` sets vertex alpha to 1 after the RGB fixup. The MOCV RGB fixup is unchanged.

**Proof**: two GPU tests in `src/rendering/terrain/terrain_objects_wmo_tests/interior_gpu.rs` render the real abbey group 0 batch through an MSAA4 camera, with a green plane behind it. `abbey_interior_wall_renders_through_world_camera_prepass` uses a depth/normal prepass; RED: the center pixel stayed the clear color `[255,0,255,255]`. `abbey_interior_wall_renders_without_prepass` has no prepass; RED: the green background showed through, `[0,254,0,255]`. Both are GREEN with the fix: `[12,3,5,255]` (the wall). The fix does not depend on whether the camera has a prepass. A live headless capture shows the interior walls, Marshal McBride and the stained glass. Evidence: `data/diagnostics/dark-world-20260924/`.

**Still open**: exterior groups 3 and 5 have no portal refs, and portal culling still hides them from inside, so outdoor terrain shows through where their geometry belongs. The interior is also dim. Textured blend-mode-0 (opaque) WMO materials still alpha-test their texture alpha.
