extends SceneTree

# Real WorldUnits.advance with synthetic native nodes. A standing remote NPC must not
# rewrite its transform every frame: each write notifies every mesh under the unit and
# queues one render-server command per mesh. A new facing must still turn the unit.
const YAWS := [0.4, 1.3, 2.9, -2.2, -0.7]
const FRAME := 1.0 / 60.0

class TransformCounter extends Node3D:
	var writes := 0

	func _ready() -> void:
		set_notify_transform(true)

	func _notification(what: int) -> void:
		if what == NOTIFICATION_TRANSFORM_CHANGED:
			writes += 1

var fixture: Node3D
var failures: Array[String] = []

func _initialize() -> void:
	call_deferred("run_test")

func run_test() -> void:
	fixture = ClassDB.instantiate("UnitTransformFixture")
	root.add_child(fixture)
	fixture.initialize(PackedFloat32Array(YAWS))
	var counters: Array[TransformCounter] = []
	for index in YAWS.size():
		var counter := TransformCounter.new()
		fixture.get_node("WorldUnits/Unit%d" % index).add_child(counter)
		counters.append(counter)
	await process_frame
	for counter in counters:
		counter.writes = 0
	for frame in 120:
		fixture.advance(FRAME)
	for index in YAWS.size():
		check(counters[index].writes == 0, "standing unit %d at yaw %s rewrote its transform %d times in 120 frames" % [index, YAWS[index], counters[index].writes])
	fixture.set_target_yaw(1, 2.0)
	for frame in 120:
		fixture.advance(FRAME)
	var turned := fixture.get_node("WorldUnits/Unit1") as Node3D
	check(counters[1].writes > 0, "a new facing turned the unit")
	var target := Quaternion(Vector3.UP, 2.0)
	# Component distance: single-precision acos cannot resolve angles this small.
	var facing := minf((turned.quaternion - target).length(), (turned.quaternion + target).length())
	check(facing < 1e-4, "unit settled %s from its target facing" % facing)
	if failures.is_empty():
		print("PASS unit_transform_writes")
	fixture.queue_free()
	await process_frame
	quit(0 if failures.is_empty() else 1)

func check(ok: bool, message: String) -> void:
	if not ok:
		failures.append(message)
		push_error(message)
