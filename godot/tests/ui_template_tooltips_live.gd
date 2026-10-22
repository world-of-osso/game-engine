extends "res://tests/tooltips_live.gd"

# Actual authenticated GameClient, never a standalone RegistryUi preview.
# Private admin changes only this fixture character's level after each refresh marker.
# Source placement: default UnitFrame owner, SpellBookItem ANCHOR_RIGHT,
# MainMenuBarBackpackButton ANCHOR_LEFT. Existing domain content remains live.
var completion
var receipts: Array[Dictionary] = []
var receipt_path := ""
var current_skin := ""

func run_test() -> void:
	root.size = Vector2i(1920, 1080)
	root.content_scale_size = Vector2i.ZERO
	completion = preload("res://tests/completion.gd").new(self, 10)
	var endpoint := OS.get_environment("GODOT_TEST_SERVER")
	var account := OS.get_environment("TOOLTIP_ACCOUNT")
	character = OS.get_environment("TOOLTIP_CHARACTER")
	shots = OS.get_environment("TOOLTIP_SHOTS")
	receipt_path = OS.get_environment("TOOLTIP_RECEIPTS")
	if endpoint != "127.0.0.1:5993" or not account.begins_with("fb_ui_generics_") or character.is_empty() or shots.is_empty() or receipt_path.is_empty():
		fail("Require owned5993/fb_ui_generics_* account, character, PNG and receipt destinations")
		return
	if DirAccess.make_dir_recursive_absolute(shots) != OK:
		fail("Cannot create PNG directory")
		return
	client = load("res://scenes/client.tscn").instantiate()
	root.add_child(client)
	var error: String = client.connect_account(endpoint, account, PASSWORD, false)
	if not error.is_empty():
		fail(error)
		return
	if not await enter_world(): return
	if not await wait_until(func(): return spells().catalog_ready and spells().known.has(133), 60000, "Fireball catalog/authority"): return
	for forever in [false, true]:
		current_skin = "forever" if forever else "modern"
		error = client.preview_world_skin(forever)
		if not error.is_empty():
			fail(error)
			return
		await wait_frames(15)
		if not await player_refresh(11 if forever else 10): return
		if not await spellbook_owner(): return
		if not await backpack_owner(): return
		await move_mouse(Vector2(960, 400))
		if not await wait_until(func(): return not tooltip().visible, 3000, "leave clears tooltip"): return
		await capture("leave.png")
	client.free()
	completion.finish(true)

func player_refresh(level: int) -> bool:
	await hover(control("UnitFramesUI", "PlayerFrame"))
	if not await wait_until(func(): return tooltip().visible and tooltip().title == character and has_line(tooltip(), "Level %d " % level), 4000, "PlayerFrame initial level"): return false
	if not at_default_anchor(tooltip()): return fail_state("Player default anchor", tooltip())
	await capture("player-before.png")
	var pointer := root.get_mouse_position()
	print("UI_TEMPLATE_GENERICS_REFRESH_ARM ", current_skin, " ", level + 1)
	if not await wait_until(func(): return has_line(tooltip(), "Level %d " % (level + 1)), 10000, "same-pointer authoritative level refresh"): return false
	if root.get_mouse_position().distance_to(pointer) > 0.01:
		return fail_state("Pointer moved during live refresh", tooltip())
	await capture("player-after.png")
	return true

func spellbook_owner() -> bool:
	await press(KEY_P)
	if not await wait_until(func(): return spells().spellbook_open and control("SpellBookUI", "SpellBookItem133Button") != null, 4000, "Fireball spellbook owner"): return false
	var owner := control("SpellBookUI", "SpellBookItem133Button")
	await hover(owner)
	if not await wait_tooltip("Fireball"): return false
	if not has_line(tooltip(), "Spell ID: 133"):
		return fail_state("Fireball record", tooltip())
	var rect := ui_rect(owner)
	if absf(tooltip().rect[0] - rect.end.x) > 1.5 or absf(tooltip().rect[1] + tooltip().rect[3] - rect.position.y) > 1.5:
		return fail_state("Spellbook Right anchor", tooltip())
	await capture("spellbook.png")
	await press(KEY_P)
	return await wait_until(func(): return not spells().spellbook_open, 3000, "spellbook close")

func backpack_owner() -> bool:
	var owner := control("BagsUI", "MainMenuBarBackpackButton")
	await hover(owner)
	if not await wait_tooltip("Backpack"): return false
	var rect := ui_rect(owner)
	if absf(tooltip().rect[0] + tooltip().rect[2] - rect.position.x) > 1.5 or absf(tooltip().rect[1] + tooltip().rect[3] - rect.position.y) > 1.5:
		return fail_state("Backpack Left anchor", tooltip())
	await capture("backpack.png")
	return true

func capture(file: String) -> void:
	await RenderingServer.frame_post_draw
	var image := root.get_texture().get_image()
	var path := shots.path_join(current_skin + "-" + file)
	if image == null or image.get_size() != Vector2i(1920, 1080) or image.save_png(path) != OK:
		fail("PNG capture failed or dimensions differ: " + path)
		return
	receipts.append({"skin": current_skin, "phase": file, "tooltip": tooltip(), "pointer": root.get_mouse_position(), "capture": path})
	var output := FileAccess.open(receipt_path, FileAccess.WRITE)
	if output == null:
		fail("Cannot write receipt file")
		return
	output.store_string(JSON.stringify(receipts, "\t"))
	output.close()
	print("UI_TEMPLATE_GENERICS_NATIVE ", current_skin, " ", file, " ", tooltip())
	completion.case_done()

func fail(message: String) -> void:
	if completion != null:
		completion.fail(message)
	else:
		push_error(message)
		quit(1)
