extends "res://tests/world_portal_particles_flow.gd"

# Controlled density-sensitive copy of portal 197007, not retail asset parity.
const SAMPLE_COUNT := 9
const SAMPLE_INTERVAL_MS := 250
const WARMUP_MS := 2200

func run_test() -> void:
	root.size = Vector2i(1280, 720)
	var endpoint := OS.get_environment("GODOT_TEST_SERVER")
	var config := OS.get_environment("XDG_CONFIG_HOME")
	var options := config.path_join("world-of-osso/options_settings.ron")
	if not endpoint.begins_with("127.0.0.1:") or endpoint.ends_with(":0") or OS.get_environment("GODOT_TEST_PARTICLES") != "density" or not config.contains("/data/native-reset-fixture-"):
		fail("Density fixture requires owned loopback, mode and staged project")
		return
	if not FileAccess.get_file_as_string(options).contains("particleDensity:100"):
		fail("Density fixture did not start at 100: " + FileAccess.get_file_as_string(options))
		return
	var client: Node = load("res://scenes/client.tscn").instantiate()
	root.add_child(client)
	if not await wait_screen(client, "Loading", 15000):
		return
	print("FIXTURE DENSITY_LOADING")
	if not await wait_world(client):
		return
	var portal := await find_portal(client, 0)
	if portal == null:
		return
	var old_id := portal.get_instance_id()
	var pools := pool_identity(client)
	if pools.size() != 6:
		fail("Controlled portal did not register six pools: " + str(pools))
		return
	var initial := await sample_portal(client, portal, "initial 100")
	if initial <= 0.0:
		return
	push_key(KEY_ESCAPE, true)
	await process_frame
	push_key(KEY_ESCAPE, false)
	if not await wait_menu(client):
		return
	await click_menu_action(client, "MenuBtnOptions")
	var menu := client.get_node_or_null("GameMenuUI")
	var tab := menu.find_child("OptionsTabgraphics", true, false) as Control if menu != null else null
	if tab == null or not tab.is_visible_in_tree():
		fail("Authored graphics tab missing")
		return
	await click(tab)
	var slider := menu.find_child("Sliderparticle_density", true, false) as Control
	if slider == null or not slider.is_visible_in_tree():
		fail("Authored Particle Density slider missing")
		return
	await set_density_minimum(slider)
	if not FileAccess.get_file_as_string(options).replace(" ", "").contains("particleDensity:10"):
		fail("Real slider did not persist density 10: " + FileAccess.get_file_as_string(options))
		return
	var done := menu.find_child("OptionsDoneButton", true, false) as Control
	if done == null:
		fail("Authored Options Done missing")
		return
	await click(done)
	if client.get_node_or_null("GameMenuUI") != null:
		await click_menu_action(client, "MenuBtnResume")
	if not await wait_menu_closed(client, null):
		return
	if portal.get_instance_id() != old_id or pool_ids(client) != ids_of(pools):
		fail("Options replaced existing portal or its pools")
		return
	var unchanged := await sample_portal(client, portal, "existing after 10")
	if unchanged <= 0.0 or unchanged < initial * 0.7 or unchanged > initial * 1.3:
		fail("Existing portal rate changed: initial=%s existing=%s" % [initial, unchanged])
		return
	print("FIXTURE DENSITY_TRANSFER")
	var transfer_deadline := Time.get_ticks_msec() + PORTAL_WAIT_MS
	while Time.get_ticks_msec() < transfer_deadline:
		await process_frame
		if client.account_state().screen == "InWorld" and client.get_node_or_null("M2Particles") != null and pool_ids(client) != ids_of(pools):
			break
	if pool_ids(client) == ids_of(pools):
		fail("NewWorld did not release original particle pool root")
		return
	for item in pools:
		if item.ref.get_ref() != null:
			fail("NewWorld retained old portal particle pool: " + str(item.id))
			return
	var replacement := await find_portal(client, old_id)
	if replacement == null:
		return
	var fresh_pools := pool_identity(client)
	if fresh_pools.size() != 6 or ids_of(fresh_pools) == ids_of(pools):
		fail("NewWorld did not register six fresh portal pools: " + str(fresh_pools))
		return
	var fresh := await sample_portal(client, replacement, "new after 10")
	if fresh <= 0.0 or fresh >= initial * 0.3:
		fail("DENSITY RED: new placement did not respond to slider 100->10: initial=%s existing=%s fresh=%s" % [initial, unchanged, fresh])
		return
	print("FIXTURE DENSITY_DONE")
	client.free()
	quit(0)

func set_density_minimum(slider: Control) -> void:
	var rect := slider.get_global_rect()
	var start := rect.get_center()
	var target := Vector2(rect.position.x - 10.0, start.y)
	var button := InputEventMouseButton.new()
	button.button_index = MOUSE_BUTTON_LEFT
	button.position = start
	button.global_position = start
	button.pressed = true
	root.push_input(button, true)
	await process_frame
	var motion := InputEventMouseMotion.new()
	motion.position = target
	motion.global_position = target
	motion.relative = target - start
	motion.button_mask = MOUSE_BUTTON_MASK_LEFT
	root.push_input(motion, true)
	await process_frame
	button.position = target
	button.global_position = target
	button.pressed = false
	root.push_input(button, true)
	await process_frame

func pool_identity(client: Node) -> Array:
	var ids := []
	for index in 6:
		var pool := client.find_child("Particles197007_%d" % index, true, false) as MultiMeshInstance3D
		if pool == null or pool.multimesh == null:
			return []
		ids.append({"id": pool.get_instance_id(), "ref": weakref(pool)})
	return ids

func ids_of(pools: Array) -> Array:
	return pools.map(func(pool): return pool.id)

func pool_ids(client: Node) -> Array:
	return ids_of(pool_identity(client))

func find_portal(client: Node, previous_id: int) -> Node3D:
	var deadline := Time.get_ticks_msec() + PORTAL_WAIT_MS
	while Time.get_ticks_msec() < deadline:
		await process_frame
		var world := client.get_node_or_null("WorldObjects")
		if world != null:
			for wmo in world.get_children():
				var portal := wmo.get_node_or_null("WmoDoodad1112") as Node3D
				if portal == null or portal.get_instance_id() == previous_id:
					continue
				var scale := portal.global_transform.basis.get_scale()
				if portal.global_position.distance_to(PORTAL) > 5.0 or scale.distance_to(Vector3.ONE * PORTAL_SCALE) > 0.002 or portal.find_children("*", "MeshInstance3D", true, false).is_empty():
					fail("Controlled portal placement/mesh incorrect: " + str(portal.global_transform))
					return null
				return portal
		var state: Dictionary = client.account_state()
		if state.screen == "InWorld" and state.world_objects.get("pending", 1) == 0:
			fail("No fresh placed portal: " + str(state.world_objects))
			return null
	fail("Timed out waiting for controlled portal placement")
	return null

func sample_portal(client: Node, portal: Node3D, label: String) -> float:
	var camera := client.get_node_or_null("WorldCamera") as Camera3D
	if camera == null:
		fail(label + ": no native camera")
		return -1.0
	var warmup := Time.get_ticks_msec() + WARMUP_MS
	while Time.get_ticks_msec() < warmup:
		await process_frame
	var total := 0.0
	var first := Time.get_ticks_msec()
	for index in SAMPLE_COUNT:
		var deadline := first + index * SAMPLE_INTERVAL_MS
		while Time.get_ticks_msec() < deadline:
			await process_frame
		if not is_instance_valid(portal) or not portal.visible or not camera.is_position_in_frustum(portal.global_position):
			fail(label + ": portal not visible to camera")
			return -1.0
		var state: Dictionary = client.account_state().world_objects.get("particles", {})
		if state.get("pools", 0) < 6 or state.get("updated_emitters", 0) < 6:
			fail(label + ": portal particles not updating: " + str(state))
			return -1.0
		var drawn := 0
		for emitter in 6:
			var pool := client.find_child("Particles197007_%d" % emitter, true, false) as MultiMeshInstance3D
			if pool == null or pool.multimesh == null:
				fail(label + ": missing known portal pool " + str(emitter))
				return -1.0
			drawn += pool.multimesh.visible_instance_count
		total += drawn
	var duration := Time.get_ticks_msec() - first
	if duration < (SAMPLE_COUNT - 1) * SAMPLE_INTERVAL_MS or total <= 0.0:
		fail(label + ": invalid sample count/duration or no quads: n=%s ms=%s total=%s" % [SAMPLE_COUNT, duration, total])
		return -1.0
	var average := total / SAMPLE_COUNT
	print("DENSITY SAMPLE %s n=%s ms=%s average=%s" % [label, SAMPLE_COUNT, duration, average])
	return average
