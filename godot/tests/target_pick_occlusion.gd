extends SceneTree

# Native unit picking through real Godot physics: a click ray selects the unit whose
# M2-bounds pick shape it crosses, visible world geometry in front occludes it, hidden
# geometry and hidden units let the ray through, and the default body-only ray used by
# the camera and ground never hits a pick shape.

const UNIT := 4242
const OTHER := 7

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

# Stand-in for an M2 model root: the loader stores its header box in model axes.
func model_with_bounds(position: Vector3) -> Node3D:
	var model := Node3D.new()
	model.name = "NpcModel"
	model.position = position
	model.set_meta("m2_bounds", AABB(Vector3(-0.5, 0, -0.5), Vector3(1, 2, 1)))
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
