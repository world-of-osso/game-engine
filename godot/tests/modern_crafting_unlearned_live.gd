extends "res://tests/modern_crafting_knowledge_live.gd"

## Physical native trainer acquisition; run only in the lead-granted UDP5951 window.
## full: trainer264578 + fourteen2881 casts, then preview/purchase2153.
## preview: explicit resume after the rank15 preparation already completed.
func run_test() -> void:
	root.size = Vector2i(1920, 1080)
	root.content_scale_size = Vector2i(1920, 1080)
	root.content_scale_mode = Window.CONTENT_SCALE_MODE_VIEWPORT
	if OS.get_environment("GODOT_TEST_SERVER") != "127.0.0.1:5951" or not OS.get_environment("BANK_ACCOUNT").begins_with("fb_moderncraft_"):
		fail("Unlearned proof requires owned modern crafting accounts on UDP5951")
		return
	role = OS.get_environment("MODERN_CRAFTING_SKIN")
	shots = OS.get_environment("BANK_SHOTS")
	client = load("res://scenes/client.tscn").instantiate()
	root.add_child(client)
	await process_frame
	var startup_deadline := Time.get_ticks_msec() + 240000
	while client.account_state().assets_starting and Time.get_ticks_msec() < startup_deadline:
		await process_frame
	if client.account_state().assets_starting:
		fail("Asset startup exceeded240s before unlearned proof")
		return
	var error = client.connect_account("127.0.0.1:5951", OS.get_environment("BANK_ACCOUNT"), PASSWORD, false)
	if error != "":
		fail(error)
		return
	if not await enter_world(OS.get_environment("BANK_CHARACTER")):
		return
	Engine.max_fps = 60
	if client.account_state().ui_skin != role:
		fail("Observed skin differs from unlearned fixture")
		return
	var phase := OS.get_environment("UNLEARNED_PHASE")
	if phase == "full":
		if not await prepare_classic_leatherworking():
			return
	elif phase == "preview":
		await tap(KEY_K)
	else:
		fail("Explicit UNLEARNED_PHASE full or preview is required")
		return
	if not await discover_and_learn_pants():
		return
	print("FIXTURE MODERN_CRAFTING_UNLEARNED_DONE skin=", role)
	client.free()
	quit(0)

func open_simon() -> bool:
	if not await click_pickable(func(node): return str(node.name) == "Simon Tanner", "unit_server_id"):
		return false
	if not await wait_until(func(): return shown("AuctionGossipOption0"), "Simon Tanner gossip"):
		return false
	await press("AuctionGossipOption0")
	if not await wait_until(func(): return shown("ClassTrainerFrame"), "real trainer56 services"):
		return false
	await press("ClassTrainerFilterDropdown")
	for index in [1, 2]:
		if shown("ClassTrainerFilter%dCheck" % index):
			await press("ClassTrainerFilter%d" % index)
	await press("ClassTrainerFilterDropdown")
	return true

func purchase_service(spell: int, cost: int) -> bool:
	var name := "ClassTrainerService%d" % spell
	if not await wait_until(func(): return shown(name), "authored trainer service " + str(spell)):
		return false
	await press(name)
	if not await wait_until(func(): return not control("ClassTrainerTrainButton").disabled, "available real trainer service"):
		return false
	var before := money()
	await press("ClassTrainerTrainButton")
	if spell == 264578:
		if not await wait_until(func(): return money() == before - cost or shown("StaticPopup1Button1"), "base-profession confirmation or payment"):
			return false
		if shown("StaticPopup1Button1"):
			await press("StaticPopup1Button1")
	return await wait_until(func(): return money() == before - cost, "authoritative trainer payment " + str(cost))

func prepare_classic_leatherworking() -> bool:
	if bag_count(2934) != 42 or bag_count(2318) != 0 or bag_count(2320) != 1:
		return fail("Preparation requires explicit fixture bags:42 scraps,0 leather,1 thread")
	if not await open_simon():
		return false
	if not await purchase_service(264578, 10):
		return false
	await tap(KEY_ESCAPE)
	if not await open_recipe_book():
		return false
	await replace_field("ProfessionsSearchBox", "Light Leather")
	if not await wait_until(func(): return shown("ProfessionRecipe2881"), "learned Light Leather"):
		return false
	await press("ProfessionRecipe2881")
	if not await wait_until(func(): return not control("ProfessionsCreateAll").disabled, "fourteen available Light Leather casts"):
		return false
	await press("ProfessionsCreateAll")
	if not await wait_until(func(): return bag_count(2934) == 0 and bag_count(2318) == 14 and text("ProfessionsSkillText").contains("15/300"), "actual14 casts consume42 scraps and raise rank1 to15"):
		return false
	print("UNLEARNED PREPARATION_DONE rank15 leather14; resume phase=preview if interrupted")
	return true

func discover_and_learn_pants() -> bool:
	if not await wait_until(func(): return shown("ProfessionsSearchBox"), "profession list"):
		return false
	await replace_field("ProfessionsSearchBox", "Handstitched Leather Pants")
	if not await wait_until(func(): return shown("ProfessionRecipe2153") and shown("ProfessionsUnlearnedLabel"), "visible unlearned recipe2153"):
		return false
	await press("ProfessionRecipe2153")
	if not await wait_until(func(): return shown("ProfessionsRecipeSourceButton") and text("ProfessionsOutputName") == "Handstitched Leather Pants", "selectable unlearned schematic and sourced acquisition header"):
		return false
	if not control("ProfessionsCreate").disabled or not control("ProfessionsCreateAll").disabled:
		return fail("Unlearned recipe must disable both craft controls")
	await capture("1-unlearned-selected-disabled")
	var point := control("ProfessionsRecipeSourceButton").get_global_rect().get_center()
	var motion := InputEventMouseMotion.new()
	motion.position = point
	motion.global_position = point
	root.push_input(motion, true)
	Input.warp_mouse(point)
	if not await wait_until(func(): return client.tooltip_state().visible and str(client.tooltip_state()).contains("Trainer 56: 50 Copper"), "source trainer56 acquisition tooltip"):
		return false
	await capture("2-trainer-source-tooltip")
	var bags: Array = client.merchant_state().bags.duplicate(true)
	var before := money()
	var skill := text("ProfessionsSkillText")
	var successes: Array = client.spells_state().cast_success.duplicate(true)
	var failure_seq: int = client.spells_state().cast_failure_seq
	await click_at(control("ProfessionsCreate").get_global_rect().get_center(), MOUSE_BUTTON_LEFT)
	await click_at(control("ProfessionsCreateAll").get_global_rect().get_center(), MOUSE_BUTTON_LEFT)
	await wait_frames(120)
	if client.merchant_state().bags != bags or money() != before or text("ProfessionsSkillText") != skill:
		return fail("Disabled preview clicks changed inventory, copper or skill")
	if client.spells_state().casting != 0 or client.spells_state().cast_success != successes or client.spells_state().cast_failure_seq != failure_seq:
		return fail("Disabled preview clicks emitted a cast or refusal")
	await capture("3-disabled-clicks-conserve-state")
	await tap(KEY_ESCAPE)
	if not await open_simon():
		return false
	if not await purchase_service(2153, 50):
		return false
	await tap(KEY_ESCAPE)
	await tap(KEY_K)
	if not await wait_until(func(): return shown("ProfessionsSearchBox"), "reopened profession book"):
		return false
	await replace_field("ProfessionsSearchBox", "Handstitched Leather Pants")
	if not await wait_until(func(): return shown("ProfessionRecipe2153") and not shown("ProfessionsUnlearnedLabel"), "trainer-learned recipe moves into learned group"):
		return false
	await press("ProfessionRecipe2153")
	if not await wait_until(func(): return not control("ProfessionsCreate").disabled and not shown("ProfessionsRecipeSourceButton"), "known schematic enables crafting without unlearned source header"):
		return false
	if client.merchant_state().bags != bags or money() != before - 50 or text("ProfessionsSkillText") != skill:
		return fail("Trainer acquisition changed more than knowledge and50 copper")
	await capture("4-learned-after-real-trainer")
	print("UNLEARNED ACQUIRED2153 trainer56 cost50 rank15 preserved; bags unchanged")
	return true
