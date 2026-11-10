extends SceneTree
# Bounded game-object floor rays (spell ground draping) answer exactly as an unbounded
# ray: game-object-layer bodies (flat, rotated ramp, scaled, steep, moved this frame) and
# one body on another layer, compared on a 0.25 yd grid in and around their bounds.
const GAME_OBJECT_LAYER := 1 << 5
const DOODAD_LAYER := 1 << 3

func _initialize() -> void:
	call_deferred("run")

func fail(reason: String) -> void:
	push_error("FLOOR_RAY_PARITY_FAIL " + reason)
	quit(1)

func quad(size: Vector2, height: float) -> PackedVector3Array:
	var a := Vector3(-size.x, height, -size.y)
	var b := Vector3(size.x, height, -size.y)
	var c := Vector3(size.x, height, size.y)
	var d := Vector3(-size.x, height, size.y)
	return PackedVector3Array([a, b, c, a, c, d])

func body(parent: Node3D, faces: PackedVector3Array, transform: Transform3D, layer: int) -> StaticBody3D:
	var shape := ConcavePolygonShape3D.new()
	shape.backface_collision = true
	shape.set_faces(faces)
	var node := CollisionShape3D.new()
	node.shape = shape
	var result := StaticBody3D.new()
	result.collision_layer = layer
	result.collision_mask = 0
	result.add_child(node)
	result.transform = transform
	parent.add_child(result)
	return result

func run() -> void:
	var fixture: Node3D = ClassDB.instantiate("FloorRayParity")
	root.add_child(fixture)
	var both := GAME_OBJECT_LAYER | DOODAD_LAYER
	body(fixture, quad(Vector2(2, 2), 0.0), Transform3D(Basis(), Vector3(0, 1, 0)), both)
	body(fixture, quad(Vector2(3, 1.5), 0.0), Transform3D(Basis(Vector3.UP, 0.6) * Basis(Vector3.RIGHT, 0.3), Vector3(7, 0.5, 2)), both)
	body(fixture, quad(Vector2(1, 1), 0.0), Transform3D(Basis().scaled(Vector3(2.5, 1, 0.5)), Vector3(-6, 2, -4)), both)
	body(fixture, quad(Vector2(1.5, 1.5), 0.0), Transform3D(Basis(Vector3.RIGHT, 1.2), Vector3(0, 1, 8)), both)
	body(fixture, quad(Vector2(2, 2), 0.0), Transform3D(Basis(), Vector3(-5, 1, 5)), DOODAD_LAYER)
	var moved := body(fixture, quad(Vector2(1, 1), 0.0), Transform3D(Basis(), Vector3(4, 3, -6)), GAME_OBJECT_LAYER)
	for frame in 3:
		await physics_frame
	moved.position = Vector3(4.5, 2.5, -5.5)
	var report: String = fixture.compare_floor_rays(Vector3(-10, -1, -9), Vector3(11, 5, 11), 0.25)
	print("FLOOR_RAY_PARITY ", report)
	if not report.begins_with("points="):
		fail(report)
		return
	var fields := {}
	for part in report.split(" "):
		var pair := part.split("=")
		if pair.size() == 2:
			fields[pair[0]] = int(pair[1])
	if fields.get("mismatches", -1) != 0:
		fail(report)
		return
	if fields.hits < 100 or fields.casts >= fields.points:
		fail("the grid must both hit floors and skip rays: " + report)
		return
	print("PASS: floor ray parity")
	quit(0)
