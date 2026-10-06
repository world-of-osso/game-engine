extends SceneTree

## Minimap and objective tracker in Northshire (docs/specs/minimap.md, quest-ui.md).
## Environment:
##   GODOT_TEST_SERVER   a private test server (never the shared :5000)
##   MMQ_ACCOUNT         account (password fbtest) of a level-1 Human warrior who has not
##                       taken "Beating Them Back!" (28766), placed within 10 yd of
##                       Marshal McBride (game-server-admin set-position <name> -8910.5 -137.5 81.2)
##   MMQ_SHOTS           screenshot directory
## Asserted: the minimap draws the local-CASC tile under the player (path from the
## player's position, composite centre = that tile's texel), the subzone text, the arrow
## rotation for the facing and its direction along forward movement, the McBride quest
## blip, wheel/button zoom; accepting 28766 puts its title and "0/6" objective line in
## the tracker at TOPRIGHT (0, -275) under the minimap, and both collapse buttons work.

const PASSWORD := "fbtest"
const QUEST_GIVER := "Marshal McBride"
const QUEST := 28766
const QUEST_TITLE := "Beating Them Back!"
const TILE_YARDS := 533.3333
const WORLD_WAIT_MS := 180000
const WAIT_MS := 30000
const MOVE_FRAMES := 45

var client: Node
var shots := "/tmp/claude/minimapquest/"

func _initialize() -> void:
	Engine.max_fps = 60
	call_deferred("run_test")

func run_test() -> void:
	root.size = Vector2i(1280, 720)
	var server := OS.get_environment("GODOT_TEST_SERVER")
	if server == "" or server.ends_with(":5000"):
		fail("GODOT_TEST_SERVER must name a private server, not :5000")
		return
	if OS.get_environment("MMQ_SHOTS") != "":
		shots = OS.get_environment("MMQ_SHOTS").trim_suffix("/") + "/"
	DirAccess.make_dir_recursive_absolute(shots)
	client = load("res://scenes/client.tscn").instantiate()
	root.add_child(client)
	var error = client.connect_account(server, OS.get_environment("MMQ_ACCOUNT"), PASSWORD, false)
	if error != "":
		fail("Fixture connection: " + error)
		return
	if not await enter_world():
		return
	if not await check_minimap_tile():
		return
	if not await check_quest_blip():
		return
	await capture("01-minimap-northshire.png")
	# McBride accepts only within 10 yd, so accept before walking.
	if not await check_tracker():
		return
	if not await check_arrow():
		return
	if not await check_zoom():
		return
	if not await check_collapse():
		return
	print("FIXTURE MINIMAP_QUEST_DONE")
	client.free()
	quit(0)

func enter_world() -> bool:
	# Character select can take a minute under a loaded machine.
	var deadline := Time.get_ticks_msec() + 60000
	var replied := false
	while Time.get_ticks_msec() < deadline and not replied:
		await process_frame
		var state: Dictionary = client.account_state()
		replied = state.reply_received
		if replied and (state.screen != "CharacterSelect" or state.character_count < 1):
			fail("Fixture needs an authenticated character: " + str(state))
			return false
	if not replied:
		fail("Timed out waiting for the login reply: " + str(client.account_state()))
		return false
	var ui = client.get_node_or_null("CharacterSelectUI")
	var enter = ui.find_child("EnterWorld", true, false) if ui != null else null
	if not enter is Button:
		fail("Enter World button missing")
		return false
	await click_point(enter.get_global_rect().get_center())
	deadline = Time.get_ticks_msec() + WORLD_WAIT_MS
	while Time.get_ticks_msec() < deadline:
		await process_frame
		var state: Dictionary = client.account_state()
		if state.screen == "InWorld" and state.local_player_position != null and not state.terrain.parsed_tiles.is_empty():
			print("FIXTURE IN_WORLD ", state.selected_character_name, " at ", state.local_player_position)
			return true
	fail("Timed out entering the world: " + str(client.account_state()))
	return false

func player_position() -> Vector3:
	return client.account_state().local_player_position

## The minimap tile key for an engine position, independently of the client.
func expected_tile(position: Vector3) -> Vector2i:
	return Vector2i(int(floor(32.0 + position.z / TILE_YARDS)), int(floor(32.0 - position.x / TILE_YARDS)))

func check_minimap_tile() -> bool:
	var state := await wait_minimap(func(s): return s.has("tile_fdid") and s.get("zone_text", "") != "")
	if state.is_empty():
		return false
	var tile := expected_tile(player_position())
	var path := "world/minimaps/azeroth/map%02d_%02d.blp" % [tile.x, tile.y]
	print("FIXTURE MINIMAP tile ", state.tile_path, " fdid ", state.tile_fdid, " uv ", state.tile_uv, " zone '", state.zone_text, "' centre ", state.center_pixel, " texel ", state.tile_texel, " tiles ", state.tiles_loaded, "/", state.tiles)
	if state.map != "azeroth" or state.tile != tile or state.tile_path != path:
		fail("Minimap tile %s is not the player's %s" % [state.tile_path, path])
		return false
	for channel in 4:
		if abs(int(state.center_pixel[channel]) - int(state.tile_texel[channel])) > 1:
			fail("Composite centre %s is not the tile texel %s" % [state.center_pixel, state.tile_texel])
			return false
	if state.center_pixel[3] != 255:
		fail("Composite centre is masked out: " + str(state.center_pixel))
		return false
	if not await check_rendered_map():
		return false
	if not String(state.zone_text).begins_with("Northshire"):
		fail("Zone text '%s' is not a Northshire subzone" % state.zone_text)
		return false
	return true

## The drawn MinimapDisplay shows the composite: screen pixels at four points between
## the arrow and the rim match the composite there.
func check_rendered_map() -> bool:
	await RenderingServer.frame_post_draw
	var image := root.get_texture().get_image()
	var display := client.get_node("MinimapUI").find_child("MinimapDisplay", true, false) as Control
	var rect := display.get_global_rect()
	# Minimap 198×198 in MinimapCluster TOPRIGHT (0, 0): container TOP (+10, −30), map centred.
	var scale := float(root.size.x) / float(client.objective_tracker_state().screen_width)
	var expected := Rect2(Vector2(root.size.x - 217.0 * scale, 44.0 * scale), Vector2(198.0, 198.0) * scale)
	if rect.position.distance_to(expected.position) > 1.0 or rect.size.distance_to(expected.size) > 1.0:
		fail("Minimap drawn at %s, expected %s" % [rect, expected])
		return false
	for uv in [Vector2(0.3, 0.5), Vector2(0.7, 0.5), Vector2(0.5, 0.3), Vector2(0.5, 0.72)]:
		var point: Vector2 = rect.position + rect.size * uv
		var drawn := image.get_pixelv(Vector2i(point))
		var composite: PackedInt32Array = client.minimap_composite_pixel(uv.x, uv.y)
		var want := Color8(composite[0], composite[1], composite[2])
		print("FIXTURE RENDERED uv ", uv, " screen ", drawn, " composite ", want)
		if abs(drawn.r - want.r) > 0.12 or abs(drawn.g - want.g) > 0.12 or abs(drawn.b - want.b) > 0.12:
			fail("Minimap at %s draws %s, composite %s" % [uv, drawn, want])
			return false
	return true

func arrow_part() -> Control:
	var arrow = client.get_node("MinimapUI").find_child("MinimapPlayerArrow", true, false)
	return arrow.find_child("Part0", true, false) as Control if arrow != null else null

func check_arrow() -> bool:
	var state: Dictionary = client.minimap_state()
	var expected := fposmod(state.facing_yaw - PI / 2.0, TAU)
	if abs(state.arrow_rotation - expected) > 0.001:
		fail("Arrow rotation %f for facing %f, expected %f" % [state.arrow_rotation, state.facing_yaw, expected])
		return false
	var part := arrow_part()
	if part == null:
		fail("No drawn minimap arrow")
		return false
	# Godot controls turn clockwise; the arrow art points up at rotation 0.
	var pointing := Vector2(sin(part.rotation), -cos(part.rotation))
	var before := player_position()
	push_key(KEY_W, true)
	for frame in range(MOVE_FRAMES):
		await process_frame
	push_key(KEY_W, false)
	for frame in range(10):
		await process_frame
	var moved := player_position() - before
	# Minimap screen: right is engine +z (east), down is engine -x (south).
	var on_map := Vector2(moved.z, -moved.x)
	print("FIXTURE ARROW rotation ", state.arrow_rotation, " yaw ", state.facing_yaw, " pointing ", pointing, " moved ", on_map)
	if on_map.length() < 1.0:
		fail("Walking forward did not move the player: " + str(moved))
		return false
	if on_map.normalized().dot(pointing) < 0.95:
		fail("Arrow points %s but the player moved %s on the minimap" % [pointing, on_map.normalized()])
		return false
	var after: Dictionary = client.minimap_state()
	var centre: Vector2 = after.center
	var position := player_position()
	if centre.distance_to(Vector2(position.x, position.z)) > 2.0:
		fail("Minimap did not follow the player: centre %s, player %s" % [centre, position])
		return false
	return true

func check_quest_blip() -> bool:
	var state := await wait_minimap(func(s): return s.get("blips", 0) >= 1)
	if state.is_empty():
		return false
	print("FIXTURE BLIPS ", state.blips)
	return true

func check_zoom() -> bool:
	var ui: Node = client.get_node("MinimapUI")
	var display := ui.find_child("MinimapDisplay", true, false) as Control
	var centre := display.get_global_rect().get_center()
	await hover_point(centre)
	await frames(3)
	if not client.minimap_state().hovered or ui.find_child("MinimapZoomOut", true, false) == null:
		fail("Hovering the map did not show the zoom buttons")
		return false
	await wheel(centre, MOUSE_BUTTON_WHEEL_UP)
	var zoomed: Dictionary = client.minimap_state()
	if zoomed.zoom != 1 or abs(zoomed.diameter - 400.0) > 0.01:
		fail("Wheel up did not zoom in: " + str(zoomed))
		return false
	var zoom_out := ui.find_child("MinimapZoomOut", true, false) as Control
	await click_point(zoom_out.get_global_rect().get_center())
	await frames(3)
	if client.minimap_state().zoom != 0:
		fail("ZoomOut did not zoom out: " + str(client.minimap_state()))
		return false
	await hover_point(Vector2(640, 400))
	await frames(3)
	if client.minimap_state().hovered:
		fail("Leaving the map kept the zoom buttons")
		return false
	return true

func tracked(state: Dictionary) -> Dictionary:
	for quest in state.get("quests", []):
		if quest.quest_id == QUEST:
			return quest
	return {}

func check_tracker() -> bool:
	var deadline := Time.get_ticks_msec() + WAIT_MS
	var error := "no attempt"
	while Time.get_ticks_msec() < deadline:
		error = client.accept_quest_from(QUEST_GIVER, QUEST)
		if error == "":
			break
		await frames(10)
	if error != "":
		fail("Accepting %d: %s" % [QUEST, error])
		return false
	var state := {}
	deadline = Time.get_ticks_msec() + WAIT_MS
	while Time.get_ticks_msec() < deadline:
		await process_frame
		state = client.objective_tracker_state()
		if not tracked(state).is_empty():
			break
	var quest := tracked(state)
	print("FIXTURE TRACKER ", state)
	if quest.is_empty() or quest.title != QUEST_TITLE:
		fail("Tracker does not show %s: %s" % [QUEST_TITLE, state])
		return false
	if quest.lines.size() != 1 or not String(quest.lines[0]).begins_with("0/6 "):
		fail("Tracker objective lines %s, expected one 0/6 line" % [quest.lines])
		return false
	await capture("02-tracker-quest.png")
	if not await check_text_visible("QuestBlock%dHeaderText" % QUEST, Color(0.75, 0.61, 0.0)):
		return false
	if not await check_text_visible("QuestBlock%dLine0Text" % QUEST, Color(0.8, 0.8, 0.8)):
		return false
	var rect: Rect2 = state.rect
	if not state.visible or abs(rect.end.x - state.screen_width) > 0.5 or abs(rect.position.y - 275.0) > 0.5:
		fail("Tracker at %s, expected TOPRIGHT (0, -275) of %s" % [rect, state.screen_width])
		return false
	return true

## A tracker label is drawn legibly: visible in the tree, opaque, its authored colour
## (`OBJECTIVE_TRACKER_COLOR`), a shadow at (1, 1), a non-zero rect, and on screen at
## least 20 more text-coloured and 15 more shadow pixels than the grass strip below.
func check_text_visible(name: String, want: Color) -> bool:
	var label = client.get_node("ObjectiveTrackerUI").find_child(name, true, false)
	if not label is Label:
		fail("Tracker label %s missing" % name)
		return false
	var color: Color = label.get_theme_color("font_color")
	var shadow: Color = label.get_theme_color("font_shadow_color")
	var rect: Rect2 = label.get_global_rect()
	var alpha: float = label.modulate.a * label.self_modulate.a
	if not label.is_visible_in_tree() or alpha < 0.99 or color.a < 0.99 or rect.size.x < 1.0 or rect.size.y < 1.0:
		fail("%s hidden or empty: visible %s alpha %f colour %s rect %s" % [name, label.is_visible_in_tree(), alpha, color, rect])
		return false
	if abs(color.r - want.r) > 0.01 or abs(color.g - want.g) > 0.01 or abs(color.b - want.b) > 0.01:
		fail("%s colour %s, expected %s" % [name, color, want])
		return false
	if shadow.a < 0.99 or label.get_theme_constant("shadow_offset_x") != 1 or label.get_theme_constant("shadow_offset_y") != 1:
		fail("%s has no (1, 1) black shadow: %s" % [name, shadow])
		return false
	await RenderingServer.frame_post_draw
	var image := root.get_texture().get_image()
	# The same-sized strip two text heights lower (grass under the tracker) is the
	# background reference: glyph and shadow counts must exceed it.
	var text := count_text_pixels(image, rect, want)
	var background := count_text_pixels(image, Rect2(rect.position + Vector2(0, rect.size.y * 2.0), rect.size), want)
	var glyph: int = text.x - background.x
	var dark: int = text.y - background.y
	print("FIXTURE TEXT ", name, " '", label.text, "' rect ", rect, " glyph px ", text.x, " (background ", background.x, ") shadow px ", text.y, " (background ", background.y, ")")
	if glyph < 20 or dark < 15:
		fail("%s not legible on screen: %d glyph and %d shadow pixels over the background" % [name, glyph, dark])
		return false
	return true

## Pixels within 0.2 of `want` (8 px glyphs antialias) and dark
## shadow pixels (r+g+b < 0.45) in `rect`.
func count_text_pixels(image: Image, rect: Rect2, want: Color) -> Vector2i:
	var counts := Vector2i.ZERO
	for y in range(int(rect.position.y), int(rect.end.y) + 2):
		for x in range(int(rect.position.x), int(rect.end.x) + 2):
			var pixel := image.get_pixel(clampi(x, 0, image.get_width() - 1), clampi(y, 0, image.get_height() - 1))
			if abs(pixel.r - want.r) < 0.2 and abs(pixel.g - want.g) < 0.2 and abs(pixel.b - want.b) < 0.2:
				counts.x += 1
			elif pixel.r + pixel.g + pixel.b < 0.45:
				counts.y += 1
	return counts

func check_collapse() -> bool:
	var ui: Node = client.get_node("ObjectiveTrackerUI")
	await click_named(ui, "QuestObjectiveTrackerHeaderMinimizeButton")
	var state: Dictionary = client.objective_tracker_state()
	if not state.quests_collapsed or not tracked(state).is_empty():
		fail("Quests header collapse left the block: " + str(state))
		return false
	await capture("03-tracker-quests-collapsed.png")
	await click_named(ui, "QuestObjectiveTrackerHeaderMinimizeButton")
	await click_named(ui, "ObjectiveTrackerFrameHeaderMinimizeButton")
	state = client.objective_tracker_state()
	if not state.collapsed or not tracked(state).is_empty():
		fail("All Objectives collapse left the block: " + str(state))
		return false
	await click_named(ui, "ObjectiveTrackerFrameHeaderMinimizeButton")
	if tracked(client.objective_tracker_state()).is_empty():
		fail("Expanding the tracker did not restore the quest")
		return false
	return true

func wait_minimap(ready: Callable) -> Dictionary:
	var deadline := Time.get_ticks_msec() + WAIT_MS
	var state: Dictionary = {}
	while Time.get_ticks_msec() < deadline:
		await process_frame
		state = client.minimap_state()
		if state.get("open", false) and ready.call(state):
			return state
	fail("Timed out waiting for the minimap: " + str(state))
	return {}

func click_named(ui: Node, name: String) -> void:
	var control = ui.find_child(name, true, false)
	if not control is Control:
		fail("Missing control " + name)
		return
	await click_point(control.get_global_rect().get_center())
	await frames(3)

func frames(count: int) -> void:
	for frame in range(count):
		await process_frame

func capture(file: String) -> void:
	await RenderingServer.frame_post_draw
	var image := root.get_texture().get_image()
	var error := image.save_png(shots + file)
	if error != OK:
		fail("Could not save " + file + ": " + str(error))

func push_key(code: Key, pressed: bool) -> void:
	var event := InputEventKey.new()
	event.keycode = code
	event.physical_keycode = code
	event.pressed = pressed
	root.push_input(event, true)

func hover_point(point: Vector2) -> void:
	var motion := InputEventMouseMotion.new()
	motion.position = point
	motion.global_position = point
	root.push_input(motion, true)
	await process_frame

func wheel(point: Vector2, button: MouseButton) -> void:
	for pressed in [true, false]:
		var event := InputEventMouseButton.new()
		event.position = point
		event.global_position = point
		event.button_index = button
		event.pressed = pressed
		root.push_input(event, true)
		await process_frame
	await frames(3)

func click_point(point: Vector2) -> void:
	await hover_point(point)
	for pressed in [true, false]:
		var event := InputEventMouseButton.new()
		event.position = point
		event.global_position = point
		event.button_index = MOUSE_BUTTON_LEFT
		event.pressed = pressed
		root.push_input(event, true)
		await process_frame
	await process_frame

func fail(message: String) -> void:
	push_error(message)
	quit(1)
