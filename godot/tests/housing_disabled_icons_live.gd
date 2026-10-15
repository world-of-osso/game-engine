extends "res://tests/housing_cleanup_live.gd"

var disabled_icon_receipts: Array[Dictionary] = []

func run_test() -> void:
	root.size = Vector2i(1920, 1080)
	housing_server = OS.get_environment("GODOT_TEST_SERVER")
	housing_account = OS.get_environment("HOUSING_ACCOUNT")
	housing_skin = OS.get_environment("HOUSING_SKIN")
	shots = OS.get_environment("HOUSING_SHOTS")
	editor_house = int(OS.get_environment("HOUSING_HOUSE_ID"))
	cleanup_role = "owner"
	if housing_server != "127.0.0.1:5870" or not housing_account.begins_with("fb_housing_") or shots.is_empty() or editor_house <= 0:
		fail("Disabled icon proof requires explicit private5870/fb_housing_* owner")
		return
	DirAccess.make_dir_recursive_absolute(shots)
	client = load("res://scenes/client.tscn").instantiate()
	root.add_child(client)
	if not await connect_housing_account(): return
	if not await open_cleanup_house(): return
	var before: Dictionary = client.housing_state()
	for name in ["HouseEditorCustomizeModeButton", "HouseEditorSnapButton", "HouseEditorFreePlaceButton"]:
		if not await prove_disabled_icon(name, "basic"): return
		await click(housing_control(name).get_global_rect().get_center(), MOUSE_BUTTON_LEFT)
		await frames(5)
		if housing_control("HouseEditorStorageFrame") == null or int(client.housing_state().get("selected", 0)) != 0 or int(client.housing_state().get("placing", 0)) != 0:
			fail("Disabled source icon activated editing")
			return
	if not await click_exposed_control(housing_control("HouseEditorExpertDecorModeButton")): return
	if not await wait_housing(func(): return housing_control("HouseEditorExpertSubmodeBar") != null, "Expert disabled-icon remount"): return
	if not await prove_disabled_icon("HouseEditorCustomizeModeButton", "expert"): return
	if client.housing_state().decor != before.decor or client.housing_state().stored != before.stored:
		fail("Disabled icon inspection changed paid authority")
		return
	print("PASS: private5870 source disabled editor icon color ", housing_skin)
	client.free()
	quit(0)

func prove_disabled_icon(name: String, phase: String) -> bool:
	if not await wait_housing(func(): return painted_icon(name) != null, "authenticated disabled icon art " + name): return false
	var button: BaseButton = housing_control(name) as BaseButton
	var frame: Control = housing_control(name + "Icon")
	var icon: TextureRect = painted_icon(name)
	push_mouse_motion(button.get_global_rect().get_center())
	await frames(5)
	var expected := Color(0.4, 0.4, 0.4, 1.0)
	var hover: CanvasItem = housing_control(name + "Hover") as CanvasItem
	if not button.disabled or not frame.modulate.is_equal_approx(Color.WHITE) or not icon.self_modulate.is_equal_approx(expected) or hover == null or absf(hover.modulate.a) > 0.001:
		fail("Disabled icon must use source DARKGRAY tint/full alpha without hover: " + name + " alpha=" + str(frame.modulate) + " color=" + str(icon.self_modulate))
		return false
	disabled_icon_receipts.append({"name": name, "phase": phase, "skin": housing_skin, "disabled": button.disabled, "frame_rgba": color_channels(frame.modulate), "tint_rgba": color_channels(icon.self_modulate), "hover_alpha": hover.modulate.a, "housing": client.housing_state()})
	var file := FileAccess.open(shots.path_join("disabled-icon-receipts.json"), FileAccess.WRITE)
	if file == null:
		fail("Cannot save disabled icon source receipts")
		return false
	file.store_string(JSON.stringify(disabled_icon_receipts, "\t"))
	file.close()
	if name == "HouseEditorCustomizeModeButton": return await capture_housing("disabled-source-icons-" + phase)
	return true

func painted_icon(name: String) -> TextureRect:
	var frame: Control = housing_control(name + "Icon")
	if frame == null: return null
	var parts: Node = frame.get_node_or_null("Parts")
	if parts == null: return null
	for child in parts.get_children():
		var texture: TextureRect = child as TextureRect
		if texture != null and texture.texture != null: return texture
	return null

func color_channels(color: Color) -> Array:
	return [color.r, color.g, color.b, color.a]
