extends "res://tests/skybox_debug_screen.gd"

# Production loader/player/Skeleton observer, not a GPU deformation oracle.
# MAIN runs with --screen skyboxdebug --skybox-fdid 525142
# --skybox-time-ms 17040001 --skybox-verify. No test-owned pose writes/seeks.
const COASTAL_HASH := "d337757ddb5848f7e2b7e9feeaa302f2c73db4e619d16aa88636feb996c30b47"
const AUTHORED_BONE := 6
const REQUESTED_MS := "17040001"
# Reuse the existing observer's pre-runtime 0.0002 pose tolerance; not fitted.
# Oracle extracted independently with Python struct from the cached MD21 payload:
# sequence table352: id0, variation0, duration320000, flags0x20 (in-file).
# bone table416, bone6 at944: flags0x200 (not billboard), parent-1.
# Pivot WoW=(18.464839935302734,28.339948654174805,7.704826354980469).
# Translation timestamps at79600, vec3 keys at79632, interpolation1/global-1:
# 80000 -> (9.193352699279785,-1.8282742500305176,0)
# 80033 -> (-25.926925659179688,-3.3072967529296875,0).
# Rotation timestamps at79712, packed keys at79728, interpolation1/global-1:
# 80000 -> (32767,32767,30084,-112)
# 80033 -> (32767,32767,-19054,-3010).
# Scale block1000 has zero outer counts: identity scale.
# Original f32-before-remainder: 17040001 -> 17040000 -> 80000 (not80001).
# At this exact key no interpolation oracle is needed. WoW -> native (x,z,-y),
# local pivot + translation; packed quaternion unpack then normalize.
const EXPECTED_ORIGIN := Vector3(27.658191680908203, 7.704826354980469, -26.511674880981445)
const EXPECTED_ROTATION := Quaternion(0.0, -0.08188358099075926, 0.0, 0.9966419011681827)


func run_test() -> void:
	var args := OS.get_cmdline_user_args()
	if (
		first_value(args, "--screen") != "skyboxdebug"
		or first_value(args, "--skybox-fdid") != "525142"
		or first_value(args, "--skybox-time-ms") != REQUESTED_MS
		or args.has("--light-skybox-id")
		or not args.has("--skybox-verify")
	):
		reject("SETUP: requires coastal fixed17040001ms verification CLI; see fixture header")
		return
	expected_file = "costalislandskybox.m2"
	fixed_time = true
	verify_only = true
	if not await mount_production():
		return
	if not observe_assets():
		return
	var source := str(sky.get_meta(SOURCE_META))
	if FileAccess.get_sha256(source) != COASTAL_HASH:
		reject("SETUP: coastal authored bytes differ from independent pinned oracle")
		return
	var skeleton := sky.get_node_or_null("Skeleton3D") as Skeleton3D
	var player := sky.get_node_or_null("M2Animation")
	if skeleton == null or player == null:
		reject("SETUP: actual authored sky lacks Skeleton3D or animation player")
		return
	if not player.is_class("WowAnimationPlayer"):
		reject("SETUP: authored sky animation node is not production WowAnimationPlayer")
		return
	if skeleton.get_bone_count() != 22 or skeleton.get_bone_parent(AUTHORED_BONE) != -1:
		reject("SETUP: coastal bone6 hierarchy differs from pinned authored data")
		return
	if not check_offline():
		return
	print("FIXTURE AUTHORED_BONE source_sha256=", COASTAL_HASH,
		" bone=6 flags=0x200 parent=-1 sequence=0 duration_ms=320000 requested_ms=17040001 f32_ms=17040000 phase_ms=80000")
	print("FIXTURE AUTHORED_KEYS T80000=(9.193352699279785,-1.8282742500305176,0) R80000=(32767,32767,30084,-112) scale=identity")
	# Observe startup seek, then real subsequent process frames. Never pause, seek,
	# advance, or write the production player/skeleton from the fixture.
	for sample in range(4):
		if sample > 0:
			await create_timer(0.05).timeout
			await process_frame
			await RenderingServer.frame_post_draw
		if not observe_authored_bone(skeleton, sample):
			return
	print("PASS: pinned coastal bone6 retains authored fixed f32 phase through production loader/player/Skeleton3D")
	print("LIMIT: one positive-duration fixed phase; no natural-live, zero-duration authored, GPU deformation or pixel-parity proof")
	quit(0)


func observe_authored_bone(skeleton: Skeleton3D, sample: int) -> bool:
	var expected := Transform3D(Basis(EXPECTED_ROTATION), EXPECTED_ORIGIN)
	# Parent-1 makes local and skeleton-space transforms identical independently
	# of the native skeleton's rest/readback; neither supplies expected values.
	var local := skeleton.get_bone_pose(AUTHORED_BONE)
	var skeleton_space := skeleton.get_bone_global_pose(AUTHORED_BONE)
	print("FIXTURE AUTHORED_POSE sample=", sample, " expected=", expected,
		" local=", local, " skeleton_space=", skeleton_space, " epsilon=", POSE_EPSILON)
	for actual in [local, skeleton_space]:
		if not actual.origin.is_finite() or not actual.basis.is_finite():
			return reject("BEHAVIOR: coastal bone6 has nonfinite pose at sample%d" % sample)
		if actual.origin.distance_to(EXPECTED_ORIGIN) > POSE_EPSILON:
			return reject("BEHAVIOR: coastal bone6 origin differs from authored80000ms key at sample%d" % sample)
		for column in range(3):
			if actual.basis[column].distance_to(expected.basis[column]) > POSE_EPSILON:
				return reject("BEHAVIOR: coastal bone6 rotation/scale differs from authored80000ms key at sample%d" % sample)
	return true
