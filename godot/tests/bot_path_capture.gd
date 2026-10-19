extends RefCounted
## Opt-in observation only. No input, target selection, or simulation systems.

var stream: FileAccess

func open_capture(directory: String) -> Error:
    var path := directory + "/bot-path-samples.jsonl"
    if FileAccess.file_exists(path):
        return ERR_ALREADY_EXISTS
    stream = FileAccess.open(path, FileAccess.WRITE)
    return FileAccess.get_open_error()

func append_sample(account: Dictionary, rotation: Dictionary, origins: Dictionary, facings: Dictionary) -> void:
    if account.get("world_map_id") != 34:
        return
    var actors := {}
    for identity in origins:
        actors[identity] = {"position": origins[identity], "facing": facings.get(identity)}
    var position: Variant = account.get("local_player_position")
    var identity: Variant = account.get("local_player_id")
    if position is Vector3 and identity != null:
        actors[str(identity)] = {"position": [position.x, position.y, position.z],
            "facing": account.get("local_player_facing"),
            "local_epoch": account.get("local_player_epoch"),
            "server_epoch": account.get("local_server_epoch")}
    stream.store_line(JSON.stringify({"at_ms": rotation["at_ms"], "map_id": 34,
        "monotonic_ns": Time.get_ticks_usec() * 1000, "simulation_s": null,
        "actors": actors, "in_combat": rotation["assisted"].get("in_combat")}))
    stream.flush()
