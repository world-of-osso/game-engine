extends "res://tests/adventure_plain_remaining_clips.gd"
# Observation only: normal physical navigation, unchanged accepted extension.
# Logger timestamps retain ordering even when the runner stores streams separately.
class PhaseErrors extends Logger:
	var mutex := Mutex.new()
	var phase := "startup"
	var events: Array[Dictionary] = []
	var basis_error_total := 0

	func tag(value: String) -> void:
		mutex.lock()
		phase = value
		mutex.unlock()

	func _log_error(function: String, file: String, line: int, code: String, rationale: String, editor_notify: bool, error_type: int, script_backtraces: Array[ScriptBacktrace]) -> void:
		var traces := []
		for trace in script_backtraces:
			traces.append(str(trace))
		mutex.lock()
		if function == "get_quaternion" and file.ends_with("basis.cpp"):
			basis_error_total += 1
		events.append({"phase":phase,"ticks_usec":Time.get_ticks_usec(),"frame":Engine.get_process_frames(),"function":function,"file":file,"line":line,"code":code,"rationale":rationale,"error_type":error_type,"script_backtraces":traces})
		mutex.unlock()

	func count_basis_errors() -> int:
		mutex.lock()
		var result := basis_error_total
		mutex.unlock()
		return result

	func drain() -> Array[Dictionary]:
		mutex.lock()
		var result := events.duplicate()
		events.clear()
		mutex.unlock()
		return result

var observer := PhaseErrors.new()
var observed_skeletons := {}
var error_count := 0

func _initialize() -> void:
	OS.add_logger(observer)
	process_frame.connect(observe_frame)
	super._initialize()

func animated_cases() -> Array:
	return [REMAINING_CASES[1]]

func tag_phase(value: String) -> void:
	observer.tag(value)
	print("JOURNAL_ATTRIBUTION_PHASE ", JSON.stringify({"phase":value,"ticks_usec":Time.get_ticks_usec(),"frame":Engine.get_process_frames()}))
	observe_frame()

func observe_frame() -> void:
	for event in observer.drain():
		error_count += 1
		print("JOURNAL_ATTRIBUTION_ERROR ", JSON.stringify(event))
	if client == null:
		return
	var host := control("EncounterJournal")
	if host == null:
		return
	for skeleton in host.find_children("*", "Skeleton3D", true, false):
		var identity: int = skeleton.get_instance_id()
		if observed_skeletons.has(identity):
			continue
		observed_skeletons[identity] = true
		var bones := []
		for index in skeleton.get_bone_count():
			var pose: Transform3D = skeleton.get_bone_pose(index)
			if absf(pose.basis.determinant()) < 0.00000001:
				bones.append({"index":index,"name":skeleton.get_bone_name(index),"parent":skeleton.get_bone_parent(index),"local_scale":str(skeleton.get_bone_pose_scale(index)),"local_basis":str(pose.basis)})
		var owner_model: Node = skeleton.get_parent()
		var player := owner_model.find_child("M2Animation", true, false)
		print("JOURNAL_ATTRIBUTION_SKELETON ", JSON.stringify({"phase":observer.phase,"ticks_usec":Time.get_ticks_usec(),"frame":Engine.get_process_frames(),"path":str(skeleton.get_path()),"identity":identity,"model_path":str(owner_model.get_path()),"model_identity":owner_model.get_instance_id(),"player_identity":player.get_instance_id() if player != null else 0,"animation":player.current_animation_id() if player != null else -1,"animation_phase":player.animation_phase() if player != null else {},"bone_count":skeleton.get_bone_count(),"degenerate_bones":bones,"journal":client.adventure_guide_state()}))

func click(name: String) -> bool:
	tag_phase("physical-click:" + name)
	var result := await super.click(name)
	tag_phase("after-click:" + name)
	return result

func open_animation_encounter(row: Array) -> bool:
	tag_phase("navigate-default-Sire-Denathrius")
	var result := await super.open_animation_encounter(row)
	tag_phase("default-model-before-Remornia-selection")
	# Observe the actual default attachment, not a substitute or injected model.
	if result:
		var source := source_row("JournalEncounterCreature", 5185)
		result = not completion.failed and completion.step(await expect_model(5185, int(source.CreatureDisplayInfoID), source.Name_lang), "default encounter model observation")
	return result

func check_authored_animation(row: Array) -> bool:
	tag_phase("Remornia-selected-before-stable213-check")
	var result := await super.check_authored_animation(row)
	tag_phase("Remornia-stable213")
	return result

func run() -> void:
	tag_phase("client-load-through-world-and-Journal")
	await super.run()
	observe_frame()
	print("JOURNAL_ATTRIBUTION_SUMMARY ", JSON.stringify({"errors":error_count,"skeletons":observed_skeletons.size(),"observation_only":true}))
