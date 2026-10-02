extends SceneTree

# Unit picking is occluded by visible world surfaces only, never by an M2 doodad's
# collision hull (`terrain/doodad_collision.rs`, camera-ray layer 1<<3). Real Northshire
# data: Milly Osworth (content_creature 20279975, HD human female) stands 3.9 yd from
# Stonepyre01 (FDID 198581, MDDF uniqueId 10325, azeroth_32_48). The player stands at the
# quest flow's teleport point facing her with the default follow camera. The pyre's
# collision frustum (9 vertices, base 4.2 yd square) is wider than its drawn column and
# lies across the click ray to every pixel of her body, though she is in plain view.

const DATA := "res://../data/"
const MILLY := 20279975
const MILLY_AT := Vector3(-8923.88, 81.1119, 135.889)
const MILLY_YAW := 1.74533
# Engine axes of the MDDF record (17206.2324, 80.8045, 25989.3477), heading 21 degrees.
const PYRE_AT := Vector3(-8922.681, 80.8045, 139.5658)
const PYRE_HEADING := 21.0
# `world_quest_flow.gd` MILLY_AT teleport (-8921.0, -138.5, 81.1).
const PLAYER_AT := Vector3(-8921.0, 81.1, 138.5)
# CameraState default distance and pitch, EYE_HEIGHT (src/camera_*_data.rs).
const CAMERA_DISTANCE := 15.0
const CAMERA_PITCH := -0.3
const EYE_HEIGHT := 1.8
const DOODAD_LAYER := 1 << 3

func _initialize() -> void:
	call_deferred("run_test")

func run_test() -> void:
	root.size = Vector2i(1280, 720)
	var world := Node3D.new()
	root.add_child(world)
	var hull := pyre_collision()
	if hull == null:
		return
	world.add_child(hull)
	var loader = ClassDB.instantiate("WowAssetLoader")
	var loaded: Dictionary = loader.load_player(1, 1, 1, [])
	if loaded.has("error"):
		fail("Load human female: " + str(loaded.error))
		return
	var unit := Node3D.new()
	unit.position = MILLY_AT
	unit.rotation.y = MILLY_YAW
	var visual := Node3D.new()
	visual.rotation.y = -PI / 2
	visual.add_child(loaded.node)
	unit.add_child(visual)
	world.add_child(unit)
	var error: String = UnitPicker.attach(visual, MILLY)
	if error != "":
		fail("attach: " + error)
		return
	var camera := follow_camera(world)
	await physics_frames(2)

	var body := MILLY_AT + Vector3(0, 1.0, 0)
	var centre := camera.unproject_position(body)
	var hull_hit := camera.get_world_3d().direct_space_state.intersect_ray(hull_ray(camera, centre))
	if hull_hit.is_empty() or hull_hit.collider != hull:
		fail("The pyre hull must lie across the ray to Milly's body: " + str(hull_hit))
		return
	var hits := 0
	for dy in range(-72, 25, 8):
		for dx in range(-32, 33, 8):
			if UnitPicker.pick(camera, centre + Vector2(dx, dy)) == MILLY:
				hits += 1
	if hits == 0:
		fail("No pixel around Milly's body (%s) picks her behind the pyre hull" % centre)
		return
	print("FIXTURE TARGET_PICK_DOODAD_HULL_DONE hits=%d" % hits)
	quit(0)

## Stonepyre01's MD20 collision triangles (0xD8 indices, 0xE0 vertices) in engine axes
## (x, z, -y), placed by its MDDF record, on the doodad layer like `attach_collision`.
func pyre_collision() -> StaticBody3D:
	var bytes := FileAccess.get_file_as_bytes(DATA + "models/198581.m2")
	if bytes.size() < 8 or bytes.slice(0, 4).get_string_from_ascii() != "MD21":
		fail("Stonepyre01 model missing or not MD21")
		return null
	var md20 := bytes.slice(8, 8 + bytes.decode_u32(4))
	var index_count := md20.decode_u32(0xD8)
	var index_offset := md20.decode_u32(0xDC)
	var vertex_offset := md20.decode_u32(0xE4)
	var faces := PackedVector3Array()
	for i in range(index_count):
		var at := vertex_offset + 12 * md20.decode_u16(index_offset + 2 * i)
		faces.append(Vector3(md20.decode_float(at), md20.decode_float(at + 8), -md20.decode_float(at + 4)))
	var shape := ConcavePolygonShape3D.new()
	shape.backface_collision = true
	shape.set_faces(faces)
	var node := CollisionShape3D.new()
	node.shape = shape
	var body := StaticBody3D.new()
	body.collision_layer = DOODAD_LAYER
	body.collision_mask = 0
	body.position = PYRE_AT
	body.rotation.y = deg_to_rad(PYRE_HEADING - 180.0)
	body.add_child(node)
	return body

## The follow camera behind the player, who faces Milly.
func follow_camera(world: Node3D) -> Camera3D:
	var camera := Camera3D.new()
	world.add_child(camera)
	camera.make_current()
	var eye := PLAYER_AT + Vector3(0, EYE_HEIGHT, 0)
	var to := MILLY_AT - PLAYER_AT
	var forward := Vector3(to.x, 0, to.z).normalized()
	var back := forward * CAMERA_DISTANCE * cos(CAMERA_PITCH) + Vector3.UP * CAMERA_DISTANCE * sin(CAMERA_PITCH)
	camera.look_at_from_position(eye - back, eye)
	return camera

func hull_ray(camera: Camera3D, point: Vector2) -> PhysicsRayQueryParameters3D:
	var origin := camera.project_ray_origin(point)
	var ray := PhysicsRayQueryParameters3D.create(origin, origin + camera.project_ray_normal(point) * 100.0)
	ray.collision_mask = DOODAD_LAYER
	return ray

func physics_frames(count: int) -> void:
	for frame in range(count):
		await physics_frame

func fail(message: String) -> void:
	push_error(message)
	quit(1)
