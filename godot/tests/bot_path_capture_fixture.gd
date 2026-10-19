extends "res://tests/bot_controller_live.gd"
## Executes the real opt-in initialization without starting a client or issuing input.

func _initialize() -> void:
    super._initialize()
    call_deferred("check_capture")

func boot() -> void:
    pass

func check_capture() -> void:
    var enabled := OS.get_environment("BOT_PATH_MAP_TRACE") == "1"
    var path := directory + "/bot-path-samples.jsonl"
    if not enabled:
        assert(path_capture == null)
        assert(not FileAccess.file_exists(path))
        print("PATH_CAPTURE_DISABLED_NO_FILE")
        quit(0)
        return
    assert(path_capture != null)
    path_capture.append_sample({"world_map_id": 34, "local_player_id": 101,
        "local_player_position": Vector3(10, 2, 20), "local_player_facing": 0.5,
        "local_player_epoch": 3, "local_server_epoch": 2},
        {"at_ms": 1000, "assisted": {"in_combat": true}},
        {"102": [11, 2, 21]}, {"102": 0.75})
    var row: Dictionary = JSON.parse_string(FileAccess.get_file_as_string(path))
    assert(row["actors"]["101"]["position"] == [10, 2, 20])
    assert(row["actors"]["101"]["facing"] == 0.5)
    assert(row["actors"]["102"]["facing"] == 0.75)
    assert(row["simulation_s"] == null and row["monotonic_ns"] > 0)
    print("PATH_CAPTURE_ENABLED_POSE_PASS")
    quit(0)
