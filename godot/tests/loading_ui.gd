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
    var artwork_image = artwork.get_node_or_null("Parts/Part0") if artwork else null
    if artwork_image == null or artwork_image.texture == null:
        fail("Authored PNG artwork missing", ui)
        return
    var shell = ui.find_child("LoadingBarBackground", true, false)
    if shell == null:
        fail("Authored three-slice shell missing", ui)
        return
    if shell.size != Vector2(610, 32):
        fail("Loading shell bounds differ from authored 610x32", ui)
        return
    var expected_positions = [Vector2(0, 0), Vector2(25, 0), Vector2(585, 0)]
    var expected_sizes = [Vector2(25, 32), Vector2(560, 32), Vector2(25, 32)]
    for index in range(3):
        var part = shell.get_node_or_null("Parts/Part%d" % index)
        if part == null or not part is TextureRect or part.texture == null:
            fail("Loading shell part %d lacks native authored texture" % index, ui)
            return
        if part.position != expected_positions[index] or part.size != expected_sizes[index]:
            fail("Loading shell part %d has incorrect geometry" % index, ui)
            return
        if part.self_modulate != Color.WHITE or part.mouse_filter != Control.MOUSE_FILTER_IGNORE:
            fail("Loading shell part %d changes tint or intercepts input" % index, ui)
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
