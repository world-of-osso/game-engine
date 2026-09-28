extends "res://tests/world_objects_flow.gd"

# Run with GODOT_TEST_SERVER=127.0.0.1:5000 and a roster character whose
# loaded tiles contain WMO 108238. Uses the real ADT object → WMO loader.
func capture() -> void:
	var objects := root.find_child("WorldObjects", true, false)
	if objects == null:
		fail("WorldObjects root missing")
		return
	var wmo := objects.get_node_or_null("Wmo108238")
	if wmo == null:
		fail("Fixture requires authored WMO 108238 in loaded tiles")
		return
	var matched := false
	for child in wmo.get_children():
		if not child.name.begins_with("Group38_Batch") or not child is MeshInstance3D:
			continue
		var material := child.get_surface_override_material(0) as ShaderMaterial
		if material == null:
			continue
		var texture := material.get_shader_parameter("base_texture") as ImageTexture
		if texture == null or texture.get_width() != 512 or texture.get_height() != 512:
			continue
		var pixels := texture.get_image()
		if pixels == null or pixels.is_empty():
			fail("WMO 108238 group 38 has no bound composite pixels")
			return
		if pixels.get_pixel(0, 0) != Color8(27, 32, 38, 255):
			continue
		matched = true
		break
	if not matched:
		fail("WMO 108238 group 38 lacks authored 512x512 shader 7 composite pixel")
		return
	print("PASS: authored WMO 108238 group 38 binds a 512x512 composite texture")
