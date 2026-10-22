extends SceneTree

# Retail70448 XML + atlas604; effective native transforms, not model/camera paint.
const REGIONS := [
    {"name":"TalkingHeadGlowTopBar", "size":Vector2(81,23), "position":Vector2(36,5.5), "pivot":Vector2(40.5,11.5), "side":false},
    {"name":"TalkingHeadGlowLeftBar", "size":Vector2(23,86), "position":Vector2(4.5,9.5), "pivot":Vector2(11.5,0), "side":true},
    {"name":"TalkingHeadGlowRightBar", "size":Vector2(23,86), "position":Vector2(125.5,9.5), "pivot":Vector2(11.5,0), "side":true},
    {"name":"TalkingHeadSheen", "size":Vector2(262,34), "position":Vector2(102,19), "pivot":Vector2(0,17), "side":false},
    {"name":"TalkingHeadTextSheen", "size":Vector2(251,71), "position":Vector2(102,45), "pivot":Vector2(0,35.5), "side":false},
]
const PHASES := [
    {"ms":200, "advance":0.0, "alpha":[0.14,0.01,0.01,0.01,0.01], "dy":0.0, "scales":[Vector2(0.5,1),Vector2.ONE,Vector2.ONE,Vector2.ONE,Vector2.ONE], "started":[true,false,false,false,false]},
    {"ms":750, "advance":0.55, "alpha":[0.21,0.7,0.7,0.35,0.0], "dy":5.0, "scales":[Vector2(1.5,1),Vector2(1,1.1285714286),Vector2(1,1.1285714286),Vector2.ONE,Vector2(0.25,1)], "started":[true,true,true,true,true]},
    {"ms":1000, "advance":0.25, "alpha":[0.0,0.28,0.56,0.7,0.35], "dy":8.125, "scales":[Vector2(1.5,1),Vector2(1,1.5214285714),Vector2(1,1.5214285714),Vector2.ONE,Vector2.ONE], "started":[true,true,true,true,true]},
    {"ms":1500, "advance":0.5, "alpha":[0.0,0.0,0.0,0.0,0.35], "dy":10.0, "scales":[Vector2(1.5,1),Vector2(1,1.6),Vector2(1,1.6),Vector2.ONE,Vector2.ONE], "started":[true,true,true,true,true]},
    {"ms":1750, "advance":0.25, "alpha":[0.0,0.0,0.0,0.0,0.0], "dy":10.0, "scales":[Vector2.ONE,Vector2.ONE,Vector2.ONE,Vector2.ONE,Vector2.ONE], "started":[true,true,true,true,true]},
]
var failures := 0
var steps := 0
var cases := 0
var checked_regions := 0
var captured_phases := 0

func _initialize() -> void:
    run_fixture.call_deferred()

func check(value: bool, detail: String) -> bool:
    steps += 1
    if not value:
        failures += 1
        push_error("TalkingHead glow step %d: %s" % [steps,detail])
    return value

func check_region(fixture: Node, phase: Dictionary, index: int, label: String) -> bool:
    var prior := failures
    var region: Dictionary = REGIONS[index]
    var name: String = region["name"]
    var frame: Control = fixture.find_child(name, true, false)
    if not check(frame != null, "source glow exists %s %s" % [label,name]):
        return false
    check(frame.size.is_equal_approx(region["size"]), "source atlas size %s %s" % [label,name])
    var alpha: float = phase["alpha"][index]
    check(absf(frame.modulate.a - alpha) < 0.001, "source alpha %s %s" % [label,name])
    if alpha > 0.0:
        var position: Vector2 = region["position"]
        if region["side"]:
            position.y += float(phase["dy"])
        check(frame.position.is_equal_approx(position), "source anchor/translation %s %s" % [label,name])
        if phase["started"][index]:
            check(frame.scale.is_equal_approx(phase["scales"][index]), "source axis scale %s %s" % [label,name])
            check(frame.pivot_offset.is_equal_approx(region["pivot"]), "source origin %s %s" % [label,name])
    checked_regions += 1
    return failures == prior

func run_case(forever: bool, payload: String, output: String) -> bool:
    var skin := "forever" if forever else "modern"
    var fixture = ClassDB.instantiate("TalkingHeadFixture")
    if not check(fixture != null, "native source consumer: " + skin):
        return false
    root.add_child(fixture)
    var error: String = fixture.initialize_at_elapsed(forever, payload, 200)
    if not check(error.is_empty(), "source sample %s: %s" % [skin,error]):
        fixture.free()
        return false
    for frame in range(30):
        error = fixture.tick(0.0)
        if not error.is_empty():
            break
        await process_frame
    if not check(error.is_empty(), "source frame readiness %s: %s" % [skin,error]):
        fixture.free()
        return false
    for phase in PHASES:
        error = fixture.tick(float(phase["advance"]))
        var elapsed: int = phase["ms"]
        var label := "%s/%dms" % [skin,elapsed]
        if not check(error.is_empty(), "source phase %s: %s" % [label,error]):
            fixture.free()
            return false
        await RenderingServer.frame_post_draw
        for index in range(REGIONS.size()):
            if not check_region(fixture, phase, index, label):
                fixture.free()
                return false
        var image := root.get_texture().get_image()
        if not check(image.get_size() == Vector2i(1920,1080), "native1920x1080: " + label):
            fixture.free()
            return false
        var filename := "talking-head-glows-%s-%dms.png" % [skin,elapsed]
        if not check(image.save_png(output.path_join(filename)) == OK, "original PNG: " + label):
            fixture.free()
            return false
        captured_phases += 1
    fixture.free()
    await process_frame
    cases += 1
    print("CASE talking_head_glows %s PASS=true" % skin)
    return true

func run_fixture() -> void:
    root.size = Vector2i(1920,1080)
    create_timer(90.0).timeout.connect(func() -> void:
        push_error("TalkingHead glow fixture timed out")
        quit(1)
    )
    var source := "res://ui-model/tests/fixtures/conversation-orator.json"
    if not check(FileAccess.file_exists(source), "authentic source payload exists"):
        quit(1)
        return
    var output := OS.get_environment("GODOT_TALKING_HEAD_OUTPUT")
    if not check(not output.is_empty(), "explicit PNG output directory"):
        quit(1)
        return
    DirAccess.make_dir_recursive_absolute(output)
    var payload := FileAccess.get_file_as_string(source)
    for forever in [false,true]:
        if not await run_case(forever, payload, output):
            quit(1)
            return
    check(cases == 2 and captured_phases == 10 and checked_regions == 50, "both skins/five source phases/five glow regions complete")
    print("TALKING_HEAD_GLOW_FIXTURE cases=%d phases=%d regions=%d steps=%d failures=%d" % [cases,captured_phases,checked_regions,steps,failures])
    if failures == 0:
        print("ALL_CASES_DONE %d/2" % cases)
    quit(1 if failures else 0)
