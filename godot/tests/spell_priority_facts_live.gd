extends SceneTree
## Real native read-only query contract. Host supplies a replicated, unselected NPC.

var client: Node
var output: String
var handled := false

func _initialize() -> void:
    output = OS.get_environment("PRIORITY_FACTS_OUTPUT")
    if output.is_empty():
        push_error("PRIORITY_FACTS_OUTPUT is required")
        quit(2)
        return
    call_deferred("boot")

func boot() -> void:
    client = load("res://scenes/client.tscn").instantiate()
    root.add_child(client)
    current_scene = client
    sample()

func publish(filename: String, value: Dictionary) -> void:
    var destination := output.path_join(filename)
    var temporary := destination + ".writing"
    var file := FileAccess.open(temporary, FileAccess.WRITE)
    if file == null:
        push_error("Cannot write native facts fixture output")
        quit(2)
        return
    file.store_string(JSON.stringify(value))
    file.close()
    var error := DirAccess.rename_absolute(temporary, destination)
    if error != OK:
        push_error("Cannot publish native facts fixture: " + str(error))
        quit(2)

func sample() -> void:
    var ids := []
    for node in root.find_children("UnitPick", "Area3D", true, false):
        if node.has_meta("unit_server_id"):
            ids.append(int(node.get_meta("unit_server_id")))
    publish("native-ready.json", {"account": client.account_state(),
        "spells": client.spells_state(), "target": client.target_state(), "unit_ids": ids})
    var path := output.path_join("facts-request.json")
    if not handled and FileAccess.file_exists(path):
        handled = true
        var request: Dictionary = JSON.parse_string(FileAccess.get_file_as_string(path))
        verify_query(request)
    create_timer(0.1).timeout.connect(sample, CONNECT_ONE_SHOT)

func verify_query(request: Dictionary) -> void:
    var before: Dictionary = client.target_state()
    if not client.has_method("spell_priority_state"):
        publish("facts-result.json", {"passed": false,
            "error": "Native read-only spell-priority facts unavailable", "before": before})
        return
    var ids := PackedInt32Array(request["spells"])
    var reply: Dictionary = client.call("spell_priority_state", int(request["target"]), ids)
    var after: Dictionary = client.target_state()
    if reply.has("error"):
        publish("facts-result.json", {"passed": false, "reply": reply, "before": before, "after": after})
        return
    var state: Dictionary = JSON.parse_string(reply["state_json"])
    var bad_target: Dictionary = client.call("spell_priority_state", 0, ids)
    var bad_spell: Dictionary = client.call("spell_priority_state", int(request["target"]), PackedInt32Array([0]))
    var known: Array = state["facts"]["known"]
    var passed: bool = (state["target"] == int(request["target"])
        and before.get("target") == after.get("target")
        and before.get("target") != int(request["target"])
        and known.has(2139) and state["facts"]["affordable"].has("2139")
        and typeof(state["player_haste_pct"]) == TYPE_FLOAT
        and state["spell_gcd_ms"]["44425"] == 1500
        and state["target_cast"].has("casting")
        and bad_target.has("error") and bad_spell.has("error"))
    publish("facts-result.json", {"passed": passed, "state": state,
        "bad_target": bad_target, "bad_spell": bad_spell, "before": before, "after": after})
