extends SceneTree

# Native paint and world-clear controls, not realm ingress or audible-output proof.
var failures := 0
var steps := 0
var cases := 0

func _initialize() -> void:
    run_fixture.call_deferred()

func check(value: bool, detail: String) -> bool:
    steps += 1
    if not value:
        failures += 1
        push_error("TalkingHead transition step %d: %s" % [steps, detail])
    return value

func paint_case(forever: bool, payload: String, output: String) -> bool:
    var prior := failures
    var skin := "forever" if forever else "modern"
    var fixture = ClassDB.instantiate("TalkingHeadFixture")
    if not check(fixture != null, "source705 consumer: " + skin):
        return false
    root.add_child(fixture)
    var error: String = fixture.initialize(forever, payload)
    check(error.is_empty(), "source705 initialize %s: %s" % [skin,error])
    var source: Dictionary = JSON.parse_string(payload)
    var lines: Array = source["conversations"][0]["lines"]
    var phases := [
        {"advance":0.625, "ms":625, "text":lines[0]["text"]["male_text"]},
        {"advance":0.25, "ms":875, "text":lines[1]["text"]["male_text"]},
        {"advance":0.25, "ms":1125, "text":lines[1]["text"]["male_text"]},
    ]
    for phase in phases:
        error = fixture.tick(phase["advance"])
        check(error.is_empty(), "source705 tick %s/%d: %s" % [skin,phase["ms"],error])
        await process_frame
        var text: Label = fixture.find_child("TalkingHeadText", true, false)
        check(text != null and text.text == phase["text"], "Lua250ms text callback %s/%d" % [skin,phase["ms"]])
        check(text != null and absf(text.modulate.a - 0.5) < 0.001, "source alpha callbacks %s/%d" % [skin,phase["ms"]])
        await RenderingServer.frame_post_draw
        var image := root.get_texture().get_image()
        check(image.get_size() == Vector2i(1920,1080), "original source705 frame: " + skin)
        var filename := "talking-head-transition-%s-%dms.png" % [skin,phase["ms"]]
        check(image.save_png(output.path_join(filename)) == OK, "source705 original PNG: " + skin)
    fixture.free()
    await process_frame
    if failures != prior:
        return false
    cases += 1
    print("CASE talking_head_transitions %s/705 PASS=true" % skin)
    return true

func world_case(forever: bool, payload: String) -> bool:
    var prior := failures
    var skin := "forever" if forever else "modern"
    var fixture = ClassDB.instantiate("TalkingHeadFixture")
    if not check(fixture != null, "world-clear native consumer: " + skin):
        return false
    root.add_child(fixture)
    var error: String = fixture.initialize(forever, payload)
    check(error.is_empty(), "world-clear initialize %s: %s" % [skin,error])
    var deadline := Time.get_ticks_msec() + 30000
    while Time.get_ticks_msec() < deadline:
        error = fixture.tick(0.0)
        if not error.is_empty() or fixture.voice_state().get("playing", false):
            break
        await process_frame
    check(error.is_empty(), "actual voice readiness %s: %s" % [skin,error])
    var player: AudioStreamPlayer = fixture.find_child("TalkingHeadVoice", true, false)
    check(is_instance_valid(player) and player.playing, "actual voice before world clear: " + skin)
    var playback: AudioStreamPlayback = player.get_stream_playback() if is_instance_valid(player) else null
    fixture.clear_world()
    error = fixture.tick(0.0)
    check(error.is_empty(), "production world-clear projection %s: %s" % [skin,error])
    var head: Control = fixture.find_child("TalkingHeadFrame", true, false)
    check(head != null and not head.is_visible_in_tree(), "world clear hides UI without source-close delay: " + skin)
    check(not is_instance_valid(player) or not player.playing, "world clear stops actual player immediately: " + skin)
    check(not fixture.voice_state().get("playing", false), "no active world-exit voice handle: " + skin)
    fixture.free()
    await process_frame
    if failures != prior:
        return false
    cases += 1
    # stop() retires asynchronously through AudioServer's mixer and main-thread cleanup.
    # Immediate UI/player stop was asserted above; process exit must observe retirement.
    var retirement_deadline := Time.get_ticks_msec() + 30000
    while playback != null and playback.get_reference_count() > 1 and Time.get_ticks_msec() < retirement_deadline:
        await process_frame
    check(playback != null and playback.get_reference_count() == 1, "audio server has retired playback before process exit: %s refs=%d" % [skin,playback.get_reference_count() if playback != null else -1])
    if failures != prior:
        return false
    print("CASE talking_head_transitions %s/world-clear PASS=true" % skin)
    return true

func run_fixture() -> void:
    root.size = Vector2i(1920,1080)
    create_timer(120.0).timeout.connect(func() -> void:
        push_error("TalkingHead transition fixture timed out")
        quit(1)
    )
    var output := OS.get_environment("GODOT_TALKING_HEAD_OUTPUT")
    if not check(not output.is_empty(), "explicit PNG output"):
        quit(1)
        return
    DirAccess.make_dir_recursive_absolute(output)
    var source705 := FileAccess.get_file_as_string("res://ui-model/tests/fixtures/conversation-705.json")
    var orator := FileAccess.get_file_as_string("res://ui-model/tests/fixtures/conversation-orator.json")
    if not check(not source705.is_empty() and not orator.is_empty(), "authentic payloads readable"):
        quit(1)
        return
    for forever in [false,true]:
        if not await paint_case(forever, source705, output) or not await world_case(forever, orator):
            quit(1)
            return
    check(cases == 4, "both skins/source705 and actual voice world-clear cases")
    print("TALKING_HEAD_TRANSITION_FIXTURE cases=%d steps=%d failures=%d" % [cases,steps,failures])
    if failures == 0:
        print("ALL_CASES_DONE %d/4" % cases)
    quit(1 if failures else 0)
