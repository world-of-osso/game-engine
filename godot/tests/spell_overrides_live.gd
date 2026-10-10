extends "res://tests/skyriding_bar_live.gd"

## Private real-server proof: aura1719 action substitution/restoration, ground leap,
## and Blink against the same Abbey wall segment used by the server's real LOS test.
var skin := ""
var scenario := ""

func run_test() -> void:
	Engine.max_fps = 30
	root.size = Vector2i(1280, 720)
	var server := OS.get_environment("GODOT_TEST_SERVER")
	var account := OS.get_environment("OVERRIDE_ACCOUNT")
	character = OS.get_environment("OVERRIDE_CHARACTER")
	skin = OS.get_environment("OVERRIDE_SKIN")
	scenario = OS.get_environment("OVERRIDE_CASE")
	shots = OS.get_environment("OVERRIDE_SHOTS")
	if server != "127.0.0.1:5184" or not account.begins_with("fb_overrides_") or shots == "":
		fail("Owned private endpoint/account/capture directory required")
		return
	DirAccess.make_dir_recursive_absolute(shots)
	client = load("res://scenes/client.tscn").instantiate()
	root.add_child(client)
	var error: String = client.connect_account(server, account, PASSWORD, false)
	if error != "":
		fail(error)
		return
	if not await enter_world():
		return
	player = client.get_node("WorldUnits/" + character) as Node3D
	client.set_world_minutes(720.0)
	client.set_camera_orbit(PI, -0.3, 12.0)
	if not await wait_until(func(): return client.spells_state().catalog_ready, 60000, "spell catalog"):
		return
	var passed := await override_proof() if scenario == "war" else await blink_proof()
	if not passed:
		return
	print("CLIENT_OVERRIDES_LIVE PASS skin=%s case=%s" % [skin, scenario])
	client.free()
	quit(0)

func override_proof() -> bool:
	var error: String = client.set_specialization(72)
	if error != "":
		fail(error)
		return false
	if not await wait_until(func(): return client.spells_state().spec == 72, 10000, "Fury specialization"):
		return false
	if not signal_setup():
		return false
	if not await wait_until(func(): return client.spells_state().known.has(85288) and client.spells_state().known.has(1719), 30000, "Fury spells"):
		return false
	await tap(KEY_P)
	await wait_frames(4)
	var book: Node = client.get_node("SpellBookUI")
	var icon: Control = await find_book_spell(book, 85288)
	if icon == null or not icon.is_visible_in_tree():
		fail("No learned Raging Blow button")
		return false
	var bar: Node = client.get_node("MainActionBarUI")
	var button := bar.find_child("ActionButton1", true, false) as Control
	await drag_spell(icon, button)
	if not await wait_until(func(): return client.spells_state().bar[0] == 85288, 5000, "base assignment"):
		return false
	await hover(button)
	await capture("override-base")
	if client.use_spell(1719) != "":
		fail("Recklessness send")
		return false
	if not await wait_until(func(): return client.spells_state().bar[0] == 335097, 5000, "Crushing Blow override"):
		return false
	await wait_frames(3)
	var label := book.find_child("SpellBookItem85288Name", true, false) as Label
	if label == null or label.text != "Crushing Blow":
		fail("Spellbook did not substitute name under base ID")
		return false
	await hover(button)
	if client.tooltip_state().title != "Crushing Blow":
		fail("Action tooltip did not substitute")
		return false
	await capture("override-active")
	await tap(KEY_1)
	if client.spells_state().sent[-1] != 335097:
		fail("Action did not submit replacement cast")
		return false
	print("PROOF replacement intent sent=", client.spells_state().sent, " errors=", client.spells_state().errors)
	if not await wait_until(func(): return client.spells_state().bar[0] == 85288, 20000, "aura expiry restores base"):
		return false
	await wait_frames(3)
	label = book.find_child("SpellBookItem85288Name", true, false) as Label
	if label == null or label.text != "Raging Blow":
		fail("Spellbook did not restore base name")
		return false
	await hover(button)
	if client.tooltip_state().title != "Raging Blow":
		fail("Action tooltip did not restore")
		return false
	await capture("override-restored")
	await tap(KEY_ESCAPE)
	return await ground_proof()

func find_book_spell(book: Node, spell: int) -> Control:
	for category in range(1, 6):
		var tab := book.find_child("SpellBookCategoryTab%d" % category, true, false) as Control
		if tab == null:
			break
		await click(tab)
		await wait_frames(3)
		for page in range(16):
			var icon := book.find_child("SpellBookItem%dButton" % spell, true, false) as Control
			if icon != null and icon.is_visible_in_tree():
				return icon
			var next := book.find_child("SpellBookNextPageButton", true, false) as Control
			if next == null:
				break
			await click(next)
			await wait_frames(3)
	return null

func ground_proof() -> bool:
	var before: int = client.spells_state().sent.size()
	var start := player.global_position
	if client.use_spell(6544) != "":
		fail("Heroic Leap targeting")
		return false
	var camera := root.get_camera_3d()
	var point := camera.unproject_position(start + Vector3(2.0, 0.0, 0.0))
	await pointer(point)
	var reticle := client.get_node_or_null("GroundSpellReticle") as Node3D
	if reticle == null or not reticle.visible or client.spells_state().sent.size() != before:
		fail("Ground targeting did not hold the intent and show reticle")
		return false
	await capture("ground-reticle")
	await mouse(point, MOUSE_BUTTON_LEFT, true)
	await mouse(point, MOUSE_BUTTON_LEFT, false)
	if not await wait_until(func(): return player.global_position.distance_to(start) > 0.5, 5000, "ground leap authoritative displacement"):
		return false
	if client.spells_state().sent[-1] != 6544 or client.get_node_or_null("GroundSpellReticle") != null:
		fail("Ground placement did not send/exit targeting")
		return false
	print("PROOF ground start=", start, " end=", player.global_position)
	await capture("ground-placed")
	return true

func signal_setup() -> bool:
	var setup := OS.get_environment("OVERRIDE_SETUP")
	if setup == "":
		fail("Owned online spell setup signal required")
		return false
	FileAccess.open(setup, FileAccess.WRITE).store_string("online")
	return true

func blink_proof() -> bool:
	if not signal_setup():
		return false
	if not await wait_until(func(): return client.spells_state().known.has(1953), 30000, "Blink learned"):
		return false
	var yaw := atan2(12.7, 27.2)
	client.set_camera_orbit(yaw - PI, -0.3, 12.0)
	await mouse(Vector2(700, 380), MOUSE_BUTTON_RIGHT, true)
	await wait_frames(4)
	await mouse(Vector2(700, 380), MOUSE_BUTTON_RIGHT, false)
	await wait_frames(12)
	var start := player.global_position
	var camera := root.get_camera_3d()
	var camera_start := camera.global_position
	await capture("blink-before-wall")
	var frames := shots.path_join(skin + "-blink-frames")
	DirAccess.make_dir_recursive_absolute(frames)
	for frame in range(60):
		if frame == 6 and client.use_spell(1953) != "":
			fail("Blink submission")
			return false
		await RenderingServer.frame_post_draw
		root.get_texture().get_image().save_png(frames.path_join("%03d.png" % frame))
		await process_frame
	var finish := player.global_position
	var distance := Vector2(finish.x - start.x, finish.z - start.z).length()
	print("PROOF blink start=", start, " end=", finish, " distance=", distance,
		" camera_before=", camera_start, " camera_after=", camera.global_position,
		" errors=", client.spells_state().errors)
	if distance <= 0.5 or distance >= 19.0:
		fail("Blink did not clamp at the Abbey wall: " + str(distance))
		return false
	await capture("blink-after-wall")
	return true

func trace(label: String) -> void:
	print("TRACE ", label, " spells=", client.spells_state(), " account=", client.account_state())

func capture(label: String) -> void:
	await RenderingServer.frame_post_draw
	var path := shots.path_join(skin + "-" + label + ".png")
	root.get_texture().get_image().save_png(path)
	print("CAPTURE ", path)

func tap(code: Key) -> void:
	push_key(code, true)
	await wait_frames(2)
	push_key(code, false)
	await wait_frames(2)

func hover(control: Control) -> void:
	await pointer(control.get_global_rect().get_center())
	await wait_frames(4)

func pointer(at: Vector2) -> void:
	var event := InputEventMouseMotion.new()
	event.position = at
	event.global_position = at
	root.push_input(event, true)
	await process_frame

func mouse(at: Vector2, button: MouseButton, pressed: bool) -> void:
	var event := InputEventMouseButton.new()
	event.position = at
	event.global_position = at
	event.button_index = button
	event.pressed = pressed
	root.push_input(event, true)
	await process_frame

func drag_spell(source: Control, destination: Control) -> void:
	await mouse(source.get_global_rect().get_center(), MOUSE_BUTTON_LEFT, true)
	await pointer(destination.get_global_rect().get_center())
	await wait_frames(2)
	await mouse(destination.get_global_rect().get_center(), MOUSE_BUTTON_LEFT, false)
	await wait_frames(3)
