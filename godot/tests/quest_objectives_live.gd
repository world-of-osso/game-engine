extends "res://tests/world_quest_flow.gd"
## Real GO/loot/item-spell quest credit. Reuses the live quest fixture's pixel clicks,
## ray picking and server travel; no quest counters or reward flags are set by admin.
## QOBJ_SCENARIO points to an authored world.db scenario JSON; QF_ACCOUNT is fb_*.

var scenario: Dictionary
var current_map := 0
var shot_prefix := ""
var desired_object_entry := 0

func run_test() -> void:
	root.size = Vector2i(1920, 1080)
	var server := OS.get_environment("GODOT_TEST_SERVER")
	if server == "" or server.ends_with(":5000"):
		fail("Quest-objectives fixture requires a private server")
		return
	var parsed = JSON.parse_string(FileAccess.get_file_as_string(OS.get_environment("QOBJ_SCENARIO")))
	if not parsed is Dictionary:
		fail("QOBJ_SCENARIO must contain the authored quest scenario")
		return
	scenario = parsed
	shot_prefix = scenario.prefix
	shots = OS.get_environment("QF_SHOTS").trim_suffix("/") + "/"
	DirAccess.make_dir_recursive_absolute(shots)
	current_map = scenario.giver.map
	client = load("res://scenes/client.tscn").instantiate()
	root.add_child(client)
	var error = client.connect_account(server, OS.get_environment("QF_ACCOUNT"), PASSWORD, false)
	if error != "":
		fail("Fixture connection: " + error)
		return
	if not await wait_frames(func(): return client.account_state().screen == "CharacterSelect", "authenticated roster", 60000):
		return
	await frames(30)
	for button in client.find_children("*", "Button", true, false):
		if button.is_visible_in_tree():
			print("FIXTURE ROSTER_BUTTON ", button.name, " text=", button.text, " disabled=", button.disabled, " rect=", button.get_global_rect())
	await capture(shot_prefix + "-character-select.png")
	if not await enter_world():
		return
	if character_name() != scenario.character:
		fail("Wrong fixture character: " + character_name())
		return
	if not in_log(client.quest_state(), scenario.quest) and not await take_quest(scenario.giver.at, scenario.giver.name, scenario.quest, scenario.title):
		return
	if client.quest_state().frame_open:
		await tap(KEY_ESCAPE)
	if scenario.kind == "spell":
		if not await credit_item_spell():
			return
	else:
		if not await credit_world_object():
			return
	if not await wait_quest(func(s): return log_entry(s, scenario.quest).get("completed", false), "authored objective credited"):
		return
	current_map = scenario.giver.map
	desired_object_entry = 0
	if not await open_quest(scenario.giver.at, scenario.giver.name, scenario.title) or not await reach_reward(scenario.quest, ""):
		return
	await capture(shot_prefix + "-reward.png")
	await click_control(quest_control("QuestFrameUI", "QuestFrameCompleteQuestButton"))
	if not await wait_quest(func(s): return not in_log(s, scenario.quest) and Array(s.system_lines).has(scenario.title + " completed."), "real quest turned in"):
		return
	await capture(shot_prefix + "-turned-in.png")
	print("FIXTURE QUEST_OBJECTIVES_DONE ", scenario.quest, " ", scenario.kind, " ", scenario.skin, " ", client.quest_state().system_lines)
	client.free()
	quit(0)

func units_named(name: String) -> Array:
	var units := client.get_node_or_null("WorldUnits")
	if units == null:
		return []
	# Replicated game objects are direct client children; creatures live in WorldUnits.
	var candidates: Array = units.get_children() + client.get_children()
	var matches: Array = candidates.filter(func(unit):
		var named: bool = unit.get_meta("unit_name", unit.get_meta("game_object_name", "")) == name
		var matching_entry: bool = desired_object_entry == 0 or unit.get_meta("game_object_entry", 0) == desired_object_entry
		return named and matching_entry)
	var here := player_position()
	matches.sort_custom(func(a, b): return a.global_position.distance_squared_to(here) < b.global_position.distance_squared_to(here))
	return matches

func teleport(at: Array) -> bool:
	if not admin(["teleport", character_name(), str(current_map), str(at[0]), str(at[1]), str(at[2])]):
		return false
	var want := Vector3(at[0], at[2], -at[1])
	if not await wait_frames(func(): return client.account_state().screen == "InWorld" and player_position().distance_to(want) < 4.0, "authored travel", 60000):
		return false
	if player_dead():
		if not admin(["revive", character_name()]):
			return false
		return await wait_frames(func(): return not player_dead(), "alive after fixture travel")
	return true

## Travel places the player three yards before the pick shape along the current
## camera direction. Native ray picking and right-click still perform interaction;
## no synthetic mouse-look or direct objective messages are used.
func find_unit(name: String) -> Dictionary:
	if (await locate(name)).is_empty():
		return {}
	var unit: Node3D = units_named(name)[0]
	var world_point = pick_point(unit)
	var forward := -camera().global_basis.z
	forward.y = 0.0
	forward = forward.normalized()
	var stand: Vector3 = unit.global_position - forward * 3.0
	if not await teleport([stand.x, -stand.z, stand.y]):
		return {}
	await frames(30)
	var id: int = unit.find_child("UnitPick", true, false).get_meta("unit_server_id")
	var point := find_unit_point(unit, id)
	if point == Vector2.INF:
		print("FIXTURE PROJECTED_PICK_FAILED ", name, " player=", player_position(), " pick=", world_point, " camera=", camera().global_transform)
		await capture("fail-projected-pick.png")
		fail(name + " has no unoccluded projected native pick")
		return {}
	return {"id": id, "name": name, "point": point}

## Search the whole projected pick box, including low ground-level object meshes.
## The base creature fixture only searches pixels above the box centre.
func find_unit_point(unit: Node, id: int) -> Vector2:
	var shape: CollisionShape3D = unit.find_child("UnitPick", true, false).get_child(0)
	var half: Vector3 = shape.shape.size * 0.5
	var low := Vector2.INF
	var high := -Vector2.INF
	var crosses_camera := false
	for x in [-half.x, half.x]:
		for y in [-half.y, half.y]:
			for z in [-half.z, half.z]:
				var corner: Vector3 = shape.global_transform * Vector3(x, y, z)
				if camera().is_position_behind(corner):
					crosses_camera = true
					continue
				var projected := camera().unproject_position(corner)
				low = low.min(projected)
				high = high.max(projected)
	if crosses_camera:
		print("FIXTURE CAMERA_INSIDE_PICK_BOUNDS ", unit.name, " half=", half)
		low = Vector2.ZERO
		high = Vector2(root.size)
	low = low.max(Vector2.ZERO)
	high = high.min(Vector2(root.size))
	var stride := 16 if crosses_camera else 4
	for y in range(int(low.y), int(high.y) + 1, stride):
		for x in range(int(low.x), int(high.x) + 1, stride):
			var point := Vector2(x, y)
			if UnitPicker.pick(camera(), point) == id:
				return point
	return Vector2.INF

func open_quest(at: Array, npc: String, title: String) -> bool:
	if not await teleport(at):
		return false
	var target := await find_unit(npc)
	if target.is_empty() or not await open_giver(target):
		return false
	var index: int = Array(client.quest_state().greeting_quests).find(title)
	if index < 0:
		fail("%s offers %s, not %s" % [npc, client.quest_state().greeting_quests, title])
		return false
	await click_control(quest_control("QuestFrameUI", "QuestTitleButton%dText" % (index + 1)))
	return true

func credit_world_object() -> bool:
	var index := 0
	for object in scenario.objects:
		if log_entry(client.quest_state(), scenario.quest).get("completed", false):
			break
		desired_object_entry = object.entry
		current_map = object.map
		if not await teleport(object.at):
			return false
		var target := await find_unit(object.name)
		if target.is_empty():
			return false
		# find_unit_point verifies UnitPicker hits the real rendered object's pick shape.
		if index == 0:
			await capture(shot_prefix + "-object.png")
		var before: Array = log_entry(client.quest_state(), scenario.quest).objectives.duplicate()
		await click(target.point, MOUSE_BUTTON_RIGHT)
		if scenario.kind == "loot":
			if not await take_chest_item():
				return false
		elif not await wait_quest(func(s): return Array(log_entry(s, scenario.quest).objectives) != before, "matching object credit"):
			return false
		index += 1
	return true

func take_chest_item() -> bool:
	if not await wait_frames(func():
		var frame = client.find_child("LootFrame", true, false)
		return frame != null and frame.is_visible_in_tree(), "chest loot window"):
		return false
	if log_entry(client.quest_state(), scenario.quest).get("completed", false):
		fail("Opening a chest awarded item credit before the item entered bags")
		return false
	await capture(shot_prefix + "-loot-before-credit.png")
	await click_control(client.find_child("LootFrameElement1", true, false) as Control)
	return await wait_frames(func(): return bag_count(scenario.item) >= 1, "authored chest item stored")

func credit_item_spell() -> bool:
	await click_control(client.find_child("MainMenuBarBackpackButton", true, false) as Control)
	if not await wait_frames(func(): return backpack_shown(), "provided item in backpack"):
		return false
	var slot := bag_entry(scenario.item)
	if slot.is_empty():
		fail("Quest did not provide its authored casting item")
		return false
	var button := client.find_child("ContainerFrame%dSlot%d" % [slot.bag, slot.slot], true, false) as Control
	if button == null or not button.is_visible_in_tree():
		fail("Provided item has no visible bag button")
		return false
	await capture(shot_prefix + "-item-before-cast.png")
	await click(button.get_global_rect().get_center(), MOUSE_BUTTON_RIGHT)
	if not await wait_quest(func(s): return log_entry(s, scenario.quest).get("completed", false), "item spell objective credit", 15000):
		return false
	await tap(KEY_ESCAPE)
	return true
