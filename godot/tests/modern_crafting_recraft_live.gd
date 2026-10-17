extends "res://tests/modern_crafting_knowledge_live.gd"

## Disabled owned-item input is not recraft eligibility or operation support.
func run_test() -> void:
	root.size = Vector2i(1920, 1080)
	root.content_scale_size = Vector2i(1920, 1080)
	root.content_scale_mode = Window.CONTENT_SCALE_MODE_VIEWPORT
	var endpoint := OS.get_environment("GODOT_TEST_SERVER")
	if endpoint != "127.0.0.1:5951" or not OS.get_environment("BANK_ACCOUNT").begins_with("fb_moderncraft_"):
		fail("Recraft proof requires owned accounts on UDP5951")
		return
	role = OS.get_environment("MODERN_CRAFTING_SKIN") + "-recraft"
	shots = OS.get_environment("BANK_SHOTS")
	client = load("res://scenes/client.tscn").instantiate()
	root.add_child(client)
	await process_frame
	var startup_deadline := Time.get_ticks_msec() + 240000
	while client.account_state().assets_starting and Time.get_ticks_msec() < startup_deadline:
		await process_frame
	if client.account_state().assets_starting:
		fail("Asset startup exceeded240s before recraft authentication")
		return
	var error = client.connect_account(endpoint, OS.get_environment("BANK_ACCOUNT"), PASSWORD, false)
	if error != "":
		fail(error)
		return
	if not await enter_world(OS.get_environment("BANK_CHARACTER")):
		return
	Engine.max_fps = 60
	if client.account_state().ui_skin != OS.get_environment("MODERN_CRAFTING_SKIN"):
		fail("Observed skin differs from recraft fixture")
		return
	var before := bag_count(22658)
	if before != 3:
		fail("Expected three persisted crafted customer items, not a new fixture grant")
		return
	if not await open_recipe_book():
		return
	await press("ProfessionsRecraftRecipe")
	if not await wait_until(func(): return shown("ProfessionsRecraftInputSlot") and shown("ProfessionsRecraftSubmit"), "owned-item recraft schematic"):
		return
	var status := text("ProfessionsRecraftStatus")
	if not status.contains("owned-item lineage") or not status.contains("No item will be consumed"):
		fail("Recraft schematic must disclose unavailable operation/lineage")
		return
	if not control("ProfessionsRecraftInputSlot").disabled or not control("ProfessionsRecraftSubmit").disabled:
		fail("Unsupported recraft input and submission must be disabled")
		return
	await capture("1-disabled-owned-input")
	await click(control("ProfessionsRecraftInputSlot"))
	await click(control("ProfessionsRecraftSubmit"))
	await wait_frames(60)
	if bag_count(22658) != before or text("ProfessionsRecraftStatus") != status:
		fail("Disabled recraft clicks changed the owned bag item or refusal state")
		return
	await capture("2-disabled-clicks-conserve-items")
	print("RECRAFT input_disabled=", control("ProfessionsRecraftInputSlot").disabled, " submit_disabled=", control("ProfessionsRecraftSubmit").disabled, " owned22658_before=", before, " after=", bag_count(22658), " status=", status)
	print("FIXTURE MODERN_CRAFTING_RECRAFT_DONE ", role)
	client.free()
	quit(0)
