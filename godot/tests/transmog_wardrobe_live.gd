extends "res://tests/world_merchant_flow.gd"
## Real native Retail12.1 Transmogrify frame on private5926; owned fb_transmog_a/TransmogA.
## Both skins run against the same account, so every step derives its targets from the
## current server state. TRANSMOG_SHOTS is a durable diagnostic directory.

const SERVER := "127.0.0.1:5926"
const SOURCES := [821, 1181]
## Smotts' Cutlass (TransmogSourceTypeEnum9): Retail lists only usable collected sources.
const NON_COLLECTABLE := 1473
## Battlegear of Might: the private character wears its Burnished Helm and Pauldrons.
const SET_ID := 853
const SET_NAME := "Battlegear of Might"
const MAIN_HAND := 16
## OutfitEntry2 "Outfit 1" and3 "Outfit 2" (automatically awarded).
const OUTFITS := [2, 3]
## Default-collected TransmogIllusion IDs: Lifestealing, Crusader, Striking.
const ILLUSIONS := [3, 13, 22]
const APPLY_COST := 1500

var shots := ""
var ui: Node
var saved := {}

func _initialize() -> void:
	if OS.get_environment("TRANSMOG_WEAPON_IMAGE") != "":
		call_deferred("check_weapon_image")
	else:
		super._initialize()

func check_weapon_image() -> void:
	var image := Image.load_from_file(OS.get_environment("TRANSMOG_WEAPON_IMAGE"))
	var span := opaque_span(image)
	print("TRANSMOG_WEAPON_SPAN ", span)
	if max(span.x, span.y) < 30:
		fail("Main-hand rendered end-on: isolated span " + str(span))
		return
	print("PASS TRANSMOG_WEAPON_PROFILE")
	quit(0)

func run_test() -> void:
	shots = OS.get_environment("TRANSMOG_SHOTS")
	if shots == "" or OS.get_environment("GODOT_TEST_SERVER") != SERVER:
		fail("Set TRANSMOG_SHOTS and private GODOT_TEST_SERVER=" + SERVER)
		return
	DirAccess.make_dir_recursive_absolute(shots)
	root.size = Vector2i(1280, 720)
	client = load("res://scenes/client.tscn").instantiate()
	root.add_child(client)
	# Wait for cold startup work; source icons also require this real catalog readiness.
	await frames(5)
	if not await wait_for(func(s): return s.item_catalog_loaded, "startup item catalog", 90000):
		return
	var error = client.connect_account(SERVER, "fb_transmog_a", "fbtest", false)
	if error != "":
		fail(error)
		return
	if not await wait_for(func(_s): return client.get_node_or_null("CharacterSelectUI") != null, "character select", 180000):
		print("TRANSMOG_ACCOUNT_STATE ", client.account_state())
		return
	var select = client.get_node("CharacterSelectUI")
	if not await wait_for(func(_s): return select.find_child("CharCard_0", true, false) != null, "roster card", 60000):
		return
	await click_control(select.find_child("CharCard_0", true, false), MOUSE_BUTTON_LEFT)
	await click_control(select.find_child("EnterWorld", true, false), MOUSE_BUTTON_LEFT)
	if not await wait_for(func(_s): return client.account_state().screen == "InWorld" and client.account_state().local_player_position != null, "enter world", 180000):
		return
	var expected_skin := OS.get_environment("TRANSMOG_SKIN")
	var account: Dictionary = client.account_state()
	print("TRANSMOG_SKIN ", account.ui_skin, " layout=", account.ui_layout)
	if expected_skin != "" and account.ui_skin != expected_skin:
		fail("Expected skin " + expected_skin + ", got " + str(account.ui_skin))
		return
	await create_timer(40.0).timeout
	await tap(KEY_Z)
	if not await wait_for(func(_s): return control("AuctionGossipOption0") != null, "Hashom gossip", 60000):
		return
	await click_control(control("AuctionGossipOption0"), MOUSE_BUTTON_LEFT)
	if not await wait_for(func(_s): return control("TransmogSlot16") != null, "native Transmogrify frame", 60000):
		return
	ui = client.get_node("TransmogUI")
	for step in ["check_layout", "check_preview_controls", "check_items", "check_filters", "check_outfits", "check_outfit_load", "check_sets", "check_custom_sets"]:
		if not await Callable(self, step).call():
			return
	await tap(KEY_ESCAPE)
	await frames(10)
	if client.get_node_or_null("TransmogUI") != null:
		fail("Escape retained the Transmogrify frame and detached previews")
		return
	print("PASS TRANSMOG_WARDROBE_LIVE")
	client.free()
	quit(0)

## TransmogFrame1618x883: outfits, character and collection regions left to right.
func check_layout() -> bool:
	await click_control(control("TransmogSlot16"), MOUSE_BUTTON_LEFT)
	if not await wait_for(func(_s): return preview_model("TransmogCharacterModelScene") != null and control("TransmogSource821") != null, "character preview and main-hand sources", 60000):
		return false
	await frames(20)
	var frame := control("TransmogFrame").get_global_rect()
	if not Rect2(Vector2.ZERO, Vector2(root.size)).encloses(frame):
		fail("Transmogrify frame %s exceeds the %s viewport" % [frame, root.size])
		return false
	var model := control("TransmogCharacterModelScene").get_global_rect()
	if control("TransmogOutfitEntry2Button").get_global_rect().end.x > model.position.x or control("TransmogSource821").get_global_rect().position.x < model.end.x:
		fail("Outfits, character preview and collection are not left to right")
		return false
	if control("TransmogSlot1").get_global_rect().get_center().x >= control("TransmogSlot10").get_global_rect().get_center().x:
		fail("Left and right gear columns are swapped")
		return false
	if control("TransmogSlot16").get_global_rect().position.y < control("TransmogSlot1").get_global_rect().end.y:
		fail("Main hand is not in the bottom weapon row")
		return false
	return true

func check_preview_controls() -> bool:
	var preview := preview_model("TransmogCharacterModelScene")
	var weapon = preview.find_child("EquipmentMainHand", true, false) as Node3D
	if weapon == null:
		fail("Preview has no main-hand weapon node")
		return false
	var weapon_pixels := await measure_weapon_pixels(weapon)
	print("TRANSMOG_WEAPON_PIXELS ", weapon_pixels, " parent=", weapon.get_parent().name, " visible=", weapon.is_visible_in_tree())
	if weapon_pixels < 25:
		fail("Main-hand weapon contributes only %d preview pixels" % weapon_pixels)
		return false
	var initial_yaw := preview.rotation.y
	await click_control(control("TransmogRotateLeft"), MOUSE_BUTTON_LEFT)
	await frames(2)
	if is_equal_approx(preview.rotation.y, initial_yaw):
		fail("Rotate-left did not rotate the detached model")
		return false
	await click_control(control("TransmogResetRotation"), MOUSE_BUTTON_LEFT)
	await click_control(control("TransmogSheatheWeaponToggle"), MOUSE_BUTTON_LEFT)
	await frames(2)
	if weapon.get_parent().name == "Attachment1":
		fail("Sheathe kept main-hand in the palm")
		return false
	await click_control(control("TransmogSheatheWeaponToggle"), MOUSE_BUTTON_LEFT)
	await frames(2)
	if weapon.get_parent().name != "Attachment1":
		fail("Draw did not restore main-hand to the palm")
		return false
	print("TRANSMOG_PREVIEW_CONTROLS_READY")
	return true

## Items tab: collected usable sources as 3D cards; the non-collectable source is absent.
func check_items() -> bool:
	for source in SOURCES:
		var host := "TransmogSource%dModelScene" % source
		if not await wait_for(func(_s): return preview_model(host) != null, "3D source %d" % source, 60000):
			return false
		var pixels := await measure_collection_pixels(source)
		print("TRANSMOG_COLLECTION_PIXELS source=", source, " pixels=", pixels)
		if pixels < 5:
			fail("Source %d model is covered by its state overlay" % source)
			return false
	if control("TransmogSource%d" % NON_COLLECTABLE) != null:
		fail("Non-collectable Smotts' Cutlass is listed")
		return false
	if not await wait_card_models("TransmogSource*ModelScene"):
		return false
	await save_shot("01-items.png")
	await type_into("TransmogSearchBox", "Gladius")
	if not await wait_for(func(_s): return control("TransmogSource821") != null and control("TransmogSource1181") == null, "filtered Gladius collection"):
		return false
	await type_into("TransmogSearchBox", "not collected")
	if not await wait_for(func(_s): return ui.frame_text("TransmogNoSources") == "No matches.", "no search matches"):
		return false
	await type_into("TransmogSearchBox", "")
	if not await wait_for(func(_s): return preview_model("TransmogSource1181ModelScene") != null, "restored collection model", 60000):
		return false
	print("TRANSMOG_SEARCH_AND_MODEL_RESTORE_READY")
	return true

## Real Enum.TransmogSource toggles; filtering changes no pending choice or quote.
func check_filters() -> bool:
	await click_control(control("TransmogSource1181"), MOUSE_BUTTON_LEFT)
	var pending_before: int = transmog().pending
	var cost_before: String = ui.frame_text("TransmogPendingCostAmount0")
	await click_control(control("TransmogFilterButton"), MOUSE_BUTTON_LEFT)
	if not await wait_for(func(_s): return control("TransmogFilterSource4") != null, "Sources menu"):
		return false
	await save_shot("11-source-filters.png")
	await click_control(control("TransmogFilterSource4"), MOUSE_BUTTON_LEFT)
	if not await wait_for(func(_s): return control("TransmogSource1181") == null and control("TransmogSource821") != null, "World Drop filtered out"):
		return false
	if transmog().pending != pending_before or ui.frame_text("TransmogPendingCostAmount0") != cost_before:
		fail("Sources filter changed staged appearances or quote")
		return false
	await click_control(control("TransmogFilterNone"), MOUSE_BUTTON_LEFT)
	if not await wait_for(func(_s): return control("TransmogSource821") == null and ui.frame_text("TransmogNoSources") == "No matches.", "Uncheck All"):
		return false
	await click_control(control("TransmogFilterAll"), MOUSE_BUTTON_LEFT)
	await click_control(control("TransmogFilterButton"), MOUSE_BUTTON_LEFT)
	if not await wait_card_models("TransmogSource*ModelScene"):
		return false
	await hover_point(control("TransmogSource821").get_global_rect().get_center())
	var hovered_pixels := await measure_collection_pixels(821)
	if hovered_pixels < 5:
		fail("Item hover border covered its model")
		return false
	await save_shot("12-source-hover.png")
	await hover_point(Vector2(5, 5))
	if transmog().pending > 0:
		await click_control(control("TransmogClearAllPendingButton"), MOUSE_BUTTON_LEFT)
	print("TRANSMOG_FILTERS_READY pending=", pending_before, " cost=", cost_before, " hover_pixels=", hovered_pixels)
	return await wait_for(func(_s): return transmog().pending == 0, "filtered staging cleared")

## Save Outfit (CommitAndApplyAllPending): paid ApplyTransmog, then the outfit commit
## with its weapon illusion. Each outfit gets a different appearance and illusion.
func check_outfits() -> bool:
	if not await save_outfit(OUTFITS[0], [], "02-outfit1-pending.png", "03-outfit1-saved.png"):
		return false
	return await save_outfit(OUTFITS[1], [saved[OUTFITS[0]].illusion], "", "04-outfit2-saved.png")

func save_outfit(outfit: int, avoid: Array, pending_shot: String, saved_shot: String) -> bool:
	if not await view_outfit(outfit):
		return false
	var state := transmog()
	var applied: int = state.applied[MAIN_HAND]
	var source: int = SOURCES[1] if applied == SOURCES[0] else SOURCES[0]
	var illusion := 0
	for candidate in ILLUSIONS:
		if candidate != state.illusions.get(MAIN_HAND, 0) and not avoid.has(candidate):
			illusion = candidate
			break
	await click_control(control("TransmogSlot16"), MOUSE_BUTTON_LEFT)
	if not await wait_for(func(_s): return control("TransmogSource%d" % source) != null, "main-hand source %d" % source):
		return false
	await click_control(control("TransmogSource%d" % source), MOUSE_BUTTON_LEFT)
	if not await wait_for(func(_s): return transmog().effective[MAIN_HAND] == source, "pending source %d" % source):
		return false
	await click_control(control("TransmogIllusion16"), MOUSE_BUTTON_LEFT)
	var entry := "TransmogIllusionEntry%d" % illusion
	if not await wait_for(func(_s): return control(entry) != null and preview_model(entry + "ModelScene") != null, "illusion %d card" % illusion, 60000):
		return false
	await click_control(control(entry), MOUSE_BUTTON_LEFT)
	if not await wait_for(func(_s): return transmog().illusions.get(MAIN_HAND, 0) == illusion and transmog().pending == 2, "pending illusion %d" % illusion):
		return false
	# ItemModelBaseMixin:OnEnter: illusion cards name the illusion in GameTooltip.
	var illusion_name: String = ui.frame_text(entry + "Name")
	await hover_point(control(entry).get_global_rect().get_center())
	if not await wait_for(func(_s): return client.tooltip_state().visible and client.tooltip_state().title == illusion_name, "illusion %s tooltip" % illusion_name, 10000):
		print("TRANSMOG_TOOLTIP ", client.tooltip_state())
		return false
	if not await wait_card_models("TransmogIllusionEntry*ModelScene"):
		return false
	if pending_shot != "":
		await save_shot(pending_shot)
	var before: int = transmog().money
	await click_control(control("TransmogSaveOutfitButton"), MOUSE_BUTTON_LEFT)
	var committed := func(_s):
		var now := transmog()
		return not now.busy and now.pending == 0 and now.money == before - APPLY_COST and now.applied[MAIN_HAND] == source and now.illusions.get(MAIN_HAND, 0) == illusion and now.error == ""
	if not await wait_for(committed, "paid Apply and outfit %d commit" % outfit, 30000):
		print("TRANSMOG_STATE ", transmog())
		return false
	saved[outfit] = {"source": source, "illusion": illusion}
	if not await wait_card_models("TransmogIllusionEntry*ModelScene"):
		return false
	await save_shot(saved_shot)
	print("TRANSMOG_OUTFIT_SAVED outfit=", outfit, " source=", source, " illusion=", illusion, " gold=", before, "->", transmog().money)
	return true

func view_outfit(outfit: int) -> bool:
	await click_control(control("TransmogOutfitEntry%dButton" % outfit), MOUSE_BUTTON_LEFT)
	if not await wait_for(func(_s): return transmog().viewed_outfit == outfit, "viewed outfit %d" % outfit):
		return false
	if transmog().pending > 0:
		await click_control(control("TransmogClearAllPendingButton"), MOUSE_BUTTON_LEFT)
		if not await wait_for(func(_s): return transmog().pending == 0, "cleared pending outfit changes"):
			return false
	return true

## ChangeViewedOutfit: Outfit 1's saved appearance becomes pending against the applied
## Outfit 2 appearance, and its saved illusion is shown.
func check_outfit_load() -> bool:
	var outfit: int = OUTFITS[0]
	var expected: Dictionary = saved[outfit]
	await click_control(control("TransmogOutfitEntry%dButton" % outfit), MOUSE_BUTTON_LEFT)
	var loaded := func(_s):
		var now := transmog()
		return now.viewed_outfit == outfit and now.effective[MAIN_HAND] == expected.source and now.applied[MAIN_HAND] != expected.source and now.illusions.get(MAIN_HAND, 0) == expected.illusion and now.pending == 1
	if not await wait_for(loaded, "loaded outfit %d" % outfit):
		print("TRANSMOG_STATE ", transmog())
		return false
	if not await wait_card_models("TransmogIllusionEntry*ModelScene"):
		return false
	await save_shot("05-outfit-load.png")
	print("TRANSMOG_OUTFIT_LOADED outfit=", outfit, " ", transmog().outfits)
	await click_control(control("TransmogClearAllPendingButton"), MOUSE_BUTTON_LEFT)
	return await wait_for(func(_s): return transmog().pending == 0, "cleared loaded outfit")

## Sets tab: the started set with collected and uncollected primary appearances.
func check_sets() -> bool:
	await click_control(control("TransmogSetsTab"), MOUSE_BUTTON_LEFT)
	var card := "TransmogSet%d" % SET_ID
	if not await wait_for(func(_s): return control(card) != null and preview_model(card + "ModelScene") != null, "set %d card" % SET_ID, 60000):
		print("TRANSMOG_STATE ", transmog())
		return false
	var progress: String = transmog().sets[SET_ID]
	print("TRANSMOG_SET ", progress, " status=", ui.frame_text(card + "Status"))
	if not ui.frame_text(card + "Status").begins_with("Incomplete"):
		fail("Set %d is not shown incomplete: %s" % [SET_ID, progress])
		return false
	if not await wait_card_models("TransmogSet*ModelScene"):
		return false
	await save_shot("06-sets.png")
	await hover_point(control(card).get_global_rect().get_center())
	if not await wait_for(func(_s): return client.tooltip_state().visible and client.tooltip_state().title == SET_NAME, "set tooltip", 10000):
		print("TRANSMOG_TOOLTIP ", client.tooltip_state())
		return false
	await save_shot("07-set-tooltip.png")
	await hover_point(Vector2(5, 5))
	await click_control(control("TransmogFilterButton"), MOUSE_BUTTON_LEFT)
	if not await wait_for(func(_s): return control("TransmogFilterUncollected") != null, "Set completion menu"):
		return false
	await click_control(control("TransmogFilterUncollected"), MOUSE_BUTTON_LEFT)
	if not await wait_for(func(_s): return control(card) == null and control("TransmogFilterButton") != null, "incomplete set excluded, filter retained"):
		return false
	await save_shot("13-set-filters.png")
	await click_control(control("TransmogFilterUncollected"), MOUSE_BUTTON_LEFT)
	await click_control(control("TransmogFilterButton"), MOUSE_BUTTON_LEFT)
	if not await wait_for(func(_s): return preview_model(card + "ModelScene") != null, "incomplete set restored", 60000):
		return false
	print("TRANSMOG_SET_FILTERS_READY")
	return true

## Custom Sets tab: New Custom Set names the current appearances (TRANSMOG_CUSTOM_SET_NAME).
func check_custom_sets() -> bool:
	await click_control(control("TransmogCustomSetsTab"), MOUSE_BUTTON_LEFT)
	if not await wait_for(func(_s): return control("TransmogNewCustomSetButton") != null, "Custom Sets tab"):
		return false
	var before: Dictionary = transmog().custom_sets
	if before.is_empty():
		if ui.frame_text("TransmogCustomSetsNone") != "You don't have any Custom Sets":
			fail("Empty Custom Sets tab lacks TRANSMOG_CUSTOM_SETS_NONE")
			return false
		await save_shot("08-custom-sets-empty.png")
	var name := OS.get_environment("TRANSMOG_SKIN") + " Blade"
	await click_control(control("TransmogNewCustomSetButton"), MOUSE_BUTTON_LEFT)
	if not await wait_for(func(_s): return control("TransmogNamePopupEditBox") != null, "custom set name popup"):
		return false
	await type_into("TransmogNamePopupEditBox", name)
	await save_shot("09-custom-set-name.png")
	await click_control(control("TransmogNamePopupButton1"), MOUSE_BUTTON_LEFT)
	var created := func(_s):
		var now := transmog()
		return not now.busy and now.custom_sets.size() == before.size() + 1 and now.custom_sets.values().has(name)
	if not await wait_for(created, "custom set " + name, 30000):
		print("TRANSMOG_STATE ", transmog())
		return false
	var id := -1
	for key in transmog().custom_sets:
		if not before.has(key):
			id = key
	var card := "TransmogCustomSet%d" % id
	if not await wait_for(func(_s): return control(card) != null and preview_model(card + "ModelScene") != null, "custom set card", 60000):
		return false
	if not await wait_card_models("TransmogCustomSet*ModelScene"):
		return false
	# The set's saved main-hand appearance is drawn in the card model's hand.
	var blade = preview_model(card + "ModelScene").find_child("EquipmentMainHand", true, false) as Node3D
	if blade == null or blade.get_parent().name != "Attachment1":
		fail("Custom set card does not hold its main-hand appearance")
		return false
	var blade_pixels := await card_node_pixels(card + "ModelScene", blade)
	print("TRANSMOG_CUSTOM_SET_BLADE_PIXELS ", blade_pixels)
	if blade_pixels < 20:
		fail("Custom set card blade contributes only %d pixels" % blade_pixels)
		return false
	await save_shot("10-custom-set-saved.png")
	print("TRANSMOG_CUSTOM_SET id=", id, " name=", name)
	return true

func transmog() -> Dictionary:
	return client.transmog_state()

## Every card on the page shows its 3D preview, so captures never show loading cells.
func wait_card_models(pattern: String) -> bool:
	if await wait_for(func(_s): return card_models_ready(pattern), "card models " + pattern, 60000):
		await frames(10)
		return true
	return false

## Pixels of one card viewport that change when `node` is hidden, animation paused.
func card_node_pixels(host_name: String, node: Node3D) -> int:
	var viewport := control(host_name).find_child("CharacterModelViewport", true, false) as SubViewport
	var animation = preview_model(host_name).find_child("M2Animation", true, false)
	if animation != null:
		animation.set_paused(true)
		animation.set_process(false)
	node.visible = false
	await frames(2)
	var hidden := viewport.get_texture().get_image()
	node.visible = true
	await frames(2)
	var shown := viewport.get_texture().get_image()
	if animation != null:
		animation.set_process(true)
		animation.set_paused(false)
	return changed_pixels(hidden, shown)

func card_models_ready(pattern: String) -> bool:
	var hosts := ui.find_children(pattern, "", true, false)
	if hosts.is_empty():
		return false
	for host in hosts:
		if preview_model(host.name) == null:
			return false
	return true

func measure_collection_pixels(source: int) -> int:
	var host_name := "TransmogSource%dModelScene" % source
	var host := control(host_name)
	var model := preview_model(host_name)
	model.visible = false
	await frames(2)
	var hidden := root.get_texture().get_image()
	model.visible = true
	await frames(2)
	var visible := root.get_texture().get_image()
	var transform := host.get_global_transform_with_canvas()
	var first := Vector2i(transform * Vector2.ZERO)
	var last := Vector2i(transform * host.size)
	var region := Rect2i(first, last - first)
	print("TRANSMOG_COLLECTION_REGION ", source, " ", region, " image=", hidden.get_size())
	return changed_pixels(hidden.get_region(region), visible.get_region(region))

func type_into(field: String, value: String) -> void:
	await click_control(control(field), MOUSE_BUTTON_LEFT)
	var select_all := InputEventKey.new()
	select_all.keycode = KEY_A
	select_all.physical_keycode = KEY_A
	select_all.ctrl_pressed = true
	select_all.pressed = true
	root.push_input(select_all, true)
	await process_frame
	select_all.pressed = false
	root.push_input(select_all, true)
	await tap(KEY_BACKSPACE)
	for character in value:
		var event := InputEventKey.new()
		event.unicode = character.unicode_at(0)
		event.pressed = true
		root.push_input(event, true)
		await process_frame
		event.pressed = false
		root.push_input(event, true)
	await frames(3)

func control(name: String) -> Control:
	return client.find_child(name, true, false) as Control

func preview_model(host_name: String) -> Node3D:
	var host := control(host_name)
	if host == null:
		return null
	var scene = host.find_child("CharacterModelSceneRoot", true, false)
	if scene == null:
		return null
	for child in scene.get_children():
		if child.name != "Camera" and child.name != "SceneLight" and child is Node3D:
			return child
	return null

func measure_weapon_pixels(weapon: Node3D) -> int:
	var host := control("TransmogCharacterModelScene")
	var viewport := host.find_child("CharacterModelViewport", true, false) as SubViewport
	var animation = preview_model("TransmogCharacterModelScene").find_child("M2Animation", true, false)
	animation.set_paused(true)
	animation.set_process(false)
	weapon.visible = false
	await frames(2)
	var hidden := viewport.get_texture().get_image()
	weapon.visible = true
	await frames(2)
	var visible := viewport.get_texture().get_image()
	hidden.save_png(shots.path_join("weapon-hidden.png"))
	visible.save_png(shots.path_join("weapon-visible.png"))
	var changed := changed_pixels(hidden, visible)
	var mesh_visibility := {}
	for mesh in preview_model("TransmogCharacterModelScene").find_children("*", "MeshInstance3D", true, false):
		if not weapon.is_ancestor_of(mesh):
			mesh_visibility[mesh] = mesh.visible
			mesh.visible = false
	await frames(2)
	var isolated := viewport.get_texture().get_image()
	isolated.save_png(shots.path_join("weapon-isolated.png"))
	var span := opaque_span(isolated)
	print("TRANSMOG_WEAPON_SPAN ", span)
	if max(span.x, span.y) < 30:
		fail("Main-hand rendered end-on: isolated span " + str(span))
	for mesh in mesh_visibility:
		mesh.visible = mesh_visibility[mesh]
	animation.set_process(true)
	animation.set_paused(false)
	return changed

func opaque_span(image: Image) -> Vector2i:
	var first := Vector2i(image.get_width(), image.get_height())
	var last := Vector2i.ZERO
	for y in image.get_height():
		for x in image.get_width():
			if image.get_pixel(x, y).a > 0.1:
				first.x = min(first.x, x)
				first.y = min(first.y, y)
				last.x = max(last.x, x)
				last.y = max(last.y, y)
	return last - first + Vector2i.ONE

func changed_pixels(first: Image, second: Image) -> int:
	var changed := 0
	for y in first.get_height():
		for x in first.get_width():
			if not first.get_pixel(x, y).is_equal_approx(second.get_pixel(x, y)):
				changed += 1
	return changed

func save_shot(name: String) -> void:
	await frames(2)
	var image := root.get_texture().get_image()
	if image.get_width() > 1280:
		image.resize(1280, int(image.get_height() * 1280.0 / image.get_width()), Image.INTERPOLATE_LANCZOS)
	var error := image.save_png(shots.path_join(name))
	if error != OK:
		fail("Screenshot " + error_string(error))
