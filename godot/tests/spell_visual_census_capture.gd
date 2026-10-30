extends SceneTree

# Private headless m2debug capture. The manifest is derived from the offline ledger.
const OFFSETS := [0.1, 0.5, 1.0]
const DELTAS := [0.1, 0.4, 0.5]
const WAIT_MS := 90000

var fixture: Node
var camera: Camera3D
var label: Label
var results: FileAccess
var output := ""

func _initialize() -> void:
	call_deferred("run_capture")

func run_capture() -> void:
	root.size = Vector2i(1280, 720)
	var manifest_path := OS.get_environment("SPELL_CENSUS_MANIFEST")
	output = OS.get_environment("SPELL_CENSUS_OUTPUT")
	if manifest_path.is_empty() or output.is_empty():
		fail("SPELL_CENSUS_MANIFEST and SPELL_CENSUS_OUTPUT are required")
		return
	var manifest: Variant = JSON.parse_string(FileAccess.get_file_as_string(manifest_path))
	if not manifest is Dictionary or not manifest.has("cases"):
		fail("invalid census manifest")
		return
	if DirAccess.make_dir_recursive_absolute(output) != OK:
		fail("cannot create output directory: " + output)
		return
	results = FileAccess.open(output.path_join("results.jsonl"), FileAccess.WRITE)
	if results == null:
		fail("cannot create capture results")
		return
	var client: Node = load("res://scenes/client.tscn").instantiate()
	root.add_child(client)
	var scene: Node = await wait_scene(client)
	if scene == null:
		fail("m2debug scene did not open")
		return
	var reference: Node3D = scene.get_node("M2DebugReferenceModel")
	reference.visible = false
	camera = scene.get_node("Camera")
	scene.set_process(false)
	camera.look_at_from_position(Vector3(0.0, 1.8, 4.0), Vector3(0.0, 1.0, 0.0))
	fixture = ClassDB.instantiate("SpellCensusFixture") as Node
	if fixture == null:
		fail("SpellCensusFixture is unavailable")
		return
	scene.add_child(fixture)
	make_label()
	var current_caster := ""
	for case in manifest.cases:
		var caster_key := "%s:%s:%s" % [case.race, case.class_id, case.sex]
		if caster_key != current_caster:
			var caster_error: String = fixture.begin_caster(case.race, case.class_id, case.sex)
			if caster_error.is_empty():
				caster_error = await wait_caster()
			if not caster_error.is_empty():
				write_result(case, {"error": "caster: " + caster_error, "frames": []})
				current_caster = ""
				continue
			current_caster = caster_key
		await capture_case(case)
	results.flush()
	var done := FileAccess.open(output.path_join("done.json"), FileAccess.WRITE)
	if done == null:
		fail("cannot write completion marker")
		return
	done.store_string(JSON.stringify({"class_id": manifest.class_id, "cases": manifest.cases.size(), "ledger_sha256": manifest.ledger_sha256}))
	done.flush()
	print("CENSUS_BATCH_DONE class=%s cases=%s" % [manifest.class_id, manifest.cases.size()])
	quit(0)

func wait_scene(client: Node) -> Node:
	var deadline := Time.get_ticks_msec() + WAIT_MS
	while Time.get_ticks_msec() < deadline:
		await process_frame
		var scene: Node = client.get_node_or_null("M2Debug")
		if scene != null:
			return scene
	return null

func wait_caster() -> String:
	var deadline := Time.get_ticks_msec() + WAIT_MS
	while Time.get_ticks_msec() < deadline:
		await process_frame
		var state: Dictionary = fixture.poll_caster()
		if not String(state.error).is_empty():
			return String(state.error)
		if state.ready:
			return ""
	return "caster load timed out"

func wait_kit() -> String:
	var deadline := Time.get_ticks_msec() + WAIT_MS
	while Time.get_ticks_msec() < deadline:
		await process_frame
		var state: Dictionary = fixture.poll_kit()
		if not String(state.error).is_empty():
			return String(state.error)
		if state.ready:
			return ""
	return "kit load timed out"

func capture_case(case: Dictionary) -> void:
	var error: String = fixture.prepare_kit(case.visual_id, case.kit_id, case.spell_id)
	if error.is_empty():
		error = await wait_kit()
	var errors: Array[String] = []
	if not error.is_empty():
		errors.append("kit preparation: " + error)
	# Clear a preceding action before capturing this kit's baseline.
	for i in range(50):
		error = fixture.tick(0.1, camera.global_transform)
		if not error.is_empty() or fixture.capture_state().action_id == -1:
			break
	if not error.is_empty() or fixture.capture_state().action_id != -1:
		write_result(case, {"error": "baseline: " + error, "frames": []})
		return
	label.visible = false
	await process_frame
	await RenderingServer.frame_post_draw
	var baseline: Image = root.get_texture().get_image()
	var skeleton: Skeleton3D = fixture.get_node_or_null("NpcModel/Skeleton3D")
	if skeleton == null or baseline == null:
		write_result(case, {"error": "caster skeleton or baseline image missing", "frames": []})
		return
	var initial_poses := bone_poses(skeleton)
	var started: Dictionary = {"error": "", "requested_animation": -1, "played_animation": -1}
	if errors.is_empty():
		started = fixture.start_kit()
		if not String(started.error).is_empty():
			errors.append("kit start: " + String(started.error))
			started["requested_animation"] = -1
			started["played_animation"] = -1
	var start_clock: float = fixture.capture_state().effect_clock
	var records: Array = []
	var pose_changed := false
	for index in range(OFFSETS.size()):
		error = fixture.tick(DELTAS[index], camera.global_transform)
		if not error.is_empty():
			errors.append(error)
		await process_frame
		await RenderingServer.frame_post_draw
		var frame: Image = root.get_texture().get_image()
		var state: Dictionary = fixture.capture_state()
		var unchanged := frame.get_data() == baseline.get_data()
		pose_changed = pose_changed or bone_poses(skeleton) != initial_poses
		var has_bounds := effect_has_bounds()
		var no_effect_bounds: bool = not has_bounds and state.particles_drawn == 0
		var outfit: Dictionary = fixture.outfit_state()
		var weapon_note := "" if outfit.weapon_model_loaded else " [NO WEAPON]"
		if not outfit.skipped.is_empty():
			weapon_note = " [GEAR CONFLICT]"
		elif case.context.weapon_subclass != null and outfit.weapon_subclass >= 0 and int(case.context.weapon_subclass) != outfit.weapon_subclass:
			weapon_note = " [GEAR MISMATCH]"
		label.text = "%s%s | %s %d | kit %d | %.1fs\nvisual %d | race %d sex %d" % [case.class_name, weapon_note, case.spell_name, int(case.spell_id), int(case.kit_id), OFFSETS[index], int(case.visual_id), int(case.race), int(case.sex)]
		label.visible = true
		await process_frame
		await RenderingServer.frame_post_draw
		var filename := "%05d-kit%d-%.1f.png" % [case.index, case.kit_id, OFFSETS[index]]
		var save_error := root.get_texture().get_image().save_png(output.path_join(filename))
		label.visible = false
		if save_error != OK:
			errors.append("cannot save frame " + filename)
			break
		var actual_offset: float = state.effect_clock - start_clock
		records.append({"offset": OFFSETS[index], "actual_offset": actual_offset,
			"clock_mismatch": abs(actual_offset - OFFSETS[index]) > 0.002, "file": filename,
			"identical_to_baseline": unchanged, "no_effect_bounds": no_effect_bounds,
			"empty_output": unchanged or no_effect_bounds,
			"active_models": Array(state.active_models), "particles_drawn": state.particles_drawn,
			"action_id": state.action_id})
	var requested: int = started.requested_animation
	var applied: int = started.played_animation
	var error_text := "; ".join(errors)
	write_result(case, {"error": error_text, "missing_attachment": "attachment" in error_text.to_lower(), "requested_animation": requested,
		"played_animation": applied, "animation_not_applied": requested >= 0 and requested != applied,
		"pose_unchanged": requested >= 0 and not pose_changed, "frames": records})

func effect_has_bounds() -> bool:
	for effect in fixture.find_children("SpellEffect*", "Node3D", true, false):
		for child in effect.find_children("*", "MeshInstance3D", true, false):
			var mesh: MeshInstance3D = child
			if mesh.is_visible_in_tree() and mesh.get_aabb().size.length_squared() > 0.0:
				return true
	return false

func bone_poses(skeleton: Skeleton3D) -> Array[Transform3D]:
	var poses: Array[Transform3D] = []
	for bone in range(skeleton.get_bone_count()):
		poses.append(skeleton.get_bone_pose(bone))
	return poses

func make_label() -> void:
	var overlay := CanvasLayer.new()
	root.add_child(overlay)
	var background := ColorRect.new()
	background.color = Color(0.0, 0.0, 0.0, 0.85)
	background.size = Vector2(1280, 110)
	overlay.add_child(background)
	label = Label.new()
	label.position = Vector2(12, 2)
	label.add_theme_font_size_override("font_size", 44)
	background.add_child(label)
	label.visible = false

func write_result(case: Dictionary, result: Dictionary) -> void:
	result["case"] = case
	if fixture != null and not String(result.get("error", "")).begins_with("caster:"):
		var outfit: Dictionary = fixture.outfit_state()
		outfit["item_ids"] = Array(outfit.item_ids)
		result["outfit"] = outfit
		result["weapon_checks_uncheckable"] = not outfit.weapon_model_loaded or not outfit.skipped.is_empty()
		if case.context.weapon_subclass != null and outfit.weapon_subclass >= 0:
			result["selection_weapon_matches_outfit"] = int(case.context.weapon_subclass) == outfit.weapon_subclass
		else:
			result["selection_weapon_matches_outfit"] = null
	results.store_line(JSON.stringify(result))
	results.flush()

func fail(message: String) -> void:
	push_error(message)
	quit(1)
