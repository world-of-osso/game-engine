extends SceneTree

func _initialize():
    call_deferred("run")

func run():
    root.size = Vector2i(1280, 720)
    var ui = ClassDB.instantiate("RegistryUi")
    root.add_child(ui)
    if not ui.has_method("show_character_select"):
        fail("RegistryUi cannot project authored character selection", ui)
        return
    var error = ui.show_character_select()
    if error != "":
        fail(error, ui)
        return
    await process_frame
    var canvas = ui.get_node("RegistryCanvas")
    var screen = canvas.find_child("CharSelectRoot", true, false)
    var enter = canvas.find_child("EnterWorld", true, false)
    if screen == null or enter == null:
        fail("Missing authored character selection controls", ui)
        return
    if enter.text != "Enter World":
        fail("Authored Enter World label changed", ui)
        return
    var back = canvas.find_child("BackToLogin", true, false)
    back.emit_signal("pressed")
    if ui.pop_action() != "back":
        fail("Authored Back action not routed", ui)
        return
    ui.free()
    print("PASS: authored empty character selection and Back action")
    quit(0)

func fail(message, ui):
    printerr(message)
    ui.free()
    quit(1)
