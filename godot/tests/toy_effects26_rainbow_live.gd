extends "res://tests/toy_effects26_bauble_live.gd"
# Behavioral raster oracle: native movement and three saturated rainbow bands
# behind the player, excluding the body and stationary near-foot sparkles.
var walking := false

func prove_toy(sample: Array) -> void:
	await learn_toy(BAUBLE_ITEM)
	if failed or not learned_toy(BAUBLE_ITEM): return
	await open_source_toy(str(sample[3]))
	if failed: return
	await click("ToySpellButton1")
	if not await wait_until(func(): return has_aura(BAUBLE_SPELL) and kit_model(BAUBLE_SPELL, BAUBLE_MODELS) == 1417024 and remaining_cooldown(BAUBLE_ITEM) > 0.0, "source Bauble state and recovery", 30000): return
	await click("CollectionsMicroButton")
	client.set_camera_orbit(PI / 2.0, -0.45, 8.0)
	var local := int(client.account_state().local_player_id)
	var start: Vector3 = client.unit_transform(local).origin
	set_walking(true)
	await create_timer(1.5).timeout
	var position: Vector3 = client.unit_transform(local).origin
	var motion := position - start
	if motion.length() < 3.0: fail("native W did not move player"); return
	await RenderingServer.frame_post_draw
	var image := root.get_texture().get_image()
	var camera := root.get_camera_3d()
	var direction := motion.normalized()
	if camera == null: fail("rainbow camera absent"); return
	var models := client.find_children("SpellEffect1417024", "Node3D", true, false)
	if models.is_empty(): fail("source ribbon model transform absent"); return
	var model := models[0] as Node3D
	var center := model.global_position
	# Source width above/below0.5555556 in WoW Y, transformed by kit pitch.
	var width: Vector3 = model.global_basis * Vector3(0.0, 0.0, -0.5555556)
	var near := center - direction * 1.0
	var far := center - direction * 3.0
	var first := camera.unproject_position(near - width)
	var region := Rect2(first, Vector2.ZERO)
	for point in [near + width, far - width, far + width]:
		region = region.expand(camera.unproject_position(point))
	region = region.grow(2.0).intersection(Rect2(Vector2.ZERO, Vector2(image.get_size())))
	var bands := {}
	var colored := 0
	for y in range(int(region.position.y), int(region.end.y)):
		for x in range(int(region.position.x), int(region.end.x)):
			var band := rainbow_band(image.get_pixel(x, y))
			if band == 0: continue
			bands[band] = true
			colored += 1
	set_walking(false)
	var path := OS.get_environment("TOYFX_SHOTS").path_join("%s-bauble26-moving-rainbow.png" % skin)
	image.resize(1280, 720, Image.INTERPOLATE_LANCZOS)
	if image.save_png(path) != OK: fail("rainbow PNG"); return
	receipts.append({"phase": "moving-rainbow", "source": "1417024 one ribbon,50edges/s,0.5s edge lifetime", "motion": [motion.x, motion.y, motion.z], "region": [region.position.x, region.position.y, region.size.x, region.size.y], "colored_pixels": colored, "bands": bands.keys(), "image": path, "visuals": client.spell_visuals_state(), "auras": client.aura_state(), "cooldown": remaining_cooldown(BAUBLE_ITEM)})
	write_receipts()
	if bands.size() < 3 or colored < 30: fail("rendered trailing rainbow absent"); return
	print("PASS toy-effects14 actual moving rainbow raster ", skin)

func rainbow_band(color: Color) -> int:
	var red := color.r > 0.6
	var green := color.g > 0.6
	var blue := color.b > 0.6
	if red and not green and not blue and color.g < 0.45 and color.b < 0.45: return 1
	if green and not red and not blue and color.r < 0.45 and color.b < 0.45: return 2
	if blue and not red and not green and color.r < 0.45 and color.g < 0.45: return 3
	if green and blue and color.r < 0.45: return 4
	if red and blue and color.g < 0.45: return 5
	if red and green and color.b < 0.45: return 6
	return 0

func set_walking(pressed: bool) -> void:
	walking = pressed
	var event := InputEventKey.new()
	event.keycode = KEY_W
	event.physical_keycode = KEY_W
	event.pressed = pressed
	root.push_input(event, true)

func fail(message: String) -> void:
	if walking: set_walking(false)
	super.fail(message)
