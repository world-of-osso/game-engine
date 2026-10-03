extends "res://tests/m2_loader_pixels.gd"

const NPC := "Fixture Creature"
const DEAD_ON_SPAWN := "Fixture Dead on Spawn"
const PLAYER := "Fixture Player"
const APPEARANCE_NPC := "Fixture Appearance"
const MISSING_TYPE6_NPC := "Fixture Missing Type6"
const HAIR_TYPE6_NPC := "Fixture Hair Type6"
const TYPE19_EFFECT_NPC := "Fixture Type19 Effect"
const EYE := Color(185.0 / 255.0, 45.0 / 255.0, 215.0 / 255.0)
const BAKED := Color(230.0 / 255.0, 40.0 / 255.0, 80.0 / 255.0)
const COMPOSED := Color(30.0 / 255.0, 210.0 / 255.0, 90.0 / 255.0)
const HEAD := Color(35.0 / 255.0, 70.0 / 255.0, 225.0 / 255.0)
const HAIR := Color(245.0 / 255.0, 175.0 / 255.0, 25.0 / 255.0)
const DISPLAY_A := 910010
const DISPLAY_B := 910011
const WAIT_MS := 15000
# The isolated user data starts with an empty CASC index cache, so login waits for a cold
# CASC initialization (16.8 s idle, over 60 s on a loaded host) before the client
# processes frames.
const STARTUP_WAIT_MS := 180000
# The isolated data starts without terrain: entering a map extracts its 3x3 ADT tiles
# (root, tex0, obj0) and their WMOs from CASC before the world attaches; 12 files took over
# 15 s on a loaded host.
const WORLD_LOAD_WAIT_MS := 180000
const GLOBAL_AMBIENT := Vector3(51.0, 102.0, 153.0) / 255.0
const GLOBAL_DIRECT := Vector3(119.0, 85.0, 51.0) / 255.0
const LOCAL_AMBIENT := Vector3(153.0, 85.0, 51.0) / 255.0
const LOCAL_DIRECT := Vector3(51.0, 119.0, 153.0) / 255.0
const MAP_AMBIENT := Vector3(34.0, 102.0, 136.0) / 255.0
const MAP_DIRECT := Vector3(153.0, 68.0, 51.0) / 255.0
# retail_fog: from the 1000-yard far clip times the fixture's FogScaler 0.2, to the far clip.
const RETAIL_FOG_RANGE := Vector2(200.0, 1000.0)

func _initialize() -> void:
	call_deferred("run_test")

func run_test() -> void:
	root.size = Vector2i(1280, 720)
	var server := OS.get_environment("GODOT_TEST_SERVER")
	if not server.begins_with("127.0.0.1:") or server.ends_with(":0"):
		fail("Fixture requires owned loopback UDP endpoint")
		return
	if not prepare_assets():
		fail("Cannot prepare isolated authored M2/skin/BLP files")
		return
	if not ClassDB.class_exists("GameClient"):
		fail("Fixture GDExtension GameClient is unavailable")
		return
	var client = load("res://scenes/client.tscn").instantiate()
	root.add_child(client)
	var error = client.connect_account(server, "fixture", "fixture", false)
	if error != "":
		fail("Fixture connection: " + error)
		return
	if not await wait_screen(client, "CharacterSelect", STARTUP_WAIT_MS):
		return
	var ui = client.get_node_or_null("CharacterSelectUI")
	var card = ui.find_child("CharCard_0", true, false) if ui != null else null
	var enter = ui.find_child("EnterWorld", true, false) if ui != null else null
	if not card is Control or not enter is Button:
		fail("Fixture character selection controls missing")
		return
	await click_control(card)
	await click_control(enter)
	if not await wait_visual(client, 1.5, WORLD_LOAD_WAIT_MS):
		return
	if not await wait_lighting(client, GLOBAL_AMBIENT, GLOBAL_DIRECT, "azeroth"):
		return
	if not await wait_authored_stand(client):
		return
	var npc: Node3D = client.get_node("WorldUnits/" + NPC)
	var unit_id := npc.get_instance_id()
	var visual_id := npc.get_node("NpcVisualRoot").get_instance_id()
	var player = client.get_node_or_null("WorldUnits/" + PLAYER)
	if not player is Node3D or player.get_node_or_null("NpcVisualRoot") != null:
		fail("NPC visual attached to player or player missing")
		return
	var player_id: int = player.get_instance_id()
	print("FIXTURE INITIAL_READY")
	if not await wait_npc_moved(client, 5.5):
		return
	if npc.get_instance_id() != unit_id or npc.get_node("NpcVisualRoot").get_instance_id() != visual_id:
		fail("Same display replaced unit or visual")
		return
	print("FIXTURE SAME_READY")
	if not await wait_visual(client, 2.0):
		return
	if npc.get_instance_id() != unit_id or npc.get_node("NpcVisualRoot").get_instance_id() == visual_id:
		fail("Changed display did not preserve unit and replace visual")
		return
	if not await wait_authored_stand(client):
		return
	visual_id = npc.get_node("NpcVisualRoot").get_instance_id()
	if not lighting_matches(client, GLOBAL_AMBIENT, GLOBAL_DIRECT, "azeroth"):
		fail("Replacement model did not inherit current global lighting")
		return
	print("FIXTURE CHANGED_READY")
	if not await wait_lighting(client, LOCAL_AMBIENT, LOCAL_DIRECT, "azeroth"):
		return
	if npc.get_node("NpcVisualRoot").get_instance_id() != visual_id:
		fail("Live light update replaced creature visual")
		return
	print("FIXTURE LIGHT_UPDATED")
	if not await wait_visual(client, 0.01):
		return
	if npc.get_instance_id() != unit_id or npc.get_node("NpcVisualRoot").get_instance_id() == visual_id:
		fail("Tiny positive display did not clamp visual scale")
		return
	if not lighting_matches(client, LOCAL_AMBIENT, LOCAL_DIRECT, "azeroth"):
		fail("Late replacement did not inherit local light")
		return
	print("FIXTURE CLAMP_READY")
	if not await wait_no_visual(client):
		return
	if npc.get_instance_id() != unit_id:
		fail("Removing ModelDisplay replaced the NPC unit")
		return
	print("FIXTURE MODEL_REMOVED")
	if not await wait_visual(client, 1.5):
		return
	if not lighting_matches(client, LOCAL_AMBIENT, LOCAL_DIRECT, "azeroth"):
		fail("Restored model lost current light")
		return
	print("FIXTURE MODEL_RESTORED")
	if not await wait_no_npc(client):
		return
	if client.account_state().unit_count != 1:
		fail("NPC removal did not leave only the selected player")
		return
	print("FIXTURE NPC_REMOVED")
	if not await wait_visual(client, 1.5):
		return
	var replacement = client.get_node("WorldUnits/" + NPC)
	if replacement.get_instance_id() == unit_id or player.get_instance_id() != player_id or player.get_node_or_null("NpcVisualRoot") != null:
		fail("NPC respawn reused old unit or altered player appearance")
		return
	if not lighting_matches(client, LOCAL_AMBIENT, LOCAL_DIRECT, "azeroth"):
		fail("Respawned creature lost current light")
		return
	var old_light_id: int = client.get_node("WorldLighting").get_instance_id()
	print("FIXTURE NPC_RESTORED")
	if not await wait_lighting(client, MAP_AMBIENT, MAP_DIRECT, "kalimdor", WORLD_LOAD_WAIT_MS):
		return
	if client.get_node("WorldLighting").get_instance_id() == old_light_id:
		fail("Map change retained previous lighting producer")
		return
	var visible_npc: Node3D = client.get_node("WorldUnits/" + NPC)
	var retained_unit_id := visible_npc.get_instance_id()
	var retained_visual: Node3D = visible_npc.get_node("NpcVisualRoot")
	var retained_visual_id := retained_visual.get_instance_id()
	var retained_model: Node3D = retained_visual.get_node("NpcModel")
	var retained_model_id := retained_model.get_instance_id()
	var retained_batch := retained_model.find_child("Batch0", true, false) as MeshInstance3D
	if retained_batch == null or retained_batch.mesh == null or not retained_batch.is_visible_in_tree():
		fail("Always-visible NPC lacks a visible mesh before policy changes")
		return
	var retained_batch_id := retained_batch.get_instance_id()
	print("FIXTURE MAP_READY")
	if not await wait_npc_visibility(client, retained_unit_id, retained_visual_id, retained_model_id, retained_batch_id, false):
		return
	print("FIXTURE HIDDEN_READY")
	if not await wait_npc_moved(client, 6.5) or not await wait_npc_visibility(client, retained_unit_id, retained_visual_id, retained_model_id, retained_batch_id, false):
		return
	print("FIXTURE DEAD_ONLY_ALIVE_READY")
	if not await wait_npc_moved(client, 7.5) or not await wait_npc_visibility(client, retained_unit_id, retained_visual_id, retained_model_id, retained_batch_id, false):
		return
	if not await wait_death_pose(client, NPC, retained_unit_id, retained_visual_id):
		return
	print("FIXTURE REMOTE_DEAD_READY")
	if not await wait_npc_visibility(client, retained_unit_id, retained_visual_id, retained_model_id, retained_batch_id, true):
		return
	print("FIXTURE LOCAL_DEAD_READY")
	if not await wait_npc_visibility(client, retained_unit_id, retained_visual_id, retained_model_id, retained_batch_id, false):
		return
	print("FIXTURE HEALTH_REMOVED_READY")
	if not await wait_npc_moved(client, 8.5) or not await wait_npc_visibility(client, retained_unit_id, retained_visual_id, retained_model_id, retained_batch_id, false):
		return
	print("FIXTURE RESURRECTED_READY")
	if not await wait_npc_visibility(client, retained_unit_id, retained_visual_id, retained_model_id, retained_batch_id, true):
		return
	print("FIXTURE DAY_READY")
	if not await wait_npc_visibility(client, retained_unit_id, retained_visual_id, retained_model_id, retained_batch_id, false):
		return
	print("FIXTURE NIGHT_READY")
	if not await wait_npc_visibility(client, retained_unit_id, retained_visual_id, retained_model_id, retained_batch_id, true):
		return
	print("FIXTURE ALWAYS_READY")
	if not await wait_initially_dead_npc(client):
		return
	print("FIXTURE DEAD_ON_SPAWN_READY")
	if not await wait_appearance(client, BAKED):
		return
	print("FIXTURE BAKED_READY")
	if not await wait_appearance(client, COMPOSED):
		return
	print("FIXTURE COMPOSED_READY")
	if not await wait_missing_type6_visual(client):
		return
	print("FIXTURE TYPE6_MISSING_READY")
	if not await wait_hair_type6(client):
		return
	print("FIXTURE TYPE6_HAIR_READY")
	if not await wait_type19(client):
		return
	print("FIXTURE TYPE19_READY")
	if not await wait_two_texture_type19(client):
		return
	print("FIXTURE EFFECT_ISOLATED_READY")
	var reconnect_error = client.connect_account(server, "fixture", "fixture", false)
	if reconnect_error != "" or client.get_node_or_null("WorldUnits") != null or client.get_node_or_null("WorldLighting") != null or client.account_state().unit_count != 0:
		fail("Reconnect retained NPC visual/root: " + reconnect_error)
		return
	print("FIXTURE RESET_READY")
	client.free()
	quit(0)

func wait_lighting(client: Node, ambient: Vector3, direct: Vector3, map: String, timeout_ms := WAIT_MS) -> bool:
	var deadline := Time.get_ticks_msec() + timeout_ms
	while Time.get_ticks_msec() < deadline:
		await process_frame
		if lighting_matches(client, ambient, direct, map):
			return true
	var lighting := client.get_node_or_null("WorldLighting")
	var npc := client.get_node_or_null("WorldUnits/" + NPC)
	var model := npc.get_node_or_null("NpcVisualRoot/NpcModel") if npc != null else null
	var batch := model.find_child("Batch0", true, false) as MeshInstance3D if model != null else null
	var material := batch.get_active_material(0) as ShaderMaterial if batch != null else null
	var actual := "no creature material" if material == null else str({
		"scene_lit": material.get_shader_parameter("scene_light"),
		"scene_light": client.account_state().scene_light,
		"fog_mode": material.get_shader_parameter("fog_mode"),
	})
	fail("Timed out waiting for %s creature lighting %s / %s; producer=%s material=%s state=%s" % [map, ambient, direct, lighting, actual, client.account_state()])
	return false

func lighting_matches(client: Node, ambient: Vector3, direct: Vector3, map: String) -> bool:
	var terrain: Dictionary = client.account_state().terrain
	if terrain.map != map or not terrain.wdt_path.ends_with(".wdt"):
		return false
	var lighting := client.get_node_or_null("WorldLighting")
	var sun := lighting.get_node_or_null("Sun") as DirectionalLight3D if lighting != null else null
	var npc := client.get_node_or_null("WorldUnits/" + NPC)
	var model := npc.get_node_or_null("NpcVisualRoot/NpcModel") if npc != null else null
	var batch := model.find_child("Batch0", true, false) as MeshInstance3D if model != null else null
	var material := batch.get_active_material(0) as ShaderMaterial if batch != null else null
	# Scene-lit materials read the scene light's global uniforms.
	var scene = client.account_state().scene_light
	if sun == null or material == null or scene == null:
		return false
	var actual_ambient = scene.ambient
	var actual_direct = scene.direct
	var direction = scene.sun_direction
	var fog = scene.fog_range
	return material.get_shader_parameter("scene_light") == true \
		and actual_ambient is Vector3 and (actual_ambient as Vector3).is_equal_approx(ambient) \
		and actual_direct is Vector3 and (actual_direct as Vector3).is_equal_approx(direct) \
		and direction is Vector3 and (direction as Vector3).is_equal_approx(-sun.global_basis.z) \
		and fog is Vector2 and (fog as Vector2).is_equal_approx(RETAIL_FOG_RANGE) \
		and int(material.get_shader_parameter("fog_mode")) == 1

func wait_authored_stand(client: Node) -> bool:
	var npc := client.get_node_or_null("WorldUnits/" + NPC)
	var model := npc.get_node_or_null("NpcVisualRoot/NpcModel") if npc != null else null
	var skeleton := model.get_node_or_null("Skeleton3D") as Skeleton3D if model != null else null
	var animation := model.get_node_or_null("M2Animation") if model != null else null
	if skeleton == null or skeleton.get_bone_count() != 1 or animation == null:
		fail("UDP NPC model lacks authored Skeleton3D bone and automatic M2Animation")
		return false
	var minimum := INF
	var maximum := -INF
	var deadline := Time.get_ticks_msec() + 1200
	while Time.get_ticks_msec() < deadline:
		await process_frame
		var pose := skeleton.get_bone_pose_position(0) - skeleton.get_bone_rest(0).origin
		print("TRACE stand pose=%s %s" % [pose, lod_trace(client, model)])
		if absf(pose.x) > 0.05 or absf(pose.y) > 1.05 or absf(pose.z) > 0.05:
			fail("NPC defaulted to non-Stand sequence; authored Stand is Y 0..1, pose=" + str(pose))
			return false
		minimum = minf(minimum, pose.y)
		maximum = maxf(maximum, pose.y)
		if maximum - minimum > 0.15:
			return true
	fail("NPC Stand pose did not advance through authored keys: Y range " + str(Vector2(minimum, maximum)))
	return false

func wait_initially_dead_npc(client: Node) -> bool:
	var deadline := Time.get_ticks_msec() + WAIT_MS
	while Time.get_ticks_msec() < deadline:
		await process_frame
		var npc := client.get_node_or_null("WorldUnits/" + DEAD_ON_SPAWN) as Node3D
		var visual := npc.get_node_or_null("NpcVisualRoot") as Node3D if npc != null else null
		var skeleton := visual.get_node_or_null("NpcModel/Skeleton3D") as Skeleton3D if visual != null else null
		if skeleton != null and skeleton.get_bone_count() == 1:
			return await wait_death_pose(client, DEAD_ON_SPAWN, npc.get_instance_id(), visual.get_instance_id())
	fail("Initially-dead NPC did not load an animated visual")
	return false

func wait_death_pose(client: Node, name: String, unit_id: int, visual_id: int) -> bool:
	var saw_advance := false
	var last_pose := Vector3.INF
	var deadline := Time.get_ticks_msec() + WAIT_MS
	while Time.get_ticks_msec() < deadline:
		await process_frame
		var npc := client.get_node_or_null("WorldUnits/" + name) as Node3D
		var visual := npc.get_node_or_null("NpcVisualRoot") as Node3D if npc != null else null
		if npc == null or npc.get_instance_id() != unit_id or visual == null or visual.get_instance_id() != visual_id:
			fail("Death animation replaced or removed retained NPC unit/visual: " + name)
			return false
		var skeleton := visual.get_node_or_null("NpcModel/Skeleton3D") as Skeleton3D
		if skeleton == null or skeleton.get_bone_count() != 1:
			fail("Death animation lost authored skeleton: " + name)
			return false
		var pose := skeleton.get_bone_pose_position(0) - skeleton.get_bone_rest(0).origin
		last_pose = pose
		print("TRACE death %s pose=%s %s" % [name, pose, lod_trace(client, visual.get_node("NpcModel") as Node3D)])
		if pose.y > 2.1 and pose.y < 2.9:
			saw_advance = true
		if saw_advance and absf(pose.y - 3.0) < 0.05:
			var hold_until := Time.get_ticks_msec() + 450
			while Time.get_ticks_msec() < hold_until:
				await process_frame
				var held_npc := client.get_node_or_null("WorldUnits/" + name) as Node3D
				var held_visual := held_npc.get_node_or_null("NpcVisualRoot") as Node3D if held_npc != null else null
				if held_npc == null or held_npc.get_instance_id() != unit_id or held_visual == null or held_visual.get_instance_id() != visual_id:
					fail("Death clip replaced or removed NPC while holding: " + name)
					return false
				var held_pose := skeleton.get_bone_pose_position(0) - skeleton.get_bone_rest(0).origin
				if absf(held_pose.y - 3.0) > 0.05:
					fail("Death clip did not hold its last pose on retained visual: " + name)
					return false
			return true
	fail("Automatic Death pose never advanced to its held Y=3 end: %s, saw motion=%s, last pose=%s, animation id=%s" % [name, saw_advance, last_pose, _death_animation_id(client, name)])
	return false

## The NPC animation LOD inputs for `model`: whether its origin is in the world
## camera's view frustum, and its distance from the camera.
func lod_trace(client: Node, model: Node3D) -> String:
	var camera := client.get_node_or_null("WorldCamera") as Camera3D
	if camera == null or model == null:
		return "lod=unknown (no camera or model)"
	return "in_frustum=%s distance=%.1f camera=%s model=%s" % [camera.is_position_in_frustum(model.global_position), camera.global_position.distance_to(model.global_position), camera.global_position, model.global_position]

func _death_animation_id(client: Node, name: String) -> Variant:
	var animation = client.get_node_or_null("WorldUnits/" + name + "/NpcVisualRoot/NpcModel/M2Animation")
	return animation.current_animation_id() if animation != null else null

func make_npc_m2() -> PackedByteArray:
	var model := make_m2(7, 0)
	var md20 := model.slice(8, 8 + model.decode_u32(4))
	md20.resize(0x3d8)
	put_u32(md20, 0x1c, 3) # Walk, Stand, Death.
	put_u32(md20, 0x20, 0x240)
	put_u32(md20, 0x2c, 1) # One root bone.
	put_u32(md20, 0x30, 0x300)
	for index in 4:
		md20[0x140 + index * 48 + 12] = 255 # All vertices bound to bone 0.
	put_u16(md20, 0x240, 4) # Walk, deliberately first.
	put_u32(md20, 0x244, 1000)
	put_u16(md20, 0x27c, 0xffff) # No variation successor.
	put_u16(md20, 0x280, 0) # Stand at index 1.
	put_u32(md20, 0x284, 1000)
	put_u16(md20, 0x2bc, 0xffff)
	put_u16(md20, 0x2c0, 1) # Death at index 2.
	put_u32(md20, 0x2c4, 1000)
	put_u16(md20, 0x2fc, 0xffff)
	for sequence in 3:
		put_u32(md20, 0x24c + sequence * 0x40, 0x20) # Keyframes in the model, not an .anim file.
	put_u32(md20, 0x300, 0xffffffff) # No key bone ID.
	put_u16(md20, 0x308, 0xffff) # Root parent.
	put_u16(md20, 0x310, 1) # Linear translation interpolation.
	put_u16(md20, 0x312, 0xffff) # Local sequence, not global.
	put_u32(md20, 0x314, 3) # Per-sequence timestamp M2Arrays.
	put_u32(md20, 0x318, 0x358)
	put_u32(md20, 0x31c, 3) # Per-sequence value M2Arrays.
	put_u32(md20, 0x320, 0x370)
	put_u16(md20, 0x326, 0xffff) # Empty rotation global sequence.
	put_u16(md20, 0x33a, 0xffff) # Empty scale global sequence.
	for index in 3:
		put_u32(md20, 0x358 + index * 8, 1 if index == 0 else 2)
		put_u32(md20, 0x370 + index * 8, 1 if index == 0 else 2)
	put_u32(md20, 0x35c, 0x388) # Walk timestamps.
	put_u32(md20, 0x364, 0x38c) # Stand timestamps.
	put_u32(md20, 0x36c, 0x394) # Death timestamps.
	put_u32(md20, 0x374, 0x39c) # Walk values.
	put_u32(md20, 0x37c, 0x3a8) # Stand values.
	put_u32(md20, 0x384, 0x3c0) # Death values.
	put_u32(md20, 0x390, 1000)
	put_u32(md20, 0x398, 1000)
	put_float(md20, 0x39c, 10.0) # Walk keeps WoW X at 10.
	put_float(md20, 0x3bc, 1.0) # Stand WoW Z becomes Godot Y.
	put_float(md20, 0x3c8, 2.0) # Death starts above Stand's entire Y range.
	put_float(md20, 0x3d4, 3.0) # Death ends at Godot Y 3.
	var authored := chunk("MD21", md20)
	authored.append_array(model.slice(8 + model.decode_u32(4))) # Retain TXID.
	return authored

func make_appearance_skin() -> PackedByteArray:
	var original := make_skin(0x10, 1)
	var skin := original.slice(0, 64)
	skin.append_array(original.slice(52, 64)) # Second submesh needs its own six indices.
	skin.append_array(original.slice(64, 112)) # Geoset variant 1.
	skin.append_array(original.slice(64, 112)) # Geoset variant 2.
	skin.append_array(original.slice(112, 136))
	skin.append_array(original.slice(112, 136))
	put_u32(skin, 12, 12) # Two sets of six indices.
	put_u32(skin, 28, 2) # Two submeshes.
	put_u32(skin, 32, 76)
	put_u32(skin, 36, 2) # Two batches.
	put_u32(skin, 40, 172)
	put_u16(skin, 76, 101)
	put_u16(skin, 124, 102)
	put_u16(skin, 124 + 8, 6) # Second submesh begins after the first six indices.
	put_u16(skin, 196 + 4, 1) # Second batch uses submesh 1.
	return skin

func make_type19_effect_skin() -> PackedByteArray:
	var skin := make_appearance_skin()
	put_u16(skin, 124, 101) # Both ordinary and effect batches remain visible.
	put_u16(skin, 196 + 2, 0x4014)
	put_u16(skin, 196 + 10, 1) # Second material has blend mode 2.
	put_u16(skin, 196 + 14, 2) # Second texture resolves from TXID slot 1.
	return skin

func prepare_assets() -> bool:
	var data := ProjectSettings.globalize_path("res://../data")
	for folder in ["models", "textures"]:
		if DirAccess.make_dir_recursive_absolute(data + "/" + folder) != OK:
			return false
	var model := make_npc_m2()
	# Authored type 11 reads creature-display skin texture slot 0, not TXID.
	put_u32(model, 8 + 0x210, 11)
	var skin_fdid := PackedByteArray()
	skin_fdid.resize(4)
	put_u32(skin_fdid, 0, 910099)
	model.append_array(chunk("SFID", skin_fdid))
	var body_model := make_npc_m2()
	put_u32(body_model, 8 + 0x210, 1) # Body atlas, unlike ordinary creature type 11.
	body_model.append_array(chunk("SFID", skin_fdid))
	var missing_type6_model := make_npc_m2()
	put_u32(missing_type6_model, 8 + 0x210, 6) # Ordinary batch requires type 6; layout has no head/hair.
	missing_type6_model.append_array(chunk("SFID", skin_fdid))
	var type19_model := make_npc_m2()
	put_u32(type19_model, 8 + 0x210, 19)
	put_u32(type19_model, 8 + 0x70, 2) # Ordinary material 0, effect material 1.
	put_u16(type19_model, 8 + 0x204, 7)
	put_u16(type19_model, 8 + 0x206, 2)
	type19_model.append_array(chunk("SFID", skin_fdid))
	return write_fixture(data + "/models/910010.m2", model) \
		and write_fixture(data + "/models/91001000.skin", make_skin(0x10, 1)) \
		and write_fixture(data + "/models/910011.m2", model) \
		and write_fixture(data + "/models/91001100.skin", make_skin(0x10, 1)) \
		and write_fixture(data + "/models/910013.m2", body_model) \
		and write_fixture(data + "/models/91001300.skin", make_appearance_skin()) \
		and write_fixture(data + "/models/910014.m2", missing_type6_model) \
		and write_fixture(data + "/models/91001400.skin", make_skin(0x10, 1)) \
		and write_fixture(data + "/models/910016.m2", missing_type6_model) \
		and write_fixture(data + "/models/91001600.skin", make_skin(0x10, 1)) \
		and write_fixture(data + "/models/910017.m2", type19_model) \
		and write_fixture(data + "/models/91001700.skin", make_type19_effect_skin()) \
		and write_fixture(data + "/textures/910001.blp", make_blp(BASE)) \
		and write_fixture(data + "/textures/910002.blp", make_blp(SECOND)) \
		and write_fixture(data + "/textures/910020.blp", make_blp(BAKED)) \
		and write_fixture(data + "/textures/910021.blp", make_blp(BASE)) \
		and write_fixture(data + "/textures/910022.blp", make_blp(COMPOSED)) \
		and write_fixture(data + "/textures/910023.blp", make_blp(HEAD)) \
		and write_fixture(data + "/textures/910024.blp", make_hair_blp()) \
		and write_fixture(data + "/textures/910025.blp", make_blp(EYE)) \
		and write_fixture(data + "/textures/3484643.blp", make_blp(BASE))

func make_hair_blp() -> PackedByteArray:
	# Target 10 fills section 10 (1024x1024), then the HD runtime crop is 512x512.
	var bytes := make_blp(HAIR)
	var width := 1024
	var height := 1024
	bytes.resize(1172 + width * height * 4)
	put_u32(bytes, 12, width)
	put_u32(bytes, 16, height)
	put_u32(bytes, 84, width * height * 4)
	for index in width * height:
		var offset := 1172 + index * 4
		bytes[offset] = roundi(HAIR.b * 255.0)
		bytes[offset + 1] = roundi(HAIR.g * 255.0)
		bytes[offset + 2] = roundi(HAIR.r * 255.0)
		bytes[offset + 3] = 255
	return bytes

func appearance_batch_material(client: Node, batch_name: String) -> ShaderMaterial:
	var npc := client.get_node_or_null("WorldUnits/" + TYPE19_EFFECT_NPC)
	var model := npc.get_node_or_null("NpcVisualRoot/NpcModel") if npc != null else null
	var batch := model.find_child(batch_name, true, false) as MeshInstance3D if model != null else null
	if batch == null or batch.mesh == null or not batch.visible:
		return null
	return batch.get_active_material(0) as ShaderMaterial

func wait_type19(client: Node) -> bool:
	var deadline := Time.get_ticks_msec() + WAIT_MS
	var actual := Color.TRANSPARENT
	while Time.get_ticks_msec() < deadline:
		await process_frame
		var material := appearance_batch_material(client, "Batch0")
		var texture := material.get_shader_parameter("base_texture") as Texture2D if material != null else null
		if texture == null or int(material.get_shader_parameter("pixel_shader")) != 1:
			continue
		actual = texture.get_image().get_pixel(0, 0)
		if absf(actual.r - EYE.r) < TOLERANCE and absf(actual.g - EYE.g) < TOLERANCE \
			and absf(actual.b - EYE.b) < TOLERANCE and absf(actual.a - EYE.a) < TOLERANCE:
			return true
	fail("NPC ordinary type-19 base expected authored RGBA %s, got %s" % [EYE, actual])
	return false

# The two-texture blend-2 batch (shader 0x4014, Combiners_Mod_Mod2x) binds both slots
# like any batch: its replaceable type-19 slot 0 takes the NPC's type-19 texture
# (WebWowViewerCpp prepearMaterial -> getTexture) and slot 1 keeps TXID 910002.
func wait_two_texture_type19(client: Node) -> bool:
	var deadline := Time.get_ticks_msec() + WAIT_MS
	var actual_base := Color.TRANSPARENT
	var actual_second := Color.TRANSPARENT
	while Time.get_ticks_msec() < deadline:
		await process_frame
		var material := appearance_batch_material(client, "Batch1")
		if material == null or int(material.get_shader_parameter("pixel_shader")) != 7:
			continue
		var base := material.get_shader_parameter("base_texture") as Texture2D
		var second := material.get_shader_parameter("second_texture") as Texture2D
		if base == null or second == null:
			continue
		actual_base = base.get_image().get_pixel(0, 0)
		actual_second = second.get_image().get_pixel(0, 0)
		if absf(actual_base.r - EYE.r) < TOLERANCE and absf(actual_base.g - EYE.g) < TOLERANCE \
			and absf(actual_base.b - EYE.b) < TOLERANCE and absf(actual_base.a - EYE.a) < TOLERANCE \
			and absf(actual_second.r - SECOND.r) < TOLERANCE and absf(actual_second.g - SECOND.g) < TOLERANCE \
			and absf(actual_second.b - SECOND.b) < TOLERANCE and absf(actual_second.a - SECOND.a) < TOLERANCE:
			return true
	fail("NPC two-texture batch expected type-19 base %s and second %s, got %s and %s" % [EYE, SECOND, actual_base, actual_second])
	return false

func wait_hair_type6(client: Node) -> bool:
	var deadline := Time.get_ticks_msec() + WAIT_MS
	var actual := Color.TRANSPARENT
	while Time.get_ticks_msec() < deadline:
		await process_frame
		var npc := client.get_node_or_null("WorldUnits/" + HAIR_TYPE6_NPC)
		var model := npc.get_node_or_null("NpcVisualRoot/NpcModel") if npc != null else null
		var batch := model.find_child("Batch0", true, false) as MeshInstance3D if model != null else null
		var material := batch.get_active_material(0) as ShaderMaterial if batch != null else null
		var texture := material.get_shader_parameter("base_texture") as Texture2D if material != null else null
		if batch == null or batch.mesh == null or texture == null:
			continue
		var image := texture.get_image()
		if image.get_size() != Vector2i(512, 512):
			fail("NPC hair type-6 crop expected 512x512, got %s" % image.get_size())
			return false
		actual = image.get_pixel(0, 0)
		if absf(actual.r - HAIR.r) < TOLERANCE and absf(actual.g - HAIR.g) < TOLERANCE \
			and absf(actual.b - HAIR.b) < TOLERANCE and absf(actual.a - HAIR.a) < TOLERANCE:
			return true
	fail("NPC ordinary type-6 batch expected authored hair RGBA %s, got %s" % [HAIR, actual])
	return false

func wait_missing_type6_visual(client: Node) -> bool:
	var deadline := Time.get_ticks_msec() + WAIT_MS
	while Time.get_ticks_msec() < deadline:
		await process_frame
		var npc := client.get_node_or_null("WorldUnits/" + MISSING_TYPE6_NPC)
		if npc == null:
			continue
		for _frame in 10:
			await process_frame
			if npc.get_node_or_null("NpcVisualRoot") != null:
				fail("Missing required type-6 texture created a base-textured NPC visual")
				return false
		return true
	fail("Missing type-6 fixture NPC never replicated")
	return false

func wait_appearance(client: Node, expected: Color) -> bool:
	var deadline := Time.get_ticks_msec() + WAIT_MS
	var actual := Color.TRANSPARENT
	while Time.get_ticks_msec() < deadline:
		await process_frame
		var npc := client.get_node_or_null("WorldUnits/" + APPEARANCE_NPC)
		var model := npc.get_node_or_null("NpcVisualRoot/NpcModel") if npc != null else null
		var batch := model.find_child("Batch0", true, false) as MeshInstance3D if model != null else null
		var material := batch.get_active_material(0) as ShaderMaterial if batch != null else null
		var texture := material.get_shader_parameter("base_texture") as Texture2D if material != null else null
		if texture == null:
			continue
		var other_batch := model.find_child("Batch1", true, false) as MeshInstance3D
		if other_batch == null or batch.mesh == null or other_batch.mesh == null:
			continue
		var expects_variant_one := expected == COMPOSED
		if batch.visible != expects_variant_one or other_batch.visible == expects_variant_one:
			continue
		actual = texture.get_image().get_pixel(0, 0)
		if absf(actual.r - expected.r) < TOLERANCE and absf(actual.g - expected.g) < TOLERANCE \
			and absf(actual.b - expected.b) < TOLERANCE and absf(actual.a - expected.a) < TOLERANCE:
			return true
	fail("NPC appearance body expected RGBA %s, got %s" % [expected, actual])
	return false

func visual_matches(client: Node, scale: float) -> bool:
	var npc = client.get_node_or_null("WorldUnits/" + NPC)
	var visual = npc.get_node_or_null("NpcVisualRoot") if npc != null else null
	var model = visual.get_node_or_null("NpcModel") if visual != null else null
	if not npc is Node3D or not visual is Node3D or not model is Node3D:
		return false
	if absf(visual.scale.x - scale) > 0.0001 or absf(visual.scale.y - scale) > 0.0001 or absf(visual.scale.z - scale) > 0.0001:
		return false
	if absf(wrapf(visual.rotation.y + PI / 2.0, -PI, PI)) > 0.001:
		return false
	var batch = model.find_child("Batch0", true, false)
	if not batch is MeshInstance3D or batch.mesh == null:
		return false
	var material = batch.get_active_material(0) as ShaderMaterial
	if material == null:
		return false
	var expected := 910002 if scale == 2.0 or scale == 0.01 else 910001
	var texture = material.get_shader_parameter("base_texture") as Texture2D
	if texture == null:
		return false
	var actual := texture.get_image().get_pixel(0, 0)
	var expected_color := SECOND if expected == 910002 else BASE
	return absf(actual.r - expected_color.r) < TOLERANCE and absf(actual.g - expected_color.g) < TOLERANCE and absf(actual.b - expected_color.b) < TOLERANCE

func wait_visual(client: Node, scale: float, timeout_ms := WAIT_MS) -> bool:
	var deadline := Time.get_ticks_msec() + timeout_ms
	while Time.get_ticks_msec() < deadline:
		await process_frame
		if visual_matches(client, scale):
			return true
	fail("Timed out waiting for NPC model and scale %f: %s" % [scale, client.account_state()])
	return false

func wait_npc_visibility(client: Node, unit_id: int, visual_id: int, model_id: int, batch_id: int, expected_visible: bool) -> bool:
	var deadline := Time.get_ticks_msec() + WAIT_MS
	while Time.get_ticks_msec() < deadline:
		await process_frame
		var npc := client.get_node_or_null("WorldUnits/" + NPC) as Node3D
		var visual := npc.get_node_or_null("NpcVisualRoot") as Node3D if npc != null else null
		var model := visual.get_node_or_null("NpcModel") as Node3D if visual != null else null
		var batch := model.find_child("Batch0", true, false) as MeshInstance3D if model != null else null
		if npc == null or npc.get_instance_id() != unit_id or visual == null or visual.get_instance_id() != visual_id \
			or model == null or model.get_instance_id() != model_id or batch == null or batch.get_instance_id() != batch_id or batch.mesh == null:
			fail("Visibility policy replaced or removed retained NPC unit/model/mesh")
			return false
		if batch.is_visible_in_tree() == expected_visible:
			return true
	fail("Timed out waiting for retained NPC mesh visibility %s" % expected_visible)
	return false

func wait_npc_moved(client: Node, minimum_x: float) -> bool:
	var deadline := Time.get_ticks_msec() + WAIT_MS
	while Time.get_ticks_msec() < deadline:
		await process_frame
		var npc = client.get_node_or_null("WorldUnits/" + NPC) as Node3D
		if npc != null and npc.position.x > minimum_x:
			return true
	fail("Same-display replicated position update did not reach NPC")
	return false

func wait_no_visual(client: Node) -> bool:
	var deadline := Time.get_ticks_msec() + WAIT_MS
	while Time.get_ticks_msec() < deadline:
		await process_frame
		var npc = client.get_node_or_null("WorldUnits/" + NPC)
		if npc != null and npc.get_node_or_null("NpcVisualRoot") == null:
			return true
	fail("Removing ModelDisplay retained visual")
	return false

func wait_no_npc(client: Node) -> bool:
	var deadline := Time.get_ticks_msec() + WAIT_MS
	while Time.get_ticks_msec() < deadline:
		await process_frame
		if client.get_node_or_null("WorldUnits/" + NPC) == null:
			return true
	fail("NPC despawn retained world node")
	return false

# The session reaches a screen while local CASC startup still holds its UI back
# (`assets_starting`); the screen is shown once startup is over.
func wait_screen(client: Node, wanted: String, timeout_ms := WAIT_MS) -> bool:
	var deadline := Time.get_ticks_msec() + timeout_ms
	while Time.get_ticks_msec() < deadline:
		await process_frame
		var state: Dictionary = client.account_state()
		if state.screen == wanted and not state.assets_starting:
			return true
	fail("Timed out waiting for " + wanted + ": " + str(client.account_state()))
	return false

func click_control(control: Control) -> void:
	var point := control.get_global_rect().get_center()
	var press := InputEventMouseButton.new()
	press.position = point
	press.button_index = MOUSE_BUTTON_LEFT
	press.pressed = true
	root.push_input(press, true)
	await process_frame
	var release := InputEventMouseButton.new()
	release.position = point
	release.button_index = MOUSE_BUTTON_LEFT
	release.pressed = false
	root.push_input(release, true)
	await process_frame
	await process_frame

func fail(message: String) -> void:
	push_error(message)
	quit(1)
