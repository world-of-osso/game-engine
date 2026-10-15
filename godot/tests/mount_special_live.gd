extends "res://tests/player_emote_live.gd"
# Real private-server journal, native jump/chat input, and read-only peer observations.
# Inherits bounded login/input helpers; never creates replicas or synthetic emote events.
var proof_dir := OS.get_environment("MOUNT_SPECIAL_PROOF")
var role := OS.get_environment("MOUNT_SPECIAL_ROLE")
var actor_name := OS.get_environment("MOUNT_SPECIAL_ACTOR")
var proof_skin := OS.get_environment("MOUNT_SPECIAL_SKIN")
var mounts := [[18, "Chestnut Mare"], [129, "Golden Gryphon"], [1532, "Soaring Spelltome"]]

func run_test() -> void:
	print("MOUNT_SPECIAL_BEGIN ", role, " ", proof_skin)
	root.size = Vector2i(1920, 1080)
	var server := OS.get_environment("GODOT_TEST_SERVER")
	if server.is_empty() or server.ends_with(":5000") or proof_dir.is_empty() or not OS.get_environment("EMOTE_ACCOUNT").begins_with("fb_mount_"):
		fail("Explicit private server, fb_mount_* account and proof directory required")
		return
	shots = OS.get_environment("GODOT_TEST_CAPTURE_DIR")
	if shots.is_empty() or actor_name.is_empty() or role not in ["actor", "observer"]:
		fail("Capture directory, actor name and actor/observer role required")
		return
	client = load("res://scenes/client.tscn").instantiate()
	root.add_child(client)
	if not await until(func(): return not client.account_state().assets_starting, "asset startup", 180000): return
	print("MOUNT_SPECIAL_ASSETS_READY ", client.account_state())
	# One admitted run starts server and clients together; login waits for the real
	# server ping and offline character setup, not an assumed startup duration.
	if not await until(func(): return FileAccess.file_exists(path("server-ready")), "private server readiness", 180000): return
	var error: String = client.connect_account(server, OS.get_environment("EMOTE_ACCOUNT"), PASSWORD, false)
	if not error.is_empty(): fail(error); return
	print("MOUNT_SPECIAL_CONNECT ", error)
	if not await enter_world(): return
	print("MOUNT_SPECIAL_WORLD_READY ", client.account_state())
	var skin_error: String = client.preview_world_skin(proof_skin == "forever")
	if not skin_error.is_empty(): fail(skin_error); return
	var expected_skin := "Modern" if proof_skin == "modern" else "Forever"
	if client.account_state().ui_skin != expected_skin: fail("Unexpected skin"); return
	client.set_world_minutes(720.0)
	client.set_camera_orbit(PI, -0.3, 12.0)
	write_record(role + "-ready", client.account_state())
	if not await until(func(): return FileAccess.file_exists(path("setup-go")), "private setup", 180000): return
	if not await until(func(): return client.account_state().local_player_health > 0, "living private character setup", 15000): return
	if role == "observer":
		await observe_actor()
	else:
		await drive_mounts()

func enter_world() -> bool:
	print("MOUNT_SPECIAL_WAIT_ROSTER")
	if not await until(func(): return client.account_state().reply_received and client.account_state().screen == "CharacterSelect" and client.account_state().character_count >= 1 and client.get_node_or_null("CharacterSelectUI") != null, "character roster", 60000): return false
	await RenderingServer.frame_post_draw
	var enter := client.get_node("CharacterSelectUI").find_child("EnterWorld", true, false) as Button
	if not await until(func(): return enter.is_visible_in_tree() and not enter.disabled and enter.get_global_rect().size.y > 0, "enabled Enter World button", 60000): return false
	print("MOUNT_SPECIAL_ROSTER ", client.account_state(), " enter=", enter.get_global_rect())
	await click_control(enter)
	print("MOUNT_SPECIAL_ENTER_CLICKED")
	if not await until(func(): return client.account_state().screen == "InWorld" and client.account_state().local_player_position != null, "world entry", 300000): return false
	for frame in range(30): await process_frame
	return true

func drive_mounts() -> void:
	if not await until(func(): return FileAccess.file_exists(path("observer-ready")), "observer login", 120000): return
	for mount in mounts:
		await open_mounts()
		if not await until(func(): return control("MountJournalSearchBox") != null, "mount journal"): return
		await click_control(control("MountJournalSearchBox"))
		# Ctrl+A uses native input, not widget/model mutation.
		push_control_a()
		await process_frame
		await type_text(mount[1])
		if not await until(func(): return int(client.mount_journal_state().selected) == mount[0] and client.mount_journal_state().preview_ready, "authentic mount selection", 120000): return
		await click_control(control("MountJournalMountButton"))
		if not await until(func(): return mount_animation() != null and rider_skeleton() != null, "loaded mounted models", 120000): return
		await tap(KEY_ESCAPE)
		if not await until(func(): return mount_animation().current_animation_id() == 0, "mount idle"): return
		var prefix := "mount-%d" % mount[0]
		await capture(prefix + "-idle.png")
		write_record(prefix + "-stage", {"mount_id": mount[0], "skin": proof_skin, "idle": pose_record()})
		if not await until(func(): return FileAccess.file_exists(path(prefix + "-observer-mounted")), "observer mounted model", 120000): return
		var before: Vector3 = client.account_state().local_player_position
		await tap(KEY_SPACE)
		if not await until(func(): return special_phase_reached(0.0), "stationary jump MountSpecial"): return
		await capture(prefix + "-stationary-special.png")
		if not await sample_stationary_special(prefix, before, mount[0] == 1532): return
		if not await until(func(): return mount_animation().current_animation_id() == 0, "special returns to idle", 30000): return
		if not await send_line("/mountspecial"): return
		if not await until(func(): return special_phase_reached(0.0), "slash same MountSpecial"): return
		await capture(prefix + "-slash-special.png")
		if not await until(func(): return mount_animation().current_animation_id() == 0, "slash returns to idle", 30000): return
		push_key(KEY_W, 0, true)
		for frame in range(6): await process_frame
		var moving_start: Vector3 = client.account_state().local_player_position
		push_key(KEY_SPACE, 0, true)
		var rose := false
		for frame in range(20):
			await process_frame
			var position: Vector3 = client.account_state().local_player_position
			rose = rose or position.y > moving_start.y + 0.1
		push_key(KEY_SPACE, 0, false)
		push_key(KEY_W, 0, false)
		if not rose: fail("Moving mounted jump did not jump"); return
		write_record(prefix + "-moving-jump", {"rose": rose, "start": moving_start, "end": client.account_state().local_player_position})
		if not await until(func(): return FileAccess.file_exists(path(prefix + "-observer-special")), "replicated peer special", 30000): return
		await open_mounts()
		await click_control(control("MountJournalMountButton"))
		if not await until(func(): return mount_animation() == null, "dismount"): return
		write_record(prefix + "-actor-dismounted", {"skin": proof_skin})
		# Keep the unmounted state stable until the real peer observes it.
		if not await until(func(): return FileAccess.file_exists(path(prefix + "-observer-dismounted")), "observer dismount acknowledgement", 120000): return
		await tap(KEY_ESCAPE)
	write_record("actor-done", {"skin": proof_skin, "mounts": mounts})
	print("MOUNT_SPECIAL_ACTOR PASS ", proof_skin)
	quit(0)

func sample_stationary_special(prefix: String, before: Vector3, book: bool) -> bool:
	var samples := []
	var captured := false
	var deadline := Time.get_ticks_msec() + 30000
	var capture_fraction := 0.875 if book else 0.4
	while mount_animation() != null and mount_animation().current_animation_id() == 94:
		var position: Vector3 = client.account_state().local_player_position
		if abs(position.y - before.y) > 0.05: fail("Stationary special left ground"); return false
		var phase := mount_animation().animation_phase()
		var fraction: float = phase.time_ms / phase.duration_ms
		if samples.size() < 5 and fraction >= samples.size() * 0.2:
			samples.append(pose_record())
		if not captured and fraction >= capture_fraction:
			await capture(prefix + "-rider-special.png")
			if book: await capture_spelltome_diagnostic()
			captured = true
		if Time.get_ticks_msec() >= deadline: fail("Stationary special did not finish"); return false
		await process_frame
	if not captured: fail("Special ended before the authored-phase capture"); return false
	write_record(prefix + "-stationary", {"before": before, "after": client.account_state().local_player_position, "poses": samples})
	return true

func observe_actor() -> void:
	for mount in mounts:
		var prefix := "mount-%d" % mount[0]
		if not await until(func(): return FileAccess.file_exists(path(prefix + "-stage")) and mount_animation() != null, "actor mounted", 180000): return
		write_record(prefix + "-observer-mounted", pose_record())
		var fraction := 0.875 if mount[0] == 1532 else 0.4
		if not await until(func(): return special_phase_reached(fraction), "real replicated special at authored phase", 120000): return
		await capture(prefix + "-observer-special.png")
		write_record(prefix + "-observer-special", pose_record())
		if not await until(func(): return mount_animation() == null, "actor dismount", 120000): return
		write_record(prefix + "-observer-dismounted", {"skin": proof_skin})
	print("MOUNT_SPECIAL_OBSERVER PASS ", proof_skin)
	quit(0)

func special_phase_reached(fraction: float) -> bool:
	var animation := mount_animation()
	if animation == null or animation.current_animation_id() != 94: return false
	var phase := animation.animation_phase()
	return phase.time_ms >= phase.duration_ms * fraction

func actor_unit() -> Node:
	var units := client.find_child("WorldUnits", true, false)
	return units.find_child(actor_name, false, false) if units != null else null

func mount_animation() -> WowAnimationPlayer:
	var unit := actor_unit()
	if unit == null: return null
	for animation_node in unit.find_children("M2Animation", "", true, false):
		if animation_node.get_parent().name == "NpcModel": return animation_node as WowAnimationPlayer
	return null

func rider_skeleton() -> Skeleton3D:
	var unit := actor_unit()
	var model := unit.find_child("PlayerModel", true, false) if unit != null else null
	return model.get_node_or_null("Skeleton3D") as Skeleton3D if model != null else null

func pose_record() -> Dictionary:
	var poses := []
	var skeleton := rider_skeleton()
	if skeleton != null:
		for bone in range(skeleton.get_bone_count()):
			poses.append(str(skeleton.get_bone_pose(bone)))
	var unit := actor_unit() as Node3D
	var animation := mount_animation()
	var model := animation.get_parent() as Node3D if animation != null else null
	var saddle := model.get_node_or_null("Skeleton3D/AttachmentBone0/Attachment0") as Node3D if model != null else null
	var rider := skeleton.get_parent() as Node3D if skeleton != null else null
	return {"mount_clip": animation.current_animation_id() if animation != null else -1, "mount_phase": animation.animation_phase() if animation != null else {}, "rider_bones": poses, "actor_name": unit.name if unit != null else "", "actor_position": unit.global_position if unit != null else null, "mount_position": model.global_position if model != null else null, "mount_visible": model.is_visible_in_tree() if model != null else false, "mount_source": model.get_meta("m2_source_path") if model != null else "", "saddle_position": saddle.global_position if saddle != null else null, "rider_position": rider.global_position if rider != null else null, "rider_local_transform": rider.transform if rider != null else null, "camera_position": root.get_camera_3d().global_position, "account": client.account_state()}

func capture(file: String) -> void:
	if role != "observer":
		await super.capture(file)
		write_capture_record(file)
		return
	# GameClient makes its own camera current each frame. The fixture camera runs
	# afterward, changing only the view of the actual replicated actor.
	var previous := root.get_camera_3d()
	var camera := load("res://tests/mount_special_observer_camera.gd").new() as Camera3D
	camera.set("actor", actor_unit())
	camera.process_priority = 1000000
	camera.fov = previous.fov
	camera.near = previous.near
	camera.far = previous.far
	root.add_child(camera)
	await process_frame
	await super.capture(file)
	write_capture_record(file)
	previous.make_current()
	camera.queue_free()

func write_capture_record(file: String) -> void:
	var record := pose_record()
	write_record("capture-" + file.get_basename(), record)
	if file.ends_with("special.png") and record.mount_clip != 94:
		fail("Rendered capture missed the active MountSpecial phase: " + file)

func capture_spelltome_diagnostic() -> void:
	# Freeze the naturally reached frame only for isolation; normal proof shots precede it.
	paused = true
	# Unit particles share model/emitter pools under WorldUnits, not each unit node.
	var pools := client.find_children("Particles4061989_*", "MultiMeshInstance3D", true, false)
	var visible_pools := []
	for child in pools:
		var pool := child as MultiMeshInstance3D
		if pool.visible:
			visible_pools.append(pool)
			pool.hide()
	write_record("spelltome-particle-isolation", {"paused_diagnostic": true, "hidden_pools": visible_pools.size(), "pose": pose_record()})
	await capture("diagnostic-spelltome-without-particles.png")
	for pool in visible_pools: pool.show()
	var model := mount_animation().get_parent()
	var materials := []
	for child in model.find_children("Batch*", "MeshInstance3D", false, false):
		var mesh := child as MeshInstance3D
		var material := mesh.material_override as ShaderMaterial
		materials.append({"name": mesh.name, "visible": mesh.visible, "shader": material.get_shader_parameter("pixel_shader") if material != null else null, "opacity": material.get_shader_parameter("transparency") if material != null else null, "mesh_color": material.get_shader_parameter("mesh_color") if material != null else null, "gx_blend": material.get_shader_parameter("gx_blend") if material != null else null, "base_input": texture_snapshot(material, "base_texture"), "second_input": texture_snapshot(material, "second_texture")})
	write_record("spelltome-materials", materials)
	# Single-batch diagnostic isolation; every visibility edit is restored immediately.
	for batch in ["Batch0", "Batch1", "Batch8", "Batch9", "Batch10"]:
		var mesh := model.get_node_or_null(batch) as MeshInstance3D
		if mesh != null and mesh.visible:
			mesh.hide()
			await capture("diagnostic-spelltome-without-" + batch + ".png")
			mesh.show()
	paused = false

func texture_snapshot(material: ShaderMaterial, parameter: String) -> Variant:
	if material == null: return null
	var texture := material.get_shader_parameter(parameter) as Texture2D
	if texture == null: return null
	var image := texture.get_image()
	return {"size": image.get_size(), "corner_rgba": image.get_pixel(0, 0).to_html(), "center_rgba": image.get_pixelv(image.get_size() / 2).to_html()}

func control(name: String) -> Control:
	return client.find_child(name, true, false) as Control

func open_mounts() -> void:
	await click_control(control("MinimapLauncherButton"))
	if not await until(func(): return control("LauncherEntryCollectionsMicroButton") != null, "Collections entry"): return
	await click_control(control("LauncherEntryCollectionsMicroButton"))

func push_control_a() -> void:
	for pressed in [true, false]:
		var event := InputEventKey.new()
		event.keycode = KEY_A
		event.physical_keycode = KEY_A
		event.ctrl_pressed = true
		event.pressed = pressed
		root.push_input(event, true)

func path(label: String) -> String:
	return proof_dir.path_join(proof_skin + "-" + label + ".json")

func write_record(label: String, data: Variant) -> void:
	var file := FileAccess.open(path(label), FileAccess.WRITE)
	if file == null: fail("Cannot write proof receipt " + path(label)); return
	file.store_string(JSON.stringify(data, "\t"))

func until(predicate: Callable, what: String, timeout_ms: int = 15000) -> bool:
	var deadline := Time.get_ticks_msec() + timeout_ms
	while Time.get_ticks_msec() < deadline:
		if predicate.call(): return true
		await process_frame
	fail("Timed out: " + what)
	return false
