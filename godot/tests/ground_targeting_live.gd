extends "res://tests/spell_overrides_live.gd"

## Private-server A/B for the real Flamestrike cursor and its authored area decal.
const FLAMESTRIKE := 2120
const CONSECRATION := 26573
const SERVER := "127.0.0.1:5993"

var phase := ""
var pointer_notified := false

func _initialize() -> void:
	Engine.max_fps = 30
	call_deferred("run_test")

func run_test() -> void:
	root.size = Vector2i(1280, 720)
	var server := OS.get_environment("GODOT_TEST_SERVER")
	var account := OS.get_environment("GROUND_ACCOUNT")
	character = OS.get_environment("GROUND_CHARACTER")
	phase = OS.get_environment("GROUND_PHASE")
	shots = OS.get_environment("GROUND_SHOTS")
	if server != SERVER or not account.begins_with("fb_groundtarget_") or character == "" or shots == "" or not phase in ["before-flamestrike", "after-flamestrike", "before-consecration", "after-consecration"]:
		fail("Private server, owned character, phase, and screenshot path are required")
		return
	DirAccess.make_dir_recursive_absolute(shots)
	client = load("res://scenes/client.tscn").instantiate()
	root.add_child(client)
	skin = "forever"
	var error: String = client.connect_account(server, account, PASSWORD, false)
	if error != "":
		fail("Private account connection: " + error)
		return
	if not await enter_ground_world():
		return
	var skin_error: String = client.preview_world_skin(true)
	if skin_error != "" or client.account_state().ui_skin != "Forever":
		fail("Forever skin unavailable: " + skin_error)
		return
	player = client.get_node("WorldUnits/" + character) as Node3D
	client.set_world_minutes(720.0)
	client.set_camera_orbit(PI, -0.3, 12.0)
	if not await wait_until(func(): return client.spells_state().catalog_ready, 60000, "spell catalog"):
		return
	if not await wait_for_spell_setup():
		return
	if phase.ends_with("consecration"):
		if not await consecration_targeting_proof(phase == "before-consecration"):
			return
		print("GROUND_TARGETING_LIVE PASS phase=", phase, " skin=Forever spell=", CONSECRATION)
		client.free()
		quit(0)
		return
	if not await wait_until(func(): return client.spells_state().known.has(FLAMESTRIKE), 30000, "Mage knows Flamestrike"):
		return

	var sent_before: int = client.spells_state().sent.size()
	error = client.use_spell(FLAMESTRIKE)
	if error != "":
		fail("Flamestrike request: " + error)
		return
	if phase == "before-flamestrike":
		await wait_frames(4)
		if client.spells_state().sent.size() != sent_before + 1 or client.get_node_or_null("GroundSpellReticle") != null:
			fail("Baseline did not reproduce an instant Flamestrike without cursor targeting")
			return
		await capture("ground-target-before")
	else:
		if not await capture_valid_reticle(sent_before):
			return
		if not await capture_out_of_range_reticle():
			return
		if not await place_flamestrike(sent_before):
			return
	print("GROUND_TARGETING_LIVE PASS phase=", phase, " skin=Forever spell=", FLAMESTRIKE)
	client.free()
	quit(0)

func enter_ground_world() -> bool:
	if not await wait_until(func():
		var state: Dictionary = client.account_state()
		var ui := client.get_node_or_null("CharacterSelectUI")
		if not state.reply_received or state.screen != "CharacterSelect" or state.character_count != 1 or ui == null:
			return false
		var enter := ui.find_child("EnterWorld", true, false) as BaseButton
		return enter != null and enter.is_visible_in_tree() and not enter.disabled
	, 60000, "single-character roster and visible EnterWorld"):
		return false
	var ui := client.get_node("CharacterSelectUI")
	var row_name := ui.find_child("CharCard_0Name", true, false) as Label
	if row_name == null or row_name.text != character:
		fail("Character-select roster does not contain " + character)
		return false
	var enter := ui.find_child("EnterWorld", true, false) as BaseButton
	var point := enter.get_global_rect().get_center()
	await pointer(point)
	await mouse(point, MOUSE_BUTTON_LEFT, true)
	await mouse(point, MOUSE_BUTTON_LEFT, false)
	await wait_frames(6)
	if not await wait_until(func():
		var state: Dictionary = client.account_state()
		return state.screen == "InWorld" and state.selected_character_name == character \
			and state.local_player_position != null and state.local_server_position != null \
			and state.terrain.pending_count == 0 and not state.terrain.parsed_tiles.is_empty()
	, 180000, "enter named character"):
		return false
	await wait_frames(60)
	return true

func wait_for_spell_setup() -> bool:
	var ready_path := OS.get_environment("GROUND_READY_FILE")
	var continue_path := OS.get_environment("GROUND_CONTINUE_FILE")
	if ready_path == "" and continue_path == "":
		return true
	if ready_path == "" or continue_path == "" or FileAccess.file_exists(ready_path) or FileAccess.file_exists(continue_path):
		fail("Spell setup requires two fresh barrier paths")
		return false
	var ready := FileAccess.open(ready_path, FileAccess.WRITE)
	if ready == null:
		fail("Could not write spell setup barrier: " + ready_path)
		return false
	ready.store_string("online " + character)
	ready.close()
	print("GROUND_TARGETING_SETUP_READY character=", character)
	return await wait_until(func(): return FileAccess.file_exists(continue_path), 120000, "admin spell setup")

func consecration_targeting_proof(baseline: bool) -> bool:
	if not await wait_until(func(): return client.spells_state().known.has(CONSECRATION), 30000, "Paladin knows Consecration"):
		return false
	var sent_before: int = client.spells_state().sent.size()
	var error: String = client.use_spell(CONSECRATION)
	if error != "":
		fail("Consecration request: " + error)
		return false
	if baseline:
		await pointer(reticle_point(8.0))
		if not await wait_until(func():
			var reticle := client.get_node_or_null("GroundSpellReticle") as Node3D
			return reticle != null and reticle.visible
		, 10000, "baseline Consecration cursor"):
			return false
		if client.spells_state().sent.size() != sent_before:
			fail("Baseline sent Consecration before a cursor destination")
			return false
		await capture("consecration-before")
		return true
	if not await wait_until(func(): return client.spells_state().sent.size() == sent_before + 1, 5000, "instant Consecration request"):
		return false
	if client.spells_state().sent[-1] != CONSECRATION or client.get_node_or_null("GroundSpellReticle") != null:
		fail("Consecration entered ground cursor targeting")
		return false
	await capture("consecration-after")
	print("PROOF Consecration request sent without a ground cursor")
	return true

func capture_valid_reticle(sent_before: int) -> bool:
	var point := reticle_point(8.0)
	await pointer(point)
	if not await wait_until(func():
		var reticle := client.get_node_or_null("GroundSpellReticle") as Decal
		return reticle != null and reticle.visible
	, 10000, "Flamestrike area decal"):
		return false
	if client.spells_state().sent.size() != sent_before:
		fail("Flamestrike was sent before a cursor destination was selected")
		return false
	var decal := client.get_node("GroundSpellReticle") as Decal
	if decal.texture_albedo == null or not is_equal_approx(decal.size.x, 16.0):
		fail("Flamestrike decal is missing its authored texture or eight-yard radius")
		return false
	if decal.modulate.g <= decal.modulate.r:
		fail("In-range Flamestrike decal is not green")
	if not await save_texture_reference(decal):
		return false
	await capture("ground-target-after")
	return true

func save_texture_reference(decal: Decal) -> bool:
	var path := OS.get_environment("GROUND_TEXTURE_REFERENCE")
	if path == "":
		return true
	var image: Image = decal.texture_albedo.get_image()
	if image.save_png(path) != OK:
		fail("Could not save authored target-area texture: " + path)
		return false
	print("GROUND_TEXTURE_REFERENCE ", path)
	return true

func capture_out_of_range_reticle() -> bool:
	client.set_camera_orbit(PI, -0.3, 120.0)
	await wait_frames(12)
	for screen_y in [160, 280, 400, 520, 640]:
		for screen_x in [160, 400, 640, 880, 1120]:
			var point := Vector2(screen_x, screen_y)
			await pointer(point)
			var decal := client.get_node_or_null("GroundSpellReticle") as Decal
			if decal == null or not decal.visible:
				continue
			var distance := player.global_position.distance_to(decal.global_position)
			print("GROUND_RANGE_PROBE point=", point, " distance=", distance,
				" max=40.0 color=", decal.modulate)
			if distance <= 40.0:
				continue
			if decal.modulate.r <= decal.modulate.g:
				fail("Out-of-range Flamestrike decal is not red")
				return false
			await capture("ground-target-invalid")
			return true
	fail("No visible ground point beyond Flamestrike's 40-yard range")
	return false

func place_flamestrike(sent_before: int) -> bool:
	client.set_camera_orbit(PI, -0.3, 12.0)
	await wait_frames(12)
	var point := reticle_point(8.0)
	await pointer(point)
	if not await wait_until(func():
		var decal := client.get_node_or_null("GroundSpellReticle") as Decal
		return decal != null and decal.visible and decal.modulate.g > decal.modulate.r
	, 10000, "valid Flamestrike destination"):
		return false
	await mouse(point, MOUSE_BUTTON_LEFT, true)
	await mouse(point, MOUSE_BUTTON_LEFT, false)
	if not await wait_until(func(): return client.spells_state().sent.size() == sent_before + 1, 10000, "placed Flamestrike"):
		return false
	if client.spells_state().sent[-1] != FLAMESTRIKE or client.get_node_or_null("GroundSpellReticle") != null:
		fail("Selected Flamestrike did not send once and exit targeting")
		return false
	await capture("ground-target-placed")
	return true

func reticle_point(distance_yd: float) -> Vector2:
	var camera := root.get_camera_3d()
	return camera.unproject_position(player.global_position + Vector3(0.0, 0.0, distance_yd))

func pointer(at: Vector2) -> void:
	if not pointer_notified:
		root.notify_mouse_entered()
		pointer_notified = true
	var event := InputEventMouseMotion.new()
	event.position = at
	event.global_position = at
	root.push_input(event, true)
	await wait_frames(6)

func mouse(at: Vector2, button: MouseButton, pressed: bool) -> void:
	var event := InputEventMouseButton.new()
	event.position = at
	event.global_position = at
	event.button_index = button
	event.button_mask = (1 << (button - 1)) if pressed else 0
	event.pressed = pressed
	root.push_input(event, true)
	await process_frame
