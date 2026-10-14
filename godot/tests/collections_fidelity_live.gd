extends "res://tests/questlog_fidelity_live.gd"

# Private UDP5801 / existing fb_quest* accounts. Ordinary UI input and real authority.
# Admin grants only the authentic toy item when absent; native bag UseItem learns it.
const FIDELITY_TOY := 32782

func control(name: String) -> Control:
	for node in client.find_children(name, "Control", true, false):
		if node.is_visible_in_tree():
			return node as Control
	return null

func box(name: String, position: Vector2, size: Vector2, owner := "CollectionsJournal") -> bool:
	var node := control(name)
	var parent := control(owner)
	if node == null or parent == null:
		fail("Missing source frame: " + name)
		return false
	var relative := node.global_position - parent.global_position
	if relative.distance_to(position) > 1.0 or node.size.distance_to(size) > 1.0:
		fail("Source rect mismatch %s: %s %s" % [name, relative, node.size])
		return false
	return true

func select_collection_tab(index: int) -> bool:
	if control("CollectionsJournal") == null:
		await tap_collections()
	if not await wait_frames(func(): return control("CollectionsJournalTab%d" % index) != null, "Collections tab"):
		return false
	await click_control(control("CollectionsJournalTab%d" % index))
	await frames(5)
	return true

# Viewport.push_input dispatches controls but leaves the engine cursor at (0, 0).
# Context menus read that cursor, so use the real Input ingestion boundary here.
func click(point: Vector2, button: MouseButton, settle_frames := 3) -> void:
	var motion := InputEventMouseMotion.new()
	motion.position = point
	motion.global_position = point
	Input.parse_input_event(motion)
	await process_frame
	for pressed in [true, false]:
		var event := InputEventMouseButton.new()
		event.position = point
		event.global_position = point
		event.button_index = button
		event.button_mask = (1 << (button - 1)) if pressed else 0
		event.pressed = pressed
		Input.parse_input_event(event)
		await process_frame
	await frames(settle_frames)

func check_shell() -> bool:
	var shell := control("CollectionsJournal")
	if shell == null or shell.size.distance_to(Vector2(703, 606)) > 1.0 or not root.get_visible_rect().encloses(shell.get_global_rect()):
		fail("PortraitFrame shell must fit at source703x606")
		return false
	return box("CollectionsJournalBg", Vector2(2, 21), Vector2(699, 583)) and box("CollectionsJournalTopTileStreaks", Vector2(6, 21), Vector2(695, 43))

func exercise_filter(prefix: String, button: String, menu: String, checkbox: String) -> bool:
	if control(button) == null or control(button).size.distance_to(Vector2(90, 22)) > 1.0:
		fail("Fixed Retail Filter owner missing: " + button)
		return false
	await click_control(control(button))
	if not await wait_frames(func(): return control(menu) != null, prefix + " source Filter menu"):
		return false
	await capture(prefix + "-filter.png")
	await click_control(control(checkbox))
	if not await wait_frames(func(): return control(button + "ResetButton") != null, prefix + " reset available"):
		return false
	await capture(prefix + "-filter-changed.png")
	await click_control(control(button + "ResetButton"))
	if not await wait_frames(func(): return control(button + "ResetButton") == null, prefix + " default filter restored"):
		return false
	if control(menu) != null:
		await click_control(control(button))
	return true

func type_search(name: String, text: String) -> bool:
	var edit := control(name)
	if edit == null:
		fail("Missing source search: " + name)
		return false
	await click_control(edit)
	for pressed in [true, false]:
		var select_all := InputEventKey.new()
		select_all.keycode = KEY_A
		select_all.physical_keycode = KEY_A
		select_all.ctrl_pressed = true
		select_all.pressed = pressed
		root.push_input(select_all, true)
		await process_frame
	await tap(KEY_BACKSPACE)
	for character in text:
		var event := InputEventKey.new()
		event.unicode = character.unicode_at(0)
		event.keycode = OS.find_keycode_from_string(character.to_upper())
		event.physical_keycode = event.keycode
		event.pressed = true
		root.push_input(event, true)
		await process_frame
		event.pressed = false
		root.push_input(event, true)
		await process_frame
	return true

func select_mare() -> bool:
	if not await select_collection_tab(1):
		return false
	if not await wait_frames(func(): return control("MountJournalSearchBox") != null, "Mount Journal"):
		return false
	if not await type_search("MountJournalSearchBox", "Chestnut Mare"):
		return false
	if not await wait_frames(func(): return control("MountJournalRow18") != null, "authored Mare row", 60000):
		return false
	await click_control(control("MountJournalRow18"))
	return true

func check_mount_controls() -> bool:
	if not await select_mare() or not check_shell():
		return false
	if not box("MountJournalRightInset", Vector2(284, 60), Vector2(413, 520)) or not box("MountJournalMountDisplay", Vector2(3, 3), Vector2(407, 514), "MountJournalRightInset"):
		return false
	if not box("MountJournalModelScene", Vector2.ZERO, Vector2(407, 514), "MountJournalMountDisplay"):
		return false
	await capture("mounts-source.png")
	if not await exercise_filter("mounts", "MountJournalFilterButton", "MountJournalFilterMenu", "MountJournalFilter1"):
		return false
	var at := control("MountJournalRow18").get_global_rect().get_center()
	await click(at, MOUSE_BUTTON_RIGHT)
	if not await wait_frames(func(): return control("MountJournalContextMenu") != null, "shared mount context menu"):
		return false
	if control("MountJournalContextMenu").global_position.distance_to(at) > 1.0:
		print("CONTEXT_GEOMETRY expected=", at, " viewport_mouse=", root.get_mouse_position(), " menu=", control("MountJournalContextMenu").global_position)
		await capture("mounts-context-failure.png")
		fail("Mount context did not open at cursor")
		return false
	await capture("mounts-context.png")
	return true

func check_mount_row_highlight() -> bool:
	if not await select_mare():
		return false
	var row := control("MountJournalRow18")
	var motion := InputEventMouseMotion.new()
	motion.position = control("MountJournalModelScene").get_global_rect().get_center()
	motion.global_position = motion.position
	Input.parse_input_event(motion)
	await frames(20)
	await capture("mount-row-selected-idle.png")
	motion.position = row.global_position + Vector2(150, 20)
	motion.global_position = motion.position
	Input.parse_input_event(motion)
	await frames(20)
	await capture("mount-row-selected-hover.png")
	var image := root.get_texture().get_image()
	var white := 0
	var samples := 0
	# Regression boundary: hovering must not replace the source row with a white box.
	for y in range(6, 40, 2):
		for x in range(48, 244, 2):
			var point := Vector2i(row.global_position) + Vector2i(x, y)
			var pixel := image.get_pixelv(point)
			if minf(pixel.r, minf(pixel.g, pixel.b)) > 0.95:
				white += 1
			samples += 1
	print("MOUNT_ROW_PIXELS skin=", OS.get_environment("QF_SKIN"), " white=", white, " samples=", samples)
	if white * 2 > samples:
		fail("MountListButton hover covers selected uncollected row with a white fill")
		return false
	print("PASS MOUNT_ROW_HIGHLIGHT skin=", OS.get_environment("QF_SKIN"))
	return true

func select_owned_pet() -> bool:
	if not await select_collection_tab(2):
		return false
	if not await wait_frames(func(): return not client.find_children("pet_select_owned_*", "Control", true, false).is_empty(), "real owned pet rows", 60000):
		return false
	for node in client.find_children("pet_select_owned_*", "Control", true, false):
		if node.is_visible_in_tree() and String(node.name).trim_prefix("pet_select_owned_").is_valid_int():
			await click_control(node as Control)
			return true
	fail("Owned row is not visible")
	return false

func check_pet_controls() -> bool:
	if not await select_owned_pet() or not check_shell():
		return false
	if not box("PetJournalPetCardBackground", Vector2(289, 63), Vector2(405, 166)):
		return false
	for slot in range(3):
		if not box("PetJournalBattleSlot%d" % slot, Vector2(291, 255 + slot * 107), Vector2(399, 106)):
			return false
	await click_control(control("PetJournalBattleSlot0"))
	if not await wait_frames(func(): return control("PetJournalBattleSlot0HealthValue") != null, "owned loadout health", 60000):
		return false
	if not box("PetJournalBattleSlot0PetTypeIcon", Vector2(0, 46), Vector2(60, 60), "PetJournalBattleSlot0"):
		return false
	var name := control("PetJournalBattleSlot0Name") as Label
	if name == null or name.vertical_alignment != VERTICAL_ALIGNMENT_BOTTOM or control("PetJournalBattleSlot0QualityBorder") == null:
		fail("Source loadout name/quality decoration missing")
		return false
	await capture("pets-source.png")
	return await exercise_filter("pets", "PetJournalFilterButton", "PetJournalFilterMenu", "PetJournalFilter1")

func owned_fidelity_toy() -> bool:
	for toy in client.toybox_state().learned:
		if int(toy.item_id) == FIDELITY_TOY:
			return true
	return false

func ensure_toy_item_learning() -> bool:
	if not await wait_frames(func(): return int(client.toybox_state().catalog_count) > 0, "authoritative toy catalog", 60000):
		return false
	if owned_fidelity_toy():
		return true
	if not admin(["grant-item", character_name(), str(FIDELITY_TOY), "1"]):
		return false
	if not await wait_frames(func(): return not bag_entry(FIDELITY_TOY).is_empty(), "authentic fixture toy in bags"):
		return false
	await click_control(control("MainMenuBarBackpackButton"))
	if not await wait_frames(backpack_shown, "native backpack"):
		return false
	var entry: Dictionary = bag_entry(FIDELITY_TOY)
	var button := control("ContainerFrame%dSlot%d" % [entry.bag, entry.slot])
	if button == null:
		fail("Granted toy bag control missing")
		return false
	await click(button.get_global_rect().get_center(), MOUSE_BUTTON_RIGHT)
	if not await wait_frames(owned_fidelity_toy, "ordinary UseItem learns toy", 60000):
		return false
	await tap(KEY_ESCAPE)
	return true

func check_toy_controls() -> bool:
	if not await select_collection_tab(3) or not check_shell():
		return false
	if not box("ToyBox", Vector2.ZERO, Vector2(703, 606)) or not box("ToyBoxIconsFrame", Vector2(4, 60), Vector2(693, 541)):
		return false
	if not box("ToyBoxBackground", Vector2(4, 4), Vector2(685, 533), "ToyBoxIconsFrame"):
		return false
	await capture("toys-source.png")
	if not await exercise_filter("toys", "ToyBoxFilterDropdown", "ToyBoxFilterMenu", "ToyFilter1"):
		return false
	if not await type_search("ToyBoxSearchBox", "Time-Lost Figurine"):
		return false
	if not await wait_frames(func(): return control("ToySpellButton1Name") != null and (control("ToySpellButton1Name") as Label).text == "Time-Lost Figurine", "real learned toy search"):
		return false
	var at := control("ToySpellButton1").get_global_rect().get_center()
	await click(at, MOUSE_BUTTON_RIGHT)
	if not await wait_frames(func(): return control("ToyBoxContextMenu") != null, "shared toy context"):
		return false
	if control("ToyBoxContextMenu").global_position.distance_to(at) > 1.0:
		print("CONTEXT_GEOMETRY expected=", at, " viewport_mouse=", root.get_mouse_position(), " menu=", control("ToyBoxContextMenu").global_position)
		await capture("toys-context-failure.png")
		fail("Toy context did not open at cursor")
		return false
	await capture("toys-context.png")
	return true

func check_collections_controls() -> bool:
	if not await ensure_toy_item_learning():
		return false
	if not await check_mount_controls() or not await check_pet_controls() or not await check_toy_controls():
		return false
	print("PASS COLLECTIONS_SOURCE_CONTROLS skin=", OS.get_environment("QF_SKIN"), " shell/filter/reset/mount-context/pet-slots/toy-allpoints/context-cursor")
	return true

func visible_models(frame_name: String) -> int:
	var host := control(frame_name)
	if host == null:
		return 0
	var count := 0
	for mesh: MeshInstance3D in host.find_children("*", "MeshInstance3D", true, false):
		if mesh.is_visible_in_tree() and mesh.mesh != null:
			count += 1
	return count

func await_models(frame_name: String) -> bool:
	var deadline := Time.get_ticks_msec() + 30000
	while Time.get_ticks_msec() < deadline:
		if visible_models(frame_name) > 0:
			await frames(30)
			return true
		await process_frame
	return false

func check_collections_models() -> bool:
	if not await select_mare():
		return false
	var mount_ok := await await_models("MountJournalModelScene")
	await capture("mounts-model.png")
	if not await select_owned_pet():
		return false
	await click_control(control("PetJournalBattleSlot0"))
	var card_ok := await await_models("PetJournalPetCardModelScene")
	var slot_ok := await await_models("PetJournalBattleSlot0ModelScene")
	await capture("pets-model.png")
	print("COLLECTIONS_MODEL_OBSERVED skin=", OS.get_environment("QF_SKIN"), " mount=", mount_ok, " card=", card_ok, " slot=", slot_ok)
	if not mount_ok or not card_ok or not slot_ok:
		fail("Source ModelScene mesh proof incomplete; retain data/loader errors, no picture fallback")
		return false
	print("PASS COLLECTIONS_SOURCE_MODELS skin=", OS.get_environment("QF_SKIN"), " existing projection; ModelScene default FOV unsourced")
	return true
