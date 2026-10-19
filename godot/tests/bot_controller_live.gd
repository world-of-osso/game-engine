extends SceneTree

var client: Node
var assisting := false
var handled_request := ""
var kit_captured := false
var last_metrics_ms := 0
var last_assisted_target: Variant = null
var clicking_loot := false
var handled_loot_request := ""
var handled_enemy_tab_request := ""
var directory: String
var spell_ids := PackedInt32Array()
var path_capture: RefCounted

func _initialize() -> void:
    directory = OS.get_environment("BOT_CONTROLLER_OUTPUT")
    var requested: Variant = JSON.parse_string(OS.get_environment("BOT_CONTROLLER_SPELL_IDS"))
    if directory.is_empty() or not DirAccess.dir_exists_absolute(directory) or not requested is Array or requested.is_empty():
        push_error("Controller requires existing BOT_CONTROLLER_OUTPUT and explicit BOT_CONTROLLER_SPELL_IDS JSON array")
        quit(2)
        return
    for identity in requested:
        if not (identity is int or identity is float) or int(identity) != identity or int(identity) <= 0:
            push_error("Invalid requested spell identity")
            quit(2)
            return
        spell_ids.append(int(identity))
    if OS.get_environment("BOT_PATH_MAP_TRACE") == "1":
        path_capture = load("res://tests/bot_path_capture.gd").new()
        var error: Error = path_capture.open_capture(directory)
        if error != OK:
            push_error("Cannot open opted-in path capture: " + str(error))
            quit(2)
            return
    call_deferred("boot")

func boot() -> void:
    client = load("res://scenes/client.tscn").instantiate()
    root.add_child(client)
    current_scene = client
    sample()

func publish(filename: String, data: Variant) -> void:
    var destination := directory + "/" + filename
    var temporary := destination + ".writing"
    var file := FileAccess.open(temporary, FileAccess.WRITE)
    if file == null:
        push_error("Cannot open fixture output " + temporary)
        quit(1)
        return
    file.store_string(JSON.stringify(data))
    file.close()
    var error := DirAccess.rename_absolute(temporary, destination)
    if error != OK:
        push_error("Cannot publish fixture output: " + str(error))
        quit(1)

func sample() -> void:
    var account: Dictionary = client.account_state()
    var centers := {}
    var origins := {}
    var facings := {}
    for node in root.find_children("UnitPick", "Area3D", true, false):
        if node.has_meta("unit_server_id") and node.get_child_count() > 0:
            var shape := node.get_child(0) as CollisionShape3D
            if shape != null:
                var center := shape.global_position
                centers[str(node.get_meta("unit_server_id"))] = [center.x, center.y, center.z]
            if node.has_meta("unit_visual"):
                var visual := instance_from_id(int(node.get_meta("unit_visual"))) as Node3D
                if visual != null:
                    var origin := visual.global_position
                    origins[str(node.get_meta("unit_server_id"))] = [origin.x, origin.y, origin.z]
                    if path_capture != null:
                        facings[str(node.get_meta("unit_server_id"))] = visual.global_rotation.y
    account["native_pick_centers"] = centers
    account["native_unit_origins"] = origins
    publish("native-account-state.json", account)
    var spells: Dictionary = client.spells_state()
    var now_ms := int(Time.get_unix_time_from_system() * 1000.0)
    var selected: Dictionary = client.target_state()
    var target_position: Array[float] = []
    if selected.get("target") != null:
        var transform: Variant = client.unit_transform(int(selected["target"]))
        if transform is Transform3D:
            target_position.assign([transform.origin.x, transform.origin.y, transform.origin.z])
    var rotation := {"at_ms": now_ms, "spells": spells,
        "target": selected.get("target"), "target_state": selected,
        "target_position": target_position, "facts": null,
        "assisted": client.assisted_combat_state(), "inventory": client.merchant_state(),
        "pet": client.pet_bar_state(), "ground_areas": client.call("ground_areas_state")}
    # Keep state_json untouched: parsing/stringifying in GDScript loses integer ID types.
    if account.get("gameplay_input_allowed", false) and selected.get("target") != null:
        rotation["visibility"] = client.call("target_visibility_state", int(selected["target"]))
        var query: Dictionary = client.call("spell_priority_state", int(selected["target"]),
            spell_ids)
        if query.has("error"):
            rotation["error"] = str(query["error"])
        else:
            var evaluation: Variant = JSON.parse_string(query["state_json"])
            if not evaluation is Dictionary:
                push_error("Native priority query returned invalid JSON")
                quit(1)
                return
            rotation.merge(evaluation, true)
    if path_capture != null:
        var party: Dictionary = path_capture.sample_admitted_party(client, directory, account.get("local_player_id"))
        path_capture.append_sample(account, rotation, origins, facings, party)
    publish("native-rotation-state.json", rotation)
    if now_ms - last_metrics_ms >= 1000:
        last_metrics_ms = now_ms
        publish("metrics-" + str(now_ms) + ".json", {"at_ms": now_ms,
            "meter": client.damage_meter_state(), "rotation": rotation,
            "unit_origins": origins,
            "player_position": account.get("local_player_position")})
    if not kit_captured and account.get("screen") == "InWorld" and account.get("local_player_health") != null and account.get("gameplay_input_allowed", false):
        if spells.get("catalog_ready", false) and not spells.get("known", []).is_empty():
            publish("native-spells-state.json", spells)
            kit_captured = true
            var controller = load("res://tests/bot_harness_talents.gd").new()
            var sheet: Dictionary = await controller.capture_sheet(self)
            publish("native-sheet-stats.json", {"at_ms": int(Time.get_unix_time_from_system() * 1000.0),
                "sheet_stats": sheet, "proof": "ordinary CharacterFrame input and visible stat labels"})
    var path := directory + "/native-assist-request.json"
    if not assisting and not clicking_loot and FileAccess.file_exists(path):
        var request: Variant = JSON.parse_string(FileAccess.get_file_as_string(path))
        if request is Dictionary and str(request.get("request_id", "")) != handled_request:
            handled_request = str(request["request_id"])
            assisting = true
            assist_target(request)
    var enemy_path := directory + "/native-enemy-tab-request.json"
    if not assisting and not clicking_loot and FileAccess.file_exists(enemy_path):
        var request: Variant = JSON.parse_string(FileAccess.get_file_as_string(enemy_path))
        if request is Dictionary and str(request.get("request_id", "")) != handled_enemy_tab_request:
            handled_enemy_tab_request = str(request["request_id"])
            assisting = true
            tab_to_enemy(request)
    var loot_path := directory + "/native-loot-click-request.json"
    if not assisting and not clicking_loot and FileAccess.file_exists(loot_path):
        var request: Variant = JSON.parse_string(FileAccess.get_file_as_string(loot_path))
        if request is Dictionary and str(request.get("request_id", "")) != handled_loot_request:
            handled_loot_request = str(request["request_id"])
            clicking_loot = true
            click_loot(request)
    create_timer(0.1).timeout.connect(sample, CONNECT_ONE_SHOT)

func finish_loot_click(request: Dictionary, result: Dictionary) -> void:
    result["request_id"] = request["request_id"]
    publish("native-loot-click-" + handled_loot_request + ".json", result)
    publish("native-loot-click-result.json", result)
    clicking_loot = false

func click_loot(request: Dictionary) -> void:
    var name := str(request.get("name", ""))
    var pattern := RegEx.create_from_string("^GroupLootFrame[0-9]+(Greed|Transmog)Button$")
    var point := Vector2(float(request.get("x", -1)), float(request.get("y", -1)))
    if pattern.search(name) == null or not point.is_finite() or not root.get_visible_rect().has_point(point):
        finish_loot_click(request, {"error": "Invalid native loot-button request"})
        return
    var path := directory + "/native-loot-click-" + handled_loot_request + ".png"
    var error := root.get_texture().get_image().save_png(path)
    if error != OK:
        finish_loot_click(request, {"error": "Cannot capture loot input: " + str(error)})
        return
    var motion := InputEventMouseMotion.new()
    motion.position = point
    motion.global_position = point
    root.push_input(motion)
    await process_frame
    for pressed in [true, false]:
        var event := InputEventMouseButton.new()
        event.button_index = MOUSE_BUTTON_LEFT
        event.position = point
        event.global_position = point
        event.pressed = pressed
        root.push_input(event)
        await process_frame
        await process_frame
    finish_loot_click(request, {"name": name, "x": point.x, "y": point.y,
        "input": "native viewport mouse motion + left press/release", "screenshot": path})

func press_release_key(code: Key) -> void:
    for pressed in [true, false]:
        var event := InputEventKey.new()
        event.keycode = code
        event.physical_keycode = code
        event.pressed = pressed
        root.push_input(event)
        await process_frame
        await process_frame

func finish_enemy_tab(request: Dictionary, result: Dictionary) -> void:
    result["request_id"] = request["request_id"]
    publish("native-enemy-tab-" + handled_enemy_tab_request + ".json", result)
    publish("native-enemy-tab-result.json", result)
    assisting = false

func tab_to_enemy(request: Dictionary) -> void:
    var before: Dictionary = client.target_state()
    await press_release_key(KEY_TAB)
    var after: Dictionary = client.target_state()
    if after.get("target") == null:
        finish_enemy_tab(request, {"selected_hostile": false, "selected_health": 0.0,
            "before": before, "after": after, "input": "one native KEY_TAB press/release; no target"})
        return
    var deadline := Time.get_ticks_msec() + 8000
    var observation := {"error": "Target-bound native hostility not observed"}
    while Time.get_ticks_msec() < deadline:
        var assisted: Dictionary = client.assisted_combat_state()
        observation = enemy_observation(int(after["target"]),
            str(assisted.get("evaluation_json", "")),
            float(client.spells_state().get("target_health", -1.0)))
        if not observation.has("error"):
            break
        await process_frame
    observation.merge({"before": before, "after": client.target_state(),
        "input": "one native KEY_TAB press/release; target-bound authored native hostility"})
    finish_enemy_tab(request, observation)

static func enemy_observation(target: int, encoded: String, health: float) -> Dictionary:
    var evaluation: Variant = JSON.parse_string(encoded)
    if not evaluation is Dictionary or evaluation.get("target") != target:
        return {"error": "Native hostility evaluation belongs to another or missing target"}
    var facts: Variant = evaluation.get("facts")
    if not facts is Dictionary or facts.get("target_hostile") != true or health <= 0.0:
        return {"error": "Native selected target lacks alive hostile observation"}
    return {"selected_hostile": true, "selected_health": health,
        "hostility_evaluation": evaluation}

func finish_assist(request: Dictionary, result: Dictionary) -> void:
    result["request_id"] = request["request_id"]
    publish("native-assist-" + handled_request + ".json", result)
    publish("native-assist-result.json", result)
    assisting = false

func assist_target(request: Dictionary) -> void:
    var started_us := Time.get_ticks_usec()
    var before: Dictionary = client.target_state()
    if str(request.get("leader_name", "")).is_empty() or before.get("target_name", "") != request["leader_name"]:
        finish_assist(request, {"error": "Native assist must start on requested leader", "before": before})
        return
    await press_release_key(KEY_F)
    var after: Dictionary = client.target_state()
    var input_done_us := Time.get_ticks_usec()
    var path := ""
    if after.get("target") != last_assisted_target:
        path = directory + "/native-assist-" + handled_request + ".png"
        var error := root.get_texture().get_image().save_png(path)
        if error != OK:
            finish_assist(request, {"error": "Cannot capture native assist frame: " + str(error)})
            return
        last_assisted_target = after.get("target")
    finish_assist(request, {"before": before, "after": after,
        "input": "native KEY_F press/release", "screenshot": path,
        "timing_us": {"native_input": input_done_us - started_us,
            "capture": Time.get_ticks_usec() - input_done_us},
        "selected_health": client.spells_state().get("target_health", -1.0)})
