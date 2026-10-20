extends "res://tests/modern_crafting_favorites_live.gd"

## Existing known375188 and an owner-projected unlearned row; no grants or crafts.
## Run each skin once on the lead-granted private5951 window after landing.
const FILTER := "ProfessionsRecipeFilterDropdown"
const LEARNED := "ProfessionsFilterLearned"
const UNLEARNED := "ProfessionsFilterUnlearned"
var unlearned_row := ""

func run_test() -> void:
	root.size = Vector2i(1920, 1080)
	root.content_scale_size = Vector2i(1920, 1080)
	root.content_scale_mode = Window.CONTENT_SCALE_MODE_VIEWPORT
	if OS.get_environment("GODOT_TEST_SERVER") != "127.0.0.1:5951" or not OS.get_environment("BANK_ACCOUNT").begins_with("fb_moderncraft_"):
		fail("Visibility proof requires owned modern crafting accounts on UDP5951")
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
		fail("Asset startup exceeded240s before visibility proof")
		return
	var error = client.connect_account("127.0.0.1:5951", OS.get_environment("BANK_ACCOUNT"), PASSWORD, false)
	if error != "":
		fail(error)
		return
	if not await enter_world(OS.get_environment("BANK_CHARACTER")):
		return
	Engine.max_fps = 60
	if client.account_state().ui_skin != role:
		fail("Observed skin differs from visibility fixture")
		return
	if not await open_recipe_book():
		return
	var before := gameplay_state()
	if shown(FAVORITE):
		fail("Visibility fixture requires the prior proof's unfavorited initial state")
		return
	unlearned_row = first_visible_unlearned_row()
	if unlearned_row.is_empty():
		fail("Owner projection has no visible unlearned row to filter")
		return
	if not await open_favorite_menu(RECIPE, "Set Favorite", "0-source-favorite-menu"):
		return
	await press(MENU)
	if not await visibility(true, true):
		return
	await press(FILTER)
	if not await filter_checks(true, true):
		return
	if not filter_geometry():
		return
	await capture("1-both-default-checked")
	await press(LEARNED)
	if not await visibility(false, true) or not await filter_checks(false, true):
		return
	await capture("2-unlearned-only")
	await press(UNLEARNED)
	if not await visibility(false, false) or not await filter_checks(false, false):
		return
	if not ordinary_rows().is_empty():
		fail("Both unchecked must leave no ordinary recipe rows")
		return
	await capture("3-both-hidden")
	await press(LEARNED)
	if not await visibility(true, false) or not await filter_checks(true, false):
		return
	await capture("4-learned-only-with-favorite")
	await press(UNLEARNED)
	if not await visibility(true, true) or not await filter_checks(true, true):
		return
	await capture("5-both-restored")
	await press(FILTER)
	await replace_field("ProfessionsSearchBox", RECIPE_NAME)
	if not await wait_until(func(): return shown(RECIPE) and not shown(FAVORITE) and not shown("ProfessionsFavorites"), "search retains ordinary match and omits Favorites"):
		return
	await capture("6-search-still-omits-favorites")
	await replace_field("ProfessionsSearchBox", "")
	if not await visibility(true, true):
		return
	await press(FILTER)
	await press(LEARNED)
	if not await visibility(false, true):
		return
	await tap(KEY_ESCAPE)
	if not await wait_until(func(): return not shown("ProfessionsFrame"), "close nondefault profession filters"):
		return
	if not await open_recipe_book():
		return
	if not await visibility(true, true):
		return
	await press(FILTER)
	if not await filter_checks(true, true):
		return
	await capture("7-reopened-defaults-no-filter-persistence")
	await press(FILTER)
	if not await open_favorite_menu(FAVORITE, "Remove Favorite", "8-restore-favorite-preference-menu"):
		return
	await press(MENU)
	if not await wait_until(func(): return not shown(FAVORITE) and shown(RECIPE), "restore original unfavorited owner state"):
		return
	if gameplay_state() != before or client.spells_state().casting != 0:
		fail("Recipe visibility changed bags, money, skill or initiated a cast")
		return
	print("FIXTURE MODERN_CRAFTING_VISIBILITY_FILTERS_DONE skin=", role, " combinations=4 search=unchanged reopen=defaults gameplay_state=unchanged unknown=", unlearned_row)
	client.free()
	quit(0)

func ordinary_rows() -> Array:
	return client.find_children("ProfessionRecipe*", "Control", true, false).filter(func(node): return str(node.name).trim_prefix("ProfessionRecipe").is_valid_int() and node.is_visible_in_tree())

func first_visible_unlearned_row() -> String:
	if not shown("ProfessionsUnlearnedLabel"):
		return ""
	var after := control("ProfessionsUnlearnedLabel").get_global_rect().end.y
	var viewport := control("ProfessionsRecipeScrollFrame").get_global_rect()
	var rows := ordinary_rows().filter(func(node): return node.get_global_rect().position.y >= after and viewport.has_point(node.get_global_rect().get_center()))
	rows.sort_custom(func(a, b): return a.get_global_rect().position.y < b.get_global_rect().position.y)
	return str(rows[0].name) if not rows.is_empty() else ""

func visibility(learned: bool, unlearned: bool) -> bool:
	return await wait_until(func(): return shown(RECIPE) == learned and shown(FAVORITE) == learned and shown("ProfessionsFavorites") == learned and shown(unlearned_row) == unlearned and shown("ProfessionsUnlearnedLabel") == unlearned, "learned=" + str(learned) + " unlearned=" + str(unlearned) + " favorite follows learned")

func filter_checks(learned: bool, unlearned: bool) -> bool:
	return await wait_until(func(): return shown(LEARNED) and shown(UNLEARNED) and shown(LEARNED + "Tick") == learned and shown(UNLEARNED + "Tick") == unlearned, "visible authoritative filter checkmarks")

func filter_geometry() -> bool:
	var filter_rect := control(FILTER).get_global_rect()
	var search := control("ProfessionsSearchBox").get_global_rect()
	var background := control("ProfessionsRecipeListBackground").get_global_rect()
	var scale := filter_rect.size.y / 18.0
	if absf(filter_rect.end.x - (background.end.x - 8.0 * scale)) > 1.1 or absf(filter_rect.position.y - (background.position.y + 9.0 * scale)) > 1.1:
		return fail("Filter does not follow authored TOPRIGHT -8,-9 anchor")
	if absf(search.end.x - (filter_rect.position.x - 4.0 * scale)) > 1.1:
		return fail("Native search/filter gap differs from authored4px")
	print("VISIBILITY GEOMETRY filter=", filter_rect, " search=", search, " background=", background, " scale=", scale)
	return true
