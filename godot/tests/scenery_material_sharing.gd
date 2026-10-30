extends SceneTree

# Exercise actual M2 scenery material interning and independent fade transitions.
func _initialize() -> void:
	call_deferred("run")

func fail(message: String) -> void:
	push_error(message)
	quit(1)

func make_model(shader: Shader, fade_shader: Shader, tint: Vector4, animated: bool = false) -> Node3D:
	var model := Node3D.new()
	var batch := MeshInstance3D.new()
	batch.name = "Batch0"
	batch.mesh = BoxMesh.new()
	var material := ShaderMaterial.new()
	material.shader = shader
	material.set_meta("scenery_fade_shader", fade_shader)
	material.set_shader_parameter("tint", tint)
	batch.material_override = material
	model.add_child(batch)
	if animated:
		var animation := Node.new()
		animation.name = "M2MaterialAnimation"
		model.add_child(animation)
	root.add_child(model)
	return model

func run() -> void:
	if not ClassDB.class_exists("WowSceneryFadeProbe"):
		fail("Scenery fade probe is unavailable")
		return
	var shader := Shader.new()
	shader.code = "shader_type spatial; uniform vec4 tint; uniform float scenery_opacity = 1.0; void fragment() { ALBEDO = tint.rgb; ALPHA = scenery_opacity; }"
	var fade_shader := Shader.new()
	fade_shader.code = shader.code
	var red := Vector4(1.0, 0.0, 0.0, 1.0)
	var blue := Vector4(0.0, 0.0, 1.0, 1.0)
	var a := make_model(shader, fade_shader, red)
	var b := make_model(shader, fade_shader, red)
	var other := make_model(shader, fade_shader, blue)
	var animated := make_model(shader, fade_shader, red, true)
	var other_shader := Shader.new()
	other_shader.code = shader.code
	var different_shader := make_model(other_shader, fade_shader, red)
	await process_frame
	await RenderingServer.frame_post_draw
	var probes := []
	for model in [a, b, other, animated, different_shader]:
		var probe = ClassDB.instantiate("WowSceneryFadeProbe")
		probe.track(model)
		probes.append(probe)
	var am := a.get_node("Batch0") as MeshInstance3D
	var bm := b.get_node("Batch0") as MeshInstance3D
	var cm := other.get_node("Batch0") as MeshInstance3D
	var dm := animated.get_node("Batch0") as MeshInstance3D
	var em := different_shader.get_node("Batch0") as MeshInstance3D
	var shared := am.material_override
	if shared != bm.material_override or shared == cm.material_override or shared == dm.material_override or shared == em.material_override:
		fail("Material equality or animation isolation is wrong")
		return
	var original_opacity = (shared as ShaderMaterial).get_shader_parameter("scenery_opacity")
	if original_opacity != null and original_opacity != 1.0:
		fail("Peer opacity was not at the shader default")
		return
	probes[0].set_opacity(0.5)
	var faded_material := am.material_override as ShaderMaterial
	if faded_material == shared or bm.material_override != shared or faded_material.shader != fade_shader or (shared as ShaderMaterial).shader != shader:
		fail("Fading one placement changed its shared peer or shader")
		return
	if absf(faded_material.get_shader_parameter("scenery_opacity") - 0.5) > 0.001:
		fail("Fading placement did not receive opacity")
		return
	var peer_opacity = (bm.material_override as ShaderMaterial).get_shader_parameter("scenery_opacity")
	# Resource duplication can cache the shader's default on the original material.
	if peer_opacity != null and peer_opacity != 1.0:
		fail("Fading placement changed its peer's effective opacity: %s -> %s" % [original_opacity, peer_opacity])
		return
	if am.cast_shadow != GeometryInstance3D.SHADOW_CASTING_SETTING_OFF:
		fail("Fading placement still casts shadows")
		return
	var late := make_model(shader, fade_shader, red)
	var late_mesh := late.get_node("Batch0") as MeshInstance3D
	# Godot fills unset shader defaults on the original when duplicating a resource.
	var duplicate := late_mesh.material_override.duplicate()
	if duplicate == null:
		fail("Could not duplicate material")
		return
	var late_probe = ClassDB.instantiate("WowSceneryFadeProbe")
	late_probe.track(late)
	if late_mesh.material_override != shared:
		fail("Explicit shader defaults did not share with equivalent unset defaults")
		return
	probes[0].set_opacity(1.0)
	if am.material_override != shared or am.cast_shadow != GeometryInstance3D.SHADOW_CASTING_SETTING_ON:
		fail("Full opacity did not restore the shared material and shadows")
		return
	print("PASS: scenery material sharing, unequal uniforms, animation isolation, independent fade and restoration")
	quit(0)
