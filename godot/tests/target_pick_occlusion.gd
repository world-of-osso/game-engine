extends SceneTree

# Native unit picking through real Godot physics: the M2-bounds pick shape is only the
# broad phase; a click selects the unit whose visible triangle the ray reaches first.
# A unit inside another unit's oversized box still wins at its own screen point, empty
# space inside a box selects nothing, hidden batches do not count, visible world
# geometry in front occludes, hidden geometry and hidden units let the ray through, and
# the default body-only ray used by the camera and ground never hits a pick shape.

const UNIT := 4242
const OTHER := 7
const PLAYER := 99

var failed := false

func _initialize() -> void:
	call_deferred("run_test")

func run_test() -> void:
	root.size = Vector2i(640, 360)
	var world := Node3D.new()
	root.add_child(world)
	var camera := Camera3D.new()
	world.add_child(camera)
	camera.position = Vector3(0, 1, 10)
	camera.look_at(Vector3(0, 1, 0))
	camera.make_current()
	var unit := model_with_bounds(Vector3(0, 0, 0))
	world.add_child(unit)
	var error: String = UnitPicker.attach(unit, UNIT)
	if error != "":
		fail("attach: " + error)
		return
	var bare := Node3D.new()
	world.add_child(bare)
	if UnitPicker.attach(bare, OTHER) == "":
		fail("A visual without M2 bounds got a pick shape")
		return
	await physics_frames(2)
	var center := camera.unproject_position(Vector3(0, 1, 0))
	expect_pick(camera, center, UNIT, "unit in the open")
	expect_pick(camera, Vector2(5, 5), null, "empty sky")

	var wall := wall_at(Vector3(0, 1, 5))
	world.add_child(wall)
	await physics_frames(2)
	expect_pick(camera, center, null, "unit behind a visible wall")

	wall.visible = false
	await physics_frames(2)
	expect_pick(camera, center, UNIT, "unit behind a hidden wall")

	wall.visible = true
	wall.position = Vector3(0, 1, -5)
	await physics_frames(2)
	expect_pick(camera, center, UNIT, "unit in front of a wall")

	# A player beside the ray whose header box (like HD human male's) encloses the
	# unit behind it and the camera: only its drawn body picks it.
	var player := model_with_bounds(Vector3(1.2, 0, 4), Vector3(0.6, 2, 0.6), AABB(Vector3(-3, 0, -6), Vector3(6, 3, 12)))
	world.add_child(player)
	UnitPicker.attach(player, PLAYER)
	await physics_frames(2)
	expect_pick(camera, center, UNIT, "unit inside the player's box")
	expect_pick(camera, camera.unproject_position(Vector3(0.6, 1, 4)), null, "empty space inside the player's box")
	var body := camera.unproject_position(Vector3(1.2, 1, 4))
	expect_pick(camera, body, PLAYER, "the player's own body")
	player.get_node("Body").visible = false
	await physics_frames(2)
	expect_pick(camera, body, null, "the player's hidden batch")
	player.queue_free()
	await physics_frames(2)

	var front := model_with_bounds(Vector3(0, 0, 3))
	world.add_child(front)
	UnitPicker.attach(front, OTHER)
	await physics_frames(2)
	expect_pick(camera, center, OTHER, "nearer unit in front")
	front.visible = false
	await physics_frames(2)
	expect_pick(camera, center, UNIT, "hidden unit in front")

	var body_ray := PhysicsRayQueryParameters3D.create(Vector3(0, 1, 10), Vector3(0, 1, -10))
	var hit := camera.get_world_3d().direct_space_state.intersect_ray(body_ray)
	if hit.is_empty() or hit.collider != wall:
		fail("Default body ray should pass the pick shape and hit the wall: " + str(hit))
		return
	if failed:
		quit(1)
		return
	print("FIXTURE TARGET_PICK_OCCLUSION_DONE")
	quit(0)

# Stand-in for an M2 model root: the loader stores its header box in model axes; a
# box mesh of `size` standing on the origin is its drawn geometry.
func model_with_bounds(position: Vector3, size := Vector3(1, 2, 1), bounds := AABB(Vector3(-0.5, 0, -0.5), Vector3(1, 2, 1))) -> Node3D:
	var model := Node3D.new()
	model.name = "NpcModel"
	model.position = position
	model.set_meta("m2_bounds", bounds)
	var body := MeshInstance3D.new()
	body.name = "Body"
	var mesh := BoxMesh.new()
	mesh.size = size
	body.mesh = mesh
	body.position = Vector3(0, size.y / 2, 0)
	model.add_child(body)
	return model

func wall_at(position: Vector3) -> StaticBody3D:
	var wall := StaticBody3D.new()
	wall.position = position
	var shape := CollisionShape3D.new()
	var box := BoxShape3D.new()
	box.size = Vector3(4, 4, 0.2)
	shape.shape = box
	wall.add_child(shape)
	return wall

func expect_pick(camera: Camera3D, point: Vector2, expected, label: String) -> void:
	var picked = UnitPicker.pick(camera, point)
	if picked != expected:
		fail("%s: picked %s, expected %s" % [label, picked, expected])

func physics_frames(count: int) -> void:
	for frame in range(count):
		await physics_frame

func fail(message: String) -> void:
	push_error(message)
	failed = true
	quit(1)
