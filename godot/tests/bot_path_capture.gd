extends RefCounted
## Opt-in observation only. No input, target selection, or simulation systems.

var stream: FileAccess
var admitted_ids: Array = []

func sample_party_actors(client: Object, identities: Array) -> Dictionary:
    var actors := {}
    var missing := []
    for identity in identities:
        var transform: Variant = client.call("unit_transform", int(identity))
        if not transform is Transform3D:
            missing.append(str(identity))
            continue
        var position: Vector3 = transform.origin
        actors[str(identity)] = {"position": [position.x, position.y, position.z],
            "facing": transform.basis.get_euler().y, "source": "native_unit_transform"}
    return {"actors": actors, "missing": missing}

func sample_admitted_party(client: Object, directory: String, owner: Variant) -> Dictionary:
    var path := directory + "/admitted-party.json"
    if admitted_ids.is_empty() and FileAccess.file_exists(path):
        var manifest: Variant = JSON.parse_string(FileAccess.get_file_as_string(path))
        if not manifest is Dictionary or not manifest.get("actors") is Array:
            push_error("Invalid admitted-party identity manifest for path capture")
            return {"actors": {}, "missing": [], "error": "invalid admitted-party manifest"}
        for actor in manifest["actors"]:
            admitted_ids.append(str(actor["entity"]))
    var identities := admitted_ids.duplicate()
    if owner != null:
        identities.append(str(owner))
    return sample_party_actors(client, identities)

func open_capture(directory: String) -> Error:
    var path := directory + "/bot-path-samples.jsonl"
    if FileAccess.file_exists(path):
        return ERR_ALREADY_EXISTS
    stream = FileAccess.open(path, FileAccess.WRITE)
    return FileAccess.get_open_error()

func append_sample(account: Dictionary, rotation: Dictionary, origins: Dictionary, facings: Dictionary, party: Dictionary = {}) -> void:
    if account.get("world_map_id") != 34:
        return
    var actors := {}
    for identity in origins:
        actors[identity] = {"position": origins[identity], "facing": facings.get(identity)}
    actors.merge(party.get("actors", {}), true)
    var position: Variant = account.get("local_player_position")
    var identity: Variant = account.get("local_player_id")
    if position is Vector3 and identity != null:
        actors[str(identity)] = {"position": [position.x, position.y, position.z],
            "facing": account.get("local_player_facing"),
            "local_epoch": account.get("local_player_epoch"),
            "server_epoch": account.get("local_server_epoch")}
    stream.store_line(JSON.stringify({"at_ms": rotation["at_ms"], "map_id": 34,
        "monotonic_ns": Time.get_ticks_usec() * 1000, "simulation_s": null,
        "actors": actors, "missing_admitted_actors": party.get("missing", []),
        "in_combat": rotation["assisted"].get("in_combat")}))
    stream.flush()
