extends "res://tests/world_merchant_live_flow.gd"

# Read-only native acceptance: genuine Zaralda (NPC 243531), not a synthetic vendor.
# Main owns the private server, isolated character, native build and GUI execution.
# Character setup: WoW 8729.9912, -4547.7160, 23.438097, beside Zaralda.
# Run from game-engine with already-built matching extension and pinned Godot:
# agent-run zaralda godot --path godot -s res://tests/world_zaralda_merchant_flow.gd
# Required environment (no credential/server defaults): GODOT_TEST_SERVER=127.0.0.1:5197,
# ZARALDA_TEST_USERNAME, ZARALDA_TEST_PASSWORD, ZARALDA_TEST_CHARACTER,
# ZARALDA_TEST_SHOTS=<absolute persistent directory>. Capture stdout to that directory.
# Bounded cold first visit: 240s world wait for local CASC only; never use CDN.
# No buy/sell/repair/junk input. Merchant snapshot exposes names, not item IDs;
# the authored item tooltip independently supplies the matching Item ID.
# Native NPC template ID is not exposed here: main must retain server identity evidence.

const ZARALDA := "Zaralda"
const ZARALDA_WORLD_WAIT_MS := 240000
const MIDNIGHT_ITEMS := {
	244586: "Smuggler's Leather Wristbands",
	244589: "Scout's Scaled Bracers",
}

var zaralda_character := ""

func run_test() -> void:
	root.size = Vector2i(1280, 720)
	var server := OS.get_environment("GODOT_TEST_SERVER")
	var username := OS.get_environment("ZARALDA_TEST_USERNAME")
	var password := OS.get_environment("ZARALDA_TEST_PASSWORD")
	zaralda_character = OS.get_environment("ZARALDA_TEST_CHARACTER")
	shots = OS.get_environment("ZARALDA_TEST_SHOTS")
	if server != "127.0.0.1:5197" or username.is_empty() or password.is_empty() or zaralda_character.is_empty():
		fail("Explicit private server and all ZARALDA_TEST credentials/character are required")
		return
	if not shots.is_absolute_path() or shots.begins_with("/tmp/"):
		fail("ZARALDA_TEST_SHOTS must be an absolute persistent directory, not /tmp")
		return
	if DisplayServer.get_name() == "headless":
		fail("Zaralda acceptance requires a native window, not headless")
		return
	var error := DirAccess.make_dir_recursive_absolute(shots)
	if error != OK:
		fail("Cannot create screenshot directory: " + str(error))
		return
	client = load("res://scenes/client.tscn").instantiate()
	root.add_child(client)
	var connection_error = client.connect_account(server, username, password, false)
	if connection_error != "":
		fail("Zaralda connection: " + connection_error)
		return
	if not await enter_zaralda_world():
		return
	var npc := await find_named_vendor(ZARALDA)
	if npc.is_empty():
		return
	print("ZARALDA PICK server_entity=", npc.id, " point=", npc.point)
	await hover_point(npc.point)
	await frames(2)
	var hover_cursor: String = client.merchant_state().cursor
	print("ZARALDA HOVER cursor=", hover_cursor)
	if not await open_named_vendor(npc, ZARALDA):
		fail("Zaralda native merchant did not open")
		return
	var state: Dictionary = client.merchant_state()
	if state.npc != npc.id or client.target_state().target != npc.id:
		fail("Merchant/target entity differs from actual UnitPick: " + str(state))
		return
	if merchant_text("MerchantFrameTitleText") != ZARALDA or not merchant_visible("ContainerFrame0"):
		fail("Zaralda title or visible backpack missing")
		return
	if not await inspect_midnight_item(state):
		return
	if not await save_shot("01-zaralda-merchant.png"):
		return
	var evidence := {
		"server": server,
		"character": zaralda_character,
		"account_state": client.account_state(),
		"unit_pick_server_entity": npc.id,
		"hover_cursor": hover_cursor,
		"merchant": state,
		"target": client.target_state(),
		"tooltip": client.tooltip_state(),
		"title": merchant_text("MerchantFrameTitleText"),
		"backpack_visible": merchant_visible("ContainerFrame0"),
	}
	await click_control(merchant_control("MerchantFrameCloseButton"), MOUSE_BUTTON_LEFT)
	if not await wait_for(func(s): return not s.open, "Zaralda close"):
		return
	if merchant_visible("MerchantFrame"):
		fail("Zaralda frame remains visible after close")
		return
	if not await save_shot("02-zaralda-closed.png"):
		return
	var file := FileAccess.open(shots.path_join("zaralda-evidence.json"), FileAccess.WRITE)
	if file == null:
		fail("Cannot write Zaralda evidence: " + str(FileAccess.get_open_error()))
		return
	file.store_string(JSON.stringify(evidence, "\t"))
	var write_error := file.get_error()
	file.close()
	if write_error != OK:
		fail("Cannot save Zaralda evidence: " + str(write_error))
		return
	print("ZARALDA READ_ONLY DONE evidence=", shots.path_join("zaralda-evidence.json"))
	client.free()
	quit(0)

func enter_zaralda_world() -> bool:
	var ui: Node = null
	var deadline := Time.get_ticks_msec() + WORLD_WAIT_MS
	while ui == null and Time.get_ticks_msec() < deadline:
		await process_frame
		var state: Dictionary = client.account_state()
		if state.reply_received and state.screen == "CharacterSelect" and state.character_count >= 1:
			ui = client.get_node_or_null("CharacterSelectUI")
	if ui == null:
		fail("No character select: " + str(client.account_state()))
		return false
	await frames(10)
	var index := 0
	var card: Control = null
	while card == null:
		var candidate = ui.find_child("CharCard_%d" % index, true, false)
		if not candidate is Control:
			break
		for label in candidate.find_children("*", "Label", true, false):
			if label.text == zaralda_character:
				card = candidate
		index += 1
	if card == null:
		fail("Roster has no " + zaralda_character)
		return false
	await click_control(card, MOUSE_BUTTON_LEFT)
	await click_control(ui.find_child("EnterWorld", true, false), MOUSE_BUTTON_LEFT)
	deadline = Time.get_ticks_msec() + ZARALDA_WORLD_WAIT_MS
	while Time.get_ticks_msec() < deadline:
		await process_frame
		var state: Dictionary = client.account_state()
		if state.screen == "InWorld" and state.local_player_position != null:
			if state.selected_character_name != zaralda_character:
				fail("Entered unexpected character")
				return false
			await frames(60)
			print("ZARALDA IN_WORLD ", state.selected_character_name, " at ", state.local_player_position)
			return true
	fail("Timed out entering Zaralda world (local CASC only): " + str(client.account_state()))
	return false

func inspect_midnight_item(state: Dictionary) -> bool:
	for item_id in MIDNIGHT_ITEMS:
		var item_name: String = MIDNIGHT_ITEMS[item_id]
		var index: int = state.items.find(item_name)
		if index < 0:
			continue
		for page in range(int(index / 10)):
			await click_control(merchant_control("MerchantNextPageButton"), MOUSE_BUTTON_LEFT)
			await frames(3)
		var cell := "MerchantItem%d" % (index % 10 + 1)
		if merchant_text(cell + "Name") != item_name:
			fail("Expected visible Midnight item " + item_name)
			return false
		await hover_point(merchant_control(cell).get_global_rect().get_center())
		var deadline := Time.get_ticks_msec() + REPLY_WAIT_MS
		while Time.get_ticks_msec() < deadline:
			await process_frame
			var tooltip: Dictionary = client.tooltip_state()
			if tooltip.visible and tooltip.title == item_name and tooltip.lines.has("Item ID: %d|" % item_id):
				print("ZARALDA MIDNIGHT item_id=", item_id, " name=", item_name, " tooltip=", tooltip)
				return true
		fail("Midnight item tooltip did not confirm ID %d: %s" % [item_id, client.tooltip_state()])
		return false
	fail("Neither source-backed Midnight wrist item is available: " + str(state.items))
	return false

func save_shot(filename: String) -> bool:
	await RenderingServer.frame_post_draw
	var error := root.get_texture().get_image().save_png(shots.path_join(filename))
	if error != OK:
		fail("Cannot save screenshot " + filename + ": " + str(error))
		return false
	return true

func fail(message: String) -> void:
	push_error(message)
	if is_instance_valid(client):
		client.free()
	quit(1)
