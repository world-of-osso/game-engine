extends "res://tests/housing_expert_source_live.gd"

var glow_receipts: Array[Dictionary] = []
var previous_sample: Dictionary = {}

func run_test() -> void:
	root.size = Vector2i(1280, 720)
	housing_server = OS.get_environment("GODOT_TEST_SERVER")
	housing_account = OS.get_environment("HOUSING_ACCOUNT")
	housing_skin = OS.get_environment("HOUSING_SKIN")
	shots = OS.get_environment("HOUSING_SHOTS")
	editor_house = int(OS.get_environment("HOUSING_HOUSE_ID"))
	cleanup_role = "owner"
	if housing_server != "127.0.0.1:5870" or not housing_account.begins_with("fb_housing_") or shots.is_empty() or editor_house <= 0:
		fail("Submode glow proof requires explicit private5870 paid owner configuration")
		return
	DirAccess.make_dir_recursive_absolute(shots)
	client = load("res://scenes/client.tscn").instantiate()
	root.add_child(client)
	if not await connect_housing_account(): return
	if not await open_cleanup_house(): return
	var before: Dictionary = client.housing_state()
	if not before.can_edit or before.decor.size() != 1 or int(before.decor[0].decor_id) != 1693:
		fail("Submode glow proof requires the genuine editable paid bookcase")
		return
	if not await configure_expert_bindings(false): return
	if not await enter_expert(): return
	push_mouse_motion(Vector2(20.0, 20.0))
	for spec in [["Translate", "TranslateSubmodeButton", KEY_F8], ["Rotate", "RotateSubmodeButton", KEY_F9], ["Scale", "ScaleSubmodeButton", KEY_F10]]:
		await source_key(spec[2])
		if not await wait_housing(func(): return client.housing_state().precision_submode == spec[0], "assigned active glow submode"): return
		if not await prove_glow(spec[1], spec[0].to_lower() + "-initial"): return
		await create_timer(0.8).timeout
		if not await prove_glow(spec[1], spec[0].to_lower() + "-rotated"): return
	# Basic has no Expert controls, but must not reset the shared first-play clock.
	await source_key(KEY_F1)
	if not await wait_housing(func(): return housing_control("HouseEditorExpertSubmodeBar") == null, "Basic removes Expert glow controls"): return
	await create_timer(0.4).timeout
	if not await enter_expert(): return
	if not await prove_glow("ScaleSubmodeButton", "mode-return"): return
	# The source sync table outlives individual editor frames.
	await press_escape()
	if not await wait_housing(func(): return client.get_node_or_null("HousingDashboardUI") == null, "close actual editor canvas"): return
	await create_timer(0.4).timeout
	if not await open_cleanup_house(): return
	if not await enter_expert(): return
	var selected: String = client.housing_state().precision_submode
	var button: String = {"Translate": "TranslateSubmodeButton", "Rotate": "RotateSubmodeButton", "Scale": "ScaleSubmodeButton"}[selected]
	if not await prove_glow(button, "canvas-recreated"): return
	if not await configure_expert_bindings(true): return
	if client.housing_state().decor != before.decor or client.housing_state().stored != before.stored:
		fail("Submode glow lifecycle changed confirmed paid decor or storage")
		return
	print("PASS: private5870 source Expert rotating fixed-mask glow ", housing_skin)
	client.free()
	quit(0)

func glow_part(button: String) -> Control:
	var glow := housing_control(button + "ActiveGlow")
	return null if glow == null else glow.get_node_or_null("Parts/Part0") as Control

func prove_glow(button: String, phase: String) -> bool:
	if not await wait_housing(func(): return glow_part(button) != null and glow_part(button).is_visible_in_tree(), "painted source ActiveGlow " + button, 3000): return false
	var glow := housing_control(button + "ActiveGlow")
	var part := glow_part(button)
	var control := housing_control(button)
	if not glow.size.is_equal_approx(Vector2(176.0, 178.0)) or not (glow.position + glow.size / 2.0).is_equal_approx(control.size / 2.0):
		fail("ActiveGlow is not source176x178 centered on its64x64 control")
		return false
	if not is_equal_approx(part.modulate.a, 1.0) or not is_equal_approx(glow.modulate.a, 1.0):
		fail("Source rotating glow was replaced with an opacity pulse")
		return false
	var material := part.material as ShaderMaterial
	if material == null or material.get_shader_parameter("border_mask") == null:
		fail("Source glow has no actual fixed alpha-mask material")
		return false
	for other in ["TranslateSubmodeButton", "RotateSubmodeButton", "ScaleSubmodeButton"]:
		var other_glow := housing_control(other + "ActiveGlow")
		if other != button and other_glow != null and other_glow.is_visible_in_tree():
			fail("Inactive Expert submode retained a visible glow")
			return false
	var sample := {"phase": phase, "skin": housing_skin, "button": button, "time_us": Time.get_ticks_usec(), "rotation": part.rotation, "size": glow.size, "position": glow.position, "mask_rect": material.get_shader_parameter("border_rect"), "mask_uv": material.get_shader_parameter("border_uv"), "housing": client.housing_state()}
	if not previous_sample.is_empty():
		var elapsed: float = float(sample.time_us - previous_sample.time_us) / 1000000.0
		var expected: float = float(previous_sample.rotation) + elapsed * TAU / 6.0
		if absf(angle_difference(expected, sample.rotation)) > 0.12:
			fail("Glow lost source6s clockwise/shared first-play phase at " + phase)
			return false
	previous_sample = sample
	glow_receipts.append(sample)
	if not await prove_masked_pixels(button, phase): return false
	var output := FileAccess.open(shots.path_join("submode-glow-receipts.json"), FileAccess.WRITE)
	if output == null:
		fail("Cannot save actual Expert glow receipts")
		return false
	output.store_string(JSON.stringify(glow_receipts, "\t"))
	await RenderingServer.frame_post_draw
	var image := root.get_texture().get_image()
	if image.save_png(shots.path_join(housing_skin.to_lower() + "-glow-" + phase + ".png")) != OK:
		fail("Cannot save actual source glow pixels")
		return false
	return true

# Compare actual painted frames with only the projected glow hidden. Source mask
# alpha is read independently from the authenticated, uncompressed BLP2 payload.
func prove_masked_pixels(button: String, phase: String) -> bool:
	var fdid: int = {"TranslateSubmodeButton": 7266217, "RotateSubmodeButton": 7266219, "ScaleSubmodeButton": 7266221}[button]
	var path := ProjectSettings.globalize_path("res://").path_join("../data/textures/%d.blp" % fdid)
	var source := FileAccess.get_file_as_bytes(path)
	if source.size() < 1172 + 64 * 64 * 4 or source[8] != 3 or source.decode_u32(12) != 64 or source.decode_u32(16) != 64:
		fail("Mask oracle requires authenticated raw64x64 BGRA BLP2 source")
		return false
	var offset: int = source.decode_u32(20)
	var part := glow_part(button)
	var ui := client.get_node("HousingDashboardUI")
	ui.set_process(false)
	await RenderingServer.frame_post_draw
	var painted := root.get_texture().get_image()
	part.visible = false
	RenderingServer.force_draw()
	var bare := root.get_texture().get_image()
	part.visible = true
	ui.set_process(true)
	var bounds: Rect2 = housing_control(button).get_global_rect()
	var area := bounds.grow(75.0).intersection(Rect2(Vector2.ZERO, Vector2(root.size)))
	var inside: int = 0
	var escaped: int = 0
	for y in range(int(area.position.y), int(area.end.y)):
		for x in range(int(area.position.x), int(area.end.x)):
			var a := painted.get_pixel(x, y)
			var b := bare.get_pixel(x, y)
			if maxf(absf(a.r-b.r), maxf(absf(a.g-b.g), absf(a.b-b.b))) < 0.035: continue
			var local: Vector2 = (Vector2(x + 0.5, y + 0.5) - bounds.position) * 64.0 / bounds.size
			var allowed := false
			# Include one texel of bilinear/filter coverage, not rotated mask coordinates.
			for my in range(maxi(0, int(floor(local.y)) - 1), mini(64, int(ceil(local.y)) + 2)):
				for mx in range(maxi(0, int(floor(local.x)) - 1), mini(64, int(ceil(local.x)) + 2)):
					if source[offset + (my * 64 + mx) * 4 + 3] > 0: allowed = true
			if allowed: inside += 1
			else: escaped += 1
	glow_receipts[-1]["painted_mask_pixels"] = inside
	glow_receipts[-1]["escaped_mask_pixels"] = escaped
	if inside < 5 or escaped != 0:
		fail("Source fixed-mask painted-pixel mismatch %s: inside%d escaped%d" % [phase, inside, escaped])
		return false
	return true
