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
    var viewport_rect := Rect2(Vector2.ZERO, Vector2(root.size))
    var action_rect: Rect2 = enter.get_global_rect()
    if not screen is Control or not screen.size.is_equal_approx(viewport_rect.size) or not viewport_rect.encloses(action_rect):
        fail("Character selection root/action outside viewport: root=" + str(screen.size) + " action=" + str(action_rect), ui)
        return
    var point := action_rect.get_center()
    var press := InputEventMouseButton.new()
    press.position = point
    press.button_index = MOUSE_BUTTON_LEFT
    press.pressed = true
    root.push_input(press, true)
    await process_frame
    var release := InputEventMouseButton.new()
    release.position = point
    release.button_index = MOUSE_BUTTON_LEFT
    release.pressed = false
    root.push_input(release, true)
    await process_frame
    ui.sync_input()
    if ui.pop_action() != "enter_world":
        fail("Viewport Enter World click did not route authored action", ui)
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
