extends SceneTree

# Real classic Avenging Wrath wings: retained gold RGB and local alpha-weight tracks.
# Fixed pose/clocks/camera across old/new binaries; no texture or track substitution.
const SIZE := Vector2i(1920, 1080)
const FDID := 165644
var model: Node3D
var player: Node
var materials: Node
var output: String
var phase: String

func _initialize() -> void:
	call_deferred("run_fixture")

func fail_fixture(message: String) -> void:
	push_error("M2_MAT_FAIL " + message)
	quit(1)

func capture(label: String) -> bool:
	# Fixed clocks let temporal rendering settle without advancing the asset.
	for frame in 32:
		await process_frame
	await RenderingServer.frame_post_draw
	var image := root.get_texture().get_image()
	if image.get_size() != SIZE or image.save_png(output.path_join(label + ".png")) != OK:
		fail_fixture("fullHD capture failed: " + label)
		return false
	return true

func parameters() -> Array:
	var values := []
	for batch in model.find_children("Batch*", "MeshInstance3D", false, false):
		var material: ShaderMaterial = batch.get_active_material(0)
		values.append({"name": batch.name, "rgb": material.get_shader_parameter("mesh_color"),
			"opacity": material.get_shader_parameter("transparency"),
			"weights": material.get_shader_parameter("texture_weights")})
	return values

func run_fixture() -> void:
	output = OS.get_environment("M2_MAT_OUTPUT")
	phase = OS.get_environment("M2_MAT_PHASE")
	if output.is_empty() or phase.is_empty():
		fail_fixture("M2_MAT_OUTPUT and M2_MAT_PHASE required")
		return
	DirAccess.make_dir_recursive_absolute(output)
	root.size = SIZE
	var client: Node = load("res://scenes/client.tscn").instantiate()
	root.add_child(client)
	# Startup includes asynchronous asset/catalog loading; frame count is not a deadline.
	for attempt in 600:
		if client.get_node_or_null("M2Debug") != null:
			break
		await create_timer(0.1).timeout
	var stage := client.get_node_or_null("M2Debug")
	if stage == null:
		fail_fixture("m2debug stage did not start")
		return
	stage.set_process(false)
	var old := stage.get_node("M2DebugReferenceModel")
	stage.remove_child(old)
	old.queue_free()
	var loaded: Dictionary = ClassDB.instantiate("WowAssetLoader").load_m2("res://../data/models/%d.m2" % FDID)
	if loaded.has("error") or not loaded.missing_texture_fdids.is_empty():
		fail_fixture("authentic wings could not load: " + str(loaded))
		return
	model = loaded.node
	stage.add_child(model)
	player = model.get_node("M2Animation")
	player.set_process(false)
	materials = model.get_node("M2MaterialAnimation")
	materials.set_process(false)
	var clock := root.get_node("M2MaterialClock")
	clock.set_process(false)
	var camera := stage.get_node("Camera") as Camera3D
	var bounds: AABB = model.get_meta("m2_bounds")
	var focus := bounds.get_center()
	var radius := maxf(bounds.size.length() * 0.5, 1.0)
	camera.look_at_from_position(focus + Vector3(1.0, 0.12, 0.0).normalized() * radius * 2.8, focus)
	var evidence := {}
	for skin in ["modern", "forever"]:
		var skin_error: String = client.preview_world_skin(skin == "forever")
		if not skin_error.is_empty():
			fail_fixture(skin_error)
			return
		# Finish any previous transition before returning to Stand at a known clock.
		player.play_sequence(0, true)
		player.advance_time_ms(500.0)
		player.play_sequence(1, true)
		player.advance_time_ms(75.0)
		materials.sample_materials()
		var halfway := parameters()
		var weights: Vector3 = halfway[0].weights
		# Skin batch0 resolves transparency lookup2: authored0→18022 over367ms.
		var incoming := 18022.0 / 32767.0
		var outgoing := float(int(18022.0 * 208.0 / 367.0)) / 32767.0
		var expected := (incoming + outgoing) * 0.5 if phase == "after" else incoming
		if absf(weights.x - expected) > 0.00001:
			fail_fixture("authored half transition weight: expected %f got %f" % [expected, weights.x])
			return
		evidence[skin + "_half"] = halfway
		if not await capture("%s-%s-half-165644" % [phase, skin]): return
		# Real Hold tint control after the transition has ended.
		player.advance_time_ms(150.0)
		materials.sample_materials()
		var hold := parameters()
		evidence[skin + "_hold"] = hold
		var gold: Vector3 = hold[0].rgb
		if gold.distance_to(Vector3(0.8666667, 0.7372549, 0.3882353)) > 0.01:
			fail_fixture("classic Hold gold tint changed: " + str(gold))
			return
		if not await capture("%s-%s-hold-165644" % [phase, skin]): return
		# Mid-blend retransition at zero elapsed must preserve eligible weights.
		player.play_sequence(2, true)
		player.advance_time_ms(60.0)
		materials.sample_materials()
		var before := parameters()
		player.play_sequence(0, true)
		materials.sample_materials()
		var after := parameters()
		evidence[skin + "_interrupted_before"] = before
		evidence[skin + "_interrupted_after"] = after
		if phase == "after":
			for index in before.size():
				var before_weights: Vector3 = before[index].weights
				if not before_weights.is_equal_approx(after[index].weights):
					fail_fixture("midblend material pop: %s -> %s" % [before, after])
					return
	var file := FileAccess.open(output.path_join(phase + "-parameters.json"), FileAccess.WRITE)
	file.store_string(JSON.stringify(evidence, "\t"))
	file.close()
	print("M2_MAT_NATIVE ", phase, " ", evidence)
	print("PASS M2-MAT both skins1920x1080 authentic165644 fixed-clock transition/tint control")
	quit(0)
