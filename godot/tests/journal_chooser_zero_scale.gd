extends SceneTree
# Authentic model, no injected tracks/pose. CPU headless engine exercises the
# Skeleton3D conversion boundary that a pure Rust matrix test cannot observe.
const MODEL := "res://../data/products/wow/88d5e8ef05ec5a8446cdec93d142d7c6/models/3049899.m2"
const MODEL_SHA256 := "52868b6dde103fe9550dad69a13d1e9353f9ff1ff859277acdc820a253aab370"
const ROTATION_KEY_OFFSETS := {3:4660232, 4:4660296}
const PACKED_COMPONENT_SCALE := 32767.0
const QUATERNION_TOLERANCE := 0.00001

class BasisErrors extends Logger:
	var mutex := Mutex.new()
	var events: Array[Dictionary] = []

	func _log_error(function: String, file: String, line: int, code: String, rationale: String, editor_notify: bool, error_type: int, script_backtraces: Array[ScriptBacktrace]) -> void:
		if function != "get_quaternion" or not file.ends_with("basis.cpp"):
			return
		mutex.lock()
		events.append({"function":function,"file":file,"line":line,"code":code,"rationale":rationale})
		mutex.unlock()

	func snapshot() -> Array[Dictionary]:
		mutex.lock()
		var result := events.duplicate()
		mutex.unlock()
		return result

var observer := BasisErrors.new()
var completion: RefCounted

func _initialize() -> void:
	OS.add_logger(observer)
	call_deferred("run")

func read_source_rotation(offset: int) -> Quaternion:
	var file := FileAccess.open(MODEL, FileAccess.READ)
	file.seek(offset)
	var values := []
	for component in 4:
		var packed := file.get_16()
		var signed := packed if packed < 32768 else packed - 65536
		values.append((float(signed) + 32768.0) / PACKED_COMPONENT_SCALE if signed < 0 else (float(signed) - PACKED_COMPONENT_SCALE) / PACKED_COMPONENT_SCALE)
	# Same authored WoW-to-engine axes, independently read from the pinned key.
	return Quaternion(values[0], values[2], -values[1], values[3]).normalized()

func check_source_pose(skeleton: Skeleton3D) -> bool:
	for bone in range(3,7):
		if not completion.require(skeleton.get_bone_parent(bone) == -1 and skeleton.get_bone_pose_scale(bone) == Vector3.ZERO, "Authored root%s zero scale/parent changed" % bone):
			return false
		if ROTATION_KEY_OFFSETS.has(bone):
			var expected := read_source_rotation(ROTATION_KEY_OFFSETS[bone])
			var actual := skeleton.get_bone_pose_rotation(bone)
			if not completion.require(absf(expected.dot(Quaternion.IDENTITY)) < 0.999 and absf(absf(expected.dot(actual)) - 1.0) < QUATERNION_TOLERANCE, "Authored root%s nonidentity rotation changed: expected%s actual%s" % [bone,expected,actual]):
				return false
	return true

func run() -> void:
	completion = load("res://tests/completion.gd").new(self, 1)
	if not completion.require(FileAccess.get_sha256(MODEL) == MODEL_SHA256, "Pinned authentic3049899 bytes differ"):
		return
	var loader := ClassDB.instantiate("WowAssetLoader")
	var loaded: Dictionary = loader.load_m2(MODEL)
	if not completion.require(not loaded.has("error"), "Model load failed: " + str(loaded.get("error", ""))):
		return
	var model: Node3D = loaded.node
	# Production portraits retain the initial built pose the same way.
	model.process_mode = Node.PROCESS_MODE_DISABLED
	root.add_child(model)
	var player: Node = model.get_node("M2Animation")
	var phase: Dictionary = player.animation_phase()
	if not completion.require(player.current_animation_id() == 0 and float(phase.time_ms) == 0.0 and float(phase.duration_ms) == 4000.0, "Authentic initial idle0/t0 changed"):
		return
	var skeleton := model.get_node("Skeleton3D") as Skeleton3D
	if not check_source_pose(skeleton):
		return
	var positions := []
	for bone in range(3,7):
		positions.append(skeleton.get_bone_pose_position(bone))
	# Resolve actual engine overrides without a GPU, ModelScene or injected pose.
	skeleton.force_update_all_bone_transforms()
	if not check_source_pose(skeleton):
		return
	for bone in range(3,7):
		if not completion.require(skeleton.get_bone_pose_position(bone) == positions[bone-3], "Bone resolution changed source position"):
			return
	var errors := observer.snapshot()
	print("JOURNAL_CHOOSER_ZERO_SCALE ", JSON.stringify({"model":3049899,"animation":player.current_animation_id(),"phase":phase,"roots":[3,4,5,6],"source_rotation_preserved":true,"basis_error_count":errors.size(),"basis_errors":errors,"headless_cpu":true}))
	model.free()
	if not completion.require(errors.is_empty(), "Authored zero-scale bone caused%s Basis quaternion errors" % errors.size()):
		return
	completion.case_done()
	completion.finish(true)
