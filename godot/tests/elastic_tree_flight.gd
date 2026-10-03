extends "res://tests/flying_mount_live.gd"

# Own a private server/account via the inherited flight fixture contract. Place a
# real annotated tree in the flight path, then drive the mounted player using W.
# The server must replicate the corrected position, not the unobstructed path.
var contact_tree: Node3D

func run_test() -> void:
	root.size = Vector2i(960, 540)
	var server := OS.get_environment("GODOT_TEST_SERVER")
	var account := OS.get_environment("FLY_ACCOUNT")
	character = OS.get_environment("FLY_CHARACTER")
	if server == "" or server == "127.0.0.1:5000" or account == "" or character == "":
		fail("Private GODOT_TEST_SERVER, FLY_ACCOUNT and FLY_CHARACTER required")
		return
	shots = OS.get_environment("FLY_SHOTS")
	if shots == "":
		fail("FLY_SHOTS required for mounted tree-contact proof")
		return
	DirAccess.make_dir_recursive_absolute(shots)
	client = load("res://scenes/client.tscn").instantiate()
	root.add_child(client)
	var error = client.connect_account(server, account, PASSWORD, false)
	if error != "":
		fail("connect: " + error)
		return
	if not await enter_world():
		return
	player = client.get_node("WorldUnits/" + character) as Node3D
	var ready_file := OS.get_environment("FLY_READY_FILE")
	print("ELASTIC_TREE_FLIGHT READY_FOR_SPELLS")
	if ready_file != "" and not await wait_until(func(): return FileAccess.file_exists(ready_file), 60000, ready_file):
		return
	client.set_world_minutes(720)
	client.set_camera_orbit(0, -0.3, 30)
	var sent: String = client.use_spell(GOLDEN_GRYPHON)
	if sent != "":
		fail("mount: " + sent)
		return
	if not await wait_until(func(): return mounted(), 30000, "mounted"):
		return
	var ground := player.position.y
	push_key(KEY_SPACE, true)
	if not await wait_until(func(): return player.position.y > ground + 35, 30000, "airborne"):
		return
	push_key(KEY_SPACE, false)
	await wait_frames(3)
	var start := player.global_position
	push_key(KEY_W, true)
	await wait_seconds(0.3)
	push_key(KEY_W, false)
	var forward := (player.global_position - start).normalized()
	if forward.length() < 0.9 or absf(forward.y) > 0.1:
		fail("Cannot calibrate level flight direction: " + str(forward))
		return
	var loader := WowAssetLoader.new()
	var loaded: Dictionary = loader.load_m2("res://../data/models/201394.m2")
	if loaded.has("error"):
		fail("tree: " + str(loaded.error))
		return
	contact_tree = loaded.node
	contact_tree.rotation.y = atan2(forward.x, forward.z)
	var midpoint := Vector3(-3, 19, -1).lerp(Vector3(-24.24, 26.39, -6.72), 0.8)
	contact_tree.position = player.global_position + Vector3.UP + forward * 8 - contact_tree.basis * midpoint
	client.add_child(contact_tree)
	var spring := contact_tree.get_node("ElasticTree")
	await wait_frames(3)
	await snapshot("tree-approach")
	start = player.global_position
	var peak := 0.0
	var start_ms := Time.get_ticks_msec()
	push_key(KEY_W, true)
	while Time.get_ticks_msec() - start_ms < 1000:
		await process_frame
		peak = maxf(peak, spring.branch_state()[0].rotation.length())
	push_key(KEY_W, false)
	var end := player.global_position
	var travel := (end - start).dot(forward)
	var lateral := (end - start - forward * travel).length()
	if peak < 0.01 or travel < 8 or lateral < 0.03:
		fail("Mounted flight did not bend/deflect through limb: peak=%s travel=%s lateral=%s" % [peak, travel, lateral])
		return
	await snapshot("tree-contact")
	if not await wait_until(func(): return server_tracks(end), 30000, "server follows deflected flight"):
		return
	print("TRACE mounted tree-contact peak=", peak, " travel=", travel, " lateral=", lateral, " client=", end, " server=", client.account_state().local_server_position)
	# Manual reverse input still works after contact; no forced stop/controller takeover.
	push_key(KEY_S, true)
	await wait_seconds(0.3)
	push_key(KEY_S, false)
	if (player.global_position - end).dot(forward) >= -0.5:
		fail("Tree contact removed reverse steering")
		return
	await wait_seconds(6)
	if spring.branch_state()[0].rotation.length() > 0.001:
		fail("Contacted branch did not recover")
		return
	await snapshot("tree-recovered")
	contact_tree.queue_free()
	await wait_frames(3)
	print("ELASTIC_TREE_FLIGHT PASS: real mount/input/limb-bend/deflection/authoritative-position/steering/recovery")
	client.queue_free()
	await wait_frames(3)
	quit(0)

func server_tracks(position: Vector3) -> bool:
	var server_position = client.account_state().local_server_position
	return server_position != null and (server_position as Vector3).distance_to(position) < 0.5
