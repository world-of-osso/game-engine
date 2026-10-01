extends "res://tests/world_merchant_flow.gd"

# Native CharacterFrame against a private game server (docs/specs/character-frame.md):
# C opens it with the model preview; dragging Gladius (2488) from the backpack onto the
# main-hand slot equips it, dragging the vest (2379) onto the chest equips it, both
# visible on the model; dragging the weapon from its slot to an empty bag slot unequips
# it; right-clicking the leggings (2381) in the backpack equips them; dropping the
# Worn Shortsword (25) on the head slot shows ERR_WRONG_SLOT; hover shows the item
# tooltip; Escape closes; the micro-menu button opens it again.
#
# Setup (game-server-admin, private UDP server): create-account fb_charframe fbtest,
# create-character fb_charframe Charframe 1 1, set-level Charframe 10 (Gladius needs 2),
# grant-item Charframe 2488/2379/2381/25 1.

const CF_ACCOUNT := "fb_charframe"
const CF_PASSWORD := "fbtest"
const CF_UI := "CharacterFrameUI"
const GLADIUS := 2488
const VEST := 2379
const LEGGINGS := 2381
const SHORTSWORD := 25
const WRONG_SLOT := "That item does not go in that slot."

var cf_shots := ""
var cf_character := ""

func run_test() -> void:
	root.size = Vector2i(1600, 900)
	cf_shots = OS.get_environment("CHARFRAME_SHOTS")
	cf_character = OS.get_environment("CHARFRAME_CHARACTER")
	var server := OS.get_environment("GODOT_TEST_SERVER")
	if cf_shots == "" or cf_character == "" or not server.begins_with("127.0.0.1:") or server == "127.0.0.1:5000":
		fail("Set CHARFRAME_SHOTS, CHARFRAME_CHARACTER and a private GODOT_TEST_SERVER (not :5000)")
		return
	DirAccess.make_dir_recursive_absolute(cf_shots)
	client = load("res://scenes/client.tscn").instantiate()
	root.add_child(client)
	var error = client.connect_account(server, CF_ACCOUNT, CF_PASSWORD, false)
	if error != "":
		fail("Connection: " + error)
		return
	if not await cf_enter_world():
		return
	if not await wait_for(func(s): return cf_bag(s, GLADIUS) != {} and cf_bag(s, VEST) != {}, "granted items in the bags", 20000):
		return
	await cf_open_backpack()
	if not await cf_open_with_key():
		return
	if not await cf_equip_by_drag(GLADIUS, "CharacterMainHandSlot", "MainHand", "02-gladius-equipped.png"):
		return
	var armor_before := cf_stat("Armor:")
	if armor_before < 0 or cf_stat("Stamina:") <= 0:
		fail("Attributes not shown: armor %d stamina %d" % [armor_before, cf_stat("Stamina:")])
		return
	if not await cf_equip_by_drag(VEST, "CharacterChestSlot", "Chest", "03-vest-equipped.png"):
		return
	if not await wait_for(func(_s): return cf_stat("Armor:") > armor_before, "Armor above %d in the stats pane" % armor_before):
		return
	print("FIXTURE CF_ARMOR ", armor_before, " -> ", cf_stat("Armor:"), " stamina ", cf_stat("Stamina:"))
	if not await cf_hover_tooltip():
		return
	if not await cf_unequip_weapon():
		return
	if not await cf_right_click_equip():
		return
	if not await cf_wrong_slot():
		return
	if not await cf_rotate():
		return
	await tap(KEY_ESCAPE)
	if client.character_frame_state().open or cf_visible("CharacterFrame"):
		fail("Escape did not close the CharacterFrame")
		return
	print("FIXTURE CF_ESCAPE_CLOSED")
	await click_control(cf_micro("CharacterMicroButton"), MOUSE_BUTTON_LEFT)
	await frames(3)
	if not client.character_frame_state().open or not cf_visible("CharacterFrame"):
		fail("CharacterMicroButton did not open the CharacterFrame")
		return
	await cf_capture("09-micro-button-open.png")
	print("FIXTURE CF_MICRO_OPEN")
	print("FIXTURE WORLD_CHARACTER_FRAME_DONE")
	client.free()
	quit(0)

func cf_enter_world() -> bool:
	var deadline := Time.get_ticks_msec() + 180000
	var ui: Node = null
	while Time.get_ticks_msec() < deadline:
		await process_frame
		var state: Dictionary = client.account_state()
		ui = client.get_node_or_null("CharacterSelectUI")
		if state.reply_received and state.screen == "CharacterSelect" and ui != null and cf_roster_card(ui) != null:
			break
	await frames(10)
	if ui == null or cf_roster_card(ui) == null:
		fail("No character select: " + str(client.account_state()))
		return false
	await click_control(cf_roster_card(ui), MOUSE_BUTTON_LEFT)
	await click_control(ui.find_child("EnterWorld", true, false), MOUSE_BUTTON_LEFT)
	deadline = Time.get_ticks_msec() + WORLD_WAIT_MS
	while Time.get_ticks_msec() < deadline:
		await process_frame
		var state: Dictionary = client.account_state()
		if state.screen == "InWorld" and state.local_player_position != null:
			print("FIXTURE CF_IN_WORLD ", state.selected_character_name)
			await frames(30)
			return true
	fail("Timed out entering the world: " + str(client.account_state()))
	return false

func cf_roster_card(ui: Node) -> Control:
	var index := 0
	while true:
		var card = ui.find_child("CharCard_%d" % index, true, false)
		if not card is Control:
			return null
		for label in card.find_children("*", "Label", true, false):
			if label.text == cf_character:
				return card
		index += 1
	return null

func cf_open_backpack() -> void:
	if cf_bag_control(0, 0) != null and cf_bag_control(0, 0).is_visible_in_tree():
		return
	var bags = client.get_node_or_null("BagsUI")
	await click_control(bags.find_child("MainMenuBarBackpackButton", true, false), MOUSE_BUTTON_LEFT)
	await frames(5)

func cf_open_with_key() -> bool:
	await tap(KEY_C)
	if not client.character_frame_state().open or not cf_visible("CharacterFrame"):
		fail("C did not open the CharacterFrame: " + str(client.character_frame_state()))
		return false
	var deadline := Time.get_ticks_msec() + 30000
	while not client.character_frame_state().model_shown:
		if Time.get_ticks_msec() > deadline:
			fail("Model preview never appeared: " + str(client.character_frame_state()))
			return false
		await process_frame
	await frames(10)
	await cf_capture("01-open.png")
	print("FIXTURE CF_OPEN ", client.character_frame_state())
	return true

# Press on the bag slot, move past the drag threshold, release over the paperdoll slot.
func cf_drag(from: Control, to: Control) -> void:
	var start := from.get_global_rect().get_center()
	var finish := to.get_global_rect().get_center()
	await hover_point(start)
	cf_button(start, true)
	await frames(2)
	for step in range(1, 6):
		var motion := InputEventMouseMotion.new()
		motion.position = start.lerp(finish, step / 5.0)
		motion.global_position = motion.position
		motion.button_mask = MOUSE_BUTTON_MASK_LEFT
		root.push_input(motion, true)
		await process_frame
	cf_button(finish, false)
	await frames(3)

func cf_button(point: Vector2, pressed: bool) -> void:
	var event := InputEventMouseButton.new()
	event.position = point
	event.global_position = point
	event.button_index = MOUSE_BUTTON_LEFT
	event.button_mask = MOUSE_BUTTON_MASK_LEFT if pressed else 0
	event.pressed = pressed
	root.push_input(event, true)

func cf_equip_by_drag(item_id: int, slot_name: String, slot: String, shot: String) -> bool:
	var source := cf_bag(client.merchant_state(), item_id)
	var before: Array = client.character_frame_state().model_slots
	await cf_drag(cf_bag_control(source.bag, source.slot), cf_control(slot_name))
	if not await wait_for(func(s): return cf_equipped(s, slot) == item_id and cf_bag(s, item_id) == {}, "%d equipped in %s" % [item_id, slot]):
		return false
	if not await cf_model_redressed(before):
		return false
	await cf_capture(shot)
	print("FIXTURE CF_EQUIPPED ", item_id, " ", slot, " model ", client.character_frame_state().model_slots)
	return true

# The preview follows the replicated appearance once the re-dress has loaded.
func cf_model_redressed(before: Array) -> bool:
	var deadline := Time.get_ticks_msec() + 20000
	while Time.get_ticks_msec() < deadline:
		await process_frame
		var state: Dictionary = client.character_frame_state()
		if state.model_shown and not state.model_pending and state.model_slots == state.world_slots and state.model_slots != before:
			await frames(10)
			return true
	fail("Model preview did not re-dress: " + str(client.character_frame_state()))
	return false

func cf_hover_tooltip() -> bool:
	await hover_point(cf_control("CharacterChestSlot").get_global_rect().get_center())
	await frames(4)
	var state: Dictionary = client.character_frame_state()
	if not state.tooltip_visible or state.tooltip_title != "Tarnished Chain Vest":
		fail("Chest slot tooltip: " + str(state))
		return false
	await cf_capture("04-vest-tooltip.png")
	await hover_point(cf_control("CharacterHeadSlot").get_global_rect().get_center())
	await frames(4)
	state = client.character_frame_state()
	if not state.tooltip_visible or state.tooltip_title != "Head":
		fail("Empty head slot tooltip: " + str(state))
		return false
	print("FIXTURE CF_TOOLTIPS vest / Head")
	return true

func cf_unequip_weapon() -> bool:
	var empty := cf_empty_backpack_slot(client.merchant_state())
	var before: Array = client.character_frame_state().model_slots
	await cf_drag(cf_control("CharacterMainHandSlot"), cf_bag_control(0, empty))
	if not await wait_for(func(s): return cf_equipped(s, "MainHand") == 0 and cf_bag(s, GLADIUS).get("slot", -1) == empty, "Gladius unequipped to bag slot %d" % empty):
		return false
	if not await cf_model_redressed(before):
		return false
	await cf_capture("05-gladius-unequipped.png")
	print("FIXTURE CF_UNEQUIPPED to 0/", empty)
	return true

func cf_right_click_equip() -> bool:
	var source := cf_bag(client.merchant_state(), LEGGINGS)
	var before: Array = client.character_frame_state().model_slots
	await click_control(cf_bag_control(source.bag, source.slot), MOUSE_BUTTON_RIGHT)
	if not await wait_for(func(s): return cf_equipped(s, "Legs") == LEGGINGS, "leggings equipped by right-click"):
		return false
	if not await cf_model_redressed(before):
		return false
	await cf_capture("06-leggings-right-click.png")
	print("FIXTURE CF_RIGHT_CLICK_EQUIPPED")
	return true

func cf_wrong_slot() -> bool:
	var source := cf_bag(client.merchant_state(), SHORTSWORD)
	await cf_drag(cf_bag_control(source.bag, source.slot), cf_control("CharacterHeadSlot"))
	var deadline := Time.get_ticks_msec() + REPLY_WAIT_MS
	while Time.get_ticks_msec() < deadline:
		await process_frame
		if cf_error_shown(WRONG_SLOT):
			await cf_capture("07-wrong-slot.png")
			if cf_equipped(client.merchant_state(), "Head") != 0:
				fail("The shortsword went into the head slot")
				return false
			print("FIXTURE CF_WRONG_SLOT ", WRONG_SLOT)
			return true
	fail("No ERR_WRONG_SLOT text after dropping the sword on the head slot")
	return false

func cf_rotate() -> bool:
	var scene := cf_control("CharacterModelScene").get_global_rect()
	var before: float = client.character_frame_state().model_yaw
	await cf_drag_points(scene.get_center(), scene.get_center() + Vector2(120, 0))
	var after: float = client.character_frame_state().model_yaw
	if after <= before:
		fail("Dragging the model did not rotate it: %f -> %f" % [before, after])
		return false
	await cf_capture("08-rotated.png")
	print("FIXTURE CF_ROTATED ", before, " -> ", after)
	return true

func cf_drag_points(start: Vector2, finish: Vector2) -> void:
	await hover_point(start)
	cf_button(start, true)
	await frames(2)
	for step in range(1, 11):
		var motion := InputEventMouseMotion.new()
		motion.position = start.lerp(finish, step / 10.0)
		motion.global_position = motion.position
		motion.relative = (finish - start) / 10.0
		motion.button_mask = MOUSE_BUTTON_MASK_LEFT
		root.push_input(motion, true)
		await process_frame
	cf_button(finish, false)
	await frames(3)

func cf_error_shown(text: String) -> bool:
	var errors = client.get_node_or_null("UIErrors")
	if errors == null:
		for child in client.get_children():
			if child is CanvasLayer:
				for label in child.find_children("*", "Label", true, false):
					if label.text == text and label.is_visible_in_tree():
						return true
		return false
	for label in errors.find_children("*", "Label", true, false):
		if label.text == text and label.is_visible_in_tree():
			return true
	return false

# The stats pane value of a label ("Armor:"), digits only; -1 when it is not shown.
func cf_stat(label: String) -> int:
	for index in range(1, 8):
		var name := cf_control("CharacterStatsPaneStat%dLabel" % index)
		if name is Label and name.text == label and name.is_visible_in_tree():
			var value := cf_control("CharacterStatsPaneStat%dValue" % index) as Label
			return int(value.text.replace(",", ""))
	return -1

func cf_bag(state: Dictionary, item_id: int) -> Dictionary:
	for item in state.bags:
		if item.item_id == item_id:
			return item
	return {}

func cf_empty_backpack_slot(state: Dictionary) -> int:
	var used := {}
	for item in state.bags:
		if item.bag == 0:
			used[item.slot] = true
	for slot in range(16):
		if not used.has(slot):
			return slot
	return -1

func cf_equipped(state: Dictionary, slot: String) -> int:
	for item in state.equipment:
		if item.slot == slot:
			return item.item_id
	return 0

func cf_control(name: String) -> Control:
	var ui = client.get_node_or_null(CF_UI)
	return ui.find_child(name, true, false) if ui != null else null

func cf_micro(name: String) -> Control:
	var ui = client.get_node_or_null("MicroMenuUI")
	return ui.find_child(name, true, false) if ui != null else null

func cf_bag_control(bag: int, slot: int) -> Control:
	var ui = client.get_node_or_null("BagsUI")
	return ui.find_child("ContainerFrame%dSlot%d" % [bag, slot], true, false) if ui != null else null

func cf_visible(name: String) -> bool:
	var control := cf_control(name)
	return control != null and control.is_visible_in_tree()

func cf_capture(file: String) -> void:
	await RenderingServer.frame_post_draw
	var image := root.get_texture().get_image()
	var error := image.save_png(cf_shots.path_join(file))
	if error != OK:
		fail("Could not save " + file + ": " + str(error))
