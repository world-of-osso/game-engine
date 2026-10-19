extends SceneTree
## Five concrete admitted identities, independent of UnitPick visibility.

class NativeTransforms extends RefCounted:
    func unit_transform(identity: int) -> Variant:
        if identity < 101 or identity > 105:
            return null
        return Transform3D(Basis(Vector3.UP, 0.25), Vector3(identity, 2, identity + 10))

func _initialize() -> void:
    if OS.get_environment("BOT_PARTY_CAPTURE_RED") != "1":
        print("IGNORED behavioral RED: all-five native transform capture not implemented")
        quit(0)
        return
    var capture = load("res://tests/bot_path_capture.gd").new()
    var result: Dictionary = capture.sample_party_actors(NativeTransforms.new(), ["101", "102", "103", "104", "105"])
    if result["actors"].size() != 5:
        push_error("Expected all5 admitted native transforms; observed " + str(result["actors"].size()))
        quit(1)
        return
    for identity in ["101", "102", "103", "104", "105"]:
        var pose: Dictionary = result["actors"][identity]
        assert(pose["position"] == [float(identity), 2.0, float(identity) + 10.0])
        assert(abs(pose["facing"] - 0.25) < 0.000001)
    assert(result["missing"].is_empty())
    print("ALL_FIVE_ADMITTED_TRANSFORMS_PASS")
    quit(0)
