extends SceneTree

# Real m2debug startup; the fixture never contacts a server or the user's display.
func _initialize() -> void:
	call_deferred("run_test")

func run_test() -> void:
	root.size = Vector2i(1280, 720)
	var client: Node = load("res://scenes/client.tscn").instantiate()
	root.add_child(client)
	var scene: Node = null
	var deadline := Time.get_ticks_msec() + 60000
	while scene == null and Time.get_ticks_msec() < deadline:
		await process_frame
		scene = client.get_node_or_null("M2Debug")
	if scene == null:
		fail("m2debug scene did not open")
		return
	var fixture := ClassDB.instantiate("SpellCensusFixture") as Node3D
	if fixture == null:
		fail("SpellCensusFixture is unavailable")
		return
	scene.add_child(fixture)
	var error: String = fixture.begin_caster(1, 1, 0)
	if not error.is_empty():
		fail("legal Human Warrior caster: " + error)
		return
	deadline = Time.get_ticks_msec() + 60000
	var state: Dictionary = {}
	while Time.get_ticks_msec() < deadline:
		await process_frame
		state = fixture.poll_caster()
		if not String(state.error).is_empty():
			fail("caster load: " + String(state.error))
			return
		if state.ready:
			break
	if not state.get("ready", false):
		fail("caster did not load")
		return
	if fixture.get_node_or_null("NpcModel/M2Animation") == null:
		fail("caster animation player missing")
		return
	var outfit: Dictionary = fixture.outfit_state()
	if not outfit.found or not outfit.item_ids.has(25) or not outfit.item_ids.has(2362):
		fail("Human Warrior outfit did not retain its authored sword and shield: " + str(outfit))
		return
	var sword: Node = fixture.find_child("EquipmentMainHand", true, false)
	var shield: Node = fixture.find_child("EquipmentOffHand", true, false)
	if sword == null or shield == null or sword.get_parent().name != "Attachment1" or shield.get_parent().name != "Attachment0":
		fail("authored sword or shield model did not attach to its drawn hand")
		return
	error = fixture.prepare_kit(190513, 278884, 1464)
	if not error.is_empty():
		fail("prepare Slam kit: " + error)
		return
	if not (await wait_kit(fixture)):
		return
	var played: Dictionary = fixture.start_kit()
	if not String(played.error).is_empty() or played.requested_animation != 812 or played.played_animation != 812:
		fail("Slam caster animation: " + str(played))
		return
	var skeleton: Skeleton3D = fixture.get_node("NpcModel/Skeleton3D")
	var initial_poses := bone_poses(skeleton)
	await process_frame
	await process_frame
	if bone_poses(skeleton) != initial_poses:
		fail("caster animation advanced without a census tick")
		return
	var camera: Camera3D = scene.get_node("Camera")
	error = fixture.tick(0.1, camera.global_transform)
	if not error.is_empty() or bone_poses(skeleton) == initial_poses:
		fail("caster animation did not advance at 0.1s: " + error)
		return
	error = fixture.prepare_kit(190513, 278885, 1464)
	if not error.is_empty():
		fail("prepare Slam impact: " + error)
		return
	if not (await wait_kit(fixture)):
		return
	played = fixture.start_kit()
	if not String(played.error).is_empty():
		fail("start Slam impact: " + str(played))
		return
	var capture: Dictionary = {}
	for i in range(20):
		await process_frame
		error = fixture.tick(0.05, camera.global_transform)
		if not error.is_empty():
			fail("Slam impact attachment: " + error)
			return
		capture = fixture.capture_state()
		if 1283017 in capture.active_models:
			break
	if not capture.get("active_models", []).has(1283017):
		fail("Slam impact model did not appear: " + str(capture))
		return
	error = fixture.begin_caster(1, 10, 0)
	if not error.is_empty():
		fail("legal Human Monk caster: " + error)
		return
	deadline = Time.get_ticks_msec() + 60000
	while Time.get_ticks_msec() < deadline:
		await process_frame
		state = fixture.poll_caster()
		if not String(state.error).is_empty():
			fail("monk caster load: " + String(state.error))
			return
		if state.ready:
			break
	outfit = fixture.outfit_state()
	if not state.get("ready", false) or outfit.found or outfit.weapon_model_loaded:
		fail("missing Human Monk outfit must stay undressed and reported: " + str(outfit))
		return
	print("PASS: offline Slam animation, impact model, authored gear and missing outfit")
	quit(0)

func bone_poses(skeleton: Skeleton3D) -> Array[Transform3D]:
	var poses: Array[Transform3D] = []
	for bone in range(skeleton.get_bone_count()):
		poses.append(skeleton.get_bone_pose(bone))
	return poses

func wait_kit(fixture: Node) -> bool:
	var deadline := Time.get_ticks_msec() + 60000
	while Time.get_ticks_msec() < deadline:
		await process_frame
		var state: Dictionary = fixture.poll_kit()
		if not String(state.error).is_empty():
			fail("kit load: " + String(state.error))
			return false
		if state.ready:
			return true
	fail("kit did not load")
	return false

func fail(message: String) -> void:
	push_error(message)
	quit(1)
