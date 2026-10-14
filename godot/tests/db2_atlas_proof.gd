extends SceneTree

# Authenticated private client; samples use RegistryUi's normal named-atlas path.
var client: Node
var panel: Node
var evidence: String
var names: Array[String] = []
var skin: String

func _initialize() -> void:
	Engine.max_fps = 60
	call_deferred("run_test")

func run_test() -> void:
	root.size = Vector2i(1920, 1080)
	evidence = OS.get_environment("DB2_PROOF_OUTPUT")
	skin = OS.get_environment("DB2_PROOF_SKIN")
	var account := OS.get_environment("DB2_PROOF_ACCOUNT")
	if OS.get_environment("GODOT_TEST_SERVER") != "127.0.0.1:5821" or not account.begins_with("fb_db2"):
		fail("Only private UDP5821 and fb_db2 accounts are allowed")
		return
	var requested = JSON.parse_string(FileAccess.get_file_as_string(OS.get_environment("DB2_PROOF_NAMES")))
	for name in requested:
		names.append(String(name))
	if names.size() < 15:
		fail("Require all twelve housing hotkeys plus other changed-sheet samples")
		return
	client = load("res://scenes/client.tscn").instantiate()
	root.add_child(client)
	if not await wait_for(func(): return client.merchant_state().item_catalog_loaded, "startup catalogs", 120000):
		return
	var error = client.connect_account("127.0.0.1:5821", account, "fbtest", false)
	if error != "":
		fail("Login: " + error)
		return
	if not await wait_for(func(): return client.account_state().screen == "CharacterSelect" and client.get_node_or_null("CharacterSelectUI") != null, "character selection", 45000):
		return
	var ui = client.get_node("CharacterSelectUI")
	await click_control(ui.find_child("CharCard_0", true, false))
	await click_control(ui.find_child("EnterWorld", true, false))
	if not await wait_for(func(): return client.account_state().screen == "InWorld" and client.account_state().world_attached, "authenticated world", 180000):
		return
	error = client.preview_world_skin(skin == "Forever")
	if error != "":
		fail("Skin: " + error)
		return
	panel = RegistryUi.new()
	panel.name = "Db2AtlasProofUI"
	panel.layer = 100
	root.add_child(panel)
	error = panel.show_db2_atlas_proof(names, skin == "Forever")
	if error != "":
		fail("Atlas panel: " + error)
		return
	if not await wait_for(samples_ready, "all native atlas textures", 90000):
		return
	await create_timer(2.0).timeout
	await RenderingServer.frame_post_draw
	var image := root.get_texture().get_image()
	var samples := []
	for index in range(names.size()):
		var texture = sample_texture(index)
		var rect := texture.get_global_rect()
		var pixels := image.get_region(Rect2i(rect))
		var lit := 0
		for y in range(pixels.get_height()):
			for x in range(pixels.get_width()):
				var pixel := pixels.get_pixel(x, y)
				if maxf(pixel.r, maxf(pixel.g, pixel.b)) > 0.25:
					lit += 1
		if lit < 25:
			fail("Blank native atlas sample: " + names[index])
			return
		var region = texture.texture as AtlasTexture
		samples.append({"name": names[index], "screen_rect": [rect.position.x, rect.position.y, rect.size.x, rect.size.y], "lit_pixels": lit,
			"native_region": [region.region.position.x, region.region.position.y, region.region.size.x, region.region.size.y] if region != null else null})
	image.save_png(evidence + "/" + skin.to_lower() + ".png")
	var receipt = JSON.parse_string(FileAccess.get_file_as_string(ProjectSettings.globalize_path("res://../data/db2/active/receipt.json")))
	var proof := {"identity": receipt.identity, "account": account, "server": "127.0.0.1:5821", "state": client.account_state(), "samples": samples}
	var file := FileAccess.open(evidence + "/" + skin.to_lower() + ".json", FileAccess.WRITE)
	file.store_string(JSON.stringify(proof, "  "))
	file.close()
	print("DB2_ATLAS_PROOF_PASS ", skin, " ", names.size(), " native named-atlas samples")
	client.free()
	quit(0)

func sample_texture(index: int) -> TextureRect:
	var frame := panel.find_child("Db2AtlasProofImage%d" % index, true, false)
	if frame == null:
		return null
	return frame.get_node_or_null("Parts/Part0") as TextureRect

func samples_ready() -> bool:
	for index in range(names.size()):
		var texture = sample_texture(index)
		if texture == null or texture.texture == null:
			return false
	return true

func click_control(control: Control) -> void:
	if control == null:
		fail("Missing native character-selection control")
		return
	var point := control.get_global_rect().get_center()
	for pressed in [true, false]:
		var event := InputEventMouseButton.new()
		event.button_index = MOUSE_BUTTON_LEFT
		event.position = point
		event.global_position = point
		event.pressed = pressed
		Input.parse_input_event(event)
		await process_frame

func wait_for(predicate: Callable, label: String, timeout_ms: int) -> bool:
	var deadline := Time.get_ticks_msec() + timeout_ms
	while Time.get_ticks_msec() < deadline:
		if predicate.call():
			return true
		await process_frame
	fail("Timed out: " + label)
	return false

func fail(message: String) -> void:
	push_error(message)
	quit(1)
