extends SceneTree
# Real RegistryUi pixels and GameClient priority/virtual routing; no realm or hardware-input claim.
var completion
var shots := ""
var receipts: Array[Dictionary] = []
var pending_marker: Label

func _initialize() -> void:
	Engine.max_fps = 60
	call_deferred("run_matrix")

func run_matrix() -> void:
	root.size = Vector2i(1920, 1080)
	root.content_scale_size = Vector2i.ZERO
	completion = preload("res://tests/completion.gd").new(self, 4)
	create_pending_marker()
	shots = OS.get_environment("ESCAPE_CAPTURE_PATH")
	if not completion.require(not shots.is_empty() and DirAccess.make_dir_recursive_absolute(shots) == OK, "PNG destination"): return
	for forever in [false, true]:
		var client = ClassDB.instantiate("GameClient")
		if not completion.require(client != null, "real GameClient"): return
		var passed := await probe_skin(client, forever)
		client.cleanup_game_menu_escape_probe()
		client.free()
		await settle()
		if not passed: return
	var output := FileAccess.open(OS.get_environment("ESCAPE_RECEIPTS"), FileAccess.WRITE)
	if not completion.require(output != null, "receipt file"): return
	output.store_string(JSON.stringify(receipts, "\t"))
	output.close()
	completion.finish(true)

func probe_skin(client: Object, forever: bool) -> bool:
	var skin := "forever" if forever else "modern"
	var startup: String = client.poll_escape_probe_startup()
	var deadline := Time.get_ticks_msec() + 60000
	while startup == "pending" and Time.get_ticks_msec() < deadline:
		await process_frame
		startup = client.poll_escape_probe_startup()
	if not completion.require(startup.is_empty(), "asset boundary: " + startup): return false
	var error: String = client.attach_game_menu_escape_probe(root, forever)
	if not completion.require(error.is_empty(), error): return false
	await settle()
	var before: Dictionary = client.game_menu_escape_probe_state()
	if not completion.require(not before.has("error") and before.menu and before.pending and menu_shown(), "mounted native menu and pending domain: " + str(before)): return false
	if not completion.require(before.get("hook") == "game_menu" and before.get("priority") == "Framework" and not before.has("group"), "same authored exclusive registration feeds actual root: " + str(before)): return false
	if not await capture(skin, "before", before): return false
	var first: Dictionary = client.step_game_menu_escape_probe()
	await settle()
	if not completion.require(not first.menu and first.pending and not menu_shown(), "Framework Escape must leave pending panel eligible: " + str(first)): return false
	if not await capture(skin, "menu-closed", first): return false
	if not completion.case_done(): return false
	var second: Dictionary = client.step_game_menu_escape_probe()
	await settle()
	update_pending_marker(second)
	receipts.append({"skin": skin, "phase": "pending-cancelled", "state": second, "marker": pending_marker.text})
	if not completion.require(not second.menu and not second.pending and not menu_shown(), "next Escape cancels pending panel without opening menu: " + str(second)): return false
	return completion.case_done()

func menu_shown() -> bool:
	var control := root.find_child("GameMenuRoot", true, false) as Control
	return control != null and control.is_visible_in_tree()

func settle() -> void:
	for _frame in range(4):
		await process_frame
		await RenderingServer.frame_post_draw

func create_pending_marker() -> void:
	var layer := CanvasLayer.new()
	layer.layer = 100
	pending_marker = Label.new()
	pending_marker.name = "EscapePendingStateMarker"
	pending_marker.position = Vector2(24, 24)
	pending_marker.mouse_filter = Control.MOUSE_FILTER_IGNORE
	pending_marker.add_theme_font_size_override("font_size", 24)
	pending_marker.add_theme_color_override("font_color", Color.WHITE)
	pending_marker.add_theme_color_override("font_outline_color", Color.BLACK)
	pending_marker.add_theme_constant_override("outline_size", 6)
	layer.add_child(pending_marker)
	root.add_child(layer)

func update_pending_marker(state: Dictionary) -> void:
	pending_marker.text = "TEST STATE | Game Menu: %s | unmounted Trading Post pending: %s" % [str(state.menu), str(state.pending)]

func capture(skin: String, phase: String, state: Dictionary) -> bool:
	update_pending_marker(state)
	await process_frame
	await RenderingServer.frame_post_draw
	if not completion.require(pending_marker.is_visible_in_tree(), "visible real-state diagnostic marker"): return false
	var image := root.get_texture().get_image()
	var path := shots.path_join(skin + "-escape-" + phase + ".png")
	if not completion.require(image != null and image.get_size() == Vector2i(1920, 1080) and image.save_png(path) == OK, "1920 PNG capture"): return false
	var receipt := {"skin": skin, "phase": phase, "state": state, "capture": path, "marker": pending_marker.text}
	receipts.append(receipt)
	print("UI_TEMPLATE_ESCAPE_MATRIX ", JSON.stringify(receipt))
	return true
