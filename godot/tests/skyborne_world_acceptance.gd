extends "res://tests/world_quest_flow.gd"

# Main supplies prepared credentials and launches twice under maintained cage/agent-run.
# Required: GODOT_TEST_SERVER, SKYBORNE_RACE (95|96), SKYBORNE_USERNAME,
# SKYBORNE_PASSWORD, SKYBORNE_SHOTS (absolute path under persistent data/), GAME_ENGINE_CLI,
# SKYBORNE_SCOPE (client-items|full). client-items proves no NPC/giver/quest behavior.
# godot --path godot -s res://tests/skyborne_world_acceptance.gd
# No admin travel/completion, direct quest acceptance or race96 quest invention.
const SKY_QUEST := 92460
const SKY_TITLE := "Coming of Age"
const STARTS := {95: Vector3(4088.501, 976.169, -1847.5), 96: Vector3(4051.417, 977.620, -1858.625)}
const CHARACTERS := {95: "Skymage", 96: "Skyshaman"}
const GIVERS := {95: "Ailee Farheart", 96: "Ventaari Brightwish"}
# Published CharacterLoadoutItem 2376/2377, also in server skyborne_kit_fixture.rs.
const KITS := {95: [35, 117, 159, 6948, 271655, 271658, 271659], 96: [117, 159, 6948, 271661, 271662, 271663, 280400]}

var race := 0
var cli_path := ""
var ipc_socket := ""

func run_test() -> void:
	root.size = Vector2i(1280, 720)
	var server := OS.get_environment("GODOT_TEST_SERVER")
	var port := server.trim_prefix("127.0.0.1:")
	race = OS.get_environment("SKYBORNE_RACE").to_int()
	var scope := OS.get_environment("SKYBORNE_SCOPE")
	if scope not in ["client-items", "full"]:
		fail("Require explicit SKYBORNE_SCOPE=client-items|full")
		return
	var username := OS.get_environment("SKYBORNE_USERNAME")
	var password := OS.get_environment("SKYBORNE_PASSWORD")
	shots = OS.get_environment("SKYBORNE_SHOTS").simplify_path()
	cli_path = OS.get_environment("GAME_ENGINE_CLI")
	if not server.begins_with("127.0.0.1:") or not port.is_valid_int() or int(port) < 1 or int(port) > 65535 or int(port) == 5000:
		fail("GODOT_TEST_SERVER requires private 127.0.0.1:port, never :5000")
		return
	if not CHARACTERS.has(race) or username.is_empty() or password.is_empty():
		fail("Require race95/96 and explicit prepared account credentials")
		return
	if not shots.is_absolute_path() or not ("/data/" in shots + "/") or shots.begins_with("/tmp/") or not cli_path.is_absolute_path() or not FileAccess.file_exists(cli_path):
		fail("Require persistent data/ shots and absolute existing GAME_ENGINE_CLI")
		return
	if DisplayServer.get_name() == "headless":
		fail("Native cage window required, not headless")
		return
	if DirAccess.make_dir_recursive_absolute(shots) != OK:
		fail("Cannot create screenshot directory")
		return
	shots = shots.trim_suffix("/") + "/"
	# Includes startup, streaming, seven bounded CLI inspections and UI acceptance.
	create_timer(600.0).timeout.connect(func(): fail("Skyborne total deadline exceeded"))
	ipc_socket = "/tmp/game-engine-%d.sock" % OS.get_process_id()
	client = load("res://scenes/client.tscn").instantiate()
	root.add_child(client)
	var error = client.connect_account(server, username, password, false)
	if error != "":
		fail("Skyborne connection: " + error)
		return
	if not await enter_named_world() or not check_start():
		return
	if not await save_capture("01-world-start.png") or not await inspect_kit():
		return
	if scope == "client-items":
		print("SKYBORNE CLIENT_ITEMS_DONE race=", race, " character=", CHARACTERS[race])
		print("SKYBORNE BLOCKED NPC/giver/quest acceptance: missing authored health/class data; no substitute")
		client.free()
		quit(0)
		return
	var giver := await locate(GIVERS[race])
	if giver.is_empty():
		return
	var unit := unit_by_id(giver.id) as Node3D
	if unit == null or unit.global_position.distance_to(player_position()) > 8.0:
		fail("Replicated giver not within 8 yards of actual start")
		return
	print("SKYBORNE GIVER name=", GIVERS[race], " entity=", giver.id, " position=", unit.global_position)
	if race == 95:
		if unit.get_node_or_null("NpcVisualRoot") != null:
			fail("Ailee136968 visual must remain withheld")
			return
		print("SKYBORNE GAP Ailee136968 visual withheld; replicated interaction only")
		if not await accept_native_quest(giver.id):
			return
	else:
		if not await wait_frames(func(): return visible_body(unit), "Ventaari native visible meshes", 60000):
			return
		if not await save_capture("02-ventaari-world.png"):
			return
		print("SKYBORNE VENTAARI_VISIBLE; GAP race96 level1 Windshaper intro unproven; no quest attempted")
	print("SKYBORNE ACCEPTANCE_DONE race=", race, " character=", CHARACTERS[race])
	client.free()
	quit(0)

func enter_named_world() -> bool:
	if not await wait_frames(func(): return client.get_node_or_null("CharacterSelectUI") != null and client.account_state().screen == "CharacterSelect", "authenticated roster", 60000):
		return false
	var ui := client.get_node("CharacterSelectUI")
	var card: Control
	for index in range(int(client.account_state().character_count)):
		var candidate := ui.find_child("CharCard_%d" % index, true, false) as Control
		if candidate == null:
			continue
		for label in candidate.find_children("*", "Label", true, false):
			if label.text == CHARACTERS[race]:
				card = candidate
	if card == null:
		fail("Prepared roster lacks named character " + CHARACTERS[race])
		return false
	await click_control(card)
	# Roster selection sets selected_index; the session name arrives on world entry.
	await process_frame
	await click_control(ui.find_child("EnterWorld", true, false))
	return await wait_frames(func():
		var state: Dictionary = client.account_state()
		return state.connected and state.screen == "InWorld" and state.world_attached and state.local_player_position != null and not state.terrain.parsed_tiles.is_empty(), "native world and terrain", WORLD_WAIT_MS)

func check_start() -> bool:
	var state: Dictionary = client.account_state()
	# LoadTerrain uses the Map directory, which the published Map2991 names "2991".
	if state.selected_character_name != CHARACTERS[race] or state.terrain.map != "2991" or not state.terrain.failures.is_empty():
		fail("Wrong named character/map or terrain failure: " + str(state))
		return false
	# WoW (x,y,z) -> native (x,z,-y); tolerate only server grounding precision.
	if state.local_server_position == null or state.local_server_position.distance_to(STARTS[race]) > 1.0 or player_position().distance_to(STARTS[race]) > 1.0:
		fail("Actual local/server start differs from Map2991 approved coordinates: " + str(state))
		return false
	var terrain := client.get_node_or_null("WorldTerrain")
	if terrain == null or terrain.find_children("*", "MeshInstance3D", true, false).is_empty():
		fail("Parsed terrain lacks native terrain meshes")
		return false
	print("SKYBORNE WORLD ", state)
	return true

func inspect_kit() -> bool:
	if not await wait_frames(func():
		var state: Dictionary = client.merchant_state()
		return state.item_catalog_loaded and not state.equipment.is_empty() and not state.bags.is_empty(), "source catalogs and authoritative starter inventory", 60000):
		return false
	var inventory: Dictionary = client.merchant_state()
	var actual := []
	for item in Array(inventory.bags) + Array(inventory.equipment):
		if int(item.count) <= 0:
			fail("Starter item has no owned quantity: " + str(item))
			return false
		if not actual.has(int(item.item_id)):
			actual.append(int(item.item_id))
	actual.sort()
	if actual != KITS[race]:
		fail("Actual starter IDs differ from published loadout: " + str(inventory))
		return false
	print("SKYBORNE OWNED_KIT ", inventory)
	if (await ipc(["inventory", "list"])).is_empty():
		return false
	for item_id in KITS[race]:
		var text := await ipc(["item", "info", "--item-id", str(item_id), "--source", "forever70205"])
		# Existing source-aware query checks bags AND equipment against requested source.
		if not text.split("\n").has("item_id: " + str(item_id)) or not text.split("\n").has("appearance_known: true"):
			fail("CLI cannot prove owned Forever70205 item " + str(item_id) + ": " + text)
			return false
	print("SKYBORNE SOURCE_KIT Forever70205 IDs=", actual, "; GAP snapshots do not expose GUID-level source")
	return true

func accept_native_quest(giver_id: int) -> bool:
	if in_log(client.quest_state(), SKY_QUEST):
		fail("Prepared Skymage already has92460; fresh native acceptance required")
		return false
	var reply := await ipc(["quest", "interact", "--npc", GIVERS[race]])
	if reply.strip_edges() != "interact " + GIVERS[race]:
		fail("CLI giver interaction failed: " + reply)
		return false
	if not await wait_quest(func(s): return s.frame_open and s.get("npc") == giver_id and s.get("page") in ["Greeting", "Detail"], "replicated giver native QuestFrame"):
		return false
	var state: Dictionary = client.quest_state()
	if state.page == "Greeting":
		var index := Array(state.greeting_quests).find(SKY_TITLE)
		if index < 0:
			fail("Ailee does not offer published92460 title: " + str(state))
			return false
		if not await save_capture("02-quest-greeting.png") or not await click_visible("QuestTitleButton%dText" % (index + 1)):
			return false
	if not await wait_quest(func(s): return s.frame_open and s.get("npc") == giver_id and s.get("page") == "Detail" and s.quest_id == SKY_QUEST, "92460 native detail"):
		return false
	if not await save_capture("03-quest-detail.png") or not await click_visible("QuestFrameAcceptButton"):
		return false
	if not await wait_quest(func(s): return in_log(s, SKY_QUEST) and not s.frame_open and Array(s.system_lines).has("Quest accepted: " + SKY_TITLE), "92460 server acceptance/log/chat"):
		return false
	if not tracker_has(client.objective_tracker_state(), SKY_QUEST):
		fail("Accepted92460 missing from native objective tracker")
		return false
	print("SKYBORNE NATIVE_ACCEPTED ", log_entry(client.quest_state(), SKY_QUEST))
	return await save_capture("04-quest-accepted.png")

func click_visible(name: String) -> bool:
	var control := quest_control("QuestFrameUI", name)
	if control == null or not control.is_visible_in_tree() or (control is BaseButton and control.disabled):
		fail("Native quest control unavailable: " + name)
		return false
	await click_control(control)
	return true

func visible_body(unit: Node) -> bool:
	var visual := unit.get_node_or_null("NpcVisualRoot")
	if visual != null:
		for mesh in visual.find_children("Batch*", "MeshInstance3D", true, false):
			if mesh.mesh != null and mesh.is_visible_in_tree():
				return true
	return false

func ipc(arguments: Array) -> String:
	# OS.execute must NOT block the frame thread that serves this client's IPC socket.
	var worker := Thread.new()
	var argv := ["--signal=KILL", "15", cli_path, "--socket", ipc_socket] + arguments
	var error := worker.start(func():
		var output := []
		var code := OS.execute("/usr/bin/timeout", argv, output, true)
		return {"code": code, "text": "\n".join(output)})
	if error != OK:
		fail("Cannot start bounded IPC worker: " + str(error))
		return ""
	while worker.is_alive():
		await process_frame
	var result: Dictionary = worker.wait_to_finish()
	print("SKYBORNE CLI ", arguments, " exit=", result.code, "\n", result.text)
	if result.code != 0:
		fail("CLI failed or exceeded15s: " + str(arguments))
		return ""
	return result.text

func save_capture(file: String) -> bool:
	await RenderingServer.frame_post_draw
	var image := root.get_texture().get_image()
	if image == null or image.is_empty() or image.save_png(shots + file) != OK:
		fail("Cannot save native capture " + shots + file)
		return false
	print("SKYBORNE SCREENSHOT ", shots + file)
	return true
