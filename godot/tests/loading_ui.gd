extends SceneTree

func _initialize():
    call_deferred("run")

func run():
    root.size = Vector2i(1280, 720)
    var ui = ClassDB.instantiate("RegistryUi")
    root.add_child(ui)
    if not ui.has_method("show_loading"):
        fail("Authored loading screen not projectable", ui)
        return
    var error = ui.show_loading()
    if error != "":
        fail(error, ui)
        return
    await process_frame
    var artwork = ui.find_child("LoadingArtwork", true, false)
    if artwork == null or artwork.texture == null:
        fail("Authored PNG artwork missing", ui)
        return
    var shell = ui.find_child("LoadingBarBackground", true, false)
    if shell == null:
        fail("Authored three-slice shell missing", ui)
        return
    if ui.frame_text("LoadingProgressText") != "0%":
        fail("Loading cannot complete before world readiness", ui)
        return
    print("PASS: authored loading PNG artwork, shell and initial progress")
    ui.free()
    quit(0)

func fail(message, ui):
    printerr(message)
    ui.free()
    quit(1)
