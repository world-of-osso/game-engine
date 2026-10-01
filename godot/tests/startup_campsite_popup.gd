extends "res://tests/world_menu_flow.gd"

# `--screen campsitepopup` (original `CampsitePopup` preview): startup authenticates with
# the saved credentials and opens character select with the campsite panel already shown,
# as if CAMPSITES had been clicked. A real click on authored scene 4's card closes the
# panel and rebuilds the campsite scene. GODOT_CAMPSITE_SCREENSHOT saves the open panel.
# Run against a private server: `-- --server <host:port> --screen campsitepopup`.
const SELECT_WAIT_MS := 20000
const SCENE_ID := 4

func run_test() -> void:
	root.size = Vector2i(1280, 720)
	var client: Node = load("res://scenes/client.tscn").instantiate()
	root.add_child(client)
	var deadline := Time.get_ticks_msec() + SELECT_WAIT_MS
	var panel: Control = null
	while Time.get_ticks_msec() < deadline:
		await process_frame
		var ui := client.get_node_or_null("CharacterSelectUI")
		panel = ui.find_child("CampsitePanel", true, false) as Control if ui != null else null
		if client.account_state().screen == "CharacterSelect" and panel != null and panel.is_visible_in_tree():
			break
		panel = null
	if panel == null:
		fail("--screen campsitepopup did not show the campsite panel: " + str(client.account_state()))
		return
	var ui: Node = client.get_node("CharacterSelectUI")
	if not await capture():
		return
	var scene_before := client.find_child("CharacterSelectScene", true, false)
	var card := ui.find_child("CampsiteScene_%d" % SCENE_ID, true, false) as Control
	var pages := 0
	while card == null and pages < 8:
		await click(ui.find_child("CampsiteNextPage", true, false))
		card = ui.find_child("CampsiteScene_%d" % SCENE_ID, true, false) as Control
		pages += 1
	if card == null:
		fail("Campsite panel does not list authored scene %d" % SCENE_ID)
		return
	await click(card)
	for frame in range(10):
		await process_frame
	panel = ui.find_child("CampsitePanel", true, false) as Control
	if panel != null and panel.is_visible_in_tree():
		fail("Selecting a campsite must close the panel")
		return
	var scene_after := client.find_child("CharacterSelectScene", true, false)
	if client.account_state().screen != "CharacterSelect" or scene_after == null or scene_after == scene_before:
		fail("Selecting a campsite must rebuild the campsite scene")
		return
	print("PASS: campsitepopup opened the panel and card %d switched the campsite" % SCENE_ID)
	quit(0)

func capture() -> bool:
	var path := OS.get_environment("GODOT_CAMPSITE_SCREENSHOT")
	if path.is_empty():
		return true
	for frame in range(30):
		await process_frame
	await RenderingServer.frame_post_draw
	var saved := root.get_texture().get_image().save_png(path)
	if saved != OK:
		fail("Save campsite screenshot: " + error_string(saved))
		return false
	print("FIXTURE CAMPSITE_CAPTURED ", path)
	return true
