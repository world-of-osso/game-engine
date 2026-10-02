extends "res://tests/world_character_frame_flow.gd"

# Retail bag bar (Blizzard_MainMenuBarBagButtons) against a private game server: dragging
# the Blue Leather Bag (856, 8 slots) from the backpack onto CharacterBag0Slot equips it
# (`PutItemInBag` -> SwapItem to Equipment(Bag1)); the slot shows the bag's icon, the
# backpack count "(N)" rises by the freed backpack slot plus the bag's 8, and clicking the
# slot opens ContainerFrame1 with 8 slots. Dropping Linen Cloth (2589) on
# CharacterBag1Slot shows the server's refusal, "This item cannot be equipped.".
# BagBarExpandToggle collapses the four bag slots (the reagent slot moves up to the
# toggle) and expands them again.
#
# Setup (game-server-admin, private UDP server): create-account fb_bagclient fbtest,
# create-character fb_bagclient <Name> 1 1, grant-item <Name> 856 1, grant-item <Name> 2589 1.

const BB_ACCOUNT := "fb_bagclient"
const BAG := 856
const BAG_SIZE := 8
const CLOTH := 2589
const NOT_EQUIPPABLE := "This item cannot be equipped."

func run_test() -> void:
	root.size = Vector2i(1600, 900)
	cf_shots = OS.get_environment("BAGBAR_SHOTS")
	cf_character = OS.get_environment("BAGBAR_CHARACTER")
	var server := OS.get_environment("GODOT_TEST_SERVER")
	if cf_shots == "" or cf_character == "" or not server.begins_with("127.0.0.1:") or server == "127.0.0.1:5000":
		fail("Set BAGBAR_SHOTS, BAGBAR_CHARACTER and a private GODOT_TEST_SERVER (not :5000)")
		return
	DirAccess.make_dir_recursive_absolute(cf_shots)
	client = load("res://scenes/client.tscn").instantiate()
	root.add_child(client)
	var error = client.connect_account(server, BB_ACCOUNT, PASSWORD, false)
	if error != "":
		fail("Connection: " + error)
		return
	if not await cf_enter_world():
		return
	if not await wait_for(func(s): return cf_bag(s, BAG) != {} and cf_bag(s, CLOTH) != {}, "granted items in the backpack", 20000):
		return
	await cf_open_backpack()
	await frames(10)
	await cf_capture("01-bag-bar-empty-slots.png")
	if not await bb_equip_bag():
		return
	if not await bb_open_container():
		return
	if not await bb_refused():
		return
	if not await bb_collapse_and_expand():
		return
	print("FIXTURE WORLD_BAG_BAR_DONE")
	client.free()
	quit(0)

func bb_control(name: String) -> Control:
	var ui = client.get_node_or_null("BagsUI")
	return ui.find_child(name, true, false) if ui != null else null

func bb_shown(name: String) -> bool:
	var control := bb_control(name)
	return control != null and control.is_visible_in_tree()

func bb_count() -> int:
	var label := bb_control("MainMenuBarBackpackButtonCount") as Label
	if label == null or not label.is_visible_in_tree():
		return -1
	return int(label.text.trim_prefix("(").trim_suffix(")"))

func bb_equip_bag() -> bool:
	var count := bb_count()
	if count < 0 or bb_shown("CharacterBag0SlotIconTexture"):
		fail("Before equip: count %d, bag 1 icon shown %s" % [count, bb_shown("CharacterBag0SlotIconTexture")])
		return false
	var source := cf_bag(client.merchant_state(), BAG)
	await cf_drag(cf_bag_control(source.bag, source.slot), bb_control("CharacterBag0Slot"))
	var expected := count + 1 + BAG_SIZE
	if not await wait_for(func(s): return cf_bag(s, BAG) == {} and bb_shown("CharacterBag0SlotIconTexture") and bb_count() == expected, "bag icon in CharacterBag0Slot and count (%d)" % expected, 10000):
		print("FIXTURE BB_STATE count ", bb_count(), " icon ", bb_shown("CharacterBag0SlotIconTexture"))
		return false
	await frames(10)
	await cf_capture("02-bag-equipped.png")
	print("FIXTURE BB_EQUIPPED count %d -> %d" % [count, bb_count()])
	return true

func bb_open_container() -> bool:
	await click_control(bb_control("CharacterBag0Slot"), MOUSE_BUTTON_LEFT)
	await frames(5)
	if not bb_shown("ContainerFrame1") or not bb_shown("ContainerFrame1Slot%d" % (BAG_SIZE - 1)) or bb_control("ContainerFrame1Slot%d" % BAG_SIZE) != null:
		fail("ContainerFrame1 with %d slots not open: frame %s" % [BAG_SIZE, bb_shown("ContainerFrame1")])
		return false
	await frames(5)
	await cf_capture("03-container-open.png")
	print("FIXTURE BB_CONTAINER_OPEN ContainerFrame1 %d slots" % BAG_SIZE)
	return true

func bb_refused() -> bool:
	var source := cf_bag(client.merchant_state(), CLOTH)
	await cf_drag(cf_bag_control(source.bag, source.slot), bb_control("CharacterBag1Slot"))
	if not await wait_for(func(_s): return cf_error_shown(NOT_EQUIPPABLE), "server error " + NOT_EQUIPPABLE, 5000):
		return false
	if bb_shown("CharacterBag1SlotIconTexture"):
		fail("Refused cloth shows in CharacterBag1Slot")
		return false
	await cf_capture("04-refused.png")
	print("FIXTURE BB_REFUSED ", NOT_EQUIPPABLE)
	return true

func bb_collapse_and_expand() -> bool:
	var reagent_expanded := bb_control("CharacterReagentBag0Slot").get_global_rect().position.x
	await click_control(bb_control("BagBarExpandToggle"), MOUSE_BUTTON_LEFT)
	await frames(5)
	var reagent_collapsed := bb_control("CharacterReagentBag0Slot").get_global_rect().position.x
	if bb_shown("CharacterBag0Slot") or not bb_shown("CharacterReagentBag0Slot") or reagent_collapsed <= reagent_expanded:
		fail("Collapse: bag 1 shown %s, reagent x %f -> %f" % [bb_shown("CharacterBag0Slot"), reagent_expanded, reagent_collapsed])
		return false
	await cf_capture("05-collapsed.png")
	await click_control(bb_control("BagBarExpandToggle"), MOUSE_BUTTON_LEFT)
	await frames(5)
	if not bb_shown("CharacterBag0SlotIconTexture") or bb_control("CharacterReagentBag0Slot").get_global_rect().position.x != reagent_expanded:
		fail("Expand did not restore the bag slots")
		return false
	await cf_capture("06-expanded.png")
	print("FIXTURE BB_TOGGLE collapsed and expanded")
	return true
