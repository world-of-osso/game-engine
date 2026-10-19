extends "res://tests/modern_crafting_knowledge_live.gd"

## Run set, then restart the private server and run readback-remove, then restart
## again and run readback-empty. Each invocation is a fresh native client process.
## Existing fixture knowledge375188 is not profession acquisition evidence.
const RECIPE := "ProfessionRecipe375188"
const FAVORITE := "ProfessionFavoriteRecipe375188"
const MENU := "ProfessionsRecipeFavoriteToggle"
const RECIPE_NAME := "Alchemist's Hat"

func run_test() -> void:
	root.size = Vector2i(1920, 1080)
	root.content_scale_size = Vector2i(1920, 1080)
	root.content_scale_mode = Window.CONTENT_SCALE_MODE_VIEWPORT
	if OS.get_environment("GODOT_TEST_SERVER") != "127.0.0.1:5951" or not OS.get_environment("BANK_ACCOUNT").begins_with("fb_moderncraft_"):
		fail("Favorites proof requires owned modern crafting accounts on UDP5951")
		return
	role = OS.get_environment("MODERN_CRAFTING_SKIN")
	shots = OS.get_environment("BANK_SHOTS")
	var baseline_path := OS.get_environment("FAVORITES_BASELINE")
	var phase := OS.get_environment("FAVORITES_PHASE")
	if baseline_path.is_empty() or phase not in ["set", "readback-remove", "readback-empty"]:
		fail("Explicit private baseline path and Favorites phase are required")
		return
	DirAccess.make_dir_recursive_absolute(shots)
	client = load("res://scenes/client.tscn").instantiate()
	root.add_child(client)
	await process_frame
	var deadline := Time.get_ticks_msec() + 240000
	while client.account_state().assets_starting and Time.get_ticks_msec() < deadline:
		await process_frame
	if client.account_state().assets_starting:
		fail("Asset startup exceeded240s before Favorites proof")
		return
	var error = client.connect_account("127.0.0.1:5951", OS.get_environment("BANK_ACCOUNT"), PASSWORD, false)
	if error != "":
		fail(error)
		return
	if not await enter_world(OS.get_environment("BANK_CHARACTER")):
		return
	Engine.max_fps = 60
	if client.account_state().ui_skin != role:
		fail("Observed skin differs from Favorites fixture")
		return
	if not await open_recipe_book():
		return
	var before := gameplay_state()
	if phase == "set":
		if FileAccess.file_exists(baseline_path):
			fail("Do not replace an existing Favorites baseline; inspect previous phase first")
			return
		var file := FileAccess.open(baseline_path, FileAccess.WRITE)
		if file == null:
			fail("Cannot create Favorites conservation baseline")
			return
		file.store_var(before)
		file.close()
		if shown(FAVORITE) or shown("ProfessionsFavorites"):
			fail("Set phase requires an initially unfavorited recipe")
			return
		if not await open_favorite_menu(RECIPE, "Set Favorite", "1-set-menu"):
			return
		await press(MENU)
		if not await wait_until(func(): return shown(FAVORITE) and shown("ProfessionsFavorites") and shown(RECIPE), "authoritative Favorites duplicate and ordinary recipe"):
			return
		await capture("2-favorite-and-ordinary")
		if not await exercise_search_and_selection():
			return
	else:
		var file := FileAccess.open(baseline_path, FileAccess.READ)
		if file == null:
			fail("Missing original Favorites conservation baseline")
			return
		var saved = file.get_var()
		file.close()
		if before != saved:
			fail("Server/client reopen changed bags, money or profession skill")
			return
		if phase == "readback-remove":
			if not await wait_until(func(): return shown(FAVORITE) and shown("ProfessionsFavorites") and shown(RECIPE), "server-persisted favorite on fresh client without re-favoriting"):
				return
			await capture("4-persisted-after-process-reopen")
			if not await open_favorite_menu(FAVORITE, "Remove Favorite", "5-remove-menu"):
				return
			await press(MENU)
			if not await wait_until(func(): return not shown(FAVORITE) and not shown("ProfessionsFavorites") and shown(RECIPE), "authoritative removal retains ordinary recipe"):
				return
			await capture("6-removed-ordinary-retained")
		else:
			if shown(FAVORITE) or shown("ProfessionsFavorites") or not shown(RECIPE):
				fail("Removed favorite returned after second server/client reopen")
				return
			await capture("7-removal-persists-after-process-reopen")
	if gameplay_state() != before or client.spells_state().casting != 0:
		fail("Favorites changed bags, money, profession skill or initiated a cast")
		return
	print("FIXTURE MODERN_CRAFTING_FAVORITES_DONE skin=", role, " phase=", phase, " recipe=375188 gameplay_state=unchanged")
	client.free()
	quit(0)

func gameplay_state() -> Dictionary:
	return {"bags": client.merchant_state().bags.duplicate(true), "money": money(), "skill": text("ProfessionsSkillText")}

func open_favorite_menu(row: String, expected: String, shot: String) -> bool:
	if not shown(row):
		return fail("Missing visible recipe instance: " + row)
	await click_at(control(row).get_global_rect().get_center(), MOUSE_BUTTON_RIGHT)
	if not await wait_until(func(): return shown(MENU) and text(MENU + "Text") == expected, "Retail context menu " + expected):
		return false
	await capture(shot)
	return true

func exercise_search_and_selection() -> bool:
	await press(FAVORITE)
	if not await wait_until(func(): return text("ProfessionsOutputName") == RECIPE_NAME, "favorite instance opens learned schematic"):
		return false
	await press(RECIPE)
	if text("ProfessionsOutputName") != RECIPE_NAME:
		return fail("Ordinary duplicate selects a different schematic")
	await press("ProfessionsFavorites")
	if not await wait_until(func(): return not shown(FAVORITE) and shown(RECIPE), "collapsed Favorites leaves ordinary recipe"):
		return false
	await press("ProfessionsFavorites")
	if not await wait_until(func(): return shown(FAVORITE), "reopened Favorites category"):
		return false
	await replace_field("ProfessionsSearchBox", RECIPE_NAME)
	if not await wait_until(func(): return shown(RECIPE) and not shown(FAVORITE) and not shown("ProfessionsFavorites"), "search omits Favorites but retains matching ordinary recipe"):
		return false
	await capture("3-search-omits-favorites")
	await replace_field("ProfessionsSearchBox", "")
	return await wait_until(func(): return shown(FAVORITE) and shown(RECIPE), "cleared search restores both instances")
