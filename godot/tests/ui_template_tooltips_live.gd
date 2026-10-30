extends "res://tests/tooltips_live.gd"

# Actual authenticated GameClient, never a standalone RegistryUi preview.
# Setup: default Arcane Mage with received Arcane Blast30451, level10.
# Private admin changes only this fixture character's level after each refresh marker.
# Source placement: default UnitFrame owner, SpellBookItem ANCHOR_RIGHT,
# MainMenuBarBackpackButton ANCHOR_LEFT. Existing domain content remains live.
var completion
var receipts: Array[Dictionary] = []
var receipt_path := ""
var current_skin := ""
var injected_pointer := Vector2.ZERO
var motion_serial := 0

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
	print("UI_TEMPLATE_GENERICS_SPELLS ", {"catalog_ready": spells().catalog_ready, "known": spells().known, "bar": spells().bar})
	if not await wait_until(func(): return spells().catalog_ready and spells().known.has(30451), 60000, "Arcane Blast catalog/authority"): return
	for forever in [true]:
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

func enter_world() -> bool:
	if not await wait_until(selection_ready, 60000, "laid-out enabled character controls"):
		print("UI_TEMPLATE_GENERICS_SELECTION_FAILURE ", client.account_state())
		return false
	await wait_frames(15)
	var selection := client.get_node("CharacterSelectUI")
	var card := selection.find_child("CharCard_0", true, false) as Control
	await click(card)
	await wait_frames(5)
	var enter := selection.find_child("EnterWorld", true, false) as BaseButton
	await move_mouse(enter.get_global_rect().get_center())
	print("UI_TEMPLATE_GENERICS_ENTER ", {"card": card.get_global_rect(), "enter": enter.get_global_rect(), "disabled": enter.disabled, "hovered": root.gui_get_hovered_control(), "viewport": root.get_visible_rect()})
	await click(enter)
	if not await wait_until(func(): return client.account_state().screen != "CharacterSelect", 8000, "Enter World click accepted"): return false
	if not await wait_until(func():
		var state: Dictionary = client.account_state()
		return state.screen == "InWorld" and state.local_player_position != null and state.local_server_position != null and state.terrain.pending_count == 0 and not state.terrain.parsed_tiles.is_empty(),
		120000, "entered world and terrain"): return false
	print("UI_TEMPLATE_GENERICS_INWORLD ", client.account_state().selected_character_name)
	await wait_frames(30)
	return true

func selection_ready() -> bool:
	var state: Dictionary = client.account_state()
	if not state.reply_received or state.screen != "CharacterSelect" or state.character_count != 1: return false
	var selection := client.get_node_or_null("CharacterSelectUI")
	if selection == null: return false
	var card := selection.find_child("CharCard_0", true, false) as Control
	var enter := selection.find_child("EnterWorld", true, false) as BaseButton
	return card != null and card.is_visible_in_tree() and card.size.x > 100 and enter != null and enter.is_visible_in_tree() and not enter.disabled and enter.size.x > 100

func player_refresh(level: int) -> bool:
	await hover(control("UnitFramesUI", "PlayerFrame"))
	if not await wait_until(func(): return tooltip().visible and tooltip().title == character and has_line(tooltip(), "Level %d " % level), 4000, "PlayerFrame initial level"): return false
	if not at_default_anchor(tooltip()): return fail_state("Player default anchor", tooltip())
	await capture("player-before.png")
	var pointer := injected_pointer
	var serial := motion_serial
	print("UI_TEMPLATE_GENERICS_REFRESH_ARM ", current_skin, " ", level + 1)
	if not await wait_until(func(): return has_line(tooltip(), "Level %d " % (level + 1)), 10000, "same-pointer authoritative level refresh"): return false
	if injected_pointer.distance_to(pointer) > 0.01 or motion_serial != serial:
		return fail_state("Pointer moved during live refresh", tooltip())
	await capture("player-after.png")
	return true

func spellbook_owner() -> bool:
	await press(KEY_P)
	var owner := control("SpellBookUI", "SpellBookItem30451Button")
	print("UI_TEMPLATE_GENERICS_BOOK ", {"open": spells().spellbook_open, "owner_present": owner != null, "owner_visible": owner.is_visible_in_tree() if owner != null else false, "owner_rect": owner.get_global_rect() if owner != null else null, "injected_pointer": injected_pointer})
	if not await wait_until(func(): return spells().spellbook_open and control("SpellBookUI", "SpellBookItem30451Button") != null, 4000, "Arcane Blast spellbook owner"): return false
	owner = control("SpellBookUI", "SpellBookItem30451Button")
	await hover(owner)
	if not await wait_tooltip("Arcane Blast"): return false
	if not has_line(tooltip(), "Spell ID: 30451"):
		return fail_state("Arcane Blast record", tooltip())
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

func move_mouse(point: Vector2) -> void:
	injected_pointer = point
	motion_serial += 1
	var motion := InputEventMouseMotion.new()
	motion.position = point
	motion.global_position = point
	root.push_input(motion, true)
	Input.warp_mouse(point)
	await wait_frames(3)

func capture(file: String) -> void:
	await RenderingServer.frame_post_draw
	var image := root.get_texture().get_image()
	var path := shots.path_join(current_skin + "-" + file)
	if image == null or image.get_size() != Vector2i(1920, 1080) or image.save_png(path) != OK:
		fail("PNG capture failed or dimensions differ: " + path)
		return
	receipts.append({"skin": current_skin, "phase": file, "tooltip": tooltip(), "injected_pointer": injected_pointer, "motion_serial": motion_serial, "root_mouse_position": root.get_mouse_position(), "capture": path})
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
