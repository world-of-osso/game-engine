extends SceneTree

const DIAGNOSTIC := preload("res://tests/anomalus_framing_diagnostic.gd")

func _initialize() -> void:
	call_deferred("run_tests")

func run_tests() -> void:
	var world := Node3D.new()
	root.add_child(world)
	var camera := Camera3D.new()
	world.add_child(camera)
	camera.set_orthogonal(4.0, 0.05, 100.0)
	camera.position = Vector3(0, 0, 10)
	root.size = Vector2i(800, 600)
	var arrays := []
	arrays.resize(Mesh.ARRAY_MAX)
	arrays[Mesh.ARRAY_VERTEX] = PackedVector3Array([
		Vector3(-1, -1, 0), Vector3(1, -1, 0), Vector3(0, 1, 0),
		Vector3(1000, 1000, 0), # Unindexed outlier must not affect triangles.
	])
	arrays[Mesh.ARRAY_INDEX] = PackedInt32Array([0, 1, 2])
	var mesh := MeshInstance3D.new()
	var geometry := ArrayMesh.new()
	geometry.add_surface_from_arrays(Mesh.PRIMITIVE_TRIANGLES, arrays)
	mesh.mesh = geometry
	var material := StandardMaterial3D.new()
	material.transparency = BaseMaterial3D.TRANSPARENCY_ALPHA
	material.cull_mode = BaseMaterial3D.CULL_DISABLED
	material.albedo_color = Color(1, 1, 1, 0.25)
	mesh.material_override = material
	world.add_child(mesh)
	await process_frame
	var view := Rect2(Vector2.ZERO, Vector2(800, 600))
	var sample: Dictionary = DIAGNOSTIC.sample_surface(mesh, 0, camera, view)
	if not require(sample.get("triangle_count", -1) == 1, "one indexed triangle"):
		return
	var triangle: Dictionary = sample.triangles[0]
	if not require(triangle.indices == [0, 1, 2], "actual triangle indices"):
		return
	if not require(triangle.outside_vertices == 0, "unindexed outlier excluded"):
		return
	if not require(triangle.projected_rect[2] < 800, "triangle extrema exclude outlier"):
		return
	if not require(sample.material.albedo_color[3] == 0.25, "material alpha recorded"):
		return
	if not require(sample.material.cull_mode == BaseMaterial3D.CULL_DISABLED, "material culling recorded"):
		return
	mesh.position.x = 10
	var moved: Dictionary = DIAGNOSTIC.sample_surface(mesh, 0, camera, view)
	if not require(moved.triangles[0].outside_vertices == 3, "actual translated geometry exceeds viewport"):
		return
	if not require(moved.triangles[0].projected_rect[0] > 800, "projected extrema follow global transform"):
		return
	mesh.position.x = 0
	var shader := Shader.new()
	shader.code = "shader_type spatial; render_mode cull_disabled, blend_add; uniform float transparency = 1.0;"
	var shader_material := ShaderMaterial.new()
	shader_material.shader = shader
	shader_material.set_shader_parameter("transparency", 0.375)
	mesh.material_override = shader_material
	var shaded: Dictionary = DIAGNOSTIC.sample_surface(mesh, 0, camera, view)
	if not require(shaded.material.shader_parameters.transparency == 0.375, "active shader alpha recorded"):
		return
	if not require(shaded.material.shader_code.contains("cull_disabled"), "active shader cull source recorded"):
		return
	print("PASS anomalus framing diagnostic: indexed extrema, unindexed exclusion, global projection, material alpha/cull, shader state")
	world.queue_free()
	quit(0)

func require(condition: bool, message: String) -> bool:
	if not condition:
		push_error("anomalus diagnostic fixture: " + message)
		quit(1)
	return condition
