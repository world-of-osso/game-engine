extends SceneTree

const DATA := "res://../data/models/humanmale_hd.m2"
const EPSILON := 0.0001

func fail(message: String) -> void:
	push_error(message)
	quit(1)

func close_pose(a: Transform3D, b: Transform3D) -> bool:
	return a.origin.distance_to(b.origin) < EPSILON and a.basis.x.distance_to(b.basis.x) < EPSILON and a.basis.y.distance_to(b.basis.y) < EPSILON and a.basis.z.distance_to(b.basis.z) < EPSILON

func _initialize() -> void:
	if not ClassDB.class_exists("WowAnimationPlayer"):
		fail("WowAnimationPlayer not registered")
		return
	var loaded: Dictionary = ClassDB.instantiate("WowAssetLoader").load_m2(DATA)
	if loaded.has("error"):
		fail("HD model: " + loaded.error)
		return
	var root: Node3D = loaded.node
	get_root().add_child(root)
	var skeleton := root.get_node("Skeleton3D") as Skeleton3D
	var player := root.get_node("WowAnimationPlayer")
	if skeleton == null or player == null or skeleton.get_bone_count() != 216:
		fail("HD animation player requires 216 Skeleton3D bones")
		return
	var idle := skeleton.get_bone_pose(0)
	if not player.advance_time_ms(1000.0):
		fail("could not advance Stand animation")
		return
	var changed_bones := 0
	for bone in skeleton.get_bone_count():
		if not close_pose(skeleton.get_bone_pose(bone), skeleton.get_bone_rest(bone)):
			changed_bones += 1
	if changed_bones < 10:
		fail("Stand did not animate authored HD bones: " + str(changed_bones))
		return
	if not player.play_sequence(4, true):
		fail("cannot select authored sequence 4")
		return
	player.advance_time_ms(70.0)
	var before := []
	for bone in skeleton.get_bone_count():
		before.append(skeleton.get_bone_pose(bone))
	if not player.play_sequence(1, true):
		fail("cannot select authored sequence 1")
		return
	for bone in skeleton.get_bone_count():
		if not close_pose(before[bone], skeleton.get_bone_pose(bone)):
			fail("midblend retransition popped bone " + str(bone))
			return
	player.set_paused(true)
	var paused := skeleton.get_bone_pose(0)
	player.advance_time_ms(300.0)
	if not close_pose(paused, skeleton.get_bone_pose(0)):
		fail("pause changed pose")
		return
	player.set_paused(false)
	if player.play_sequence(99999, true):
		fail("invalid sequence accepted")
		return
	print("PASS: %d HD bones have authored Stand motion, midblend transition continuous" % changed_bones)
	quit(0)
