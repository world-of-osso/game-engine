extends "res://tests/world_merchant_flow.gd"
## Real native Wardrobe on private55087; owned fb_transmog_a/TransmogA fixture.
## Both skins use the same test. TRANSMOG_SHOTS is a durable diagnostic directory.

var shots := ""

func run_test() -> void:
	shots = OS.get_environment("TRANSMOG_SHOTS")
	if shots == "" or OS.get_environment("GODOT_TEST_SERVER") != "127.0.0.1:55087":
		fail("Set TRANSMOG_SHOTS and private GODOT_TEST_SERVER=127.0.0.1:55087")
		return
	DirAccess.make_dir_recursive_absolute(shots)
	root.size = Vector2i(1280, 720)
	client = load("res://scenes/client.tscn").instantiate()
	root.add_child(client)
	# Let startup initialize its native login destination before fixture authentication.
	await frames(5)
	var error = client.connect_account("127.0.0.1:55087", "fb_transmog_a", "fbtest", false)
	if error != "":
		fail(error)
		return
	if not await wait_for(func(_s): return client.get_node_or_null("CharacterSelectUI") != null, "character select", 180000):
		print("TRANSMOG_ACCOUNT_STATE ", client.account_state())
		return
	var select = client.get_node("CharacterSelectUI")
	if not await wait_for(func(_s): return select.find_child("CharCard_0", true, false) != null, "roster card", 60000):
		return
	await click_control(select.find_child("CharCard_0", true, false), MOUSE_BUTTON_LEFT)
	await click_control(select.find_child("EnterWorld", true, false), MOUSE_BUTTON_LEFT)
	if not await wait_for(func(_s): return client.account_state().screen == "InWorld" and client.account_state().local_player_position != null, "enter world", 180000):
		return
	await create_timer(40.0).timeout
	await tap(KEY_Z)
	if not await wait_for(func(_s): return control("AuctionGossipOption0") != null, "Hashom gossip", 60000):
		return
	await click_control(control("AuctionGossipOption0"), MOUSE_BUTTON_LEFT)
	if not await wait_for(func(_s): return control("TransmogSlot16") != null, "native Wardrobe", 60000):
		return
	await click_control(control("TransmogSlot16"), MOUSE_BUTTON_LEFT)
	if not await wait_for(func(_s): return preview_model("TransmogCharacterModelScene") != null, "character preview", 60000):
		return
	await frames(20)
	await save_shot("01-applied.png")
	var preview := preview_model("TransmogCharacterModelScene")
	var weapon = preview.find_child("EquipmentMainHand", true, false) as Node3D
	if weapon == null:
		fail("Preview has no main-hand weapon node")
		return
	var weapon_pixels := await measure_weapon_pixels(weapon)
	print("TRANSMOG_WEAPON_PIXELS ", weapon_pixels, " parent=", weapon.get_parent().name, " visible=", weapon.is_visible_in_tree())
	if weapon_pixels < 25:
		fail("Main-hand weapon contributes only %d preview pixels" % weapon_pixels)
		return
	for source in [821, 1181]:
		var host := "TransmogSource%dModelScene" % source
		if not await wait_for(func(_s): return preview_model(host) != null, "3D source%d" % source, 60000):
			return
		var actor := preview_model(host)
		if actor.find_children("*", "MeshInstance3D", true, false).is_empty():
			fail("Source%d has no 3D meshes" % source)
			return
	print("TRANSMOG_COLLECTION_3D_READY")
	var ui = client.get_node("TransmogUI")
	var before: int = client.merchant_state().money
	var applied = ui.frame_text("TransmogSource821Status")
	var next_source := 1181 if applied == "Applied" else 821
	await click_control(control("TransmogSource%d" % next_source), MOUSE_BUTTON_LEFT)
	if not await wait_for(func(_s): return ui.frame_text("TransmogSource%dStatus" % next_source) == "Pending", "pending source"):
		return
	await save_shot("02-pending.png")
	await click_control(control("TransmogApplyButton"), MOUSE_BUTTON_LEFT)
	if not await wait_for(func(_s): return ui.frame_text("TransmogSource%dStatus" % next_source) == "Applied" and client.merchant_state().money == before - 1500, "paid authoritative Apply", 30000):
		return
	await save_shot("03-applied.png")
	print("TRANSMOG_PAID_APPLY source=", next_source, " gold=", before, "->", client.merchant_state().money)
	await tap(KEY_ESCAPE)
	await frames(10)
	if client.get_node_or_null("TransmogUI") != null:
		fail("Escape retained the Wardrobe and detached previews")
		return
	print("PASS TRANSMOG_WARDROBE_LIVE")
	client.free()
	quit(0)

func control(name: String) -> Control:
	return client.find_child(name, true, false) as Control

func preview_model(host_name: String) -> Node3D:
	var host := control(host_name)
	if host == null:
		return null
	var scene = host.find_child("CharacterModelSceneRoot", true, false)
	if scene == null:
		return null
	for child in scene.get_children():
		if child.name != "Camera" and child.name != "SceneLight" and child is Node3D:
			return child
	return null

func measure_weapon_pixels(weapon: Node3D) -> int:
	var host := control("TransmogCharacterModelScene")
	var viewport := host.find_child("CharacterModelViewport", true, false) as SubViewport
	var animation = preview_model("TransmogCharacterModelScene").find_child("M2Animation", true, false)
	animation.set_paused(true)
	animation.set_process(false)
	weapon.visible = false
	await frames(2)
	var hidden := viewport.get_texture().get_image()
	weapon.visible = true
	await frames(2)
	var visible := viewport.get_texture().get_image()
	hidden.save_png(shots.path_join("weapon-hidden.png"))
	visible.save_png(shots.path_join("weapon-visible.png"))
	var changed := changed_pixels(hidden, visible)
	var mesh_visibility := {}
	for mesh in preview_model("TransmogCharacterModelScene").find_children("*", "MeshInstance3D", true, false):
		if not weapon.is_ancestor_of(mesh):
			mesh_visibility[mesh] = mesh.visible
			mesh.visible = false
	await frames(2)
	var isolated := viewport.get_texture().get_image()
	isolated.save_png(shots.path_join("weapon-isolated.png"))
	for mesh in mesh_visibility:
		mesh.visible = mesh_visibility[mesh]
	animation.set_process(true)
	animation.set_paused(false)
	return changed

func changed_pixels(first: Image, second: Image) -> int:
	var changed := 0
	for y in first.get_height():
		for x in first.get_width():
			if not first.get_pixel(x, y).is_equal_approx(second.get_pixel(x, y)):
				changed += 1
	return changed

func save_shot(name: String) -> void:
	await frames(2)
	var image := root.get_texture().get_image()
	if image.get_width() > 1280:
		image.resize(1280, int(image.get_height() * 1280.0 / image.get_width()), Image.INTERPOLATE_LANCZOS)
	var error := image.save_png(shots.path_join(name))
	if error != OK:
		fail("Screenshot " + error_string(error))
