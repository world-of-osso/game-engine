extends SceneTree

## TargetFrame and PlayerFrame portraits against a private server.
## Environment:
##   GODOT_TEST_SERVER        server address (a private test server)
##   PORTRAIT_ACCOUNT / PORTRAIT_CHARACTER  account (password fbtest) and a character in
##                            sight of Marshal McBride (world.db creature 197, Northshire)
##   PORTRAIT_ADMIN           game-server-admin binary for that server (its
##                            GAME_SERVER_ADMIN_SOCKET in the environment)
##   PORTRAIT_SHOTS           screenshot directory
## Targets Marshal McBride, then teleports beside Timber (1132, rank 4 rare) and targets
## it: each target's portrait is its own model, masked round at the Retail anchor; the
## rare star sits on the portrait's bottom and the target of target clears the frame.

const PASSWORD := "fbtest"
## world.db content_creature spawn of Timber (map 0), a few yards off.
const TIMBER_SPAWN := [0, -5170.0, -24.0, 387.0]
## TargetFrame.xml:58-74: Portrait 58×58 TOPRIGHT (-26, -19) of the 232×100 frame; its
## CircleMask from the portrait's TOPLEFT (0, -1) to its BOTTOMRIGHT (-1, 0).
const TARGET_PORTRAIT := Rect2(148, 19, 58, 58)
const TARGET_MASK_RECT := Vector4(0.0, 1.0 / 58.0, 57.0 / 58.0, 57.0 / 58.0)
## PlayerFrame.xml:27-42: PlayerPortrait 60×60 TOPLEFT (24, -19), its mask the same rect.
const PLAYER_PORTRAIT := Rect2(24, 19, 60, 60)

var client: Node
var shots := "/tmp/claude/portrait-live/"
var character := ""
var creature := ""

func _initialize() -> void:
	Engine.max_fps = 60
	call_deferred("run_test")

func run_test() -> void:
	root.size = Vector2i(1280, 720)
	var server := OS.get_environment("GODOT_TEST_SERVER")
	var account := OS.get_environment("PORTRAIT_ACCOUNT")
	character = OS.get_environment("PORTRAIT_CHARACTER")
	if server == "" or account == "" or character == "" or OS.get_environment("PORTRAIT_ADMIN") == "":
		fail("GODOT_TEST_SERVER, PORTRAIT_ACCOUNT, PORTRAIT_CHARACTER and PORTRAIT_ADMIN are required")
		return
	if OS.get_environment("PORTRAIT_SHOTS") != "":
		shots = OS.get_environment("PORTRAIT_SHOTS")
	DirAccess.make_dir_recursive_absolute(shots)
	client = load("res://scenes/client.tscn").instantiate()
	root.add_child(client)
	var error = client.connect_account(server, account, PASSWORD, false)
	if error != "":
		fail("Fixture connection: " + error)
		return
	if not await enter_world():
		return
	var player = await shown_portrait("PlayerPortrait", PLAYER_PORTRAIT, Vector4(0, 0, 1, 1), "PlayerFrame", "player " + character)
	if player.is_empty():
		return
	creature = "Marshal McBride"
	var mcbride = await target_portrait()
	if mcbride.is_empty():
		return
	await capture("01-mcbride.png")
	if not teleport_beside_timber():
		return
	if not await wait_until(func(): return client.account_state().terrain.pending_count == 0, 60000, "terrain at Timber"):
		return
	creature = "Timber"
	var timber = await target_portrait()
	if timber.is_empty():
		return
	if timber.appearance == mcbride.appearance:
		fail("TargetFrame portrait kept %s after the target changed" % mcbride.appearance)
		return
	if image_difference(mcbride.image, timber.image) < 0.05:
		fail("Timber's portrait render matches McBride's")
		return
	if not check_star(timber):
		return
	if not await check_target_of_target():
		return
	await capture("02-timber.png")
	print("FIXTURE PORTRAIT_LIVE_DONE")
	client.free()
	quit(0)

## Target `creature` and wait for its portrait; checked like every portrait.
func target_portrait() -> Dictionary:
	if (await find_unit()).is_empty():
		return {}
	if not await track_until(func(): return client.target_state().target_name == creature, MOUSE_BUTTON_LEFT, "%s targeted" % creature):
		return {}
	return await shown_portrait("TargetFramePortrait", TARGET_PORTRAIT, TARGET_MASK_RECT, "TargetFrame", "creature display")

## Wait for portrait `name` to show a model, then check it sits at `anchor` of `frame`
## with its mask and renders a model over a masked-out outside.
func shown_portrait(name: String, anchor: Rect2, mask_rect: Vector4, frame: String, appearance: String) -> Dictionary:
	var ready := func():
		var state: Dictionary = client.unit_portrait_state(name)
		return state.get("model_shown", false) and state.mask_loaded and not state.pending and str(state.appearance).begins_with(appearance)
	if not await wait_until(ready, 30000, "%s showing %s" % [name, appearance]):
		return {}
	await wait_frames(10)
	await RenderingServer.frame_post_draw
	var state: Dictionary = client.unit_portrait_state(name)
	var frame_rect := control("UnitFramesUI", frame).get_global_rect()
	var scale := frame_rect.size.y / 100.0
	var expected := Rect2(frame_rect.position + anchor.position * scale, anchor.size * scale)
	var rect: Rect2 = state.rect
	if not rect.position.is_equal_approx(expected.position) or not rect.size.is_equal_approx(expected.size):
		fail("%s at %s, Retail anchor %s" % [name, rect, expected])
		return {}
	if not state.visible or not (state.mask_rect as Vector4).is_equal_approx(mask_rect):
		fail("%s unmasked or hidden: %s" % [name, state])
		return {}
	var coverage := opaque_fraction(state.image)
	if coverage < 0.1:
		fail("%s renders %.3f of its pixels" % [name, coverage])
		return {}
	var screen := root.get_texture().get_image()
	var centre := screen.get_pixelv(Vector2i(rect.get_center()))
	print("FIXTURE PORTRAIT %s appearance=%s rect=%s coverage=%.3f centre=%s camera=%s" % [name, state.appearance, rect, coverage, centre, state.camera_position])
	return state

## The rare star centred on the TargetFrame portrait's bottom edge (TargetFrame.xml:281-284).
func check_star(portrait: Dictionary) -> bool:
	var star := control("UnitFramesUI", "TargetBossIcon")
	if star == null or not star.is_visible_in_tree():
		fail("Timber shows no rare star")
		return false
	var rect: Rect2 = portrait.rect
	var bottom := Vector2(rect.get_center().x, rect.end.y)
	if not star.get_global_rect().get_center().is_equal_approx(bottom):
		fail("Rare star at %s, portrait bottom %s" % [star.get_global_rect().get_center(), bottom])
		return false
	print("FIXTURE STAR centre=%s portrait_bottom=%s" % [star.get_global_rect().get_center(), bottom])
	return true

## Attack Timber so it targets the player: the target-of-target frame shows clear of the
## whole TargetFrame.
func check_target_of_target() -> bool:
	var tot := control("UnitFramesUI", "TargetOfTargetFrame")
	if not await track_until(func(): return tot.is_visible_in_tree(), MOUSE_BUTTON_RIGHT, "Timber targeting the player"):
		return false
	var target := control("UnitFramesUI", "TargetFrame").get_global_rect()
	if tot.get_global_rect().intersects(target):
		fail("TargetOfTargetFrame %s overlaps TargetFrame %s" % [tot.get_global_rect(), target])
		return false
	print("FIXTURE TOT rect=%s target_frame=%s" % [tot.get_global_rect(), target])
	return true

func teleport_beside_timber() -> bool:
	var output := []
	var args := ["teleport", character] + TIMBER_SPAWN.map(func(value): return str(value))
	var code := OS.execute(OS.get_environment("PORTRAIT_ADMIN"), args, output, true)
	print("FIXTURE TELEPORT ", args, " -> ", code, " ", output)
	if code != 0:
		fail("Teleport failed: %s" % [output])
		return false
	return true

func opaque_fraction(image: Image) -> float:
	var opaque := 0
	for y in range(image.get_height()):
		for x in range(image.get_width()):
			if image.get_pixel(x, y).a > 0.5:
				opaque += 1
	return float(opaque) / float(image.get_width() * image.get_height())

## Mean per-channel difference of two renders, compared at the first one's size.
func image_difference(first: Image, second: Image) -> float:
	var other := second.duplicate() as Image
	other.resize(first.get_width(), first.get_height())
	var total := 0.0
	for y in range(first.get_height()):
		for x in range(first.get_width()):
			var a := first.get_pixel(x, y)
			var b := other.get_pixel(x, y)
			total += absf(a.r - b.r) + absf(a.g - b.g) + absf(a.b - b.b) + absf(a.a - b.a)
	return total / float(4 * first.get_width() * first.get_height())

## Hover-click the creature where it is now with `button` until `done`, for 10 s.
func track_until(done: Callable, button: MouseButton, what: String) -> bool:
	var deadline := Time.get_ticks_msec() + 10000
	while Time.get_ticks_msec() < deadline:
		var point = unit_point()
		if point != null:
			await click_point(point, button)
		else:
			await process_frame
		if done.call():
			return true
	fail("Timed out waiting for %s: target=%s" % [what, client.target_state()])
	return false

## The creature's pick point on screen when the native ray selects it, else null.
func unit_point():
	var units = client.get_node_or_null("WorldUnits")
	if units == null:
		return null
	for unit in units.get_children():
		if str(unit.name) != creature:
			continue
		var area := unit.find_child("UnitPick", true, false) as Area3D
		if area == null:
			continue
		var world_point := (area.get_child(0) as Node3D).global_position
		if not camera().is_position_in_frustum(world_point):
			continue
		var point := camera().unproject_position(world_point)
		if UnitPicker.pick(camera(), point) == area.get_meta("unit_server_id"):
			return point
	return null

## The creature's pick shape centre on screen, selected by the native ray; turn until seen.
func find_unit() -> Dictionary:
	var deadline := Time.get_ticks_msec() + 60000
	var turned := 0
	while Time.get_ticks_msec() < deadline:
		await process_frame
		var point = unit_point()
		if point != null:
			return {"point": point}
		if turned < 60:
			push_key(KEY_RIGHT, true)
			await wait_frames(3)
			push_key(KEY_RIGHT, false)
			turned += 1
	var names := []
	var units = client.get_node_or_null("WorldUnits")
	if units != null:
		for unit in units.get_children():
			names.append(str(unit.name))
	fail("%s is not visible and unoccluded; units %s" % [creature, names])
	return {}

func control(host: String, name: String) -> Control:
	var ui := client.get_node_or_null(host)
	return ui.find_child(name, true, false) as Control if ui != null else null

func camera() -> Camera3D:
	return root.get_viewport().get_camera_3d()

func enter_world() -> bool:
	var deadline := Time.get_ticks_msec() + 60000
	var ui = null
	while Time.get_ticks_msec() < deadline:
		await process_frame
		var state: Dictionary = client.account_state()
		ui = client.get_node_or_null("CharacterSelectUI")
		if state.reply_received and state.screen == "CharacterSelect" and state.character_count >= 1 and ui != null:
			break
	if ui == null:
		fail("No character select: " + str(client.account_state()))
		return false
	await click(ui.find_child("CharCard_0", true, false))
	var selected = ui.find_child("CharSelectCharacterName", true, false)
	if selected.text != character:
		fail("Card 0 is %s, not %s" % [selected.text, character])
		return false
	await click(ui.find_child("EnterWorld", true, false))
	deadline = Time.get_ticks_msec() + 120000
	while Time.get_ticks_msec() < deadline:
		await process_frame
		var state: Dictionary = client.account_state()
		if state.screen == "InWorld" and state.local_player_position != null and state.local_server_position != null and state.terrain.pending_count == 0 and not state.terrain.parsed_tiles.is_empty():
			print("FIXTURE IN_WORLD at ", state.local_player_position)
			await wait_frames(60)
			return true
	fail("Timed out entering the world: " + str(client.account_state()))
	return false

func wait_until(predicate: Callable, timeout_ms: int, what: String) -> bool:
	var deadline := Time.get_ticks_msec() + timeout_ms
	while Time.get_ticks_msec() < deadline:
		await process_frame
		if predicate.call():
			return true
	fail("Timed out waiting for %s: target=%s" % [what, client.target_state()])
	return false

func push_key(code: Key, pressed: bool) -> void:
	var event := InputEventKey.new()
	event.keycode = code
	event.physical_keycode = code
	event.pressed = pressed
	root.push_input(event, true)

func move_mouse(point: Vector2) -> void:
	var motion := InputEventMouseMotion.new()
	motion.position = point
	motion.global_position = point
	root.push_input(motion, true)
	Input.warp_mouse(point)
	await wait_frames(3)

func click(target: Control) -> void:
	await click_point(target.get_global_rect().get_center(), MOUSE_BUTTON_LEFT)

func click_point(point: Vector2, button: MouseButton) -> void:
	await move_mouse(point)
	for pressed in [true, false]:
		var event := InputEventMouseButton.new()
		event.position = point
		event.global_position = point
		event.button_index = button
		event.pressed = pressed
		root.push_input(event, true)
		await process_frame
	await wait_frames(3)

func capture(file: String) -> void:
	await RenderingServer.frame_post_draw
	var image := root.get_texture().get_image()
	var error := image.save_png(shots + file)
	if error != OK:
		fail("Could not save " + file + ": " + str(error))

func wait_frames(count: int) -> void:
	for frame in range(count):
		await process_frame

func fail(message: String) -> void:
	push_error(message)
	quit(1)
