extends "res://tests/m2_loader_pixels.gd"

const NPC := "Fixture Creature"
const PLAYER := "Fixture Player"
const DISPLAY_A := 910010
const DISPLAY_B := 910011
const WAIT_MS := 15000
const GLOBAL_AMBIENT := Vector3(51.0, 102.0, 153.0) / 255.0
const GLOBAL_DIRECT := Vector3(119.0, 85.0, 51.0) / 255.0
const LOCAL_AMBIENT := Vector3(153.0, 85.0, 51.0) / 255.0
const LOCAL_DIRECT := Vector3(51.0, 119.0, 153.0) / 255.0
const MAP_AMBIENT := Vector3(34.0, 102.0, 136.0) / 255.0
const MAP_DIRECT := Vector3(153.0, 68.0, 51.0) / 255.0

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
	if not await wait_screen(client, "CharacterSelect"):
		return
	var ui = client.get_node_or_null("CharacterSelectUI")
	var card = ui.find_child("CharCard_0", true, false) if ui != null else null
	var enter = ui.find_child("EnterWorld", true, false) if ui != null else null
	if not card is Control or not enter is Button:
		fail("Fixture character selection controls missing")
		return
	await click_control(card)
	await click_control(enter)
	if not await wait_visual(client, 1.5):
		return
	if not await wait_lighting(client, GLOBAL_AMBIENT, GLOBAL_DIRECT, "azeroth"):
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
	if not await wait_lighting(client, MAP_AMBIENT, MAP_DIRECT, "kalimdor"):
		return
	if client.get_node("WorldLighting").get_instance_id() == old_light_id:
		fail("Map change retained previous lighting producer")
		return
	print("FIXTURE MAP_READY")
	var reconnect_error = client.connect_account(server, "fixture", "fixture", false)
	if reconnect_error != "" or client.get_node_or_null("WorldUnits") != null or client.get_node_or_null("WorldLighting") != null or client.account_state().unit_count != 0:
		fail("Reconnect retained NPC visual/root: " + reconnect_error)
		return
	print("FIXTURE RESET_READY")
	client.free()
	quit(0)

func wait_lighting(client: Node, ambient: Vector3, direct: Vector3, map: String) -> bool:
	var deadline := Time.get_ticks_msec() + WAIT_MS
	while Time.get_ticks_msec() < deadline:
		await process_frame
		if lighting_matches(client, ambient, direct, map):
			return true
	var lighting := client.get_node_or_null("WorldLighting")
	var npc := client.get_node_or_null("WorldUnits/" + NPC)
	var model := npc.get_node_or_null("NpcVisualRoot/NpcModel") if npc != null else null
	var batch := model.find_child("Batch0", true, false) as MeshInstance3D if model != null else null
	var material := batch.get_surface_override_material(0) as ShaderMaterial if batch != null else null
	var actual := "no creature material" if material == null else str({
		"ambient": material.get_shader_parameter("ambient"),
		"direct": material.get_shader_parameter("direct"),
		"fog_range": material.get_shader_parameter("fog_range"),
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
	var material := batch.get_surface_override_material(0) as ShaderMaterial if batch != null else null
	if sun == null or material == null:
		return false
	var actual_ambient = material.get_shader_parameter("ambient")
	var actual_direct = material.get_shader_parameter("direct")
	var direction = material.get_shader_parameter("sun_direction")
	var fog = material.get_shader_parameter("fog_range")
	return actual_ambient is Vector3 and (actual_ambient as Vector3).is_equal_approx(ambient) \
		and actual_direct is Vector3 and (actual_direct as Vector3).is_equal_approx(direct) \
		and direction is Vector3 and (direction as Vector3).is_equal_approx(-sun.global_basis.z) \
		and fog is Vector2 and (fog as Vector2).is_equal_approx(Vector2(200.0 / 36.0, 1000.0 / 36.0)) \
		and int(material.get_shader_parameter("fog_mode")) == 1

func prepare_assets() -> bool:
	var data := ProjectSettings.globalize_path("res://../data")
	for folder in ["models", "textures"]:
		if DirAccess.make_dir_recursive_absolute(data + "/" + folder) != OK:
			return false
	var model := make_m2(7, 0)
	# Authored type 11 reads creature-display skin texture slot 0, not TXID.
	put_u32(model, 8 + 0x210, 11)
	var skin_fdid := PackedByteArray()
	skin_fdid.resize(4)
	put_u32(skin_fdid, 0, 910099)
	model.append_array(chunk("SFID", skin_fdid))
	return write_fixture(data + "/models/910010.m2", model) \
		and write_fixture(data + "/models/91001000.skin", make_skin(0x10, 1)) \
		and write_fixture(data + "/models/910011.m2", model) \
		and write_fixture(data + "/models/91001100.skin", make_skin(0x10, 1)) \
		and write_fixture(data + "/textures/910001.blp", make_blp(BASE)) \
		and write_fixture(data + "/textures/910002.blp", make_blp(SECOND))

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
	var material = batch.get_surface_override_material(0) as ShaderMaterial
	if material == null:
		return false
	var expected := 910002 if scale == 2.0 or scale == 0.01 else 910001
	var texture = material.get_shader_parameter("base_texture") as Texture2D
	if texture == null:
		return false
	var actual := texture.get_image().get_pixel(0, 0)
	var expected_color := SECOND if expected == 910002 else BASE
	return absf(actual.r - expected_color.r) < TOLERANCE and absf(actual.g - expected_color.g) < TOLERANCE and absf(actual.b - expected_color.b) < TOLERANCE

func wait_visual(client: Node, scale: float) -> bool:
	var deadline := Time.get_ticks_msec() + WAIT_MS
	while Time.get_ticks_msec() < deadline:
		await process_frame
		if visual_matches(client, scale):
			return true
	fail("Timed out waiting for NPC model and scale %f: %s" % [scale, client.account_state()])
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

func wait_screen(client: Node, wanted: String) -> bool:
	var deadline := Time.get_ticks_msec() + WAIT_MS
	while Time.get_ticks_msec() < deadline:
		await process_frame
		if client.account_state().screen == wanted:
			return true
	fail("Timed out waiting for " + wanted)
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
