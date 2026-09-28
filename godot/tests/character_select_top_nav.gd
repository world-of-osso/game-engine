extends SceneTree

# Real-server character-select top navigation through viewport mouse input:
# hover boxes a tab, MODE/SHOP stay inert, MENU overlays character select, CAMPSITES opens the
# campsite panel, whose pages keep every card inside 720px, and a card switches the campsite,
# REALMS returns to login. GODOT_CAMPSITE_CAPTURE optionally saves each open panel page
# as <path without .png>-page<N>.png.

const TABS := [
	["CharSelectModeTab", "MODE"],
	["CharSelectShopTab", "SHOP"],
	["CharSelectMenuTab", "MENU"],
	["CharSelectRealmsTab", "REALMS"],
	["CharSelectCampsitesTab", "CAMPSITES"],
]

func _initialize() -> void:
	call_deferred("run")

func fail(message: String, client: Node) -> void:
	push_error(message)
	client.queue_free()
	quit(1)

func settle() -> void:
	for frame in range(3):
		await process_frame

func move_to(point: Vector2) -> void:
	var motion := InputEventMouseMotion.new()
	motion.position = point
	motion.global_position = point
	root.push_input(motion, true)
	await settle()

func click(control: Control) -> void:
	var point := control.get_global_rect().get_center()
	await move_to(point)
	for pressed in [true, false]:
		var button := InputEventMouseButton.new()
		button.position = point
		button.global_position = point
		button.button_index = MOUSE_BUTTON_LEFT
		button.pressed = pressed
		root.push_input(button, true)
		await process_frame
	await settle()

func boxed(ui: Node, tab: String) -> bool:
	var box = ui.find_child(tab + "Box", true, false)
	return box != null and box.is_visible_in_tree()

func label_color(ui: Node, tab: String) -> Color:
	return ui.find_child(tab + "Label", true, false).get_theme_color("font_color")

# Visible campsite card ids on the current page; each card must sit inside the viewport,
# above the paging controls.
func page_cards(ui: Node, viewport: Rect2) -> Array:
	var ids := []
	var paging: Rect2 = ui.find_child("CampsitePaging", true, false).get_global_rect()
	for card in ui.find_children("CampsiteScene_*", "", true, false):
		if not card.is_visible_in_tree():
			continue
		var rect: Rect2 = card.get_global_rect()
		if not viewport.encloses(rect) or rect.end.y > paging.position.y:
			return [-1, "%s %s (paging %s)" % [card.name, rect, paging]]
		ids.append(int(card.name.trim_prefix("CampsiteScene_")))
	return ids

func capture_page(page: int) -> void:
	var capture_path = OS.get_environment("GODOT_CAMPSITE_CAPTURE")
	if capture_path.is_empty():
		return
	for frame in range(3):
		await process_frame
		await RenderingServer.frame_post_draw
	root.get_texture().get_image().save_png("%s-page%d.png" % [capture_path.get_basename(), page])

# Pages through the campsite panel like retail's paging controls; every card must be
# reachable inside the 1280x720 window. Returns "" or a failure.
func check_campsite_pages(ui: Node) -> String:
	var viewport := Rect2(Vector2.ZERO, Vector2(root.size))
	var panel: Control = ui.find_child("CampsitePanel", true, false)
	if not viewport.encloses(panel.get_global_rect()):
		return "Campsite panel %s must fit the %s window" % [panel.get_global_rect(), viewport.size]
	var seen := []
	var page_text: String = ui.frame_text("CampsitePageText")
	var pages := int(page_text.get_slice("/", 1))
	for page in range(pages):
		var ids := page_cards(ui, viewport)
		if ids.size() > 0 and ids[0] == -1:
			return "Campsite card %s escapes the window or overlaps paging on page %d" % [ids[1], page + 1]
		seen.append_array(ids)
		await capture_page(page + 1)
		if page + 1 < pages:
			await click(ui.find_child("CampsiteNextPage", true, false))
			if ui.frame_text("CampsitePageText") != "Page %d/%d" % [page + 2, pages]:
				return "Next page must advance to %d: %s" % [page + 2, ui.frame_text("CampsitePageText")]
	if pages < 2 or seen.size() < 5:
		return "Expected the authored campsites to span pages, saw %s over %d" % [seen, pages]
	for page in range(pages - 1):
		await click(ui.find_child("CampsitePrevPage", true, false))
	if ui.frame_text("CampsitePageText") != "Page 1/%d" % pages:
		return "Prev page must return to page 1: %s" % ui.frame_text("CampsitePageText")
	return ""

func still_selecting(client: Node) -> String:
	var state = client.account_state()
	if state.screen != "CharacterSelect" or state.status != "":
		return "screen=%s status=%s" % [state.screen, state.status]
	return ""

func run() -> void:
	root.size = Vector2i(1280, 720)
	var server = OS.get_environment("GODOT_TEST_SERVER")
	var client = load("res://scenes/client.tscn").instantiate()
	root.add_child(client)
	if server.is_empty():
		fail("GODOT_TEST_SERVER must select a local server", client)
		return
	# GODOT_TEST_USER/GODOT_TEST_PASSWORD select a per-agent dev account.
	var user = OS.get_environment("GODOT_TEST_USER") if OS.has_environment("GODOT_TEST_USER") else "admin"
	var password = OS.get_environment("GODOT_TEST_PASSWORD") if OS.has_environment("GODOT_TEST_PASSWORD") else "admin"
	var error = client.connect_account(server, user, password, false)
	if error != "":
		fail(error, client)
		return
	var deadline = Time.get_ticks_msec() + 15000
	while client.account_state().screen != "CharacterSelect":
		if Time.get_ticks_msec() > deadline:
			fail("Timed out waiting for character selection", client)
			return
		await process_frame
	await settle()
	var ui = client.get_node("CharacterSelectUI")
	var previous_right := -1.0
	for entry in TABS:
		var tab = ui.find_child(entry[0], true, false)
		var label = ui.find_child(entry[0] + "Label", true, false)
		if tab == null or label == null or label.text != entry[1]:
			fail("Missing top navigation tab %s" % entry[1], client)
			return
		var rect: Rect2 = tab.get_global_rect()
		if rect.position.x < previous_right or rect.position.y > 60.0:
			fail("Tab %s out of left-to-right top order: %s" % [entry[1], rect], client)
			return
		previous_right = rect.end.x
		if boxed(ui, entry[0]):
			fail("Idle tab %s must not be boxed" % entry[1], client)
			return

	var mode = ui.find_child("CharSelectModeTab", true, false)
	await move_to(mode.get_global_rect().get_center())
	if not boxed(ui, "CharSelectModeTab") or label_color(ui, "CharSelectModeTab") != Color(1, 1, 1, 1):
		fail("Hovered MODE must be boxed with a white label", client)
		return
	await move_to(Vector2(640, 400))
	if boxed(ui, "CharSelectModeTab") or not label_color(ui, "CharSelectModeTab").is_equal_approx(Color(1, 0.82, 0, 1)):
		fail("Unhovered MODE must return to a gold unboxed label", client)
		return

	for tab in ["CharSelectModeTab", "CharSelectShopTab"]:
		await click(ui.find_child(tab, true, false))
		var problem = still_selecting(client)
		var closed = ui.find_child("CampsitePanel", true, false)
		if problem != "" or (closed != null and closed.is_visible_in_tree()):
			fail("%s must stay inert on character select: %s" % [tab, problem], client)
			return

	await click(ui.find_child("CharSelectMenuTab", true, false))
	var menu = client.get_node_or_null("GameMenuUI")
	if menu == null or still_selecting(client) != "":
		fail("MENU must overlay character select", client)
		return
	await click(menu.find_child("MenuBtnResume", true, false))
	if client.get_node_or_null("GameMenuUI") != null or still_selecting(client) != "":
		fail("Return must dismiss MENU without leaving character select", client)
		return

	var scene_before = client.find_child("CharacterSelectScene", true, false)
	await click(ui.find_child("CharSelectCampsitesTab", true, false))
	var panel = ui.find_child("CampsitePanel", true, false)
	if panel == null or not panel.is_visible_in_tree():
		fail("CAMPSITES must open the campsite panel", client)
		return
	await move_to(Vector2(640, 700))
	if not boxed(ui, "CharSelectCampsitesTab"):
		fail("Open campsite panel must keep CAMPSITES boxed", client)
		return
	var paging_problem = await check_campsite_pages(ui)
	if paging_problem != "":
		fail(paging_problem, client)
		return
	var card = ui.find_child("CampsiteScene_4", true, false)
	if card == null:
		fail("Campsite panel must list authored scene 4", client)
		return
	await click(card)
	for frame in range(10):
		await process_frame
	panel = ui.find_child("CampsitePanel", true, false)
	if panel != null and panel.is_visible_in_tree():
		fail("Selecting a campsite must close the panel", client)
		return
	var scene_after = client.find_child("CharacterSelectScene", true, false)
	var problem = still_selecting(client)
	if problem != "" or scene_after == null or scene_after == scene_before:
		fail("Selecting a campsite must rebuild the campsite scene: %s" % problem, client)
		return

	await click(ui.find_child("CharSelectRealmsTab", true, false))
	if client.account_state().screen != "Login":
		fail("REALMS must return to login like Back: %s" % client.account_state().screen, client)
		return
	print("PASS: top navigation hover, inert tabs, MENU, campsite panel/selection and REALMS")
	client.queue_free()
	quit(0)
