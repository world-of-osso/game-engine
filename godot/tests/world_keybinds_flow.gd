extends "res://tests/world_ui_ownership_flow.gd"

# native_input_fixture keybinds: physical key presses of the Retail default bag,
# targeting and minimap zoom bindings, the world map tiled at three zones, and QuestFrame /
# QuestLogFrame raise-on-click over the MerchantFrame. Every check fails the run.
# KEYBINDS_SHOTS=<dir> saves screenshots (needs GODOT_TEST_VISUAL=1).
const KB_BAGS := [0, 1, 2, 4]
const KB_MAP := "WorldMapUI"

var kb_client: Node
var kb_shots := ""

func run_test() -> void:
	root.size = Vector2i(1920, 1080)
	var endpoint := OS.get_environment("GODOT_TEST_SERVER")
	if not endpoint.begins_with("127.0.0.1:") or endpoint.ends_with(":0"):
		fail("keybinds requires owned UDP endpoint")
		return
	kb_shots = OS.get_environment("KEYBINDS_SHOTS")
	kb_client = load("res://scenes/client.tscn").instantiate()
	root.add_child(kb_client)
	if not await wait_screen(kb_client, "Loading", 180000):
		return
	print("FIXTURE KEYBINDS_LOADING")
	if not await wait_world(kb_client):
		return
	print("FIXTURE KEYBINDS_READY")
	if not await uo_wait(func(): return kb_client.merchant_state().bags.size() == 4 and kb_unit_id("Fixture Wolf") != -1, 15000):
		fail("keybinds snapshot/wolf not applied: " + str(kb_client.merchant_state()))
		return
	if not await kb_bag_cases():
		return
	if not await kb_target_cases():
		return
	if not await kb_minimap_zoom_cases():
		return
	if not await kb_world_map_cases():
		return
	if not await kb_quest_raise_cases():
		return
	print("FIXTURE KEYBINDS_DONE")
	while true:
		await process_frame

# ---------- helpers ----------

func kb_tap(code: Key, shift: bool = false) -> void:
	if shift:
		kb_key(KEY_SHIFT, true, true)
		await process_frame
	kb_key(code, true, shift)
	await process_frame
	kb_key(code, false, shift)
	await process_frame
	if shift:
		kb_key(KEY_SHIFT, false, false)
	for frame in range(4):
		await process_frame

func kb_key(code: Key, pressed: bool, shift: bool) -> void:
	var event := InputEventKey.new()
	event.keycode = code
	event.physical_keycode = code
	event.pressed = pressed
	event.shift_pressed = shift
	root.push_input(event, true)

func kb_open_bags() -> Array:
	var open := []
	for bag in KB_BAGS + [3]:
		if uo_visible(uo_ctl(kb_client, UO_BAGS, "ContainerFrame%d" % bag)):
			open.append(bag)
	return open

func kb_expect_bags(label: String, expected: Array) -> bool:
	var open := kb_open_bags()
	print("KEYBINDS BAGS ", label, " open=", open)
	if open != expected:
		fail("%s: open bags %s, expected %s" % [label, open, expected])
		return false
	return true

func kb_capture(file: String) -> void:
	if kb_shots.is_empty():
		return
	await RenderingServer.frame_post_draw
	var image := root.get_texture().get_image()
	if image == null or image.save_png(kb_shots.path_join(file)) != OK:
		fail("Could not save screenshot " + file)
		return
	print("KEYBINDS SCREENSHOT ", kb_shots.path_join(file))

func kb_unit_id(unit_name: String) -> int:
	var units := kb_client.get_node_or_null("WorldUnits")
	var unit := units.get_node_or_null(unit_name) if units != null else null
	var area := unit.find_child("UnitPick", true, false) if unit != null else null
	return area.get_meta("unit_server_id") if area != null else -1

func kb_target() -> int:
	var target = kb_client.target_state().target
	return target if target != null else -1

func kb_expect_target(label: String, unit_name: String) -> bool:
	var expected := kb_unit_id(unit_name)
	print("KEYBINDS TARGET ", label, " target=", kb_target(), " ", unit_name, "=", expected)
	if expected == -1 or kb_target() != expected:
		fail("%s: target %s, expected %s (%s)" % [label, kb_target(), unit_name, expected])
		return false
	return true

# ---------- bags: OPENALLBAGS B, TOGGLEBACKPACK Shift-B, TOGGLEBAG1-4 F8-F11 ----------

func kb_bag_cases() -> bool:
	await kb_tap(KEY_B)
	if not kb_expect_bags("B", KB_BAGS):
		return false
	await kb_capture("bags-B-all-open.png")
	await kb_tap(KEY_B)
	if not kb_expect_bags("B again", []):
		return false
	await kb_tap(KEY_B, true)
	if not kb_expect_bags("Shift-B", [0]):
		return false
	await kb_capture("bags-shift-B-backpack.png")
	await kb_tap(KEY_B, true)
	if not kb_expect_bags("Shift-B again", []):
		return false
	# TOGGLEBAGn runs ToggleBag(5 - n); bag 3 is not held, so F9 does nothing.
	await kb_tap(KEY_F8)
	if not kb_expect_bags("F8", [4]):
		return false
	await kb_tap(KEY_F9)
	if not kb_expect_bags("F9 (bag 3 not held)", [4]):
		return false
	await kb_tap(KEY_F10)
	await kb_tap(KEY_F11)
	if not kb_expect_bags("F10 F11", [1, 2, 4]):
		return false
	await kb_capture("bags-F8-F10-F11.png")
	await kb_tap(KEY_F11)
	if not kb_expect_bags("F11 again", [2, 4]):
		return false
	# Half open: B opens every held bag; a second B closes them all.
	await kb_tap(KEY_B)
	if not kb_expect_bags("B with some open", KB_BAGS):
		return false
	await kb_tap(KEY_B)
	return kb_expect_bags("B closes all", [])

# ---------- targeting: TARGETNEARESTENEMY Tab, TARGETPREVIOUSENEMY Shift-Tab, ASSISTTARGET F ----------

func kb_target_cases() -> bool:
	if kb_target() != -1:
		fail("keybinds starts with a target: " + str(kb_client.target_state()))
		return false
	await kb_tap(KEY_TAB)
	if not kb_expect_target("Tab", "Fixture Vendor"):
		return false
	await kb_tap(KEY_TAB)
	if not kb_expect_target("Tab again", "Fixture Wolf"):
		return false
	await kb_tap(KEY_TAB, true)
	if not kb_expect_target("Shift-Tab", "Fixture Vendor"):
		return false
	await kb_tap(KEY_ESCAPE)
	if kb_target() != -1:
		fail("Escape did not clear the target: " + str(kb_client.target_state()))
		return false
	await kb_tap(KEY_TAB, true)
	if not kb_expect_target("Shift-Tab from none", "Fixture Wolf"):
		return false
	await kb_tap(KEY_TAB, true)
	if not kb_expect_target("Shift-Tab to vendor", "Fixture Vendor"):
		return false
	# The peer gave the vendor the remote player as its own target.
	await kb_tap(KEY_F)
	if not kb_expect_target("F assist", "Remote Fixture"):
		return false
	await uo_settle(300)
	await kb_capture("assist-F-remote-target.png")
	await kb_tap(KEY_F)
	if not kb_expect_target("F with untargeted target", "Remote Fixture"):
		return false
	await kb_tap(KEY_ESCAPE)
	return kb_target() == -1 or fail_false("Escape did not clear the assisted target")

func fail_false(message: String) -> bool:
	fail(message)
	return false

# ---------- minimap: MINIMAPZOOMIN Num Pad +, MINIMAPZOOMOUT Num Pad - ----------

func kb_expect_zoom(label: String, expected: int) -> bool:
	var zoom: int = kb_client.minimap_state().zoom
	print("KEYBINDS MINIMAP ", label, " zoom=", zoom)
	return zoom == expected or fail_false("%s: minimap zoom %s, expected %s" % [label, zoom, expected])

func kb_minimap_zoom_cases() -> bool:
	if not kb_client.minimap_state().open:
		fail("minimap not shown: " + str(kb_client.minimap_state()))
		return false
	if not kb_expect_zoom("start", 0):
		return false
	await kb_tap(KEY_KP_ADD)
	if not kb_expect_zoom("Num Pad +", 1):
		return false
	await kb_tap(KEY_KP_ADD)
	if not kb_expect_zoom("Num Pad + again", 2):
		return false
	await kb_capture("minimap-numpad-plus-zoom2.png")
	await kb_tap(KEY_KP_SUBTRACT)
	if not kb_expect_zoom("Num Pad -", 1):
		return false
	await kb_tap(KEY_KP_SUBTRACT)
	await kb_tap(KEY_KP_SUBTRACT)
	return kb_expect_zoom("Num Pad - at the widest", 0)

# ---------- world map: every art tile of three zones is drawn ----------

func kb_map_state() -> Dictionary:
	return kb_client.world_map_state()

func kb_canvas_rect() -> Rect2:
	var canvas := uo_ctl(kb_client, KB_MAP, "WorldMapCanvas")
	return canvas.get_global_rect() if canvas != null else Rect2()

func kb_canvas_point(uv: Vector2) -> Vector2:
	var rect := kb_canvas_rect()
	return rect.position + rect.size * uv

func kb_canvas_click(button: MouseButton, uv: Vector2) -> void:
	var point := kb_canvas_point(uv)
	mc_motion(point, false)
	await process_frame
	for down in [true, false]:
		var event := InputEventMouseButton.new()
		event.position = point
		event.global_position = point
		event.button_index = button
		event.pressed = down
		root.push_input(event, true)
		await process_frame
	await uo_settle(400)

func kb_expect_tiled(label: String, name: String) -> bool:
	# Tile textures extract from local CASC on first draw; let them settle.
	await uo_settle(1500)
	var state := kb_map_state()
	print("KEYBINDS MAP ", label, " name=", state.map_name, " id=", state.map_id, " kind=", state.map_kind, " tiles=", state.tile_count, " missing=", state.missing_tiles)
	if state.map_name != name:
		fail("%s: map %s, expected %s" % [label, state.map_name, name])
		return false
	if state.tile_count == 0 or state.missing_tiles != 0:
		fail("%s: %s art tiles drawn, %s missing" % [label, state.tile_count, state.missing_tiles])
		return false
	return true

func kb_find_highlight(name: String) -> Vector2:
	for y in range(1, 24):
		for x in range(1, 30):
			var uv := Vector2(x / 30.0, y / 24.0)
			mc_motion(kb_canvas_point(uv), false)
			await process_frame
			await process_frame
			if kb_map_state().highlight == name:
				return uv
	return Vector2.INF

func kb_world_map_cases() -> bool:
	await kb_tap(KEY_M)
	if not await uo_wait(func(): return uo_ctl(kb_client, KB_MAP, "WorldMapCanvas") != null, 20000):
		fail("M did not open the world map")
		return false
	if not await kb_expect_tiled("M (player zone)", "Northshire"):
		return false
	await kb_capture("worldmap-northshire.png")
	await kb_canvas_click(MOUSE_BUTTON_RIGHT, Vector2(0.5, 0.5))
	if not await kb_expect_tiled("right-click", "Elwynn Forest"):
		return false
	await kb_capture("worldmap-elwynn-forest.png")
	await kb_canvas_click(MOUSE_BUTTON_RIGHT, Vector2(0.5, 0.5))
	var continent := kb_map_state()
	print("KEYBINDS MAP continent name=", continent.map_name, " kind=", continent.map_kind)
	var westfall := await kb_find_highlight("Westfall")
	if westfall == Vector2.INF:
		fail("Westfall not found on " + str(continent.map_name))
		return false
	await kb_canvas_click(MOUSE_BUTTON_LEFT, westfall)
	if not await kb_expect_tiled("click Westfall", "Westfall"):
		return false
	await kb_capture("worldmap-westfall.png")
	await kb_tap(KEY_M)
	return await uo_wait(func(): return kb_client.get_node_or_null(KB_MAP) == null, 5000) or fail_false("M did not close the world map")

# ---------- QuestFrame / QuestLogFrame raise-on-click (toplevel) ----------

func kb_rect(ui_name: String, control_name: String) -> Rect2:
	var control := uo_ctl(kb_client, ui_name, control_name)
	return control.get_global_rect() if uo_visible(control) else Rect2()

# A point of `rect` outside `other` that the pointer reaches inside `ui_name`, not on
# a button or slot that would act on the click.
func kb_only_point(ui_name: String, rect: Rect2, other: Rect2) -> Vector2:
	for y in [0.3, 0.5, 0.7, 0.15, 0.85]:
		for step in range(1, 50):
			var point := rect.position + rect.size * Vector2(step / 50.0, y)
			if other.has_point(point):
				continue
			var path := uo_hover_path(point)
			if path.contains("/" + ui_name + "/") and not path.contains("Button") and not path.contains("Slot") and not path.contains("Item"):
				return point
	return Vector2.INF

func kb_topmost_is(point: Vector2, ui_name: String) -> bool:
	return uo_hover_path(point).contains("/" + ui_name + "/")

# Clicking `ui_name` where `cover` does not reach raises it over `cover` at `covered`.
func kb_raise_case(case_name: String, ui_name: String, rect: Rect2, cover: Rect2, covered: Vector2) -> bool:
	var cover_point := kb_only_point(UO_MERCHANT, cover, rect)
	var window_point := kb_only_point(ui_name, rect, cover)
	if cover_point == Vector2.INF or window_point == Vector2.INF:
		fail("%s geometry: no uncovered click points (merchant %s, %s %s)" % [case_name, cover, ui_name, rect])
		return false
	await uo_click(cover_point)
	await uo_settle(300)
	var before := uo_hover_path(covered)
	await uo_click(window_point)
	await uo_settle(300)
	var after := uo_hover_path(covered)
	print("KEYBINDS RAISE ", case_name, " covered=", covered, " before=", before, " clicked=", window_point, " after=", after)
	if not before.contains("/" + UO_MERCHANT + "/") or not after.contains("/" + ui_name + "/"):
		fail("%s: clicking %s did not raise it over the MerchantFrame (before %s, after %s)" % [case_name, ui_name, before, after])
		return false
	return true

func kb_quest_raise_cases() -> bool:
	var vendor := await find_vendor(kb_client)
	if vendor.is_empty() or not await open_vendor(kb_client, vendor):
		return false
	await uo_settle(300)
	# Shift the movable MerchantFrame right so it half covers both quest panels.
	var merchant := uo_ctl(kb_client, UO_MERCHANT, "MerchantFrame")
	var canvas := uo_ctl(kb_client, UO_MERCHANT, "RegistryCanvas")
	var scale := canvas.get_global_transform().get_scale().x
	var m := merchant.get_global_rect()
	var title := m.position + Vector2(m.size.x * 0.4, 10.0 * scale)
	await drag_title(title, title + Vector2(200.0 * scale, 0.0))
	await uo_settle(300)
	print("FIXTURE KEYBINDS_QUEST_OPEN")
	if not await uo_wait(func(): return kb_client.quest_state().frame_open and uo_visible(uo_ctl(kb_client, "QuestFrameUI", "QuestFrame")), 10000):
		fail("QuestGiver interaction did not open QuestFrame: " + str(kb_client.quest_state()))
		return false
	if not kb_client.merchant_state().open:
		fail("QuestFrame closed the MerchantFrame; raise case needs both")
		return false
	await uo_settle(300)
	var cover := kb_rect(UO_MERCHANT, "MerchantFrame")
	var frame := kb_rect("QuestFrameUI", "QuestFrame")
	if not await kb_raise_case("Q1_QUESTFRAME_RAISE_ON_CLICK", "QuestFrameUI", frame, cover, frame.intersection(cover).get_center()):
		return false
	await kb_capture("quest-frame-raised.png")
	await kb_tap(KEY_L)
	if not await uo_wait(func(): return kb_client.quest_state().log_open and uo_visible(uo_ctl(kb_client, "QuestLogUI", "QuestLogFrame")), 5000):
		fail("L did not open QuestLogFrame: " + str(kb_client.quest_state()))
		return false
	await uo_settle(300)
	cover = kb_rect(UO_MERCHANT, "MerchantFrame")
	var log := kb_rect("QuestLogUI", "QuestLogFrame")
	if not log.intersects(cover):
		fail("Q2 geometry: QuestLogFrame %s does not overlap MerchantFrame %s" % [log, cover])
		return false
	if not await kb_raise_case("Q2_QUESTLOG_RAISE_ON_CLICK", "QuestLogUI", log, cover, log.intersection(cover).get_center()):
		return false
	await kb_capture("quest-log-raised.png")
	return true
