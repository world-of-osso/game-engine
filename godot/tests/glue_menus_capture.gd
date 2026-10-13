extends SceneTree
# Offline production projection, both skins. No accounts, session or mutable cache.
func _initialize() -> void:
    call_deferred("capture_cases")

func capture_cases() -> void:
    root.content_scale_size = Vector2i.ZERO
    root.size = Vector2i(1600, 900)
    var output := OS.get_environment("GLUE_MENUS_CAPTURE_PATH")
    var stage := OS.get_environment("GLUE_MENUS_CAPTURE_STAGE")
    if output.is_empty() or stage.is_empty():
        push_error("Set GLUE_MENUS_CAPTURE_PATH and GLUE_MENUS_CAPTURE_STAGE")
        quit(1)
        return
    for forever in [false, true]:
        for case in ["nav", "delete", "campsite", "menu", "options:graphics", "options:keybindings"]:
            var ui = ClassDB.instantiate("RegistryUi")
            root.add_child(ui)
            var error
            if case in ["nav", "delete", "campsite"]:
                error = ui.call("show_glue_menus_preview", case, forever)
            else:
                error = ui.call("show_buttonfit_service_preview", case, forever)
            if error != "":
                push_error(error)
                quit(1)
                return
            for frame in range(120):
                await process_frame
                await RenderingServer.frame_post_draw
            if case == "nav":
                for name in ["CharSelectTopNav", "CharSelectTopNavTabs", "CharSelectModeTab", "CharSelectMenuTab", "CharSelectCampsitesTab", "CharSelectModeTabNormal"]:
                    var control := ui.find_child(name, true, false) as Control
                    print("GEOMETRY: ", name, " ", control.get_global_rect() if control else "missing")
            var image := root.get_texture().get_image()
            var name := "%s-%s-%s.png" % [stage, case.replace(":", "-"), "forever" if forever else "modern"]
            if image.save_png(output.path_join(name)) != OK:
                push_error("Cannot save " + name)
                quit(1)
                return
            print("CAPTURE: ", name)
            if stage.begins_with("final") and not await verify_geometry_and_click(ui, case):
                quit(1)
                return
            ui.queue_free()
            await process_frame
    quit(0)

func verify_geometry_and_click(ui: Node, case: String) -> bool:
    var frame_name: String
    var expected_size: Vector2
    var button_name: String
    var action: String
    match case:
        "nav":
            frame_name = "CharSelectTopNav"
            expected_size = Vector2(1600, 55)
            button_name = "CharSelectMenuTab"
            action = "menu"
        "delete":
            frame_name = "DeleteCharacterDialog"
            expected_size = Vector2(512, 256)
            button_name = "DeleteCharacterConfirmButton"
            action = "confirm_delete_char"
        "campsite":
            frame_name = "CampsitePanel"
            expected_size = Vector2(703, 595)
            button_name = "CampsiteScene_2"
            action = "select_campsite:2"
        "menu":
            frame_name = "GameMenuPanel"
            expected_size = Vector2(260, 358)
            button_name = "MenuBtnOptions"
            action = "menu_options"
        _:
            frame_name = "OptionsRoot"
            expected_size = Vector2(920, 724)
            button_name = "OptionsTabsound"
            action = "options_category:sound"
    var control := ui.find_child(frame_name, true, false) as Control
    var button := ui.find_child(button_name, true, false) as Control
    if not control or not button or not control.size.is_equal_approx(expected_size):
        push_error("XML geometry mismatch: " + frame_name + " " + str(control.size if control else "missing"))
        return false
    if frame_name != "CharSelectTopNav" and not Rect2(Vector2.ZERO, Vector2(root.size)).encloses(control.get_global_rect()):
        push_error("Source frame outside capture viewport: " + frame_name)
        return false
    var point := button.get_global_rect().get_center()
    for down in [true, false]:
        var event := InputEventMouseButton.new()
        event.position = point
        event.button_index = MOUSE_BUTTON_LEFT
        event.pressed = down
        root.push_input(event, true)
        await process_frame
    ui.call("sync_input")
    var observed: String = ui.call("pop_action")
    if observed != action:
        push_error("Native action mismatch: " + button_name + " " + observed)
        return false
    print("PASS: XML geometry and native click ", frame_name, " ", action)
    return true
