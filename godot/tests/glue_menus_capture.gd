extends SceneTree
# Offline production projection, both skins. No accounts, session or mutable cache.
func _initialize() -> void:
    call_deferred("capture_cases")

func capture_cases() -> void:
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
            var image := root.get_texture().get_image()
            var name := "%s-%s-%s.png" % [stage, case.replace(":", "-"), "forever" if forever else "modern"]
            if image.save_png(output.path_join(name)) != OK:
                push_error("Cannot save " + name)
                quit(1)
                return
            print("CAPTURE: ", name)
            ui.queue_free()
            await process_frame
    quit(0)
