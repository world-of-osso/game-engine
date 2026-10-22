extends SceneTree

# Actual source voice/player and production state projection; no audible-output claim.
var failures := 0
var steps := 0
var cases := 0

func _initialize() -> void:
    run_fixture.call_deferred()

func check(value: bool, detail: String) -> bool:
    steps += 1
    if not value:
        failures += 1
        push_error("TalkingHead Close step %d: %s" % [steps, detail])
    return value

func run_case(forever: bool, payload: String) -> bool:
    var prior_failures := failures
    var skin := "forever" if forever else "modern"
    var fixture = ClassDB.instantiate("TalkingHeadFixture")
    if not check(fixture != null, "native consumer: " + skin):
        return false
    root.add_child(fixture)
    var error: String = fixture.initialize(forever, payload)
    check(error.is_empty(), "source setup %s: %s" % [skin, error])
    var deadline := Time.get_ticks_msec() + 30000
    while Time.get_ticks_msec() < deadline:
        error = fixture.tick(0.0)
        if not error.is_empty() or fixture.voice_state().get("playing", false):
            break
        await process_frame
    check(error.is_empty(), "source voice worker %s: %s" % [skin, error])
    var voice: Dictionary = fixture.voice_state()
    check(voice.get("playing", false) and voice.get("fdid", 0) == 5834729, "real source clip: " + skin)
    var player: AudioStreamPlayer = fixture.find_child("TalkingHeadVoice", true, false)
    check(is_instance_valid(player) and player.playing, "actual active voice node: " + skin)
    var initial_gain := player.volume_linear if is_instance_valid(player) else 0.0
    error = fixture.tick(0.75)
    check(error.is_empty(), "source opening settled: " + skin)
    var head: Control = fixture.find_child("TalkingHeadFrame", true, false)
    var text: Control = fixture.find_child("TalkingHeadText", true, false)
    var clear: Dictionary = JSON.parse_string(payload)
    clear["conversations"] = []
    error = fixture.receive(JSON.stringify(clear), 400)
    check(error.is_empty(), "authoritative source clear: " + skin)
    error = fixture.tick(0.0)
    check(error.is_empty(), "begin production Close: " + skin)
    check(is_instance_valid(player) and player.playing, "StopSound2000ms must fade, not cut off: " + skin)
    check(head != null and head.is_visible_in_tree(), "source close retains UI: " + skin)
    await create_timer(0.5).timeout
    error = fixture.tick(0.5)
    check(error.is_empty(), "half-second close tick: " + skin)
    check(text != null and is_equal_approx(text.modulate.a, 1.0), "source voice delays UI Close by1s: " + skin)
    var fading_gain := player.volume_linear if is_instance_valid(player) else -1.0
    check(fading_gain > 0.0 and fading_gain < initial_gain, "actual voice gain fades: " + skin)
    await create_timer(1.0).timeout
    error = fixture.tick(1.0)
    check(error.is_empty(), "one-and-half-second close tick: " + skin)
    check(head != null and head.is_visible_in_tree() and text != null and is_equal_approx(text.modulate.a, 0.5), "authored UI Close halfway at1.5s: " + skin)
    await create_timer(0.55).timeout
    error = fixture.tick(0.55)
    check(error.is_empty(), "close completion tick: " + skin)
    check(head != null and not head.is_visible_in_tree(), "source UI hidden after2s: " + skin)
    check(not is_instance_valid(player) or not player.playing, "source2s voice fade completes: " + skin)
    fixture.free()
    await process_frame
    if failures != prior_failures:
        return false
    cases += 1
    print("CASE talking_head_source_close %s PASS=true" % skin)
    return true

func run_fixture() -> void:
    root.size = Vector2i(1920,1080)
    create_timer(90.0).timeout.connect(func() -> void:
        push_error("TalkingHead source Close fixture timed out")
        quit(1)
    )
    var source := "res://ui-model/tests/fixtures/conversation-orator.json"
    if not check(FileAccess.file_exists(source), "authentic source payload exists"):
        quit(1)
        return
    var payload := FileAccess.get_file_as_string(source)
    check(not payload.is_empty(), "source payload is readable")
    for forever in [false, true]:
        if not await run_case(forever, payload):
            quit(1)
            return
    check(cases == 2, "both source Close cases completed")
    check(steps == 39, "exact checked-step contract before final assertion")
    print("TALKING_HEAD_CLOSE_FIXTURE cases=%d steps=%d expected_steps=40 failures=%d" % [cases,steps,failures])
    if failures == 0:
        print("ALL_CASES_DONE %d/2" % cases)
    quit(1 if failures else 0)
