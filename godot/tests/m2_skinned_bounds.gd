extends SceneTree

# Skinned M2 batches cull by the playing sequence's M2Sequence.bounds, replaced on
# every sequence change (WebWowViewerCpp m2Object.cpp isNeedUpdateBB/getAnimatinonBB),
# so Godot never re-derives their AABB from bones. Each box must hold every posed
# vertex of its sequence.
# creature/owl/owl.m2: no bone track runs on a global sequence, so each pose is the
# sequence's own.
const OWL := "res://../data/models/125378.m2"
# owl.m2 M2Sequence bounds (min xyz, max xyz) per sequence index, WoW axes.
const OWL_BOUNDS := [
	[-2.591072, -2.98111, 0.2193217, 1.547392, 3.062344, 6.303792],
	[-2.591072, -2.98111, 0.2193217, 1.547392, 3.062344, 6.303792],
	[-2.591072, -2.98111, 0.2193217, 1.547392, 3.062344, 6.303792],
	[-2.591072, -2.98111, 0.2193217, 1.547392, 3.062344, 6.303792],
	[-2.591072, -2.98111, 0.2193217, 1.547392, 3.062344, 6.303792],
	[-2.591072, -2.98111, 0.2193217, 1.547392, 3.062344, 6.303792],
	[-2.591072, -2.98111, 0.2193217, 1.547392, 3.062344, 6.303792],
	[-2.591072, -2.98111, 0.2193217, 1.547392, 3.062344, 6.303792],
	[-2.471237, -4.411331, -0.3449457, 3.524707, 3.496222, 5.890997],
	[-2.471237, -4.411331, -0.3449457, 3.524707, 3.496222, 5.890997],
	[-5.306696, -4.447066, -0.6534677, 1.547392, 4.176586, 6.811716],
	[-5.306696, -4.447066, -0.6534677, 1.547392, 4.176586, 6.811716],
	[-2.58782, -3.098671, 0.8510454, 2.857324, 3.177077, 5.921073],
	[-2.58782, -3.098671, 0.8510454, 2.857324, 3.177077, 5.921073],
	[-2.435665, -2.955204, 0.6662376, 1.915022, 2.94961, 5.536253],
	[-3.06288, -3.115625, 0.3276739, 1.547392, 3.001621, 6.192356],
	[-2.568368, -3.227252, 0.5427849, 1.610652, 3.303165, 6.674711],
	[-2.568368, -3.227252, 0.5427849, 1.610652, 3.303165, 6.674711],
	[-2.786269, -3.701791, -0.5608977, 2.306275, 2.91969, 5.994421],
	[-2.786269, -3.701791, -0.5608977, 2.306275, 2.91969, 5.994421],
	[-2.497612, -3.294042, -0.1464996, 2.224703, 3.334189, 7.515058],
	[-2.497612, -3.294042, -0.1464996, 2.224703, 3.334189, 7.515058],
	[-2.638677, -3.098671, -2.267681, 2.600184, 3.177077, 2.97785],
	[-2.411454, -2.955204, 0.3728257, 2.497092, 2.94961, 5.396069],
	[-2.420818, -3.343589, -0.1478673, 2.497092, 3.343717, 5.396069],
	[-2.954437, -3.032493, 0.1596703, 2.804831, 3.046476, 3.440565],
	[-2.643722, -2.898359, -0.8599172, 2.163674, 2.82686, 4.409657],
	[-3.842642, -3.44493, -1.775188, 1.647807, 3.127888, 3.345442],
	[-3.80251, -2.031125, -1.621651, 0.2928271, 1.440229, 2.038944],
	[-2.036306, -3.385216, -0.1711365, 1.357963, 3.385215, 1.192325],
	[-2.036306, -3.385216, -0.1711365, 1.357963, 3.385215, 1.192325],
]
# creature/boar/boar.m2: root bone 0 (pivot x -0.282) translates x +0.282 on global
# sequence 1, keyed only in timeline 0; every sequence's bounds include it.
const BOAR := "res://../data/models/boar.m2"
const BOAR_BOUNDS := [
	[-1.726403, -0.5548675, -0.03874029, 1.997907, 0.6349198, 2.177546],
	[-1.156953, -0.452104, -0.0280929, 1.783582, 0.4520297, 2.095408],
	[-1.40088, -0.4906308, -0.04115301, 1.798844, 0.5034214, 2.03946],
	[-1.207575, -1.01305, -0.1516759, 2.999217, 0.5954136, 2.241199],
	[-1.299169, -0.8180271, -0.1860881, 2.666161, 0.7006209, 2.305634],
	[-1.699293, -1.032954, -0.02809412, 1.745816, 1.154383, 2.066858],
	[-2.363588, -0.7746127, -0.09942739, 1.745816, 2.170983, 2.066858],
	[-1.894735, -1.342723, -0.02809337, 1.745481, 0.8120615, 2.06389],
	[-1.750982, -1.117696, -0.3745092, 1.943876, 1.560557, 2.168841],
	[-1.655347, -1.327324, -0.02813705, 1.747189, 1.618923, 2.115901],
	[-1.853935, -0.4344849, -0.4125419, 1.677978, 0.4550636, 2.444112],
	[-1.820788, -0.491787, -0.2857573, 1.747213, 0.5221748, 2.471987],
	[-1.933008, -0.4331515, -0.06284507, 1.370816, 0.4583513, 2.103845],
	[-2.003383, -0.5104572, -0.2291575, 1.608948, 0.4999057, 2.038019],
	[-1.478665, -0.5284975, -0.8087125, 1.234046, 0.5111005, 2.199663],
	[-1.995018, -0.8537464, -1.123487, 1.078402, 0.8409551, 2.290086],
	[-2.277039, -0.5474126, -0.9969122, 0.7471374, 0.5572421, 1.484951],
]
# Stand (animation 0, variation 0) is the sequence a loaded model plays: owl index 0,
# boar index 1 (after Run).
const OWL_STAND := 0
const BOAR_STAND := 1
# Past the longest (400 ms) crossfade from the previous sequence's pose.
const SETTLE_MS := 500.0
const STEP_MS := 100.0
# Non-looping: 7 s covers the longest owl (and boar) sequence, then holds its last frame.
const STEPS := 70
# Interpolated poses pass the authored boxes by up to 0.004 (owl EmoteExclamation).
const TOLERANCE := 0.01

func fail(message: String) -> void:
	push_error(message)
	quit(1)

# WoW (x,y,z) -> Godot (x,z,-y): the Y extremes swap on the negated axis.
func godot_box(wow: Array) -> AABB:
	var low := Vector3(wow[0], wow[2], -wow[4])
	var high := Vector3(wow[3], wow[5], -wow[1])
	return AABB(low, high - low)

func skinned_batches(root: Node3D) -> Array[MeshInstance3D]:
	var batches: Array[MeshInstance3D] = []
	for child in root.get_children():
		if child is MeshInstance3D and child.skin != null:
			batches.append(child)
	return batches

# Composed from the local poses the player wrote: Skeleton3D refreshes its cached
# global poses on its deferred update, which never runs inside _initialize.
func global_poses(skeleton: Skeleton3D) -> Array[Transform3D]:
	var poses: Array[Transform3D] = []
	for bone in skeleton.get_bone_count():
		var parent := skeleton.get_bone_parent(bone)
		var local := skeleton.get_bone_pose(bone)
		poses.append(local if parent < 0 else poses[parent] * local)
	return poses

func posed_vertices(batch: MeshInstance3D, skeleton: Skeleton3D) -> PackedVector3Array:
	var arrays := batch.mesh.surface_get_arrays(0)
	var vertices: PackedVector3Array = arrays[Mesh.ARRAY_VERTEX]
	var bones: PackedInt32Array = arrays[Mesh.ARRAY_BONES]
	var weights: PackedFloat32Array = arrays[Mesh.ARRAY_WEIGHTS]
	var per_vertex := bones.size() / vertices.size()
	var globals := global_poses(skeleton)
	var palette: Array[Transform3D] = []
	for bind in batch.skin.get_bind_count():
		palette.append(globals[batch.skin.get_bind_bone(bind)] * batch.skin.get_bind_pose(bind))
	var posed := PackedVector3Array()
	for v in vertices.size():
		var position := Vector3.ZERO
		for k in per_vertex:
			var weight := weights[v * per_vertex + k]
			if weight > 0.0:
				position += palette[bones[v * per_vertex + k]] * vertices[v] * weight
		posed.append(position)
	return posed

func expect_box(batches: Array[MeshInstance3D], bounds: Array, sequence: int, when: String) -> bool:
	var expected := godot_box(bounds[sequence])
	for batch in batches:
		if not batch.custom_aabb.is_equal_approx(expected):
			fail("%s: %s custom AABB %s, expected sequence %d bounds %s" % [when, batch.name, batch.custom_aabb, sequence, expected])
			return false
	return true

func _initialize() -> void:
	if check_model("owl", OWL, OWL_BOUNDS, OWL_STAND) and check_model("boar", BOAR, BOAR_BOUNDS, BOAR_STAND):
		quit(0)

func check_model(label: String, path: String, bounds: Array, stand: int) -> bool:
	var loaded: Dictionary = ClassDB.instantiate("WowAssetLoader").load_m2(path)
	if loaded.has("error"):
		fail(label + ": " + loaded.error)
		return false
	var root: Node3D = loaded.node
	get_root().add_child(root)
	var skeleton := root.get_node("Skeleton3D") as Skeleton3D
	var player := root.get_node("M2Animation")
	var batches := skinned_batches(root)
	if batches.is_empty():
		fail(label + " has no skinned batches")
		return false
	if not expect_box(batches, bounds, stand, label + " loaded"):
		return false
	var rest := AABB()
	for batch in batches:
		rest = batch.mesh.get_aabb() if rest.size == Vector3.ZERO else rest.merge(batch.mesh.get_aabb())
	var sequences := 0
	var beyond_rest := 0.0
	while player.play_sequence(sequences, false):
		if not expect_box(batches, bounds, sequences, "%s play_sequence(%d)" % [label, sequences]):
			return false
		player.advance_time_ms(SETTLE_MS)
		for step in STEPS:
			player.advance_time_ms(STEP_MS)
			var box := godot_box(bounds[sequences]).grow(TOLERANCE)
			for batch in batches:
				for position in posed_vertices(batch, skeleton):
					if not box.has_point(position):
						fail("%s sequence %d at %d ms: %s vertex %s outside its bounds %s" % [label, sequences, SETTLE_MS + (step + 1) * STEP_MS, batch.name, position, box])
						return false
					beyond_rest = max(beyond_rest, position.distance_to(position.clamp(rest.position, rest.end)))
		sequences += 1
	if sequences != bounds.size():
		fail("expected %d %s sequences, played %d" % [bounds.size(), label, sequences])
		return false
	if beyond_rest < 0.05:
		fail("%s animation never left its rest-pose AABB (max %.3f)" % [label, beyond_rest])
		return false
	print("PASS: %s: %d skinned batches use each of %d sequences' bounds; posed vertices stay inside, reaching %.3f beyond the rest AABB" % [label, batches.size(), sequences, beyond_rest])
	root.free()
	return true
