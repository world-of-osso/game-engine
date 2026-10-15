extends SceneTree
## Read-only native snapshots plus the existing private specialization controller.
## BOT_HARNESS_OUTPUT and BOT_HARNESS_SPEC isolate each real-player run.

var client: Node
var output: String
var spec_requested := false
var target_requested := false

func _initialize() -> void:
    output = OS.get_environment("BOT_HARNESS_OUTPUT")
    if output.is_empty() or OS.get_environment("BOT_HARNESS_SPEC").is_empty():
        push_error("bot harness requires output directory and specialization")
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
    if not spec_requested and account.get("gameplay_input_allowed", false):
        error = client.set_specialization(int(OS.get_environment("BOT_HARNESS_SPEC")))
        spec_requested = true
    if not target_requested and spells.get("spec", 0) == int(OS.get_environment("BOT_HARNESS_SPEC")) and account.get("gameplay_input_allowed", false):
        target_requested = true
        select_nearest()
    var snapshot := {"at_ms": int(Time.get_unix_time_from_system() * 1000.0),
        "account": account, "spells": spells, "spec_request_error": error,
        "target": client.target_state(), "auras": client.aura_state(),
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

func select_nearest() -> void:
    var event := InputEventKey.new()
    event.pressed = true
    event.keycode = KEY_TAB
    event.physical_keycode = KEY_TAB
    root.push_input(event)
    await process_frame
    event.pressed = false
    root.push_input(event)
