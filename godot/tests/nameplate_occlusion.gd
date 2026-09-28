extends SceneTree

# Native nameplate occlusion through real Godot physics (docs/specs/nameplate-style.md):
# terrain (layer 1) and WMO collision (layer 2) between the camera and a unit dim its
# plate to nameplateOccludedAlphaMult 0.4; a wall behind the unit, a body on another
# layer and a unit pick shape (layer 3) do not.

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
	var unit := Vector3(0, 1, 0)
	await physics_frames(2)
	expect(camera, unit, false, "unit in the open")

	var terrain := wall_at(Vector3(0, 1, 5), 1)
	world.add_child(terrain)
	await physics_frames(2)
	expect(camera, unit, true, "unit behind terrain")

	terrain.collision_layer = 2
	await physics_frames(2)
	expect(camera, unit, true, "unit behind a WMO wall")

	terrain.position = Vector3(0, 1, -5)
	await physics_frames(2)
	expect(camera, unit, false, "wall behind the unit")

	terrain.position = Vector3(0, 1, 5)
	terrain.collision_layer = 1 << 3
	await physics_frames(2)
	expect(camera, unit, false, "body on a non-occluding layer")

	var pick := Area3D.new()
	pick.collision_layer = 1 << 2
	pick.position = Vector3(0, 1, 3)
	var pick_shape := CollisionShape3D.new()
	var pick_box := BoxShape3D.new()
	pick_box.size = Vector3(1, 2, 1)
	pick_shape.shape = pick_box
	pick.add_child(pick_shape)
	world.add_child(pick)
	await physics_frames(2)
	expect(camera, unit, false, "another unit's pick shape in front")

	if not is_equal_approx(NameplateProbe.alpha(true), 0.4) or NameplateProbe.alpha(false) != 1.0:
		fail("alpha occluded %s clear %s" % [NameplateProbe.alpha(true), NameplateProbe.alpha(false)])
	if failed:
		quit(1)
		return
	print("FIXTURE NAMEPLATE_OCCLUSION_DONE")
	quit(0)

func wall_at(position: Vector3, layer: int) -> StaticBody3D:
	var wall := StaticBody3D.new()
	wall.position = position
	wall.collision_layer = layer
	var shape := CollisionShape3D.new()
	var box := BoxShape3D.new()
	box.size = Vector3(4, 4, 0.2)
	shape.shape = box
	wall.add_child(shape)
	return wall

func expect(camera: Camera3D, point: Vector3, expected: bool, label: String) -> void:
	var actual: bool = NameplateProbe.occluded(camera, point)
	if actual != expected:
		fail("%s: occluded %s, expected %s" % [label, actual, expected])

func physics_frames(count: int) -> void:
	for frame in range(count):
		await physics_frame

func fail(message: String) -> void:
	push_error(message)
	failed = true
	quit(1)
