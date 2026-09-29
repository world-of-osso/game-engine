extends "res://tests/world_menu_flow.gd"

# Authenticated GameClient, not WowWmoPlacementProbe: the latter enables particles
# regardless of the saved graphics setting.
const PORTAL := Vector3(-8761.85, 87.81, -848.56)
const PORTAL_SCALE := 1.3597486
# The in-world AllObjects queue spans nine Azeroth tiles before the MODD is reached.
const PORTAL_WAIT_MS := 720000

func run_test() -> void:
	root.size = Vector2i(1280, 720)
	var endpoint := OS.get_environment("GODOT_TEST_SERVER")
	var mode := OS.get_environment("GODOT_TEST_PARTICLES")
	var config := OS.get_environment("XDG_CONFIG_HOME")
	var path := config.path_join("world-of-osso/options_settings.ron")
	if not endpoint.begins_with("127.0.0.1:") or endpoint.ends_with(":0"):
		fail("Portal fixture requires owned loopback endpoint")
		return
	if not config.contains("/data/diagnostics/native-input-") or not FileAccess.file_exists(path) or not ["enabled", "disabled"].has(mode):
		fail("Portal fixture requires owned persisted graphics setting and mode")
		return
	var saved := FileAccess.get_file_as_string(path)
	if not saved.contains("particleEffectsEnabled:%s" % ("true" if mode == "enabled" else "false")):
		fail("Portal fixture graphics setting does not match mode: " + saved)
		return
	var client: Node = load("res://scenes/client.tscn").instantiate()
	root.add_child(client)
	if not await wait_screen(client, "Loading", 15000):
		return
	print("FIXTURE PORTAL_LOADING")
	if not await wait_world(client):
		return
	var player := client.get_node_or_null("WorldUnits/" + NAME) as Node3D
	if player == null or player.global_position.distance_to(Vector3(-8766.11, 88.5, -845.5)) > 1.0:
		fail("Portal fixture selected player did not reach the authored doorway: " + str(client.account_state()))
		return
	var deadline := Time.get_ticks_msec() + PORTAL_WAIT_MS
	var portal: Node3D = null
	var wmo: Node3D = null
	while Time.get_ticks_msec() < deadline:
		await process_frame
		var objects: Dictionary = client.account_state().world_objects
		var world := client.get_node_or_null("WorldObjects")
		if world != null:
			for candidate in world.get_children():
				if candidate.name.begins_with("Wmo") and candidate.get_node_or_null("WmoDoodad1112") != null:
					wmo = candidate as Node3D
					portal = wmo.get_node("WmoDoodad1112") as Node3D
					break
		if portal != null:
			break
		if objects.pending == 0:
			fail("No MODD 1112 under placed sw_magicdistrict: " + str(objects))
			return
	if portal == null:
		fail("Timed out waiting for placed MODD 1112: " + str(client.account_state().world_objects))
		return
	var scale := portal.global_transform.basis.get_scale()
	if portal.global_position.distance_to(PORTAL) > 5.0 or scale.distance_to(Vector3.ONE * PORTAL_SCALE) > 0.002 or portal.find_children("*", "MeshInstance3D", true, false).is_empty():
		fail("Placed portal transform/meshes incorrect: WMO=%s position=%s scale=%s" % [wmo.name, portal.global_position, scale])
		return
	var camera := client.get_node_or_null("WorldCamera") as Camera3D
	if camera == null:
		fail("No native world camera")
		return
	if mode == "disabled":
		for frame in 10:
			await process_frame
		var state: Dictionary = client.account_state().world_objects
		if state.has("particles") or client.get_node_or_null("M2Particles") != null or not client.find_children("Particles*", "MultiMeshInstance3D", true, false).is_empty():
			fail("Disabled particles retained pools or emitter state: " + str(state))
			return
		print("FIXTURE PORTAL_DONE disabled meshes=present pools=absent emitters=absent")
	else:
		var drawn := []
		var state: Dictionary = {}
		deadline = Time.get_ticks_msec() + 20000
		while Time.get_ticks_msec() < deadline:
			await process_frame
			state = client.account_state().world_objects
			var particles: Dictionary = state.get("particles", {})
			drawn.clear()
			for index in 6:
				var pool := client.find_child("Particles197007_%d" % index, true, false) as MultiMeshInstance3D
				drawn.append(-1 if pool == null else pool.multimesh.visible_instance_count)
			if camera.is_position_in_frustum(portal.global_position) and portal.visible and particles.get("pools", 0) >= 6 and particles.get("emitters", 0) >= 6 and particles.get("updated_emitters", 0) >= 6 and drawn.all(func(count): return count > 0):
				break
		var particles: Dictionary = state.get("particles", {})
		if drawn.size() != 6 or drawn.any(func(count): return count <= 0) or not camera.is_position_in_frustum(portal.global_position) or not portal.visible or particles.get("pools", 0) < 6 or particles.get("emitters", 0) < 6 or particles.get("updated_emitters", 0) < 6:
			fail("Enabled portal did not draw all six authored emitters: drawn=%s camera=%s portal=%s state=%s" % [drawn, camera.global_position, portal.global_position, state])
			return
		print("FIXTURE PORTAL_DONE enabled meshes=present pools=%s emitters=%s drawn=%s" % [state.particles.pools, state.particles.emitters, drawn])
	client.free()
	quit(0)
