extends "res://tests/bank_live.gd"
# R Blizzard_FlightMap.lua1 registers FlightMapFrame as an ordinary UIPanel; CloseAllWindows
# closes bags plus panels before the World fallback. No taxi activation is performed.
var completion
var current_skin := ""
var taxi_receipts: Array[Dictionary] = []

func run_test() -> void:
	root.size = Vector2i(1920, 1080)
	root.content_scale_size = Vector2i.ZERO
	completion = preload("res://tests/completion.gd").new(self, 4)
	role = "ui-generics-taxi"
	shots = OS.get_environment("BANK_SHOTS")
	var endpoint := OS.get_environment("GODOT_TEST_SERVER")
	var account := OS.get_environment("BANK_ACCOUNT")
	if not completion.require(endpoint == "127.0.0.1:5993" and account.begins_with("fb_ui_generics_") and not shots.is_empty(), "owned5993 account and PNG destination required"): return
	if not completion.require(DirAccess.make_dir_recursive_absolute(shots) == OK, "PNG directory"): return
	client = load("res://scenes/client.tscn").instantiate()
	root.add_child(client)
	var error: String = client.connect_account(endpoint, account, PASSWORD, false)
	if not completion.require(error.is_empty(), error): return
	if not await enter_world(OS.get_environment("BANK_CHARACTER")): return
	for forever in [false, true]:
		current_skin = "forever" if forever else "modern"
		error = client.preview_world_skin(forever)
		if not completion.require(error.is_empty(), error): return
		await wait_frames(30)
		if not await taxi_and_bag(): return
	client.free()
	completion.finish(true)

func taxi_and_bag() -> bool:
	if not await click_pickable(func(node): return str(node.name) == "Dungar Longdrink", "unit_server_id"): return false
	if not await wait_until(func(): return shown("GossipOption0Text"), "flight-master ride option"): return false
	await press("GossipOption0Text")
	if not await wait_until(func(): return client.flight_map_state().open, "real taxi map"): return false
	await tap(KEY_B)
	if not await wait_until(func(): return shown("ContainerFrame0"), "manual backpack with taxi map"): return false
	var inventory: Dictionary = client.merchant_state()
	var target: Variant = client.target_state().target
	if not completion.require(client.flight_map_state().open and shown("ContainerFrame0"), "simultaneous real taxi panel and bag"): return false
	if not await capture_taxi("before"): return false
	if not completion.case_done(): return false
	await tap(KEY_ESCAPE)
	if not await wait_until(func(): return not client.flight_map_state().open, "taxi closes on Escape"): return false
	await wait_frames(12)
	if not await capture_taxi("after"): return false
	if not completion.require(not shown("ContainerFrame0"), "ordinary TaxiFrame Escape left a manually opened bag visible"): return false
	if not completion.require(client.target_state().target == target, "handled UI Escape fell through into ClearTarget"): return false
	if not completion.require(client.merchant_state().money == inventory.money and client.merchant_state().bags == inventory.bags, "Escape changed item or money authority"): return false
	if not completion.require(not client.flight_map_state().controlled, "Escape activated a taxi flight"): return false
	return completion.case_done()

func capture_taxi(phase: String) -> bool:
	await RenderingServer.frame_post_draw
	var image := root.get_texture().get_image()
	var path := shots.path_join(current_skin + "-taxi-" + phase + ".png")
	if not completion.require(image != null and image.get_size() == Vector2i(1920, 1080) and image.save_png(path) == OK, "1920 PNG capture"): return false
	taxi_receipts.append({"skin": current_skin, "phase": phase, "taxi": client.flight_map_state(), "bag_visible": shown("ContainerFrame0"), "target": client.target_state(), "capture": path})
	print("UI_TEMPLATE_TAXI_ESCAPE ", JSON.stringify(taxi_receipts.back()))
	var output := FileAccess.open(OS.get_environment("TAXI_RECEIPTS"), FileAccess.WRITE)
	if not completion.require(output != null, "private receipt file"): return false
	output.store_string(JSON.stringify(taxi_receipts, "\t"))
	output.close()
	return true

func fail(message: String) -> bool:
	if completion != null: return completion.fail(message)
	push_error(message)
	quit(1)
	return false
