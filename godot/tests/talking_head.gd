extends SceneTree

# Sampled native source-payload presentation, not end-to-end ingress/navigation.
# Source server lifetime and real UDP relay have separate behavioral tests.
var failures := 0
var steps := 0

func _initialize() -> void:
    run_fixture.call_deferred()

func check(value: bool, detail: String) -> bool:
    steps += 1
    if not value:
        failures += 1
        push_error("TalkingHead step %d: %s" % [steps, detail])
    return value

func run_fixture() -> void:
    root.size = Vector2i(1920, 1080)
    create_timer(90.0).timeout.connect(func() -> void:
        push_error("TalkingHead native fixture timed out")
        quit(1)
    )
    var source := "res://ui-model/tests/fixtures/conversation-orator.json"
    check(FileAccess.file_exists(source), "authentic source fixture exists")
    var payload := FileAccess.get_file_as_string(source)
    var output := OS.get_environment("GODOT_TALKING_HEAD_OUTPUT")
    if not check(not output.is_empty(), "explicit native PNG output directory"):
        quit(1)
        return
    DirAccess.make_dir_recursive_absolute(output)
    for forever in [false, true]:
        var skin := "forever" if forever else "modern"
        var fixture = ClassDB.instantiate("TalkingHeadFixture")
        if not check(fixture != null, "native TalkingHead consumer class"):
            quit(1)
            return
        root.add_child(fixture)
        check(fixture.initialize(forever, payload).is_empty(), "actual source state and authored UI: " + skin)
        for frame in range(30):
            check(fixture.tick(0.0).is_empty(), "sampled native frame consumer: " + skin)
            await process_frame
        var host: Control = fixture.find_child("TalkingHeadModel", true, false)
        check(host != null, "source115x115 model host: " + skin)
        if host != null:
            check(host.size.is_equal_approx(Vector2(115,115)), "native authored model host size: " + skin)
        var text: Label = fixture.find_child("TalkingHeadText", true, false)
        check(text != null, "native source dialogue label: " + skin)
        if text != null:
            check(text.get_line_count() > 1, "actual source dialogue wraps within authored text allocation: " + skin)
            check(text.size.x <= 378.01, "dialogue stays inside source570-150-42 width: " + skin)
        await RenderingServer.frame_post_draw
        var image := root.get_texture().get_image()
        check(image.get_size() == Vector2i(1920,1080), "native1920x1080 image: " + skin)
        check(image.save_png(output.path_join("talking-head-" + skin + ".png")) == OK, "save unscaled original PNG: " + skin)
        check(fixture.model_ready(), "actual source display116692 and camera1751 painted: " + skin)
        fixture.free()
        await process_frame
    print("TALKING_HEAD_FIXTURE steps=%d failures=%d" % [steps, failures])
    if failures == 0:
        print("PASS talking_head_native_both_skins")
    quit(1 if failures else 0)
