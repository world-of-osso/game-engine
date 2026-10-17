extends SceneTree
## A mismatched/missing received tree must stop before any native UI input.

class ReceivedClient:
    extends Node
    var config := {"spec_id": 72, "tree_id": 850, "entries": [], "unspent": []}

    func account_state() -> Dictionary:
        return {"local_player_id": 123}

    func spells_state() -> Dictionary:
        return {"known": [1464]}

    func spell_priority_state(_target: int, _spells: PackedInt32Array) -> Dictionary:
        return {"state_json": JSON.stringify({"trait_config": config})}

var failed := false

func _initialize() -> void:
    call_deferred("run_guard_cases")

func require_error(receipt: Dictionary, label: String) -> void:
    if not receipt.has("error"):
        push_error(label + ": expected explicit refusal, got " + JSON.stringify(receipt))
        failed = true

func run_guard_cases() -> void:
    var client := ReceivedClient.new()
    var controller = load("res://tests/bot_harness_talents.gd").new()
    var wrong_tree: Dictionary = await controller.apply_plan(self, client, {"spec": 72, "tree": 880})
    require_error(wrong_tree, "Fury tree880 plan against received tree850")
    if wrong_tree.get("native_config", {}).get("tree_id") != 850:
        push_error("Refusal lost received tree850 evidence")
        failed = true
    require_error(await controller.apply_plan(self, client, {"spec": 71, "tree": 850}), "wrong spec")
    client.config = {}
    require_error(await controller.apply_plan(self, client, {"spec": 72, "tree": 850}), "missing tree")
    client.free()
    if not failed:
        print("PASS native talent tree guards: wrong tree, wrong spec, unavailable tree")
    quit(1 if failed else 0)
