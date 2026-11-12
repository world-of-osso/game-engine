extends SceneTree

func _initialize() -> void:
	call_deferred("run")

func make_batch(mode: int) -> MeshInstance3D:
	var parent := Node3D.new()
	var batch := MeshInstance3D.new()
	batch.mesh = BoxMesh.new()
	batch.cast_shadow = mode
	var shader := Shader.new()
	shader.code = "shader_type spatial; void fragment() { ALBEDO = vec3(1.0); }"
	var material := ShaderMaterial.new()
	material.shader = shader
	batch.material_override = material
	parent.add_child(batch)
	root.add_child(parent)
	return batch

func run() -> void:
	var on := make_batch(GeometryInstance3D.SHADOW_CASTING_SETTING_ON)
	var off := make_batch(GeometryInstance3D.SHADOW_CASTING_SETTING_OFF)
	var double := make_batch(GeometryInstance3D.SHADOW_CASTING_SETTING_DOUBLE_SIDED)
	var initially_hidden := make_batch(GeometryInstance3D.SHADOW_CASTING_SETTING_ON)
	initially_hidden.visible = false
	await process_frame
	await RenderingServer.frame_post_draw
	var probes := []
	for batch in [on, off, double, initially_hidden]:
		var probe = ClassDB.instantiate("WowSceneryFadeProbe")
		probe.track(batch.get_parent())
		probes.append(probe)
		probe.set_camera_admitted(false)
	if not (on.visible and on.cast_shadow == GeometryInstance3D.SHADOW_CASTING_SETTING_SHADOWS_ONLY):
		push_error("Unexpected scenery camera/shadow state")
		quit(1)
		return
	if not (not off.visible and off.cast_shadow == GeometryInstance3D.SHADOW_CASTING_SETTING_OFF):
		push_error("Unexpected scenery camera/shadow state")
		quit(1)
		return
	if not (double.visible and double.cast_shadow == GeometryInstance3D.SHADOW_CASTING_SETTING_DOUBLE_SIDED):
		push_error("Unexpected scenery camera/shadow state")
		quit(1)
		return
	probes[0].set_opacity(0.5)
	if not (not on.visible and on.cast_shadow == GeometryInstance3D.SHADOW_CASTING_SETTING_OFF):
		push_error("Unexpected scenery camera/shadow state")
		quit(1)
		return
	probes[0].set_opacity(1.0)
	if not (on.visible and on.cast_shadow == GeometryInstance3D.SHADOW_CASTING_SETTING_SHADOWS_ONLY):
		push_error("Unexpected scenery camera/shadow state")
		quit(1)
		return
	for probe in probes:
		probe.set_camera_admitted(true)
	if not (on.visible and on.cast_shadow == GeometryInstance3D.SHADOW_CASTING_SETTING_ON):
		push_error("Unexpected scenery camera/shadow state")
		quit(1)
		return
	if not (off.visible and off.cast_shadow == GeometryInstance3D.SHADOW_CASTING_SETTING_OFF):
		push_error("Unexpected scenery camera/shadow state")
		quit(1)
		return
	if not (double.visible and double.cast_shadow == GeometryInstance3D.SHADOW_CASTING_SETTING_DOUBLE_SIDED):
		push_error("Unexpected scenery camera/shadow state")
		quit(1)
		return
	if initially_hidden.visible:
		push_error("Camera readmission revealed an initially hidden batch")
		quit(1)
		return
	# Shadow radius, independent of the fade band: a placement outside it draws without
	# shadows, and a fade ending outside it does not restore them.
	probes[0].set_shadow_admitted(false)
	if not (on.visible and on.cast_shadow == GeometryInstance3D.SHADOW_CASTING_SETTING_OFF):
		return fail_state("shadow radius drops an admitted batch's shadows", on)
	probes[0].set_opacity(0.5)
	probes[0].set_opacity(1.0)
	if not (on.visible and on.cast_shadow == GeometryInstance3D.SHADOW_CASTING_SETTING_OFF):
		return fail_state("a fade ending outside the shadow radius keeps shadows off", on)
	probes[0].set_camera_admitted(false)
	if on.visible:
		return fail_state("a camera-rejected batch outside the shadow radius is hidden", on)
	probes[0].set_shadow_admitted(true)
	if not (on.visible and on.cast_shadow == GeometryInstance3D.SHADOW_CASTING_SETTING_SHADOWS_ONLY):
		return fail_state("re-entering the shadow radius restores a camera-rejected caster", on)
	probes[0].set_camera_admitted(true)
	if not (on.visible and on.cast_shadow == GeometryInstance3D.SHADOW_CASTING_SETTING_ON):
		return fail_state("camera readmission restores the authored shadows", on)
	probes[2].set_shadow_admitted(false)
	if not (double.visible and double.cast_shadow == GeometryInstance3D.SHADOW_CASTING_SETTING_OFF):
		return fail_state("shadow radius drops double-sided shadows", double)
	probes[2].set_shadow_admitted(true)
	if double.cast_shadow != GeometryInstance3D.SHADOW_CASTING_SETTING_DOUBLE_SIDED:
		return fail_state("double-sided shadows return inside the radius", double)
	print("PASS outdoor scenery camera and shadow admission")
	quit(0)

func fail_state(expected: String, batch: MeshInstance3D) -> void:
	push_error("Expected: %s; visible=%s cast_shadow=%d" % [expected, batch.visible, batch.cast_shadow])
	quit(1)
