extends SceneTree

# Authored loading screen: art/shell/progress projection, full fit at 1280x720 and 1920x1080,
# shared default zone/tip text, and bar easing toward readiness instead of jumping.

const ELEMENTS := [
    "LoadingLogo",
    "LoadingZoneText",
    "LoadingBarBackground",
    "LoadingStatusText",
    "LoadingProgressText",
    "LoadingTipText",
]

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
    if ui.frame_text("LoadingZoneText") != "Entering Elwynn Forest":
        fail("Zone text must default to the shared zone line: '%s'" % ui.frame_text("LoadingZoneText"), ui)
        return
    if not ui.frame_text("LoadingTipText").begins_with("Tip: "):
        fail("Tip text must default to the shared tip line", ui)
        return

    var problem = await fit_problem(ui, Vector2(1280, 720))
    if problem != "":
        fail(problem, ui)
        return
    await capture("GODOT_LOADING_CAPTURE_720")
    problem = await fit_problem(ui, Vector2(1920, 1080))
    if problem != "":
        fail(problem, ui)
        return
    await capture("GODOT_LOADING_CAPTURE_1080")

    problem = eased_progress_problem(ui)
    if problem != "":
        fail(problem, ui)
        return
    print("PASS: loading art/shell, fit at 1280x720 and 1920x1080, default zone/tip, eased progress")
    ui.free()
    quit(0)

func fit_problem(ui, size: Vector2) -> String:
    root.size = Vector2i(size)
    var error = ui.sync_input()
    if error != "":
        return error
    for frame in range(2):
        await process_frame
    var viewport = Rect2(Vector2.ZERO, size)
    var rects = {}
    for name in ELEMENTS:
        var node = ui.find_child(name, true, false)
        if node == null or not node.is_visible_in_tree():
            return "%s missing at %s" % [name, size]
        var rect: Rect2 = node.get_global_rect()
        if not viewport.encloses(rect):
            return "%s %s escapes viewport %s" % [name, rect, size]
        rects[name] = rect
    var art: Rect2 = ui.find_child("LoadingArtwork", true, false).get_global_rect()
    if art.position.y != 0.0 or art.size.y != size.y:
        return "Artwork %s must fill viewport height %s" % [art, size.y]
    var order = ["LoadingLogo", "LoadingZoneText", "LoadingBarBackground", "LoadingTipText"]
    for index in range(order.size() - 1):
        if rects[order[index]].end.y > rects[order[index + 1]].position.y:
            return "%s overlaps %s at %s" % [order[index], order[index + 1], size]
    var bar: Rect2 = rects["LoadingBarBackground"]
    for inside in ["LoadingStatusText", "LoadingProgressText"]:
        var rect: Rect2 = rects[inside]
        if rect.position.y < bar.position.y or rect.end.y > bar.end.y + 1.0:
            return "%s %s must sit inside the bar %s" % [inside, rect, bar]
    return ""

# Displayed progress eases at the shared 6%/s rate toward readiness without overshoot.
func eased_progress_problem(ui) -> String:
    var steps = [[80, 1.0, "6%"], [80, 1.0, "12%"], [80, 20.0, "80%"], [80, 1.0, "80%"], [100, 0.5, "83%"]]
    for step in steps:
        var error = ui.advance_loading_progress(step[0], "Loading terrain", step[1])
        if error != "":
            return error
        var shown = ui.frame_text("LoadingProgressText")
        if shown != step[2]:
            return "Progress toward %d after %.1fs shows %s, expected %s" % [step[0], step[1], shown, step[2]]
    if ui.frame_text("LoadingStatusText") != "Loading terrain":
        return "Status text must follow readiness"
    if ui.frame_text("LoadingZoneText") == "" or ui.frame_text("LoadingTipText") == "":
        return "Zone/tip text must survive readiness updates"
    return ""

func capture(variable: String) -> void:
    var output = OS.get_environment(variable)
    if output.is_empty():
        return
    for frame in range(3):
        await process_frame
        await RenderingServer.frame_post_draw
    var image = root.get_texture().get_image()
    if image == null or image.is_empty() or image.save_png(output) != OK:
        push_error("Loading capture failed for " + output)

func fail(message, ui):
    printerr(message)
    ui.free()
    quit(1)
