extends SceneTree
## Read-only native snapshots plus the existing private specialization controller.
## BOT_HARNESS_OUTPUT and BOT_HARNESS_SPEC isolate each real-player run.

var client: Node
var output: String
var spec_requested := false
var target_attempts := 0
var talent_status := "pending"
var gear_status := "pending"
var setup_started := false

func _initialize() -> void:
    output = OS.get_environment("BOT_HARNESS_OUTPUT")
    for required in ["BOT_HARNESS_OUTPUT", "BOT_HARNESS_SPEC", "BOT_HARNESS_TALENTS", "BOT_HARNESS_GEAR"]:
        if OS.get_environment(required).is_empty():
            push_error("bot harness requires " + required)
            quit(2)
            return
    call_deferred("boot")

func boot() -> void:
    client = load("res://scenes/client.tscn").instantiate()
    root.add_child(client)
    current_scene = client
    sample()

func sample() -> void:
    var account: Dictionary = client.account_state()
    var spells: Dictionary = client.spells_state()
    var error := ""
    if not spec_requested and spells.get("catalog_ready", false) and spells.get("spec", 0) != 0 and account.get("gameplay_input_allowed", false):
        error = client.set_specialization(int(OS.get_environment("BOT_HARNESS_SPEC")))
        spec_requested = true
    if not setup_started and spells.get("spec", 0) == int(OS.get_environment("BOT_HARNESS_SPEC")) and spells.get("catalog_ready", false) and account.get("gameplay_input_allowed", false):
        setup_started = true
        prepare_player()
    var target: Dictionary = client.target_state()
    if talent_status == "observed" and target_attempts < 32 and target.get("target_name", "") != "Training Dummy" and spells.get("spec", 0) == int(OS.get_environment("BOT_HARNESS_SPEC")) and account.get("gameplay_input_allowed", false):
        target_attempts += 1
        select_nearest()
    var target_position: Array[float] = []
    if target.get("target") != null:
        var transform: Variant = client.unit_transform(int(target["target"]))
        if transform is Transform3D:
            target_position.assign([transform.origin.x, transform.origin.y, transform.origin.z])
    var snapshot := {"at_ms": int(Time.get_unix_time_from_system() * 1000.0),
        "account": account, "spells": spells, "spec_request_error": error,
        "target": target, "target_position": target_position,
        "target_attempts": target_attempts, "spec_requested": spec_requested,
        "talent_status": talent_status, "gear_status": gear_status,
        "inventory": client.merchant_state(), "auras": client.aura_state(),
        "assisted": client.assisted_combat_state(), "cooldowns": client.cooldown_viewer_state()}
    var destination := output.path_join("native-state.json")
    var temporary := destination + ".writing"
    var file := FileAccess.open(temporary, FileAccess.WRITE)
    if file == null:
        push_error("Cannot write bot observation: " + str(FileAccess.get_open_error()))
        quit(2)
        return
    file.store_string(JSON.stringify(snapshot))
    file.close()
    var renamed := DirAccess.rename_absolute(temporary, destination)
    if renamed != OK:
        push_error("Cannot publish bot observation: " + str(renamed))
        quit(2)
        return
    create_timer(0.2).timeout.connect(sample, CONNECT_ONE_SHOT)

func publish_receipt(filename: String, receipt: Dictionary) -> bool:
    var file := FileAccess.open(output.path_join(filename), FileAccess.WRITE)
    if file == null:
        push_error("Cannot write " + filename)
        return false
    file.store_string(JSON.stringify(receipt))
    file.close()
    return true

func prepare_player() -> void:
    var plan: Variant = JSON.parse_string(FileAccess.get_file_as_string(OS.get_environment("BOT_HARNESS_GEAR")))
    if not plan is Dictionary:
        gear_status = "blocked"
        publish_receipt("native-gear.json", {"error": "Invalid sourced gear plan"})
        return
    gear_status = "applying"
    var controller = load("res://tests/bot_harness_gear.gd").new()
    var receipt: Dictionary = await controller.apply_plan(self, client, plan)
    if not publish_receipt("native-gear.json", receipt) or receipt.has("error"):
        gear_status = "blocked"
        return
    gear_status = "observed"
    await apply_talents()

func apply_talents() -> void:
    var path := OS.get_environment("BOT_HARNESS_TALENTS")
    var plan: Variant = JSON.parse_string(FileAccess.get_file_as_string(path))
    if not plan is Dictionary:
        talent_status = "blocked"
        publish_receipt("native-talents.json", {"error": "Invalid sourced talent plan"})
        return
    talent_status = "applying"
    var controller = load("res://tests/bot_harness_talents.gd").new()
    var receipt: Dictionary = await controller.apply_plan(self, plan)
    var written := publish_receipt("native-talents.json", receipt)
    talent_status = "blocked" if not written or receipt.has("error") else "observed"

func select_nearest() -> void:
    var event := InputEventKey.new()
    event.pressed = true
    event.keycode = KEY_TAB
    event.physical_keycode = KEY_TAB
    root.push_input(event)
    await process_frame
    event.pressed = false
    root.push_input(event)
