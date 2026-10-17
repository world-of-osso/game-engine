extends "res://tests/modern_crafting_knowledge_live.gd"

## Authored currency metadata is displayable; initial grant/recharge origin stay unknown.
func run_test() -> void:
	root.size = Vector2i(1920, 1080)
	root.content_scale_size = Vector2i(1920, 1080)
	root.content_scale_mode = Window.CONTENT_SCALE_MODE_VIEWPORT
	var endpoint := OS.get_environment("GODOT_TEST_SERVER")
	if endpoint != "127.0.0.1:5951" or not OS.get_environment("BANK_ACCOUNT").begins_with("fb_moderncraft_"):
		fail("Concentration proof requires owned accounts on UDP5951")
		return
	role = OS.get_environment("MODERN_CRAFTING_SKIN")
	shots = OS.get_environment("BANK_SHOTS")
	client = load("res://scenes/client.tscn").instantiate()
	root.add_child(client)
	await process_frame
	if not await wait_until(func(): return not client.account_state().assets_starting, "asset startup before authentication"):
		return
	var error = client.connect_account(endpoint, OS.get_environment("BANK_ACCOUNT"), PASSWORD, false)
	if error != "":
		fail(error)
		return
	if not await enter_world(OS.get_environment("BANK_CHARACTER")):
		return
	Engine.max_fps = 60
	if client.account_state().ui_skin != role:
		fail("Observed skin differs from concentration fixture")
		return
	if not await open_recipe_book():
		return
	if not await wait_until(func(): return text("ProfessionsConcentrationAmount") == "—/1000", "server concentration metadata without an initial grant"):
		return
	await wait_frames(30)
	await capture("1-concentration-origin-gap")
	var point := control("ProfessionsConcentrationDisplay").get_global_rect().get_center()
	var motion := InputEventMouseMotion.new()
	motion.position = point
	motion.global_position = point
	root.push_input(motion, true)
	Input.warp_mouse(point)
	if not await wait_until(func(): return client.tooltip_state().visible and client.tooltip_state().title == "Leatherworking Concentration", "Retail currency tooltip"):
		return
	if not str(client.tooltip_state()).contains("CRAFTING_CONCENTRATION_ORIGIN"):
		fail("Currency tooltip must disclose unsourced grant/recharge origin")
		return
	await capture("2-concentration-currency-tooltip")
	print("CONCENTRATION rect=", control("ProfessionsConcentrationDisplay").get_global_rect(), " amount=", text("ProfessionsConcentrationAmount"), " tooltip=", client.tooltip_state())
	if not control("ProfessionsCreate").disabled:
		fail("Concentration metadata cannot enable an unsupported craft")
		return
	print("FIXTURE MODERN_CRAFTING_CONCENTRATION_DONE skin=", role)
	client.free()
	quit(0)
