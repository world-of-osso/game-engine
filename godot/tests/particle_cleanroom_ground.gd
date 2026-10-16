extends "res://tests/m2_loader_pixels.gd"

# P7 integration boundary: real Godot collision bodies, shared births and native pools.
var stage: Node3D
var probe: Object

func add_surface(x: float, height: float) -> void:
	var body := StaticBody3D.new()
	body.position = Vector3(x, height - 0.5, 0.0)
	body.collision_layer = 1 # existing TERRAIN_LAYER
	body.collision_mask = 0
	var shape := BoxShape3D.new()
	shape.size = Vector3(2.0, 1.0, 2.0)
	var collision := CollisionShape3D.new()
	collision.shape = shape
	body.add_child(collision)
	var mesh := MeshInstance3D.new()
	var box := BoxMesh.new()
	box.size = shape.size
	mesh.mesh = box
	var material := StandardMaterial3D.new()
	material.shading_mode = BaseMaterial3D.SHADING_MODE_UNSHADED
	material.albedo_color = Color(0.15, 0.2, 0.25)
	mesh.material_override = material
	body.add_child(mesh)
	stage.add_child(body)

func run_cases() -> void:
	root.size = Vector2i(1920, 1080)
	DirAccess.make_dir_recursive_absolute(ProjectSettings.globalize_path(FIXTURE + "/textures"))
	if not write_fixture(FIXTURE + "/textures/910001.blp", make_blp(Color.WHITE)):
		quit(1)
		return
	stage = Node3D.new()
	root.add_child(stage)
	probe = ClassDB.instantiate("WowParticleProbe")
	add_surface(-3.0, 0.0)
	add_surface(0.0, 3.0)
	# From birth height5, floor-16 lies beyond the authored20-yard segment.
	add_surface(6.0, -16.0)
	var camera := Camera3D.new()
	camera.projection = Camera3D.PROJECTION_ORTHOGONAL
	camera.size = 12.0
	stage.add_child(camera)
	camera.look_at_from_position(Vector3(1.5, 8.0, 13.0), Vector3(1.5, 2.0, 0.0))
	camera.current = true
	await physics_frame
	await physics_frame
	var passed := true
	var cases := [
		["flat", -3.0, 0x2000, 0.5],
		["raised", 0.0, 0x2000, 3.5],
		["opening", 3.0, 0x2000, 5.0],
		["beyond20", 6.0, 0x2000, 5.0],
		["unflagged", 0.0, 0, 5.0],
		["modelspace", 0.0, 0x2010, 5.0],
	]
	for sample in cases:
		var result = probe.projected_birth_node(stage, FIXTURE + "/textures", sample[2], Vector3(sample[1], 5.0, 0.0))
		if not result is MultiMeshInstance3D:
			push_error("birth node: " + str(result))
			passed = false
			continue
		var node: MultiMeshInstance3D = result
		stage.add_child(node)
		if node.multimesh.visible_instance_count != 1:
			push_error("Expected one birth: " + str(sample))
			passed = false
			continue
		var position := node.multimesh.get_instance_transform(0).origin
		var correct := absf(position.y - float(sample[3])) < 0.0001
		print("P7 ", sample[0], " position=", position, " expectedY=", sample[3], " passed=", correct)
		passed = correct and passed
	await process_frame
	await RenderingServer.frame_post_draw
	var output := OS.get_environment("GODOT_PARTICLE_SCREENSHOTS")
	if not output.is_empty():
		var image := root.get_texture().get_image()
		if image.get_size() != Vector2i(1920, 1080) or image.save_png(output + "/p7-ground-births.png") != OK:
			push_error("P7 screenshot failed")
			passed = false
	stage.free()
	probe = null
	cleanup()
	print("PASS: P7 native collision births" if passed else "FAIL: P7 native collision births")
	quit(0 if passed else 1)
