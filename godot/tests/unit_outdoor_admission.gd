extends SceneTree

# Real WorldUnits.cull_outdoor with synthetic native units. From an indoor camera, a
# remote unit outside the interior and every exterior window is drawn only into
# shadows; a double-sided caster stays drawn, a non-caster is untouched, the local
# player is always drawn, and inactive attachments keep casting nothing.
const ON := GeometryInstance3D.SHADOW_CASTING_SETTING_ON
const OFF := GeometryInstance3D.SHADOW_CASTING_SETTING_OFF
const DOUBLE := GeometryInstance3D.SHADOW_CASTING_SETTING_DOUBLE_SIDED
const ONLY := GeometryInstance3D.SHADOW_CASTING_SETTING_SHADOWS_ONLY
const INSIDE := 0
const WINDOWED := 1
const OUTSIDE := 2
const LOCAL := 3
const INTERIOR_MIN := Vector3(-3, -3, -3)
const INTERIOR_MAX := Vector3(3, 3, 3)
# Half space x >= 20: a window onto the street the windowed unit stands in.
const WINDOW := Vector4(1, 0, 0, -20)

var fixture: Node3D
var failures: Array[String] = []

func _initialize() -> void:
	call_deferred("run_test")

func run_test() -> void:
	fixture = ClassDB.instantiate("UnitAdmissionFixture")
	root.add_child(fixture)
	fixture.initialize(PackedVector3Array([Vector3(0, 0, 0), Vector3(30, 0, 0), Vector3(-30, 0, 0), Vector3(-60, 0, 0)]))
	fixture.set_local_player(LOCAL)

	fixture.cull(true, INTERIOR_MIN, INTERIOR_MAX, PackedVector4Array())
	for unit in 4:
		expect(unit, [ON, DOUBLE, OFF], "outdoor camera")

	fixture.cull(false, INTERIOR_MIN, INTERIOR_MAX, PackedVector4Array([WINDOW]))
	expect(INSIDE, [ON, DOUBLE, OFF], "unit overlapping the interior")
	expect(WINDOWED, [ON, DOUBLE, OFF], "unit through an exterior window")
	expect(OUTSIDE, [ONLY, DOUBLE, OFF], "unit outside interior and windows")
	expect(LOCAL, [ON, DOUBLE, OFF], "local player outside interior and windows")
	for batch in ["On", "DoubleSided", "Off"]:
		check(batch_of(OUTSIDE, batch).visible, "rejected %s batch stays visible" % batch)

	# An enclosed interior admits only what overlaps it.
	fixture.cull(false, INTERIOR_MIN, INTERIOR_MAX, PackedVector4Array())
	expect(WINDOWED, [ONLY, DOUBLE, OFF], "unit once its window closes")

	# A new visual on a rejected unit is rejected with it.
	fixture.replace_visual(OUTSIDE)
	expect(OUTSIDE, [ON, DOUBLE, OFF], "fresh visual before culling")
	fixture.cull(false, INTERIOR_MIN, INTERIOR_MAX, PackedVector4Array())
	expect(OUTSIDE, [ONLY, DOUBLE, OFF], "fresh visual on a rejected unit")

	# Attachment participation and camera admission compose on one authored mode.
	var model := fixture.get_node("WorldUnits/Unit%d/NpcVisualRoot/NpcModel" % OUTSIDE) as Node3D
	var attachment := Node3D.new()
	attachment.set_meta("m2_attachment_active", false)
	model.add_child(attachment)
	var item := MeshInstance3D.new()
	item.mesh = BoxMesh.new()
	attachment.add_child(item)
	fixture.refresh_shadows(OUTSIDE)
	check(item.cast_shadow == OFF, "inactive attachment on a rejected unit casts nothing")
	attachment.set_meta("m2_attachment_active", true)
	fixture.refresh_shadows(OUTSIDE)
	check(item.cast_shadow == ONLY, "active attachment on a rejected unit draws only shadows")
	fixture.cull(true, INTERIOR_MIN, INTERIOR_MAX, PackedVector4Array())
	check(item.cast_shadow == ON, "admitted unit restores its attachment caster")
	for unit in 4:
		expect(unit, [ON, DOUBLE, OFF], "outdoor camera again")

	if failures.is_empty():
		print("PASS unit_outdoor_admission")
	fixture.queue_free()
	await process_frame
	quit(0 if failures.is_empty() else 1)

func batch_of(unit: int, batch: String) -> MeshInstance3D:
	return fixture.get_node("WorldUnits/Unit%d/NpcVisualRoot/NpcModel/%s" % [unit, batch]) as MeshInstance3D

func expect(unit: int, modes: Array, label: String) -> void:
	var names := ["On", "DoubleSided", "Off"]
	for index in names.size():
		var actual := batch_of(unit, names[index]).cast_shadow
		check(actual == modes[index], "%s: unit %d %s batch casts %d, expected %d" % [label, unit, names[index], actual, modes[index]])

func check(ok: bool, message: String) -> void:
	if not ok:
		failures.append(message)
		push_error(message)
