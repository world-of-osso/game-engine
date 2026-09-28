extends "res://tests/world_collision_flow.gd"

func _initialize() -> void:
	Engine.max_fps = 60
	super._initialize()

func wait_camera(camera: Camera3D, expected: Vector3) -> bool:
	var deadline := Time.get_ticks_msec() + 15000
	while Time.get_ticks_msec() < deadline:
		await process_frame
		if camera.global_position.distance_to(expected) < 0.08:
			return true
	fail("World camera did not converge: %s, expected %s" % [camera.global_position, expected])
	return false

func spawn_blocker(parent: Node3D, position: Vector3) -> MeshInstance3D:
	var mesh := MeshInstance3D.new()
	mesh.mesh = BoxMesh.new()
	var body := StaticBody3D.new()
	var shape := CollisionShape3D.new()
	shape.shape = BoxShape3D.new()
	body.add_child(shape)
	mesh.add_child(body)
	parent.add_child(mesh)
	mesh.global_position = position
	return mesh

func inspect_material_tiles(client: Node, parsed_tiles: Array) -> bool:
	if not await super.inspect_material_tiles(client, parsed_tiles):
		return false
	var camera := client.get_node_or_null("WorldCamera") as Camera3D
	if camera == null or not camera.current:
		fail("Native world lacks its selected-player follow camera")
		return false
	if not is_equal_approx(camera.fov, 90.0) or not is_equal_approx(camera.near, 0.1) or not is_equal_approx(camera.far, 1000.0):
		fail("World camera does not preserve original projection defaults")
		return false
	var state: Dictionary = client.account_state()
	var player := client.get_node("WorldUnits/" + state.selected_character_name) as Node3D
	# Move only this fixture's native node above terrain; no movement is sent to the server.
	player.position.y += 100.0
	var focus := player.global_position + Vector3.UP * 1.8
	var orbit := Vector3(0.0, sin(-0.3), -cos(-0.3))
	var expected := focus - orbit * 15.0
	if not await wait_camera(camera, expected):
		return false
	if (-camera.global_basis.z).dot((focus - camera.global_position).normalized()) < 0.999:
		fail("World camera does not look at the original eye target")
		return false

	var self_blocker := spawn_blocker(player, focus - orbit * 2.5)
	await physics_frame
	await process_frame
	for _frame in range(10):
		await process_frame
	if camera.global_position.distance_to(expected) > 0.08:
		fail("Camera collided with its own selected-player descendant")
		return false
	self_blocker.queue_free()

	var blocker := spawn_blocker(client, focus - orbit * 5.0)
	await physics_frame
	await process_frame
	var query := PhysicsRayQueryParameters3D.create(focus, expected)
	var hit: Dictionary = client.get_world_3d().direct_space_state.intersect_ray(query)
	if hit.is_empty():
		fail("Camera obstruction fixture has no actual physics hit")
		return false
	var hit_distance := focus.distance_to(hit.position)
	if not await wait_camera(camera, focus - orbit * maxf(hit_distance - 0.3, 0.5)):
		return false
	blocker.hide()
	if not await wait_camera(camera, expected):
		return false
	blocker.queue_free()
	if client.account_state().screen != "InWorld":
		fail("Camera fixture lost readiness after center terrain attachment")
		return false
	print("PASS: original camera orbit/projection, self exclusion, mesh obstruction and hidden-mesh recovery")
	return true

func inspect_reset(client: Node) -> bool:
	if not await super.inspect_reset(client):
		return false
	if client.get_node_or_null("WorldCamera") != null:
		fail("Reconnect retained previous world camera")
		return false
	print("PASS: reconnect removes world camera")
	return true
