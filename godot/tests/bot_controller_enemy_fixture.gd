extends SceneTree

func _initialize() -> void:
    var controller = load("res://tests/bot_controller_live.gd")
    var alive := JSON.stringify({"target": 41, "facts": {"target_hostile": true},
        "selection": {"Err": "unsupported authored distance predicate"}})
    var observed: Dictionary = controller.enemy_observation(41, alive, 2190.0)
    assert(observed.get("selected_hostile") == true)
    assert(observed.get("selected_health") == 2190.0)
    assert(not observed.has("error"))
    assert(controller.enemy_observation(42, alive, 2190.0).has("error"))
    assert(controller.enemy_observation(41, alive, 0.0).has("error"))
    var unavailable := JSON.stringify({"target": 41, "facts": {"target_hostile": null}})
    assert(controller.enemy_observation(41, unavailable, 2190.0).has("error"))
    var friendly := JSON.stringify({"target": 41, "facts": {"target_hostile": false}})
    assert(controller.enemy_observation(41, friendly, 2190.0).has("error"))
    print("native enemy observation: target-bound hostility/health controls PASS")
    quit(0)
