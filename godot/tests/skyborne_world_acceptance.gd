extends "res://tests/world_quest_flow.gd"

# Main supplies prepared credentials and launches twice under maintained cage/agent-run.
# Required: GODOT_TEST_SERVER, SKYBORNE_RACE (95|96), SKYBORNE_USERNAME,
# SKYBORNE_PASSWORD, SKYBORNE_SHOTS (absolute path under persistent data/), GAME_ENGINE_CLI,
# SKYBORNE_SCOPE (client-items|full|turn-in|reward-reload|shoulder-assets|ailee-assets). Prepared scopes require race95;
# turn-in requires active completed92460; reward-reload requires rewarded character30/XP40.
# Neither proves fresh acceptance. client-items proves no NPC/giver/quest behavior.
# godot --path godot -s res://tests/skyborne_world_acceptance.gd
# No admin travel/completion, direct quest acceptance or race96 quest invention.
const SKY_QUEST := 92460
const SKY_TITLE := "Coming of Age"
const SKY_ENDER := "Rorian the Dayseeker"
# Entry255979/display139403 -> Extra163071 -> shoulder display734870 -> ModelResources84883.
const SHOULDER_NPC := "Thendal Grove Ranger"
# Captured SQL entry251361/guid3251361000, WoW (x,y,z) -> native (x,z,-y).
const SKY_ENDER_AT := Vector3(4088.500790283203, 976.5145996532464, -1895.0)
const SKY_REWARD_XP := 40
# Server npc_interaction.rs: default reach1.5 +4 +both combat reaches.
const SKY_INTERACTION_RANGE := 8.5
const STARTS := {95: Vector3(4088.501, 976.169, -1847.5), 96: Vector3(4051.417, 977.620, -1858.625)}
const CHARACTERS := {95: "Skymage", 96: "Skyshaman"}
const GIVERS := {95: "Ailee Farheart", 96: "Ventaari Brightwish"}
# Published CharacterLoadoutItem 2376/2377, also in server skyborne_kit_fixture.rs.
const KITS := {95: [35, 117, 159, 6948, 271655, 271658, 271659], 96: [117, 159, 6948, 271661, 271662, 271663, 280400]}

var race := 0
var cli_path := ""
var ipc_socket := ""
var catalog_wait_ms := 60000

func run_test() -> void:
	root.size = Vector2i(1280, 720)
	var server := OS.get_environment("GODOT_TEST_SERVER")
	var port := server.trim_prefix("127.0.0.1:")
	race = OS.get_environment("SKYBORNE_RACE").to_int()
	var scope := OS.get_environment("SKYBORNE_SCOPE")
	if scope not in ["client-items", "full", "turn-in", "reward-reload", "shoulder-assets", "ailee-assets"]:
		fail("Require explicit SKYBORNE_SCOPE=client-items|full|turn-in|reward-reload|shoulder-assets|ailee-assets")
		return
	var prepared := scope in ["turn-in", "reward-reload", "shoulder-assets", "ailee-assets"]
	if prepared:
		if race != 95:
			fail("Prepared scopes require race95 Skymage, never race96")
			return
		catalog_wait_ms = 120000
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
	if not await enter_named_world() or not check_start(not prepared):
		return
	if not await save_capture("01-world-start.png") or not await inspect_kit():
		return
	if scope == "client-items":
		print("SKYBORNE CLIENT_ITEMS_DONE race=", race, " character=", CHARACTERS[race])
		print("SKYBORNE UNTESTED NPC/giver/quest acceptance in client-items scope; server NPC stats are explicitly estimated")
		client.free()
		quit(0)
		return
	if scope == "turn-in":
		if not await turn_in_prepared_quest():
			return
		print("SKYBORNE PREPARED_TURN_IN_DONE race=95 character=Skymage quest=92460 xp=40")
		print("SKYBORNE UNTESTED fresh acceptance/full fixture/original script; no follow-up accepted")
		client.free()
		quit(0)
		return
	if scope == "reward-reload":
		if not await check_reward_reload():
			return
		print("SKYBORNE REWARD_RELOAD_DONE race=95 character=Skymage character_id=30 quest=92460 xp=40")
		print("SKYBORNE UNTESTED fresh acceptance/full fixture/original script; accepted/rewarded nothing")
		client.free()
		quit(0)
		return
	if scope == "ailee-assets":
		if not await check_ailee_baked_appearance() or not await check_authored_shoulders():
			return
		print("SKYBORNE AILEE_ASSETS_DONE race=95 character=Skymage display=136968 bake=7352105")
		print("SKYBORNE UNTESTED quest changes/full fixture/original script; accepted/rewarded nothing")
		client.free()
		quit(0)
		return
	if scope == "shoulder-assets":
		if not await check_authored_shoulders():
			return
		print("SKYBORNE SHOULDER_ASSETS_DONE race=95 character=Skymage npc=", SHOULDER_NPC)
		print("SKYBORNE UNTESTED quest changes/full fixture/original script; accepted/rewarded nothing")
		client.free()
		quit(0)
		return
	var unit := await locate_logical_giver(GIVERS[race])
	if unit == null or unit.global_position.distance_to(player_position()) > 8.0:
		fail("Replicated giver not within 8 yards of actual start")
		return
	print("SKYBORNE GIVER name=", GIVERS[race], " position=", unit.global_position)
	if race == 95:
		if unit.get_node_or_null("NpcVisualRoot") != null:
			fail("Ailee136968 visual must remain withheld")
			return
		print("SKYBORNE GAP Ailee136968 visual withheld; replicated interaction only")
		if not await accept_native_quest():
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
	if not await wait_frames(func():
		var preview := client.get_node_or_null("CharacterSelectScene")
		if preview == null:
			return false
		var tile := preview.find_child("Tile31_37", true, false)
		var selected := preview.get_node_or_null("SelectedCharacter")
		return tile != null and not tile.find_children("*", "MeshInstance3D", true, false).is_empty() and selected != null and not selected.find_children("*", "MeshInstance3D", true, false).is_empty(), "current-WDT preview terrain and selected body", 60000):
		return false
	if not await save_capture("00-character-select-current-wdt.png"):
		return false
	print("SKYBORNE CURRENT_WDT_PREVIEW_READY race=", race)
	await click_control(ui.find_child("EnterWorld", true, false))
	return await wait_frames(func():
		var state: Dictionary = client.account_state()
		return state.connected and state.screen == "InWorld" and state.world_attached and state.local_player_position != null and not state.terrain.parsed_tiles.is_empty(), "native world and terrain", WORLD_WAIT_MS)

func check_start(require_original_start: bool = true) -> bool:
	var state: Dictionary = client.account_state()
	# LoadTerrain uses the Map directory, which the published Map2991 names "2991".
	if state.selected_character_name != CHARACTERS[race] or state.terrain.map != "2991" or not state.terrain.failures.is_empty():
		fail("Wrong named character/map or terrain failure: " + str(state))
		return false
	# WoW (x,y,z) -> native (x,z,-y); tolerate only server grounding precision.
	if state.local_server_position == null or (require_original_start and (state.local_server_position.distance_to(STARTS[race]) > 1.0 or player_position().distance_to(STARTS[race]) > 1.0)):
		fail("Actual local/server start differs from Map2991 approved coordinates: " + str(state))
		return false
	var terrain := client.get_node_or_null("WorldTerrain")
	if terrain == null or terrain.find_children("*", "MeshInstance3D", true, false).is_empty():
		fail("Parsed terrain lacks native terrain meshes")
		return false
	print("SKYBORNE WORLD ", state)
	return true

func inspect_kit() -> bool:
	var readiness_sample := {"next": Time.get_ticks_msec()}
	if not await wait_frames(func():
		var state: Dictionary = client.merchant_state()
		if Time.get_ticks_msec() >= readiness_sample.next:
			print("SKYBORNE INVENTORY_READINESS ", state)
			readiness_sample.next = Time.get_ticks_msec() + 10000
		return state.item_catalog_loaded and not state.equipment.is_empty() and not state.bags.is_empty(), "source catalogs and authoritative starter inventory", catalog_wait_ms):
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

func locate_logical_giver(name: String) -> Node3D:
	var deadline := Time.get_ticks_msec() + 60000
	while Time.get_ticks_msec() < deadline:
		for unit in units_named(name):
			if unit is Node3D:
				return unit
		await process_frame
	fail("Replicated logical giver unavailable: " + name)
	return null

func accept_native_quest() -> bool:
	if in_log(client.quest_state(), SKY_QUEST):
		fail("Prepared Skymage already has92460; fresh native acceptance required")
		return false
	var reply := await ipc(["quest", "interact", "--npc", GIVERS[race]])
	if reply.strip_edges() != "interact " + GIVERS[race]:
		fail("CLI giver interaction failed: " + reply)
		return false
	if not await wait_quest(func(s): return s.frame_open and s.get("npc_name") == GIVERS[race] and s.get("page") in ["Greeting", "Detail"], "replicated giver native QuestFrame"):
		return false
	var state: Dictionary = client.quest_state()
	var giver_id := int(state.npc)
	if giver_id <= 0 or not client.unit_alive(giver_id):
		fail("Quest dialog lacks a living replicated giver")
		return false
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

func check_prepared_turn_in() -> bool:
	var state: Dictionary = client.quest_state()
	var entry := log_entry(state, SKY_QUEST)
	if entry.is_empty() or entry.get("title") != SKY_TITLE or not entry.get("completed", false) or not Array(entry.get("objectives", [])).is_empty():
		fail("Require already-active completed92460 with zero objectives; absent/rewarded quests rejected: " + str(state))
		return false
	if int(state.get("xp", -1)) < 0 or Array(state.system_lines).has(SKY_TITLE + " completed."):
		fail("Prepared turn-in lacks native XP or already has completion receipt: " + str(state))
		return false
	if not tracker_has(client.objective_tracker_state(), SKY_QUEST):
		fail("Prepared92460 missing from native objective tracker")
		return false
	print("SKYBORNE PREPARED_TURN_IN ", state)
	return true

func walk_to_sky_ender() -> Node3D:
	var ender := await locate_logical_giver(SKY_ENDER)
	if ender == null:
		return null
	if ender.global_position.distance_to(SKY_ENDER_AT) > 1.0:
		fail("Replicated Rorian position differs from captured SQL: " + str(ender.global_position))
		return null
	if not visible_body(ender):
		print("SKYBORNE GAP Rorian136966 visible body unavailable; logical replicated ender only")
	if not await walk_to(ender.global_position, 1.0):
		fail("Native key-event walk to Rorian exceeded45s")
		return null
	# walk_to can return after passing the target: require actual settled proximity.
	print("SKYBORNE WALK_RETURN local=", player_position(), " server=", client.account_state().local_server_position, " ender=", ender.global_position)
	var proximity_sample := {"next": Time.get_ticks_msec()}
	if not await wait_frames(func():
		var state: Dictionary = client.account_state()
		if Time.get_ticks_msec() >= proximity_sample.next:
			print("SKYBORNE PROXIMITY local=", player_position(), " server=", state.local_server_position, " ender=", ender.global_position)
			proximity_sample.next = Time.get_ticks_msec() + 1000
		return player_position().distance_to(ender.global_position) <= SKY_INTERACTION_RANGE and state.local_server_position != null and state.local_server_position.distance_to(ender.global_position) <= SKY_INTERACTION_RANGE, "actual local/server server-contract proximity to Rorian", WAIT_MS):
		return null
	var to := ender.global_position - player_position()
	await face_direction(atan2(to.x, to.z))
	print("SKYBORNE ENDER_NEAR replicated=", ender.global_position, " local=", player_position(), " server=", client.account_state().local_server_position)
	return ender

func open_sky_ender(ender: Node3D, pages: Array) -> int:
	var reply := await ipc(["quest", "interact", "--npc", SKY_ENDER])
	if reply.strip_edges() != "interact " + SKY_ENDER:
		fail("CLI ender interaction failed: " + reply)
		return 0
	if not await wait_quest(func(s): return s.frame_open and s.get("npc_name") == SKY_ENDER and s.get("page") in pages, "Rorian native QuestFrame"):
		return 0
	var state: Dictionary = client.quest_state()
	var ender_id := int(state.npc)
	var transform = client.unit_transform(ender_id)
	if ender_id <= 0 or not client.unit_alive(ender_id) or not (transform is Transform3D):
		fail("Native QuestFrame lacks a living replicated Rorian identity")
		return 0
	if transform.origin.distance_to(ender.global_position) > 0.1:
		fail("Native QuestFrame NPC differs from located Rorian")
		return 0
	return ender_id

func check_ailee_baked_appearance() -> bool:
	if int(client.account_state().get("selected_character_id", -1)) != 30 or not check_reward_reload_state():
		return false
	var unit := await locate_logical_giver(GIVERS[95])
	if unit == null:
		return false
	if not await wait_frames(func(): return visible_body(unit), "Ailee authored baked body", WAIT_MS):
		return false
	var visual := unit.get_node("NpcVisualRoot") as Node3D
	var model := visual.get_node_or_null("NpcModel") as Node3D
	if model == null or str(model.get_meta("m2_source_path", "")).get_file() != "7478494.m2":
		fail("Ailee lacks original authored female body7478494")
		return false
	var meshes := model.find_children("Batch*", "MeshInstance3D", true, false)
	var textured := 0
	for mesh in meshes:
		if mesh.mesh == null or not mesh.is_visible_in_tree() or mesh.get_aabb().size.length() <= 0.0:
			continue
		var material := mesh.get_active_material(0) as ShaderMaterial
		if material == null:
			fail("Ailee visible batch lacks authored shader material")
			return false
		var texture := material.get_shader_parameter("base_texture") as Texture2D
		if texture == null or texture.get_image() == null or texture.get_image().is_empty():
			fail("Ailee visible batch lacks nonempty texture")
			return false
		textured += 1
		print("SKYBORNE AILEE_MATERIAL mesh=", mesh.name, " resource=", texture.get_instance_id(), " size=", texture.get_size())
	if textured == 0:
		fail("Ailee lacks visible textured authored meshes")
		return false
	print("SKYBORNE AILEE_VISIBLE name=", GIVERS[95], " path=", model.get_path(), " meshes=", textured)
	return await capture_npc_views(visual, "02-ailee-authored")

func check_authored_shoulders() -> bool:
	if int(client.account_state().get("selected_character_id", -1)) != 30 or not check_reward_reload_state():
		return false
	var unit := await locate_logical_giver(SHOULDER_NPC)
	if unit == null:
		return false
	for candidate in units_named(SHOULDER_NPC):
		if candidate is Node3D and candidate.global_position.distance_to(player_position()) < unit.global_position.distance_to(player_position()):
			unit = candidate
	if not await wait_frames(func(): return visible_body(unit), "Grove Ranger authored body before shoulder attachment check", WAIT_MS):
		return false
	var visual := unit.get_node("NpcVisualRoot")
	for side in ["Left", "Right"]:
		var item := visual.find_child("EquipmentShoulder" + side, true, false) as Node3D
		var attachment := 6 if side == "Left" else 5
		if item == null or item.get_parent().name != "Attachment%d" % attachment:
			fail("Grove Ranger original shoulder " + side + " missing from authored attachment " + str(attachment))
			return false
		var source := str(item.get_meta("m2_source_path", ""))
		var expected_model := "7579617.m2" if side == "Left" else "7579618.m2"
		if source.get_file() != expected_model:
			fail("Unexpected original shoulder source model: " + source)
			return false
		var meshes := item.find_children("Batch*", "MeshInstance3D", true, false)
		if meshes.is_empty():
			fail("Grove Ranger shoulder " + side + " has no native geometry")
			return false
		for mesh in meshes:
			if mesh.mesh == null or not mesh.is_visible_in_tree() or mesh.get_aabb().size.length() <= 0.0:
				fail("Grove Ranger shoulder " + side + " lacks visible nonempty geometry")
				return false
			if not check_shoulder_material(mesh, side):
				return false
		print("SKYBORNE SHOULDER_ATTACHED side=", side, " attachment=", attachment, " path=", item.get_path(), " transform=", item.transform, " meshes=", meshes.size())
	return await capture_npc_views(visual, "02-grove-ranger-shoulders")

func check_shoulder_material(mesh: MeshInstance3D, side: String) -> bool:
	var material := mesh.get_active_material(0) as ShaderMaterial
	if material == null or material.shader == null:
		fail("Shoulder " + side + " lacks active shader material")
		return false
	for parameter in ["base_texture", "second_texture", "third_texture", "fourth_texture"]:
		var texture := material.get_shader_parameter(parameter) as Texture2D
		if texture == null:
			if parameter == "base_texture":
				fail("Shoulder " + side + " lacks base texture binding")
				return false
			continue
		var image := texture.get_image()
		if image == null or image.is_empty():
			fail("Shoulder " + side + " has empty bound texture " + parameter)
			return false
		var file: String = shots + "material-" + side + "-" + str(mesh.name) + "-" + parameter + ".png"
		if image.save_png(file) != OK:
			fail("Cannot save actual bound shoulder texture " + file)
			return false
		# GPU resources do not expose source FDIDs; do not infer identity from names.
		print("SKYBORNE SHOULDER_MATERIAL side=", side, " mesh=", mesh.name, " parameter=", parameter, " resource=", texture.get_instance_id(), " size=", image.get_size(), " format=", image.get_format(), " file=", file)
	return true

func capture_npc_views(visual: Node3D, prefix: String) -> bool:
	# Share the actual world; moving the visual into a new World3D produced blank captures.
	var capture := SubViewport.new()
	capture.size = Vector2i(1024, 1024)
	capture.world_3d = visual.get_world_3d()
	capture.render_target_update_mode = SubViewport.UPDATE_ALWAYS
	root.add_child(capture)
	var camera := Camera3D.new()
	camera.projection = Camera3D.PROJECTION_ORTHOGONAL
	camera.size = 3.4
	capture.add_child(camera)
	camera.make_current()
	for view in ["front", "back"]:
		var direction := 1.0 if view == "front" else -1.0
		camera.position = visual.global_position + Vector3(4.0 * direction, 1.35, 5.0 * direction)
		camera.look_at(visual.global_position + Vector3(0.0, 1.0, 0.0))
		for frame in 8:
			await process_frame
		await RenderingServer.frame_post_draw
		var image := capture.get_texture().get_image()
		var file: String = shots + prefix + "-" + view + ".png"
		if image == null or image.is_empty() or image.save_png(file) != OK:
			capture.queue_free()
			fail("Cannot save actual authored NPC view " + file)
			return false
		print("SKYBORNE NPC_VIEW ", file, " origin=", visual.global_position)
	capture.queue_free()
	return true

func check_reward_reload_state() -> bool:
	var state: Dictionary = client.quest_state()
	var tracker: Dictionary = client.objective_tracker_state()
	if int(state.get("xp", -1)) != SKY_REWARD_XP or in_log(state, SKY_QUEST) or Array(state.watched).has(SKY_QUEST) or tracker_has(tracker, SKY_QUEST):
		fail("Reward reload requires exact40XP and92460 absent from log/watched/tracker: " + str(state) + " tracker=" + str(tracker))
		return false
	print("SKYBORNE REWARD_RELOAD_NATIVE state=", state, " tracker=", tracker)
	return true

func check_reward_reload() -> bool:
	if int(client.account_state().get("selected_character_id", -1)) != 30:
		fail("Reward reload requires prepared character30 Skymage")
		return false
	if not check_reward_reload_state():
		return false
	var ender := await walk_to_sky_ender()
	if ender == null:
		return false
	var ender_id := await open_sky_ender(ender, ["Greeting"])
	if ender_id <= 0:
		return false
	var state: Dictionary = client.quest_state()
	if Array(state.greeting_quests).has(SKY_TITLE):
		fail("Rewarded92460 still offered in Rorian native Greeting: " + str(state))
		return false
	if not check_reward_reload_state():
		return false
	return await save_capture("02-reward-reload-greeting-40xp.png")

func turn_in_prepared_quest() -> bool:
	if not check_prepared_turn_in():
		return false
	var ender := await walk_to_sky_ender()
	if ender == null:
		return false
	var ender_id := await open_sky_ender(ender, ["Greeting", "Reward"])
	if ender_id <= 0:
		return false
	var state: Dictionary = client.quest_state()
	if state.page == "Greeting":
		var index := Array(state.greeting_quests).find(SKY_TITLE)
		if index < 0:
			fail("Rorian greeting lacks Coming of Age: " + str(state))
			return false
		if not await click_visible("QuestTitleButton%dText" % (index + 1)):
			return false
	if not await wait_quest(func(s): return s.frame_open and s.get("npc") == ender_id and s.get("npc_name") == SKY_ENDER and s.get("page") == "Reward" and s.quest_id == SKY_QUEST, "92460 native reward"):
		return false
	state = client.quest_state()
	if int(state.get("choices", -1)) != 0:
		fail("Prepared92460 reward must have zero choices: " + str(state))
		return false
	if not check_prepared_turn_in():
		return false
	var xp_before := int(state.xp)
	print("SKYBORNE REWARD_BEFORE xp=", xp_before, " state=", state)
	if not await save_capture("02-prepared-reward.png") or not await click_visible("QuestFrameCompleteQuestButton"):
		return false
	if not await wait_quest(func(s): return not in_log(s, SKY_QUEST) and Array(s.system_lines).has(SKY_TITLE + " completed.") and int(s.get("xp", -1)) == xp_before + SKY_REWARD_XP, "92460 removed/log/chat and native +40XP"):
		return false
	if tracker_has(client.objective_tracker_state(), SKY_QUEST):
		fail("Turned-in92460 remains in native objective tracker")
		return false
	print("SKYBORNE NATIVE_TURNED_IN xp=", xp_before, " -> ", client.quest_state().xp, " state=", client.quest_state(), " tracker=", client.objective_tracker_state())
	return await save_capture("03-prepared-tracker-40xp.png")

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
