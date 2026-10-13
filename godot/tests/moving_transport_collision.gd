extends SceneTree

# Actual Alliance airship deck group 318244, same moving parent/model/body hierarchy.
# Queries run immediately after a parent transform changes, as native carry/input does.
func _initialize() -> void:
	call_deferred("run")

func chunks(raw: PackedByteArray) -> Dictionary:
	var result := {}
	var offset := 0
	while offset + 8 <= raw.size():
		var tag := raw.slice(offset, offset + 4).get_string_from_ascii().reverse()
		var length := raw.decode_u32(offset + 4)
		result[tag] = raw.slice(offset + 8, offset + 8 + length)
		offset += 8 + length
	return result

func deck_faces() -> PackedVector3Array:
	var outer := chunks(FileAccess.get_file_as_bytes("res://../data/models/318244.wmo"))
	var group: PackedByteArray = outer.MOGP
	var geometry := chunks(group.slice(0x44))
	var faces := PackedVector3Array()
	var flags: PackedByteArray = geometry.MOPY
	var indices: PackedByteArray = geometry.MOVI
	var vertices: PackedByteArray = geometry.MOVT
	for face in range(flags.size() / 2):
		var flag := int(flags[2 * face])
		var material := int(flags[2 * face + 1])
		if not (flag & 0x8 or (flag & 0x20 and not flag & 0x4) or material == 0xFF):
			continue
		for corner in range(3):
			var offset := int(indices.decode_u16(6 * face + 2 * corner)) * 12
			faces.append(Vector3(vertices.decode_float(offset), vertices.decode_float(offset + 8), -vertices.decode_float(offset + 4)))
	return faces

func run() -> void:
	var ship := Node3D.new()
	root.add_child(ship)
	var model := Node3D.new()
	model.rotation.y = -PI / 2.0
	ship.add_child(model)
	var body := StaticBody3D.new()
	body.collision_layer = 1 << 5
	body.collision_mask = 0
	model.add_child(body)
	var collision := CollisionShape3D.new()
	var shape := ConcavePolygonShape3D.new()
	shape.backface_collision = true
	shape.set_faces(deck_faces())
	collision.shape = shape
	body.add_child(collision)
	ship.position = Vector3(1193.9063, 297.08664, -2677.0313)
	ship.rotation.y = PI / 2.0
	await physics_frame
	await process_frame
	var force := OS.get_cmdline_user_args().has("--force")
	for step in range(1, 4):
		ship.position.x += 50.0
		if force:
			body.force_update_transform()
		var feet: Vector3 = ship.global_transform * model.transform * Vector3(-30.0, 20.55, 0.0)
		var query := PhysicsRayQueryParameters3D.create(feet + Vector3.UP * 1.6, feet - Vector3.UP * 50.0, 1 << 5)
		var hit := ship.get_world_3d().direct_space_state.intersect_ray(query)
		print("MOVING_TRANSPORT_COLLISION step=", step, " force=", force, " feet=", feet, " hit=", hit)
		if not hit.has("position") or absf(hit.position.y - 317.68984) > 0.01:
			push_error("Moved authored deck must be queryable in the same frame")
			quit(1)
			return
	ship.queue_free()
	quit(0)
