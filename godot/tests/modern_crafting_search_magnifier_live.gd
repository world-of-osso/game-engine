extends "res://tests/modern_crafting_visibility_filters_live.gd"

const SEARCH_FIELD := "ProfessionsSearchBox"
const MAGNIFIER := "ProfessionsSearchBoxIcon"
const CLEAR := "ProfessionsSearchBoxClearButton"

func run_test() -> void:
	root.size = Vector2i(1920, 1080)
	root.content_scale_size = Vector2i(1920, 1080)
	root.content_scale_mode = Window.CONTENT_SCALE_MODE_VIEWPORT
	if OS.get_environment("GODOT_TEST_SERVER") != "127.0.0.1:5951" or not OS.get_environment("BANK_ACCOUNT").begins_with("fb_moderncraft_"):
		fail("Magnifier proof requires owned modern crafting accounts on UDP5951")
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
		fail("Asset startup exceeded240s before magnifier proof")
		return
	var error = client.connect_account("127.0.0.1:5951", OS.get_environment("BANK_ACCOUNT"), PASSWORD, false)
	if error != "":
		fail(error)
		return
	if not await enter_world(OS.get_environment("BANK_CHARACTER")):
		return
	Engine.max_fps = 60
	if client.account_state().ui_skin != role:
		fail("Observed skin differs from magnifier fixture")
		return
	if not await open_recipe_book():
		return
	var before := gameplay_state()
	var selected := text("ProfessionsOutputName")
	var favorite := shown(FAVORITE)
	await press(FILTER)
	await press(UNLEARNED)
	await press(FILTER)
	if not await magnifier_state(false, "", 0.6):
		return
	await capture("1-empty-unfocused-dim")
	var dim := await painted_icon()
	await press(SEARCH_FIELD)
	if not await magnifier_state(true, "", 1.0):
		return
	await capture("2-empty-focused-bright")
	var bright := await painted_icon()
	if bright.get_data() == dim.get_data() or icon_energy(bright) <= icon_energy(dim):
		fail("Actual painted magnifier did not brighten with source focus tint")
		return
	await type_text("a")
	if not await magnifier_state(true, "a", 1.0):
		return
	await capture("3-query-focused-bright")
	await press("ProfessionsQuantity")
	if not await magnifier_state(false, "a", 1.0):
		return
	await capture("4-query-unfocused-bright")
	await press(CLEAR)
	if not await magnifier_state(false, "", 0.6):
		return
	await capture("5-clear-restores-dim")
	var restored := await painted_icon()
	if restored.get_data() != dim.get_data():
		fail("Clear did not restore identical painted magnifier pixels")
		return
	if text("ProfessionsOutputName") != selected or shown(FAVORITE) != favorite or shown("ProfessionsUnlearnedLabel"):
		fail("Magnifier state changed selected schematic or visibility filter")
		return
	await press(FILTER)
	if not await filter_checks(true, false):
		return
	await press(UNLEARNED)
	await press(FILTER)
	if gameplay_state() != before or client.spells_state().casting != 0:
		fail("Magnifier proof changed bags, copper, skill or initiated a cast")
		return
	print("FIXTURE MODERN_CRAFTING_SEARCH_MAGNIFIER_DONE skin=", role, " focus_query_states=4 geometry=source tint=source pixels=brighten-and-exact-restore selection=unchanged filters=restored gameplay_state=unchanged")
	client.free()
	quit(0)

func magnifier_state(focused: bool, query: String, expected: float) -> bool:
	if not await wait_until(func(): return control(SEARCH_FIELD).has_focus() == focused and str(control(SEARCH_FIELD).get("text")) == query and absf(icon_tint().r - expected) < 0.001, "magnifier focus/query/source tint"):
		return false
	var tint := icon_tint()
	if absf(tint.g - expected) > 0.001 or absf(tint.b - expected) > 0.001 or absf(tint.a - 1.0) > 0.001:
		return fail("Magnifier RGB/alpha differs from source tint")
	var icon := control(MAGNIFIER).get_global_rect()
	var field := control(SEARCH_FIELD).get_global_rect()
	var scale := field.size.y / 20.0
	if absf(icon.size.x - 10.0 * scale) > 1.1 or absf(icon.size.y - 10.0 * scale) > 1.1 or absf(icon.position.x - field.position.x - scale) > 1.1 or absf(icon.get_center().y - field.get_center().y - scale) > 1.1:
		return fail("Magnifier differs from source10x10 LEFT(+1,-1)")
	print("MAGNIFIER STATE focus=", focused, " query=", query, " tint=", tint, " icon=", icon, " search=", field)
	return true

func icon_tint() -> Color:
	var parts := control(MAGNIFIER).find_children("*", "TextureRect", true, false)
	if parts.size() != 1:
		return Color(-1, -1, -1, -1)
	return parts[0].get_self_modulate()

func painted_icon() -> Image:
	await RenderingServer.frame_post_draw
	var bounds := control(MAGNIFIER).get_global_rect()
	return root.get_texture().get_image().get_region(Rect2i(Vector2i(bounds.position.round()), Vector2i(bounds.size.round())))

func icon_energy(image: Image) -> float:
	var total := 0.0
	for y in range(image.get_height()):
		for x in range(image.get_width()):
			var pixel := image.get_pixel(x, y)
			total += pixel.r + pixel.g + pixel.b
	return total
