extends SceneTree

# Uses the production portrait owner and real HD model loader, offline.
var fixture: Node
var failed := false

func _initialize() -> void:
	call_deferred("run_test")

func run_test() -> void:
	root.size = Vector2i(1280, 720)
	fixture = ClassDB.instantiate("PartyPortraitFixture")
	root.add_child(fixture)
	check(fixture.initialize(false).is_empty(), "initialize fixture")
	var names: Array[String] = ["Theron", "Jaina", "Valeera", "Uther"]
	check(fixture.set_members(names, false, true).is_empty(), "request heads")
	for attempt in range(1200):
		check(fixture.tick().is_empty(), "load heads")
		await process_frame
		if fixture.ready_heads() == 4:
			break
	check(fixture.ready_heads() == 4, "all heads loaded")
	for name in names:
		var state: Dictionary = fixture.portrait_state(name)
		var host := fixture.find_child(state.frame, true, false)
		var camera := host.find_child("PortraitCamera", true, false) as Camera3D
		var meshes := host.find_children("*", "MeshInstance3D", true, false)
		check(not meshes.is_empty(), "head has drawn meshes")
		for mesh in meshes:
			var material := mesh.get_active_material(0) as ShaderMaterial
			var rays: Vector3 = material.get_shader_parameter("sun_direction")
			# The shader lights with -sun_direction; rays must travel into the face.
			check(rays.normalized().dot(camera.global_basis.z) < -0.95,
				"camera-facing portrait key light: " + name)
	if not failed:
		print("PASS portrait_camera_frontal_light_four_heads")
	fixture.free()
	await process_frame
	quit(1 if failed else 0)

func check(condition: bool, message: String) -> void:
	if not condition:
		push_error(message)
		failed = true
