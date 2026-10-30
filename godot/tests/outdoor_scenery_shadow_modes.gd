extends SceneTree

func _initialize() -> void:
	call_deferred("run")

func make_batch(mode: int) -> MeshInstance3D:
	var parent := Node3D.new()
	var batch := MeshInstance3D.new()
	batch.mesh = BoxMesh.new()
	batch.cast_shadow = mode
	var shader := Shader.new()
	shader.code = "shader_type spatial; uniform float scenery_opacity = 1.0; void fragment() { ALBEDO = vec3(1.0); }"
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
	var probes := []
	for batch in [on, off, double]:
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
	print("PASS outdoor scenery camera and shadow admission")
	quit(0)
