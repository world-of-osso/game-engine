extends SceneTree

# Sampled authentic payload, not end-to-end ingress/navigation or audible-output proof.
# Model painting is excluded under the lead-approved named source projection gap.
var failures := 0
var steps := 0
var cases := 0

func _initialize() -> void:
    run_fixture.call_deferred()

func check(value: bool, detail: String) -> bool:
    steps += 1
    if not value:
        failures += 1
        push_error("TalkingHead step %d: %s" % [steps, detail])
    return value

func click_at(position: Vector2, button: MouseButton) -> void:
    var motion := InputEventMouseMotion.new()
    motion.position = position
    motion.global_position = position
    Input.parse_input_event(motion)
    await process_frame
    for pressed in [true, false]:
        var event := InputEventMouseButton.new()
        event.position = position
        event.global_position = position
        event.button_index = button
        event.button_mask = (1 << (button - 1)) if pressed else 0
        event.pressed = pressed
        Input.parse_input_event(event)
        await process_frame

func run_case(forever: bool, close_method: String, payload: String, output: String) -> bool:
    var prior_failures := failures
    var skin := "forever" if forever else "modern"
    var label := skin + "/" + close_method
    var fixture = ClassDB.instantiate("TalkingHeadFixture")
    if not check(fixture != null, "native consumer: " + label):
        return false
    root.add_child(fixture)
    var error: String = fixture.initialize(forever, payload)
    check(error.is_empty(), "authentic source state/UI %s: %s" % [label,error])
    for frame in range(30):
        error = fixture.tick(0.0)
        check(error.is_empty(), "sampled native frame %s: %s" % [label,error])
        await process_frame
    var host: Control = fixture.find_child("TalkingHeadModel", true, false)
    check(host != null, "authored model host: " + label)
    if host != null:
        check(host.size.is_equal_approx(Vector2(115,115)), "source115x115 host: " + label)
    var text: Label = fixture.find_child("TalkingHeadText", true, false)
    check(text != null, "source dialogue: " + label)
    if text != null:
        check(text.get_line_count() > 1, "dialogue wraps: " + label)
        check(text.size.x <= 378.01, "source570-150-42 dialogue allocation: " + label)
    var camera: Dictionary = fixture.camera_state()
    check(camera.get("model_gap", "") == "TALKING_HEAD_MODEL_PROJECTION_UNSOURCED", "explicit projection gap: " + label)
    check(camera.get("source_ui_camera", 0) == 1751 and camera.get("source_flags", -1) == 0, "source camera/raw flags: " + label)
    var model_position: Vector3 = camera.get("model_position", Vector3.ZERO)
    var angles: Vector3 = camera.get("yaw_pitch_roll", Vector3.ZERO)
    check(model_position.distance_to(Vector3(2.82894468,-0.0669228658,-2.48507595)) < 0.000001, "model translation, not fitted eye: " + label)
    check(angles.distance_to(Vector3(0.220450103,0,0)) < 0.000001, "LookAt as yaw/pitch/roll: " + label)
    check(camera.get("camera_position", Vector3.ZERO) == Vector3(4,0,0) and camera.get("camera_target", Vector3.ONE) == Vector3.ZERO, "fixed camera-space4 to target0: " + label)
    await RenderingServer.frame_post_draw
    var image := root.get_texture().get_image()
    check(image.get_size() == Vector2i(1920,1080), "native1920x1080: " + label)
    var filename := "talking-head-%s-%s.png" % [skin, close_method]
    check(image.save_png(output.path_join(filename)) == OK, "unscaled original PNG: " + label)
    # Worker/CASC readiness follows wall time, not renderer throughput.
    var voice_deadline := Time.get_ticks_msec() + 30000
    error = ""
    while Time.get_ticks_msec() < voice_deadline:
        if fixture.voice_state().get("playing", false):
            break
        error = fixture.tick(0.0)
        if not error.is_empty():
            break
        await process_frame
    check(error.is_empty(), "voice worker %s: %s" % [label,error])
    var voice: Dictionary = fixture.voice_state()
    check(voice.get("playing", false), "native playback, not audible-output proof: " + label)
    check(voice.get("kit", 0) == 250774 and voice.get("fdid", 0) == 5834729, "source kit/clip: " + label)
    check(voice.get("offset_ms", -1) == 750, "source elapsed750ms seek: " + label)
    var close: Control = fixture.find_child("TalkingHeadCloseButton", true, false)
    if close_method == "button":
        if close != null:
            await click_at(close.get_global_rect().get_center(), MOUSE_BUTTON_LEFT)
    else:
        if host != null:
            await click_at(host.get_global_rect().get_center(), MOUSE_BUTTON_RIGHT)
    error = fixture.tick(0.0)
    check(error.is_empty(), "native input consumption %s: %s" % [label,error])
    check(not fixture.voice_state().get("playing", false), "native local close stops voice: " + label)
    fixture.free()
    await process_frame
    if failures != prior_failures:
        return false
    cases += 1
    print("CASE talking_head %s PASS=true GAP=TALKING_HEAD_MODEL_PROJECTION_UNSOURCED" % label)
    return true

func run_fixture() -> void:
    root.size = Vector2i(1920,1080)
    create_timer(90.0).timeout.connect(func() -> void:
        push_error("TalkingHead native fixture timed out")
        quit(1)
    )
    var source := "res://ui-model/tests/fixtures/conversation-orator.json"
    check(FileAccess.file_exists(source), "authentic source payload exists")
    var payload := FileAccess.get_file_as_string(source)
    var output := OS.get_environment("GODOT_TALKING_HEAD_OUTPUT")
    if not check(not output.is_empty(), "explicit PNG output directory"):
        quit(1)
        return
    DirAccess.make_dir_recursive_absolute(output)
    for forever in [false, true]:
        for close_method in ["button", "right"]:
            if not await run_case(forever, close_method, payload, output):
                quit(1)
                return
    check(cases == 4, "all four skin/input cases completed")
    check(steps == 203, "exact checked-step contract before final assertion")
    print("TALKING_HEAD_FIXTURE cases=%d expected_cases=4 steps=%d expected_steps=204 failures=%d" % [cases,steps,failures])
    if failures == 0:
        print("ALL_CASES_DONE %d/4" % cases)
        print("PASS talking_head_native_both_skins chrome_text_voice_input_only MODEL_GAP=TALKING_HEAD_MODEL_PROJECTION_UNSOURCED")
    quit(1 if failures else 0)
