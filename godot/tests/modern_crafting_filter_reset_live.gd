extends "res://tests/modern_crafting_visibility_filters_live.gd"

## Reset only the two implemented visibility flags; source-owned search/selection
## and favorite preferences survive. No grants/crafts or other filter families.
const RESET := "ProfessionsRecipeFilterDropdownResetButton"

func run_test() -> void:
	root.size = Vector2i(1920, 1080)
	root.content_scale_size = Vector2i(1920, 1080)
	root.content_scale_mode = Window.CONTENT_SCALE_MODE_VIEWPORT
	if OS.get_environment("GODOT_TEST_SERVER") != "127.0.0.1:5951" or not OS.get_environment("BANK_ACCOUNT").begins_with("fb_moderncraft_"):
		fail("Reset proof requires owned modern crafting accounts on UDP5951")
		return
	role = OS.get_environment("MODERN_CRAFTING_SKIN")
	shots = OS.get_environment("BANK_SHOTS")
	DirAccess.make_dir_recursive_absolute(shots)
	client = load("res://scenes/client.tscn").instantiate()
	root.add_child(client)
	await process_frame
	var deadline := Time.get_ticks_msec() + 240000
	while client.account_state().assets_starting and Time.get_ticks_msec() < deadline:
		await process_frame
	if client.account_state().assets_starting:
		fail("Asset startup exceeded240s before Reset proof")
		return
	var error = client.connect_account("127.0.0.1:5951", OS.get_environment("BANK_ACCOUNT"), PASSWORD, false)
	if error != "":
		fail(error)
		return
	if not await enter_world(OS.get_environment("BANK_CHARACTER")):
		return
	Engine.max_fps = 60
	if client.account_state().ui_skin != role:
		fail("Observed skin differs from Reset fixture")
		return
	if not await open_recipe_book():
		return
	var before := gameplay_state()
	if shown(FAVORITE):
		fail("Reset proof requires the prior proof's unfavorited initial state")
		return
	unlearned_row = first_visible_unlearned_row()
	if unlearned_row.is_empty():
		fail("Owner projection has no visible unlearned control recipe")
		return
	if not await open_favorite_menu(RECIPE, "Set Favorite", "0-prepare-owner-favorite-menu"):
		return
	await press(MENU)
	if not await visibility(true, true):
		return
	await press(FAVORITE)
	if not await wait_until(func(): return shown(FAVORITE + "Selected"), "selected favorite instance before Reset"):
		return
	await replace_field("ProfessionsSearchBox", RECIPE_NAME)
	await press(FILTER)
	if not await filter_checks(true, true):
		return
	if shown(RESET):
		fail("Active search alone must not expose Reset")
		return
	await capture("1-default-search-reset-hidden")
	await press(FILTER)
	var index := 0
	for hidden in [[true, false], [false, true], [true, true]]:
		await press(FILTER)
		if hidden[0]:
			await press(LEARNED)
		if hidden[1]:
			await press(UNLEARNED)
		if not await filter_checks(not hidden[0], not hidden[1]):
			return
		if not await wait_until(func(): return shown(RESET), "Reset visible for nondefault flags"):
			return
		if not reset_geometry():
			return
		await capture(str(2 + index * 2) + "-nondefault-" + str(index) + "-reset-visible")
		await press(FILTER)
		await press(RESET)
		if not await wait_until(func(): return not shown(RESET) and shown(RECIPE), "one Reset click restores learned visibility and hides Reset"):
			return
		if str(control("ProfessionsSearchBox").get("text")) != RECIPE_NAME or text("ProfessionsOutputName") != RECIPE_NAME:
			fail("Reset changed search text or selected recipe")
			return
		if shown(FAVORITE) or shown("ProfessionsFavorites"):
			fail("Reset changed search's Favorites omission")
			return
		await press(FILTER)
		if not await filter_checks(true, true):
			return
		await capture(str(3 + index * 2) + "-reset-" + str(index) + "-defaults-search-preserved")
		await press(FILTER)
		await replace_field("ProfessionsSearchBox", "")
		if not await visibility(true, true):
			return
		if not shown(FAVORITE + "Selected") or shown(RECIPE + "Selected"):
			fail("Reset changed favorite/ordinary instance selection")
			return
		await replace_field("ProfessionsSearchBox", RECIPE_NAME)
		index += 1
	await replace_field("ProfessionsSearchBox", "")
	if not await visibility(true, true):
		return
	await capture("8-favorite-and-selection-preserved")
	if not await open_favorite_menu(FAVORITE, "Remove Favorite", "9-restore-owner-preference-menu"):
		return
	await press(MENU)
	if not await wait_until(func(): return not shown(FAVORITE) and shown(RECIPE), "restore original unfavorited preference"):
		return
	if gameplay_state() != before or client.spells_state().casting != 0:
		fail("Reset changed bags, money, skill or initiated a cast")
		return
	print("FIXTURE MODERN_CRAFTING_FILTER_RESET_DONE skin=", role, " resets=3 search=preserved selection=preserved favorites=preserved gameplay_state=unchanged")
	client.free()
	quit(0)

func reset_geometry() -> bool:
	var reset := control(RESET).get_global_rect()
	var owner := control(FILTER).get_global_rect()
	var scale := owner.size.y / 18.0
	var expected := Vector2(owner.end.x - 3.0 * scale, owner.position.y)
	if absf(reset.size.x - 23.0 * scale) > 1.1 or absf(reset.size.y - 23.0 * scale) > 1.1 or reset.get_center().distance_to(expected) > 1.1:
		return fail("Reset differs from UIResetButtonTemplate23px/CENTER TOPRIGHT -3,0")
	print("RESET GEOMETRY reset=", reset, " owner=", owner, " scale=", scale)
	return true
