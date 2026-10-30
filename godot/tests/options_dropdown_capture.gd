extends SceneTree
func _initialize() -> void:
    call_deferred("run")
func run() -> void:
    root.content_scale_size = Vector2i.ZERO
    root.size = Vector2i(1600,900)
    var output := OS.get_environment("GLUE_MENUS_CAPTURE_PATH")
    for forever in [true]:
        var ui = ClassDB.instantiate("RegistryUi")
        root.add_child(ui)
        var error = ui.call("show_buttonfit_service_preview","options:interface",forever)
        if error != "":
            fail(error)
            return
        for frame in range(120):
            await process_frame
            await RenderingServer.frame_post_draw
        var dropdown := ui.find_child("Choicestatus_text_display",true,false) as Control
        if not dropdown or dropdown.size != Vector2(220,25):
            fail("SettingsDropdownControlMixin width220 / WowStyle2 height25 missing")
            return
        var background := ui.find_child("Choicestatus_text_displayBackground",true,false) as Control
        if not background or background.mouse_filter != Control.MOUSE_FILTER_IGNORE:
            fail("Decorative dropdown Background must not intercept pointer events")
            return
        await click(ui,"Choicestatus_text_display")
        if not ui.find_child("OptionsChoicePopupstatus_text_display",true,false):
            fail("Dropdown click did not open radio popup")
            return
        await click(ui,"Choiceencounter_view")
        if ui.find_child("OptionsChoicePopupstatus_text_display",true,false) or not ui.find_child("OptionsChoicePopupencounter_view",true,false):
            fail("Only one Options menu may be open")
            return
        await click(ui,"Choicestatus_text_display")
        for frame in range(30):
            await process_frame
            await RenderingServer.frame_post_draw
        var name := "final-options-radio-%s.png" % ["forever" if forever else "modern"]
        if root.get_texture().get_image().save_png(output.path_join(name)) != OK:
            fail("Cannot save " + name)
            return
        await click(ui,"Choicestatus_text_display1Hit")
        if ui.call("pop_action") != "options_toggle:status_text_display:1" or ui.find_child("OptionsChoicePopupstatus_text_display",true,false):
            fail("Selection must forward existing action and close")
            return
        await click(ui,"Choicestatus_text_display")
        var escape := InputEventKey.new()
        escape.keycode = KEY_ESCAPE
        escape.pressed = true
        root.push_input(escape,true)
        await process_frame
        if ui.find_child("OptionsChoicePopupstatus_text_display",true,false) or not ui.find_child("OptionsRoot",true,false):
            fail("Escape must close popup, not Options")
            return
        await click(ui,"Choicestatus_text_display")
        await click_at(ui,Vector2(20,20))
        if ui.find_child("OptionsChoicePopupstatus_text_display",true,false):
            fail("Outside click must close popup")
            return
        print("PASS: native one-open/select/Escape/outside Options popup ","forever" if forever else "modern")
        ui.queue_free()
        await process_frame
    quit(0)
func click(ui: Node, name: String) -> void:
    var button := ui.find_child(name,true,false) as Control
    if not button:
        fail("Missing " + name)
        return
    await click_at(ui,button.get_global_rect().get_center())
func click_at(ui: Node, point: Vector2) -> void:
    for down in [true,false]:
        var event := InputEventMouseButton.new()
        event.position = point
        event.button_index = MOUSE_BUTTON_LEFT
        event.pressed = down
        root.push_input(event,true)
        await process_frame
    ui.call("sync_input")
    await process_frame
func fail(message: String) -> void:
    push_error(message)
    quit(1)
