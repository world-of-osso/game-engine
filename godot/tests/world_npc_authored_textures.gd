# Real CreatureDisplayInfoExtra profiles, through UDP -> worker -> native batch materials.
# Oracle is independently decoded from canonical DB2 caches/BLPs by the fixture.
extends "res://tests/world_npc_visual_flow.gd"

func run_test() -> void:
	root.size = Vector2i(1280, 720)
	if not prepare_assets():
		fail("Cannot prepare bootstrap assets")
		return
	var client = load("res://scenes/client.tscn").instantiate()
	root.add_child(client)
	var error: String = client.connect_account(OS.get_environment("GODOT_TEST_SERVER"), "fixture", "fixture", false)
	if error != "" or not await wait_screen(client, "CharacterSelect", STARTUP_WAIT_MS):
		fail("Authored fixture login: " + error)
		return
	var ui = client.get_node("CharacterSelectUI")
	await click_control(ui.find_child("CharCard_0", true, false))
	await click_control(ui.find_child("EnterWorld", true, false))
	if not await wait_visual(client, 1.5, WORLD_LOAD_WAIT_MS):
		return
	var oracle := ProjectSettings.globalize_path("res://../oracle")
	var captures := ProjectSettings.globalize_path("res://../captures")
	DirAccess.make_dir_recursive_absolute(captures)
	var cases := read_cases(oracle)
	var failures := 0
	for display in [825, 1322, 1285, 90209, 110154, 150, 35297]:
		var old_visual: Node = client.get_node("WorldUnits/" + NPC + "/NpcVisualRoot")
		var old_id := old_visual.get_instance_id()
		print("FIXTURE AUTHORED_REQUEST ", display)
		var model := await wait_real_model(client, old_id)
		if model == null:
			print("AUTHORED_FAIL display=", display, " missing native visual")
			failures += 1
			continue
		failures += check_case(model, display, cases.get(display, []), oracle, captures)
	client.free()
	if failures != 0:
		fail("Authored NPC texture oracle: %d failures; captures %s" % [failures, captures])
		return
	print("FIXTURE AUTHORED_COMPLETE")
	quit(0)

func read_cases(oracle: String) -> Dictionary:
	var cases := {}
	for line in FileAccess.get_file_as_string(oracle + "/bindings.tsv").split("\n", false):
		var columns := line.split("\t")
		var display := int(columns[0])
		if not cases.has(display):
			cases[display] = []
		cases[display].append({"kind": int(columns[1]), "batch": columns[2], "slot": int(columns[3]), "width": int(columns[4]), "height": int(columns[5])})
	return cases

func wait_real_model(client: Node, old_id: int) -> Node3D:
	var deadline := Time.get_ticks_msec() + WORLD_LOAD_WAIT_MS
	while Time.get_ticks_msec() < deadline:
		await process_frame
		var visual := client.get_node_or_null("WorldUnits/" + NPC + "/NpcVisualRoot")
		if visual != null and visual.get_instance_id() != old_id:
			return visual.get_node("NpcModel") as Node3D
	return null

func check_case(model: Node3D, display: int, bindings: Array, oracle: String, captures: String) -> int:
	var failures := 0
	var counts := {}
	if bindings.is_empty():
		print("AUTHORED_FAIL display=", display, " empty oracle")
		return 1
	for binding in bindings:
		var kind: int = binding.kind
		var batch := model.find_child(binding.batch, true, false) as MeshInstance3D
		var material := batch.get_active_material(0) as ShaderMaterial if batch != null else null
		var uniform: String = ["base_texture", "second_texture", "third_texture", "fourth_texture"][binding.slot]
		var texture := material.get_shader_parameter(uniform) as Texture2D if material != null else null
		var expected := FileAccess.get_file_as_bytes("%s/%d-%d.rgba" % [oracle, display, kind])
		var image := texture.get_image() if texture != null else null
		var correct: bool = image != null and image.get_width() == binding.width and image.get_height() == binding.height
		if correct:
			correct = image.get_data().slice(0, expected.size()) == expected
		if not correct:
			print("AUTHORED_FAIL display=%d type=%d batch=%s slot=%d expected=%dx%d actual=%s" % [display, kind, binding.batch, binding.slot, binding.width, binding.height, image.get_size() if image != null else Vector2i.ZERO])
			failures += 1
		if image != null:
			var path := "%s/%d-type%d-%s-slot%d.png" % [captures, display, kind, binding.batch, binding.slot]
			var save_error := image.save_png(path)
			if save_error != OK:
				print("AUTHORED_FAIL capture=", path, " error=", save_error)
				failures += 1
		if not counts.has(kind):
			counts[kind] = {"slots": 0, "visible": 0, "failures": 0}
		counts[kind].slots += 1
		counts[kind].visible += int(batch != null and batch.is_visible_in_tree())
		counts[kind].failures += int(not correct)
	for kind in counts:
		print("AUTHORED_RESULT display=%d type=%d slots=%d visible=%d failures=%d" % [display, kind, counts[kind].slots, counts[kind].visible, counts[kind].failures])
	return failures
