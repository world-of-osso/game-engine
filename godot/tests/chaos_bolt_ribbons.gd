extends SceneTree

func _initialize() -> void:
	call_deferred("run_test")

func run_test() -> void:
	var fixture = ClassDB.instantiate("RibbonMaterialFixture")
	root.add_child(fixture)
	var error: String = fixture.spawn_chaos_bolt_ribbons()
	if not error.is_empty():
		push_error("Chaos Bolt ribbons: " + error)
		quit(1)
		return
	var strips := fixture.find_children("M2RibbonStrip*", "MeshInstance3D", true, false)
	if strips.is_empty():
		push_error("Chaos Bolt produced no ribbon strips")
		quit(1)
		return
	print("PASS Chaos Bolt ribbon strips: ", strips.size())
	quit(0)
