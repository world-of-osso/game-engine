extends "res://tests/world_quest_flow.gd"

# Reuses real Northshire acceptance, progress and POI steps. Run with:
# QF_STEPS=check_skin,check_pickup,check_progress,check_map_objectives,check_quest_log_fidelity
# QF_SKIN=modern|forever. Owned private server only; no invented quest snapshots.
func check_skin() -> bool:
	var error: String = client.preview_world_skin(OS.get_environment("QF_SKIN") == "forever")
	if not error.is_empty():
		fail(error)
		return false
	return true

func check_quest_log_fidelity() -> bool:
	await tap(KEY_L)
	if not await wait_quest(func(s): return s.log_open, "L opens map quest dock"):
		return false
	if client.get_node_or_null("QuestLogUI") != null:
		fail("Standalone QuestLogUI survived migration")
		return false
	var map := client.get_node_or_null("WorldMapUI")
	if map == null or not quest_geometry(false):
		return false
	await capture("fidelity-list.png")
	await click_control(quest_control("WorldMapUI", "QuestLogTitle%d" % QUEST))
	if not await wait_frames(func(): return client.world_map_state().get("quest_details", false), "details replace list"):
		return false
	if quest_control("WorldMapUI", "QuestLogTitle%d" % QUEST) != null or not quest_geometry(true):
		fail("Quest list remains alongside details")
		return false
	await capture("fidelity-details.png")

	await click_control(quest_control("WorldMapUI", "QuestMapFrameTrackButton"))
	if not await wait_quest(func(s): return not Array(s.watched).has(QUEST), "server untrack"):
		return false
	await click_control(quest_control("WorldMapUI", "QuestMapFrameTrackButton"))
	if not await wait_quest(func(s): return Array(s.watched).has(QUEST), "server track"):
		return false
	var share := quest_control("WorldMapUI", "QuestMapFrameShareButton") as BaseButton
	if share == null or not share.disabled:
		fail("Retail Share must be disabled for this non-pushable solo quest")
		return false
	await capture("fidelity-share-disabled.png")

	await click_control(quest_control("WorldMapUI", "QuestMapFrameBackButton"))
	if not await wait_frames(func(): return not client.world_map_state().get("quest_details", true), "Back returns list"):
		return false
	await tap(KEY_L)
	if not await wait_quest(func(s): return not s.log_open, "L closes dock owner"):
		return false
	var poi := client.find_child("QuestBlock%dPOIButton" % QUEST, true, false) as Control
	if poi == null:
		fail("Tracked quest POI link missing")
		return false
	await click_control(poi)
	if not await wait_frames(func(): return client.world_map_state().get("quest_details", false), "tracker POI opens same dock details"):
		return false
	await capture("fidelity-tracker-link.png")

	await click_control(quest_control("WorldMapUI", "WorldMapMaximizeMinimizeButton"))
	if not await wait_frames(func(): return client.world_map_state().get("maximized", false), "map maximized"):
		return false
	await tap(KEY_L)
	if not await wait_frames(func(): return client.quest_state().log_open and not client.world_map_state().maximized, "L restores minimized dock"):
		return false
	await click_control(quest_control("WorldMapUI", "QuestMapFrameAbandonButton"))
	await frames(3)
	var yes := quest_control("StaticPopupUI", "StaticPopup1Button1")
	if yes == null or not yes.is_visible_in_tree():
		fail("Abandon confirmation missing")
		return false
	await capture("fidelity-abandon-confirm.png")
	await click_control(yes)
	if not await wait_quest(func(s): return not in_log(s, QUEST), "server abandon"):
		return false
	await capture("fidelity-abandoned.png")
	await tap(KEY_L)
	if not await wait_quest(func(s): return not s.log_open, "final L closes map owner"):
		return false
	print("PASS QUEST_LOG_FIDELITY skin=", OS.get_environment("QF_SKIN"), " L/list/details/Back/track/non-shareable-disabled/POI/maximize/abandon")
	return true

# Optional independent slice: real item4401 learning, owned selection/equip, slot model.
func check_pet_loadout_models() -> bool:
	if not await wait_frames(func(): return client.merchant_state().item_catalog_loaded, "Retail item catalog before learning", 120000):
		return false
	if not admin(["grant-item", character_name(), "4401", "1"]):
		return false
	var backpack := client.find_child("MainMenuBarBackpackButton", true, false) as Control
	await click_control(backpack)
	if not await wait_frames(func():
		var bag := client.find_child("ContainerFrame0", true, false) as Control
		return bag != null and bag.is_visible_in_tree() and bag.size.x > 0, "visible backpack"):
		return false
	var slot_name := ""
	for item in client.merchant_state().bags:
		if item.item_id == 4401:
			slot_name = "ContainerFrame%dSlot%d" % [item.bag, item.slot]
	if slot_name.is_empty():
		fail("Mechanical Squirrel Box missing from real bags")
		return false
	# Actual item use is a right-click, not a staged journal snapshot.
	print("PET_LEARN_INPUT slot=", slot_name, " bags=", client.merchant_state().bags)
	var item_control := client.find_child(slot_name, true, false) as Control
	if not await wait_frames(func(): return item_control != null and item_control.is_visible_in_tree() and item_control.size.x > 0, "resolved item control"):
		return false
	print("PET_LEARN_RECT ", item_control.get_global_rect(), " account=", client.account_state())
	await capture("fidelity-pet-learning-input.png")
	await click(item_control.get_global_rect().get_center(), MOUSE_BUTTON_RIGHT)
	await frames(5)
	await click_control(backpack)
	await tap_collections()
	await frames(3)
	if not await wait_frames(func(): return client.find_child("CollectionsJournalTab2", true, false) != null, "collections shell"):
		return false
	await click_control(client.find_child("CollectionsJournalTab2", true, false) as Control)
	if not await wait_frames(func(): return client.get_node_or_null("CollectionsJournalUI") != null, "Pet Journal"):
		return false
	var journal := client.get_node("CollectionsJournalUI")
	if not await wait_frames(func(): return not journal.find_children("pet_select_owned_*", "Control", true, false).is_empty(), "owned journal rows after account update"):
		print("PET_JOURNAL_CONTROLS ", journal.find_children("*", "Control", true, false).map(func(node): return node.name))
		return false
	var owned: Control
	for candidate in journal.find_children("pet_select_owned_*", "Control", true, false):
		if String(candidate.name).trim_prefix("pet_select_owned_").is_valid_int():
			owned = candidate
			break
	if owned == null:
		fail("Real item learning produced no owned journal row")
		return false
	await click_control(owned)
	if not await wait_frames(func(): return model_meshes("PetJournalPetCardModelScene") > 0, "authentic pet card model prerequisite", 60000):
		return false
	for slot in [0, 1, 2, 0]:
		var button := journal.find_child("PetJournalBattleSlot%d" % slot, true, false) as BaseButton
		if not await wait_frames(func(): return not button.disabled, "loadout accepts next equip"):
			return false
		await click_control(button)
		if not await wait_frames(func(): return journal.frame_text("PetJournalBattleSlot%dName" % slot) == "Mechanical Squirrel", "server loadout equip"):
			return false
		if not await wait_frames(func(): return model_meshes("PetJournalBattleSlot%dModelScene" % slot) > 0, "owned loadout model slot %d" % slot, 60000):
			return false
		if not await wait_frames(func(): return not button.disabled, "server loadout acknowledgement"):
			return false
		await frames(3)
		for empty_slot in range(3):
			var empty := journal.find_child("PetJournalBattleSlot%dEmptySlot" % empty_slot, true, false) != null
			var meshes := model_meshes("PetJournalBattleSlot%dModelScene" % empty_slot)
			print("PET_LOADOUT_STATE equip=", slot, " slot=", empty_slot, " empty=", empty, " meshes=", meshes)
			if empty and meshes != 0:
				fail("Superseded model survived in an authoritative empty slot")
				return false
	await capture("fidelity-pet-loadout-model.png")
	await tap_collections()
	if not await wait_frames(func(): return client.get_node_or_null("CollectionsJournalUI") == null, "Shift+P closes active pet journal"):
		return false
	await tap_collections()
	if not await wait_frames(func(): return client.find_child("CollectionsJournalTab2", true, false) != null, "Shift+P reopens Collections"):
		return false
	await click_control(client.find_child("CollectionsJournalTab2", true, false) as Control)
	if not await wait_frames(func(): return model_meshes("PetJournalBattleSlot0ModelScene") > 0, "loadout model after reopening", 60000):
		return false
	await capture("fidelity-pet-loadout-reopened.png")
	print("PASS PET_LOADOUT_MODEL skin=", OS.get_environment("QF_SKIN"), " real item4401/owned/equip/all3hosts/move/close/reopen")
	return true

func tap_collections() -> void:
	for pressed in [true, false]:
		var event := InputEventKey.new()
		event.keycode = KEY_P
		event.physical_keycode = KEY_P
		event.shift_pressed = true
		event.pressed = pressed
		root.push_input(event, true)
		await process_frame
	await frames(2)

func model_meshes(frame_name: String) -> int:
	var journal := client.get_node_or_null("CollectionsJournalUI")
	if journal == null:
		return 0
	var host := journal.find_child(frame_name, true, false)
	if host == null:
		return 0
	return host.find_children("*", "MeshInstance3D", true, false).size()

func quest_geometry(details: bool) -> bool:
	var window := quest_control("WorldMapUI", "WorldMapBorderFrame")
	var dock := quest_control("WorldMapUI", "QuestMapFrame")
	if window == null or dock == null:
		fail("Map owner/dock missing")
		return false
	var scale: float = window.size.x / 1035.0
	if abs(window.size.y - 534.0 * scale) > 1.0 or dock.size.distance_to(Vector2(330, 506) * scale) > 1.0:
		fail("Owner/dock size differs from Retail XML")
		return false
	var relative: Vector2 = (dock.global_position - window.global_position) / scale
	if relative.distance_to(Vector2(702, 25)) > 1.0:
		fail("Dock anchors differ from Retail Lua: " + str(relative))
		return false
	var name := "QuestMapDetailsScrollFrame" if details else "QuestScrollFrame"
	var scroll := quest_control("WorldMapUI", name)
	var expected := Vector2(298, 430) if details else Vector2(308, 477)
	if scroll == null or scroll.size.distance_to(expected * scale) > 1.0:
		fail("Scroll viewport differs from XML: " + name)
		return false
	print("PASS XML_GEOMETRY ", name, " owner=", window.get_global_rect(), " dock=", dock.get_global_rect(), " viewport=", scroll.get_global_rect())
	return true
