extends "res://tests/modern_crafting_knowledge_live.gd"

## Real owner catalog; both broad matches must remain more than two pages.
## No profession switching, fixture recipe grants or server mutations.
const SCROLL_LIST := "ProfessionsRecipeScrollFrame"
const SCROLL_CHILD := "ProfessionsRecipeScrollChildFrame"

func run_test() -> void:
	root.size = Vector2i(1920, 1080)
	root.content_scale_size = Vector2i(1920, 1080)
	root.content_scale_mode = Window.CONTENT_SCALE_MODE_VIEWPORT
	if OS.get_environment("GODOT_TEST_SERVER") != "127.0.0.1:5951" or not OS.get_environment("BANK_ACCOUNT").begins_with("fb_moderncraft_"):
		fail("Search-scroll proof requires owned modern crafting accounts on UDP5951")
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
		fail("Asset startup exceeded240s before search-scroll proof")
		return
	var error = client.connect_account("127.0.0.1:5951", OS.get_environment("BANK_ACCOUNT"), PASSWORD, false)
	if error != "":
		fail(error)
		return
	if not await enter_world(OS.get_environment("BANK_CHARACTER")):
		return
	Engine.max_fps = 60
	if client.account_state().ui_skin != role:
		fail("Observed skin differs from search-scroll fixture")
		return
	if not await open_recipe_book():
		return
	var bags: Array = client.merchant_state().bags.duplicate(true)
	var copper := money()
	var skill := text("ProfessionsSkillText")
	var selected := text("ProfessionsOutputName")
	if not multipage("initial owner catalog"):
		return
	if not await scroll_deep():
		return
	var initial_offset := child_offset()
	await wait_frames(120)
	if absf(child_offset() - initial_offset) > 1.0:
		fail("Unchanged non-search refresh lost the user's scroll position")
		return
	await capture("1-initial-deep-offset-retained")
	var index := 2
	for query in ["a", "e"]:
		await replace_field("ProfessionsSearchBox", query)
		if not await wait_until(func(): return absf(child_offset() - 5.0) < 1.0, "new nonempty query discards old offset"):
			return
		if not multipage("matching query " + query):
			return
		if shown("ProfessionsFavorites") or text("ProfessionsOutputName") != selected:
			fail("Search-scroll changed selection or search's Favorites omission")
			return
		await capture(str(index) + "-query-" + query + "-top-with-multiple-pages")
		index += 1
		if not await scroll_deep():
			return
		var offset := child_offset()
		await wait_frames(120)
		if absf(child_offset() - offset) > 1.0:
			fail("Unchanged query prevents scrolling its multi-page results")
			return
		await capture(str(index) + "-query-" + query + "-user-scroll-retained")
		index += 1
	if client.merchant_state().bags != bags or money() != copper or text("ProfessionsSkillText") != skill or client.spells_state().casting != 0:
		fail("Search-scroll changed gameplay state or initiated a cast")
		return
	print("FIXTURE MODERN_CRAFTING_SEARCH_SCROLL_DONE skin=", role, " queries=2 multipage=proved retention=proved selection=unchanged gameplay_state=unchanged")
	client.free()
	quit(0)

func child_offset() -> float:
	return control(SCROLL_CHILD).position.y

func multipage(label: String) -> bool:
	var extent := control(SCROLL_CHILD).size.y
	var viewport := control(SCROLL_LIST).size.y
	if extent <= viewport * 2.0:
		return fail(label + " requires more than two actual pages; clamping is not reset proof")
	print("SEARCH_SCROLL EXTENT ", label, " content=", extent, " viewport=", viewport, " offset=", child_offset())
	return true

func scroll_deep() -> bool:
	var point := control(SCROLL_LIST).get_global_rect().get_center()
	for tick in range(18):
		await click_at(point, MOUSE_BUTTON_WHEEL_DOWN)
	return await wait_until(func(): return child_offset() < -100.0, "physical wheel establishes a deep scroll offset")
