extends "res://tests/toy_effects_live.gd"
# Reuses native input/wait helpers only. Real private realm, authored resource/NPC spawns.
var skin := OS.get_environment("GATHERING_SKIN")
var scenario := OS.get_environment("GATHERING_SCENARIO")
var shots := OS.get_environment("GATHERING_SHOTS")
var failed := false
var skill_text := ""

func run_fixture() -> void:
	if OS.get_environment("GAME_SERVER_ADMIN_SOCKET") != "/tmp/game-server-admin-5720.sock": fail("private socket required"); return
	root.size = Vector2i(1920, 1080)
	client = load("res://scenes/client.tscn").instantiate()
	root.add_child(client)
	if not await wait_until(func(): return not client.account_state().assets_starting, "assets", 180000): return
	var error: String = client.connect_account("127.0.0.1:5720", "fb_gathering_nodes", "fbtest", false)
	if error != "": fail(error); return
	if not await wait_until(func(): return control("CharCard_0") != null, "roster", 120000): return
	await checkpoint("roster")
	var card := ""
	for index in range(int(client.account_state().character_count)):
		var title := control("CharCard_%dName" % index)
		if title != null and title.text == character: card = "CharCard_%d" % index; break
	if card.is_empty(): fail("requested roster character absent: " + character); return
	await click(card)
	await click("EnterWorld")
	if not await wait_until(func(): return client.account_state().screen == "InWorld" and client.account_state().world_attached, "world", 180000): return
	if not await wait_until(func():
		var nodes := client.find_children(character, "Node3D", true, false)
		if nodes.is_empty(): return false
		actor = nodes[0]
		return not actor.find_children("PlayerModel", "Node3D", true, false).is_empty(), "native player", 120000): return
	if client.account_state().ui_skin != skin: fail("requested skin not applied: " + str(client.account_state())); return
	await capture("world-baseline")
	await checkpoint("world")
	if scenario == "skin": await prove_skinning()
	else:
		for resource in JSON.parse_string(OS.get_environment("GATHERING_NODES")):
			await prove_node(resource)
			if failed: return
	if failed: return
	write_receipts()
	print("PASS gathering live ", skin, " ", scenario, " ", JSON.stringify(receipts))
	client.free()
	quit(0)

func admin(arguments: Array) -> bool:
	return command(OS.get_environment("GATHERING_AGENT_RUN"), ["gathering-live", OS.get_environment("GATHERING_ADMIN")] + arguments)

func prove_node(resource: Dictionary) -> void:
	var label: String = resource.label
	if not admin(["set-position", character, str(float(resource.x) - 5.0), str(resource.y), str(resource.z)]): return
	if not await wait_until(func(): return resource_node(int(resource.entry)) != null, label + " model", 60000): return
	await create_timer(2.0).timeout
	var object := resource_node(int(resource.entry))
	var model := object.get_node("GameObjectModel") as Node3D
	if int(object.get_meta("game_object_display_id")) != int(resource.display): fail("authored resource display mismatch"); return
	if model.find_children("*", "MeshInstance3D", true, false).is_empty(): fail("resource has no meshes"); return
	await checkpoint(label + "-position", object)
	await capture_skill(label + "-skill-before", label)
	var before := skill_rank()
	if label == "herb":
		await click("MinimapClusterTrackingBackground")
		if not await wait_until(func(): return control("MinimapTrackingChoice9") != null, "learned Find Herbs"): return
		await click("MinimapTrackingRow9")
		await click("MinimapTrackingRow10")
		await capture("tracking")
		await click("MinimapClusterTrackingBackground")
	await click_world(object)
	if not await wait_until(func(): return int(client.spells_state().casting) == int(resource.spell), label + " cast", 5000): return
	await capture(label + "-cast")
	if not await wait_until(func(): return control("LootFrame") != null, label + " loot"): return
	await capture(label + "-loot")
	await checkpoint(label + "-loot", object)
	if not await wait_until(func(): return control("LootFrame") == null, label + " taken loot"): return
	await capture_skill(label + "-skill-after", label)
	if skill_rank() != before + 1: fail(label + " did not raise skill: " + skill_text); return
	if not await wait_until(func(): return not is_instance_valid(object), label + " despawn", 60000): return
	await capture(label + "-despawn")
	await checkpoint(label + "-despawn")
	var position := Vector3(float(resource.x), float(resource.z), -float(resource.y))
	if not await wait_until(func():
		var respawn := resource_node(int(resource.entry))
		return respawn != null and respawn.global_position.distance_to(position) < 0.1, label + " respawn", 240000): return
	await capture(label + "-respawn")
	await checkpoint(label + "-respawn", resource_node(int(resource.entry)))

func capture_skill(label: String, resource: String) -> void:
	await tap(KEY_K)
	if not await wait_until(func():
		var text := control("ProfessionsSkillText")
		return text != null and text.text.begins_with("Classic"), label + " skill display", 60000): return
	if resource == "ore":
		if not admin(["learn-spell", character, "2657"]): return
		if not await wait_until(func(): return control("ProfessionRecipe2657") != null, "learned Smelt Copper"): return
		await click("ProfessionRecipe2657")
		await create_timer(0.5).timeout
	skill_text = control("ProfessionsSkillText").text
	var expected: String = {"herb": "Classic Herbalism", "ore": "Classic Mining", "skin": "Classic Skinning"}[resource]
	if not skill_text.begins_with(expected): fail("wrong progress line: " + skill_text); return
	await capture(label)
	await tap(KEY_ESCAPE)

func skill_rank() -> int:
	return skill_text.get_slice("/", 0).get_slice(" ", 2).to_int()

func prove_skinning() -> void:
	await capture_skill("skinning-skill-before", "skin")
	var before := skill_rank()
	var resource: Dictionary = JSON.parse_string(OS.get_environment("GATHERING_WOLF"))
	if not admin(["set-position", character, str(float(resource.x) - 3.0), str(resource.y), str(resource.z)]): return
	if not await wait_until(func(): return wolf_node() != null, "authored wolf model", 60000): return
	await create_timer(2.0).timeout
	var wolf := wolf_node()
	await checkpoint("wolf-position", wolf)
	await click_world(wolf)
	await checkpoint("kill-started", wolf)
	if not await wait_until(func(): return client.target_state().health_text == "Dead", "own wolf kill", 90000): return
	await capture("wolf-kill")
	await click_world(wolf)
	if not await wait_until(func(): return control("LootFrame") != null, "corpse loot"): return
	await checkpoint("corpse-loot", wolf)
	if not await wait_until(func(): return control("LootFrame") == null, "corpse emptied"): return
	await create_timer(1.0).timeout
	await click_world(wolf)
	if not await wait_until(func(): return int(client.spells_state().casting) == 8613, "skinning cast", 5000): return
	await capture("skinning-cast")
	if not await wait_until(func(): return control("LootFrame") != null, "skinning loot"): return
	await capture("skinning-loot")
	await checkpoint("skinning-loot", wolf)
	if not await wait_until(func(): return control("LootFrame") == null, "skinning loot emptied"): return
	await capture_skill("skinning-skill-after", "skin")
	if skill_rank() != before + 1: fail("Skinning did not raise skill: " + skill_text); return

func resource_node(entry: int) -> Node3D:
	var closest: Node3D
	var distance := INF
	for node in client.find_children("*", "Node3D", true, false):
		if node.has_meta("game_object_entry") and int(node.get_meta("game_object_entry")) == entry:
			var current: float = node.global_position.distance_to(actor.global_position)
			if current < distance: closest = node; distance = current
	return closest

func wolf_node() -> Node3D:
	var closest: Node3D
	var distance := INF
	for node in client.find_children("Mangy Wolf*", "Node3D", true, false):
		if node.find_children("NpcModel", "Node3D", true, false).is_empty(): continue
		var current: float = node.global_position.distance_to(actor.global_position)
		if current < distance: closest = node; distance = current
	return closest

func click_world(node: Node3D) -> void:
	var camera := root.get_camera_3d()
	var point := camera.unproject_position(node.global_position + Vector3.UP)
	if not Rect2(Vector2.ZERO, Vector2(root.size)).has_point(point): fail("resource outside view " + str(point)); return
	var motion := InputEventMouseMotion.new()
	motion.position = point
	motion.global_position = point
	root.push_input(motion, true)
	await process_frame
	for down in [true, false]:
		var event := InputEventMouseButton.new()
		event.position = point
		event.global_position = point
		event.button_index = MOUSE_BUTTON_RIGHT
		event.pressed = down
		root.push_input(event, true)
		await process_frame

func checkpoint(label: String, object: Node3D = null) -> void:
	var record := {"label": label, "pid": OS.get_process_id(), "account": client.account_state()}
	if object != null:
		var delta := object.global_position - actor.global_position
		record["yaw"] = rad_to_deg(atan2(-delta.x, -delta.z))
		record["position"] = [object.global_position.x, object.global_position.y, object.global_position.z]
	var ready := output.path_join(skin + "-" + scenario + "-" + label + ".ready.json")
	var file := FileAccess.open(ready, FileAccess.WRITE)
	file.store_string(JSON.stringify(record))
	file.close()
	var ack := output.path_join(skin + "-" + scenario + "-" + label + ".ack")
	await wait_until(func(): return FileAccess.file_exists(ack), "external proof " + label, 240000)

func capture(label: String) -> void:
	await RenderingServer.frame_post_draw
	var path := shots.path_join(skin.to_lower() + "-" + scenario + "-" + label + ".png")
	if root.get_texture().get_image().save_png(path) != OK: fail("capture " + path); return
	receipts.append({"skin": skin, "scenario": scenario, "label": label, "png": path,
		"account": client.account_state(), "spells": client.spells_state(), "target": client.target_state(),
		"minimap": client.minimap_state(), "bags": client.merchant_state().bags, "skill_text": skill_text})
	write_receipts()

func write_receipts() -> void:
	var file := FileAccess.open(output.path_join(skin + "-" + scenario + "-receipts.json"), FileAccess.WRITE)
	file.store_string(JSON.stringify(receipts, "\t"))
	file.close()

func fail(message: String) -> void:
	failed = true
	push_error("gathering live: " + message)
	quit(1)
