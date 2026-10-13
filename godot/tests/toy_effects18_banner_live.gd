extends "res://tests/toy_effects14_ground_live.gd"
# Actual journal use places the Chilling Celebration Banner at clicked ground; a real
# right-click on it applies its audited cosmetic aura 424023 to the user.
const RACK := 409869
const AURA := 424023

func proof_cases() -> Array:
	return [[210042, 424022, -1, "Chilling Celebration Banner"]]

func prove_toy(sample: Array) -> void:
	await learn_toy(int(sample[0]))
	if failed: return
	for unit in client.find_children(character, "Node3D", true, false):
		if unit.get_node_or_null("PlayerModel") != null:
			actor = unit
			break
	if actor == null: fail("native actor missing"); return
	if has_aura(AURA): fail("proof must start without the banner aura"); return
	# A rack left by an earlier run stays for its own 180s; prove the one placed now.
	var earlier := racks()
	await start_cursor(str(sample[3]))
	if failed: return
	var camera := root.get_camera_3d()
	var forward := -camera.global_basis.z
	forward.y = 0.0
	await click_world(camera.unproject_position(actor.global_position + forward.normalized() * 3.0), MOUSE_BUTTON_LEFT)
	if not await wait_until(func(): return new_rack(earlier) != null, "placed banner", 60000): return
	var rack := new_rack(earlier)
	if rack.find_children("*", "MeshInstance3D", true, false).is_empty(): fail("rack model has no meshes"); return
	# Stand behind and beside the rack, inside its use reach, so the camera sees it past the player.
	var view := forward.normalized()
	var beside := rack.global_position - view * 2.0 + camera.global_basis.x.normalized() * 2.5
	if not command(OS.get_environment("TOYFX_ADMIN"), ["set-position", character, str(beside.x), str(-beside.z), str(beside.y)]): return
	await create_timer(2.0).timeout
	await capture("toy210042-banner")
	var point = await hover_rack(rack)
	if point == null: fail("pointer never shows the rack's Interact cursor: " + str(client.merchant_state().cursor)); return
	await click_world(point, MOUSE_BUTTON_RIGHT)
	if not await wait_until(func(): return has_aura(AURA), "banner cosmetic aura", 30000): return
	if not native_display(-1): fail("banner aura replaced native model"); return
	receipts.append({"phase": "banner-use", "aura": AURA})
	await capture("toy210042-banner-aura")

func click_world(point: Vector2, button: MouseButton) -> void:
	var motion := InputEventMouseMotion.new()
	motion.position = point
	motion.global_position = point
	root.push_input(motion, true)
	await create_timer(0.5).timeout
	for down in [true, false]:
		var event := InputEventMouseButton.new()
		event.position = point
		event.global_position = point
		event.button_index = button
		event.pressed = down
		root.push_input(event, true)
		await process_frame

# Only a usable object under the pointer shows the Interact cursor; probe up the rack.
func hover_rack(rack: Node3D):
	var camera := root.get_camera_3d()
	for height in [0.4, 0.8, 1.2, 0.2, 1.6]:
		var point := camera.unproject_position(rack.global_position + Vector3.UP * height)
		var motion := InputEventMouseMotion.new()
		motion.position = point
		motion.global_position = point
		root.push_input(motion, true)
		await create_timer(0.6).timeout
		if client.merchant_state().cursor == "Interact": return point
	return null

func racks() -> Array:
	var found: Array = []
	for node in client.find_children("*", "Node3D", true, false):
		if node.has_meta("game_object_entry") and int(node.get_meta("game_object_entry")) == RACK: found.append(node)
	return found

func new_rack(earlier: Array) -> Node3D:
	for rack in racks():
		if not earlier.has(rack): return rack
	return null

