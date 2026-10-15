extends SceneTree

## Private UDP5921 authored Stockade ramp regression. Spawn via admin teleport at
## WoW (108.2204, -3.454892, -26.26199), map34. Uses real turn/W input, no position writes.
const NAME := "Ramplip"
const FEET := Vector3(108.2204, -26.26199, 3.454892)
const HEADING := Vector2(0.96152194, -0.27472818)
const SHOTS := "/syncthing/AgentShared/2026-10-15/ramp-lip"
var client: Node
var turn_sign := 0.0
var receipts: Dictionary = {}

func _initialize() -> void:
    Engine.max_fps = 60
    call_deferred("run_test")

func run_test() -> void:
    root.size = Vector2i(1280, 720)
    client = load("res://scenes/client.tscn").instantiate()
    root.add_child(client)
    current_scene = client
    var error: String = client.connect_account("127.0.0.1:5921", "fb_movement_ramp_lip", "fbtest", false)
    if error != "":
        fail("connect: " + error)
        return
    if not await wait_for(func(state): return state.screen == "CharacterSelect" and state.reply_received, 30000):
        return
    var ui := client.get_node("CharacterSelectUI")
    await click_control(ui.find_child("CharCard_0", true, false))
    await click_control(ui.find_child("EnterWorld", true, false))
    if not await wait_for(func(state): return state.screen == "InWorld" and state.terrain.pending_count == 0, 180000):
        return
    var player := client.get_node("WorldUnits/" + NAME) as Node3D
    for _frame in 240:
        await process_frame
    if player.position.distance_to(FEET) > 0.05:
        fail("spawn is not captured pose: " + str(player.position))
        return
    if not await face(player, atan2(HEADING.x, HEADING.y)):
        return
    client.set_camera_orbit(deg_to_rad(165.0), -0.3, 8.0)
    for _frame in 30:
        await process_frame
    if not verify_authored_wall(player):
        return
    receipts.start = client.account_state()
    await capture("before")
    push_key(KEY_W, true)
    for _frame in 7:
        await process_frame
    push_key(KEY_W, false)
    for _frame in 120:
        await process_frame
    receipts.end = client.account_state()
    await capture("after")
    var server: Vector3 = receipts.end.local_server_position
    print("RAMP_LIP start=", FEET, " end=", player.position, " server=", server)
    var file := FileAccess.open(OS.get_environment("RAMP_LIP_RECEIPT"), FileAccess.WRITE)
    file.store_string(JSON.stringify(receipts, "  "))
    if player.position.x < 108.8 or server.x < 108.8:
        fail("did not advance through the authored lip")
        return
    if abs(player.position.y + 25.606234) > 0.02 or abs(server.y + 25.606234) > 0.02:
        fail("client/server not on upper floor")
        return
    print("PASS ramp_lip: native held-W crosses face3153 onto upper floor; server agrees")
    client.free()
    quit(0)

func verify_authored_wall(player: Node3D) -> bool:
    var origin: Vector3 = player.position + Vector3.UP * 0.6
    var direction: Vector3 = Vector3(HEADING.x, 0.0, HEADING.y)
    var query: PhysicsRayQueryParameters3D = PhysicsRayQueryParameters3D.create(origin, origin + direction * (7.0 / 60.0 + 0.05))
    query.collision_mask = 1 << 1 # wmo::collision::WMO_LAYER
    var hit: Dictionary = player.get_world_3d().direct_space_state.intersect_ray(query)
    if hit.is_empty():
        fail("authored low wall ray missing; physics bodies are not ready")
        return false
    var position: Vector3 = hit.position
    var distance: float = origin.distance_to(position)
    receipts.wall_ray_origin = origin
    receipts.wall_ray_hit = position
    receipts.wall_ray_distance = distance
    if distance > 0.05:
        fail("low wall contact does not reproduce the captured margin clamp: " + str(distance))
        return false
    print("RAMP_LIP old_wall_ray hit=", position, " distance=", distance)
    return true

func wait_for(predicate: Callable, timeout_ms: int) -> bool:
    var deadline := Time.get_ticks_msec() + timeout_ms
    while Time.get_ticks_msec() < deadline:
        await process_frame
        if predicate.call(client.account_state()):
            return true
    fail("timeout: " + str(client.account_state()))
    return false

func face(player: Node3D, target: float) -> bool:
    for _attempt in 600:
        var facing := player.rotation.y + PI / 2.0
        var error := angle_difference(facing, target)
        if abs(error) < 0.035:
            release_turns()
            await process_frame
            return true
        if turn_sign == 0.0:
            push_key(KEY_RIGHT, true)
            for _frame in 5:
                await process_frame
            release_turns()
            turn_sign = sign(angle_difference(facing, player.rotation.y + PI / 2.0))
            if turn_sign == 0.0:
                fail("turn input did not change facing")
                return false
            continue
        var right: bool = sign(error) == turn_sign
        push_key(KEY_RIGHT, right)
        push_key(KEY_LEFT, not right)
        await process_frame
        release_turns()
    fail("failed to face captured heading")
    return false

func release_turns() -> void:
    push_key(KEY_RIGHT, false)
    push_key(KEY_LEFT, false)

func push_key(code: Key, pressed: bool) -> void:
    var key := InputEventKey.new()
    key.physical_keycode = code
    key.pressed = pressed
    root.push_input(key, true)

func click_control(control: Control) -> void:
    var point := control.get_global_rect().get_center()
    for pressed in [true, false]:
        var event := InputEventMouseButton.new()
        event.position = point
        event.button_index = MOUSE_BUTTON_LEFT
        event.pressed = pressed
        root.push_input(event, true)
        await process_frame
    await process_frame

func capture(label: String) -> void:
    DirAccess.make_dir_recursive_absolute(SHOTS)
    await RenderingServer.frame_post_draw
    var path := SHOTS.path_join(label + ".png")
    root.get_texture().get_image().save_png(path)
    print("SHOT ", path)

func fail(message: String) -> void:
    release_turns()
    push_key(KEY_W, false)
    push_error(message)
    quit(1)
