extends "res://tests/modern_crafting_visibility_filters_live.gd"

const CLEAR := "ProfessionsSearchBoxClearButton"
const SEARCH_FIELD := "ProfessionsSearchBox"

func run_test() -> void:
	root.size = Vector2i(1920, 1080)
	root.content_scale_size = Vector2i(1920, 1080)
	root.content_scale_mode = Window.CONTENT_SCALE_MODE_VIEWPORT
	if OS.get_environment("GODOT_TEST_SERVER") != "127.0.0.1:5951" or not OS.get_environment("BANK_ACCOUNT").begins_with("fb_moderncraft_"):
		fail("Search-clear proof requires owned modern crafting accounts on UDP5951")
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
		fail("Asset startup exceeded240s before search-clear proof")
		return
	var error = client.connect_account("127.0.0.1:5951", OS.get_environment("BANK_ACCOUNT"), PASSWORD, false)
	if error != "":
		fail(error)
		return
	if not await enter_world(OS.get_environment("BANK_CHARACTER")):
		return
	Engine.max_fps = 60
	if client.account_state().ui_skin != role:
		fail("Observed skin differs from search-clear fixture")
		return
	if not await open_recipe_book():
		return
	var before := gameplay_state()
	if shown(CLEAR) or control(SEARCH_FIELD).has_focus():
		fail("Initial search must be empty/unfocused with hidden Clear")
		return
	if shown(FAVORITE):
		if role != "Modern" or OS.get_environment("SEARCH_CLEAR_RESUME_FAVORITE") != "1":
			fail("Existing favorite requires explicit Modern resume after the retained interrupted fixture")
			return
		print("SEARCH_CLEAR RESUME existing owner favorite; no repeated favorite request")
	else:
		if not await open_favorite_menu(RECIPE, "Set Favorite", "0-prepare-owner-favorite-menu"):
			return
		await press(MENU)
		if not await wait_until(func(): return shown(FAVORITE), "owner Favorite before search-clear controls"):
			return
	await press(FAVORITE)
	await press(FILTER)
	await press(UNLEARNED)
	await press(FILTER)
	await capture("1-empty-unfocused-clear-hidden")
	await press(SEARCH_FIELD)
	if not await wait_until(func(): return control(SEARCH_FIELD).has_focus() and shown(CLEAR), "empty focused search exposes Clear"):
		return
	if not clear_geometry():
		return
	await capture("2-empty-focused-clear-visible")
	await press(CLEAR)
	if not await cleared():
		return
	await capture("3-empty-clear-releases-focus")
	await replace_field(SEARCH_FIELD, RECIPE_NAME)
	if not await wait_until(func(): return shown(RECIPE) and not shown(FAVORITE) and shown(CLEAR) and control(SEARCH_FIELD).has_focus(), "typed query shows Clear while omitting Favorites"):
		return
	await capture("4-typed-focused-clear-visible")
	await press("ProfessionsQuantity")
	if not await wait_until(func(): return control("ProfessionsQuantity").has_focus() and not control(SEARCH_FIELD).has_focus() and shown(CLEAR), "native EditBox focus transfer retains nonempty search Clear"):
		return
	await capture("5-typed-unfocused-clear-visible")
	await press(CLEAR)
	if not await cleared():
		return
	if not shown(FAVORITE + "Selected") or shown(RECIPE + "Selected") or text("ProfessionsOutputName") != RECIPE_NAME:
		fail("Clear changed selected recipe/favorite instance")
		return
	if shown("ProfessionsUnlearnedLabel"):
		fail("Clear changed the hidden-unlearned filter")
		return
	await press(FILTER)
	if not await filter_checks(true, false):
		return
	await capture("6-cleared-selection-favorite-filter-preserved")
	await press(UNLEARNED)
	await press(FILTER)
	if not await open_favorite_menu(FAVORITE, "Remove Favorite", "7-restore-owner-preference-menu"):
		return
	await press(MENU)
	if not await wait_until(func(): return not shown(FAVORITE) and shown(RECIPE), "restore original unfavorited preference"):
		return
	if gameplay_state() != before or client.spells_state().casting != 0:
		fail("Search Clear changed bags, copper, skill or initiated a cast")
		return
	print("FIXTURE MODERN_CRAFTING_SEARCH_CLEAR_DONE skin=", role, " focus_query_states=4 clears=2 selection=preserved favorites=preserved filters=preserved gameplay_state=unchanged")
	client.free()
	quit(0)

func cleared() -> bool:
	return await wait_until(func(): return str(control(SEARCH_FIELD).get("text")).is_empty() and not control(SEARCH_FIELD).has_focus() and not shown(CLEAR) and shown(FAVORITE), "ClearText empties query/releases focus/hides Clear/restores Favorites")

func clear_geometry() -> bool:
	var clear := control(CLEAR).get_global_rect()
	var field := control(SEARCH_FIELD).get_global_rect()
	var scale := field.size.y / 20.0
	if absf(clear.size.x - 17.0 * scale) > 1.1 or absf(clear.size.y - 17.0 * scale) > 1.1 or absf(clear.end.x - (field.end.x - 3.0 * scale)) > 1.1 or absf(clear.get_center().y - field.get_center().y) > 1.1:
		return fail("Search Clear differs from source17x17 RIGHT-3 centered anchor")
	print("SEARCH_CLEAR GEOMETRY clear=", clear, " search=", field, " scale=", scale)
	return true
