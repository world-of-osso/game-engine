extends SceneTree

func _initialize():
    call_deferred("run")

func run():
    root.size = Vector2i(1920, 1080)
    var ui = ClassDB.instantiate("RegistryUi")
    root.add_child(ui)
    if not ui.has_method("show_character_create"):
        fail("RegistryUi cannot project authored character creation", ui)
        return
    var error = ui.show_character_create()
    if error != "":
        fail(error, ui)
        return
    await process_frame
    var canvas = ui.get_node("RegistryCanvas")
    var screen = canvas.find_child("CharCreateRoot", true, false)
    var alliance = canvas.find_child("AllianceRaces", true, false)
    var horde = canvas.find_child("HordeRaces", true, false)
    var alliance_label = canvas.find_child("AllianceLabel", true, false)
    var horde_label = canvas.find_child("HordeLabel", true, false)
    var race = canvas.find_child("Race_1", true, false)
    var klass = canvas.find_child("Class_2", true, false)
    var back = canvas.find_child("CharCreateBack", true, false)
    var next = canvas.find_child("CharCreateNext", true, false)
    if not screen is Control or not alliance is Control or not horde is Control or not race is Button or not klass is Button or not back is Button or not next is Button:
        fail("Missing authored faction, race, class, or navigation controls", ui)
        return
    if not alliance_label is Label or alliance_label.text != "Alliance" or not horde_label is Label or horde_label.text != "Horde":
        fail("Authored faction labels missing", ui)
        return
    if not screen.size.is_equal_approx(Vector2(root.size)) or not race.size.is_equal_approx(Vector2(79, 79)) or not back.size.is_equal_approx(Vector2(250, 66)) or not next.size.is_equal_approx(Vector2(250, 66)):
        fail("Authored root/race/navigation dimensions changed", ui)
        return
    # Retail ClassName: GameFontNormalMed2 (Friz Quadrata 13) in an 85x50 box that
    # word-wraps (Blizzard_CharacterCreate.xml:72-77), so "Demon Hunter" takes two lines.
    var dh_label = canvas.find_child("Class_12_Label", true, false)
    var dh_class = canvas.find_child("Class_12", true, false)
    print("FIXTURE DH_LABEL size=%s lines=%d font=%s rect=%s button=%s" % [dh_label.size, dh_label.get_line_count(), dh_label.get_theme_font_size("font_size"), dh_label.get_global_rect(), dh_class.get_global_rect()])
    if dh_label.get_line_count() != 2 or dh_label.size.x > 85.5:
        fail("Demon Hunter class label must wrap into two lines within 85 px: " + str(dh_label.size), ui)
        return
    var viewport_rect := Rect2(Vector2.ZERO, Vector2(root.size))
    var action_rect: Rect2 = next.get_global_rect()
    if not viewport_rect.encloses(action_rect):
        fail("Customize action outside viewport: " + str(action_rect), ui)
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
    if ui.pop_action() != "next_mode":
        fail("Viewport Customize click did not route authored action", ui)
        return
    back.emit_signal("pressed")
    if ui.pop_action() != "back":
        fail("Authored Back action not routed", ui)
        return
    root.size = Vector2i(1600, 900)
    await process_frame
    if ui.sync_input() != "":
        fail("Character creation failed to resize", ui)
        return
    if not screen.size.is_equal_approx(Vector2(1600, 900)):
        fail("Authored root did not follow viewport resize: " + str(screen.size), ui)
        return
    if not Rect2(Vector2.ZERO, Vector2(root.size)).encloses(next.get_global_rect()):
        fail("Customize action outside resized viewport", ui)
        return
    ui.free()
    print("PASS: authored race/class creation projection and viewport navigation")
    quit(0)

func fail(message, ui):
    printerr(message)
    ui.free()
    quit(1)
