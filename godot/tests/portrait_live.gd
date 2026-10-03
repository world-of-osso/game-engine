extends SceneTree

## TargetFrame and PlayerFrame portraits against a private server.
## Environment:
##   GODOT_TEST_SERVER        server address (a private test server)
##   PORTRAIT_ACCOUNT / PORTRAIT_CHARACTER  account (password fbtest) and a character
##   PORTRAIT_ADMIN           game-server-admin binary for that server (its
##                            GAME_SERVER_ADMIN_SOCKET in the environment)
##   PORTRAIT_SHOTS           screenshot directory
## Teleports beside Marshal McBride (world.db creature 197) and targets him, then beside
## Timber (1132, rank 4 rare) and targets it: each target's portrait is its own model, masked round at the Retail anchor; the
## rare star sits on the portrait's bottom and the target of target clears the frame.

const PASSWORD := "fbtest"
## world.db content_creature spawns (map 0), a few yards off: McBride inside Northshire
## Abbey, Timber at Iceflow Lake.
const MCBRIDE_SPAWN := [0, -8920.0, -137.5, 81.0]
## 25 yards off Timber's spawn point, outside its aggro; then on it, where it aggroes the
## player (it wanders within 8 yards and outlevels an ungeared player).
const TIMBER_VIEW := [0, -5151.4, -24.0, 386.5]
const TIMBER_SPAWN := [0, -5176.4, -24.0, 386.5]
## The warrior's Auto Attack and the action bar keys.
const ATTACK := 88163
const BAR_KEYS := [KEY_1, KEY_2, KEY_3, KEY_4, KEY_5, KEY_6, KEY_7, KEY_8, KEY_9, KEY_0, KEY_MINUS, KEY_EQUAL]
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
	if not await teleport(MCBRIDE_SPAWN):
		return
	var mcbride = await target_portrait()
	if mcbride.is_empty():
		return
	await capture("01-mcbride.png")
	creature = "Timber"
	if not await teleport(TIMBER_VIEW):
		return
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

## Target `creature` (Tab, nearest first) and wait for its portrait; checked like every
## portrait.
func target_portrait() -> Dictionary:
	for attempt in range(20):
		if client.target_state().target_name == creature:
			break
		await press(KEY_TAB)
		await wait_frames(6)
	if client.target_state().target_name != creature:
		fail("Tab never targeted %s: %s" % [creature, client.target_state()])
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

## Step onto Timber and attack it so it targets the player: the target-of-target frame
## shows clear of the
## whole TargetFrame.
func check_target_of_target() -> bool:
	if not await teleport(TIMBER_SPAWN, 10):
		return false
	if not client.unit_alive(client.account_state().local_player_id):
		fail("%s is dead: a dead player is nobody's target" % character)
		return false
	await press(BAR_KEYS[client.spells_state().bar.find(ATTACK)])
	var tot := control("UnitFramesUI", "TargetOfTargetFrame")
	if not await wait_until(func(): return tot.is_visible_in_tree(), 15000, "Timber targeting the player"):
		return false
	await wait_frames(5)
	var target := control("UnitFramesUI", "TargetFrame").get_global_rect()
	if tot.get_global_rect().intersects(target):
		fail("TargetOfTargetFrame %s overlaps TargetFrame %s" % [tot.get_global_rect(), target])
		return false
	print("FIXTURE TOT rect=%s target_frame=%s" % [tot.get_global_rect(), target])
	return true

## Teleport the character to `spawn` (map, x, y, z) and wait for the terrain there.
func teleport(spawn: Array, settle_frames := 60) -> bool:
	var output := []
	var args := ["teleport", character] + spawn.map(func(value): return str(value))
	var code := OS.execute(OS.get_environment("PORTRAIT_ADMIN"), args, output, true)
	print("FIXTURE TELEPORT ", args, " -> ", code, " ", output)
	if code != 0:
		fail("Teleport failed: %s" % [output])
		return false
	var at := Vector2(spawn[1], spawn[2])
	var arrived := func():
		var state: Dictionary = client.account_state()
		# Godot (x, height, -y) of the WoW position.
		var position = state.local_player_position
		return position != null and Vector2(position.x, -position.z).distance_to(at) < 5.0 and state.terrain.pending_count == 0
	if not await wait_until(arrived, 60000, "arrival at %s" % [spawn]):
		return false
	await wait_frames(settle_frames)
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

func control(host: String, name: String) -> Control:
	var ui := client.get_node_or_null(host)
	return ui.find_child(name, true, false) as Control if ui != null else null

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
	deadline = Time.get_ticks_msec() + 300000
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

func press(code: Key) -> void:
	for pressed in [true, false]:
		var event := InputEventKey.new()
		event.keycode = code
		event.physical_keycode = code
		event.pressed = pressed
		root.push_input(event, true)
		await wait_frames(2)

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
	root.get_texture().get_image().save_png(shots + "fail.png")
	quit(1)
