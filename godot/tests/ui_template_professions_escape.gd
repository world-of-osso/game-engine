extends SceneTree
# Real mounted profession roots and manual WindowManager bag state; no realm/NPC request.
var completion
var shots := ""
var marker: Label
var receipts: Array[Dictionary] = []

func _initialize() -> void:
	Engine.max_fps = 60
	call_deferred("run_cases")

func run_cases() -> void:
	root.size = Vector2i(1920, 1080)
	root.content_scale_size = Vector2i.ZERO
	completion = preload("res://tests/completion.gd").new(self, 6)
	shots = OS.get_environment("PROFESSIONS_ESCAPE_CAPTURE_PATH")
	if not completion.require(not shots.is_empty() and DirAccess.make_dir_recursive_absolute(shots) == OK, "PNG destination"): return
	create_marker()
	for forever in [false, true]:
		for customer in [false, true]:
			var client = ClassDB.instantiate("GameClient")
			if not completion.require(client != null, "real GameClient"): return
			var passed := await probe_case(client, forever, customer)
			client.cleanup_professions_escape_probe()
			client.free()
			await settle()
			if not passed: return
	var output := FileAccess.open(OS.get_environment("PROFESSIONS_ESCAPE_RECEIPTS"), FileAccess.WRITE)
	if not completion.require(output != null, "receipt file"): return
	output.store_string(JSON.stringify(receipts, "\t"))
	output.close()
	completion.finish(true)

func probe_case(client: Object, forever: bool, customer: bool) -> bool:
	var skin := "forever" if forever else "modern"
	var kind := "customer" if customer else "owner"
	var startup: String = client.poll_escape_probe_startup()
	var deadline := Time.get_ticks_msec() + 60000
	while startup == "pending" and Time.get_ticks_msec() < deadline:
		await process_frame
		startup = client.poll_escape_probe_startup()
	if not completion.require(startup.is_empty(), "asset boundary: " + startup): return false
	var error: String = client.attach_professions_escape_probe(root, forever, customer)
	if not completion.require(error.is_empty(), error): return false
	await settle()
	var before: Dictionary = client.professions_escape_probe_state()
	if not completion.require(before.ui and before.book and before.bag and profession_shown(customer), "real root and manual bag must start open: " + str(before)): return false
	if not await capture(skin, kind, "before", before): return false
	var first: Dictionary = client.step_professions_escape_probe()
	await settle()
	if not completion.require(not first.ui and not first.book and not profession_shown(customer), "profession root closes: " + str(first)): return false
	if not await capture(skin, kind, "after", first): return false
	if customer:
		if not completion.require(not first.bag, "customer orders is not the owner ProfessionsFrame AddOn handler: " + str(first)): return false
		return completion.case_done()
	if not completion.require(first.bag, "Professions AddOn8 must consume before all-window bag sweep: " + str(first)): return false
	if not completion.case_done(): return false
	var second: Dictionary = client.step_professions_escape_probe()
	update_marker(second)
	receipts.append({"skin": skin, "kind": kind, "phase": "second", "state": second, "marker": marker.text})
	if not completion.require(not second.ui and not second.bag, "next Escape sweeps remaining manual bag: " + str(second)): return false
	return completion.case_done()

func profession_shown(customer: bool) -> bool:
	var name := "ProfessionsCustomerOrdersFrame" if customer else "ProfessionsFrame"
	var control := root.find_child(name, true, false) as Control
	return control != null and control.is_visible_in_tree()

func settle() -> void:
	for _frame in range(4):
		await process_frame
		await RenderingServer.frame_post_draw

func create_marker() -> void:
	var layer := CanvasLayer.new()
	layer.layer = 100
	marker = Label.new()
	marker.position = Vector2(24, 24)
	marker.mouse_filter = Control.MOUSE_FILTER_IGNORE
	marker.add_theme_font_size_override("font_size", 24)
	marker.add_theme_color_override("font_color", Color.WHITE)
	marker.add_theme_color_override("font_outline_color", Color.BLACK)
	marker.add_theme_constant_override("outline_size", 6)
	layer.add_child(marker)
	root.add_child(layer)

func update_marker(state: Dictionary) -> void:
	marker.text = "TEST STATE | profession UI: %s | manual backpack open (model): %s" % [str(state.ui), str(state.bag)]

func capture(skin: String, kind: String, phase: String, state: Dictionary) -> bool:
	update_marker(state)
	await process_frame
	await RenderingServer.frame_post_draw
	var image := root.get_texture().get_image()
	var path := shots.path_join(skin + "-professions-" + kind + "-" + phase + ".png")
	if not completion.require(marker.is_visible_in_tree() and image != null and image.get_size() == Vector2i(1920, 1080) and image.save_png(path) == OK, "visible-state1920 PNG"): return false
	var receipt := {"skin": skin, "kind": kind, "phase": phase, "state": state, "capture": path, "marker": marker.text}
	receipts.append(receipt)
	print("UI_TEMPLATE_PROFESSIONS_ESCAPE ", JSON.stringify(receipt))
	return true
