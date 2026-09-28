extends SceneTree

# MODF flag 0x80 (MWDS): `azeroth_31_48_obj0` places the Stormwind warrior district
# house `9sw_warriordistrict_house2` (root FDID 3389421, uniqueId 5478984) with
# MWDR range 0 = doodad sets 1 and 2 ("Training Hall", MODD 1..=253). The in-world
# object path must spawn that set's doodads, and not `$DefaultGlobal`'s MODD 0,
# which the MWDS sets replace (WebWowViewerCpp `setActiveDoodadFromMWDR`).

const HOUSE := 3389421

func fail(message: String) -> void:
	push_error(message)
	quit(1)

func _initialize() -> void:
	run.call_deferred()

func run() -> void:
	var probe = ClassDB.instantiate("WowWmoPlacementProbe")
	root.add_child(probe)
	var error: String = probe.load("azeroth", 31, 48, HOUSE)
	if error != "":
		fail("Load failed: " + error)
		return
	var deadline := Time.get_ticks_msec() + 300000
	var state: Dictionary = probe.objects_state()
	while Time.get_ticks_msec() < deadline:
		await process_frame
		state = probe.objects_state()
		if state.error != "" or state.get("map_error", "") != "" or state.get("terrain_errors", "") != "":
			fail("Probe failed: " + str(state))
			return
		if state.get("parsed", false) and state.spawned > 0 and state.pending == 0:
			break
	var wmo := probe.find_child("Wmo5478984", true, false) as Node3D
	if wmo == null or state.pending != 0:
		fail("Warrior district house did not finish spawning: " + str(state))
		return
	var doodads := wmo.find_children("WmoDoodad*", "", false, false)
	var indices := []
	for doodad in doodads:
		indices.append(int(String(doodad.name).trim_prefix("WmoDoodad")))
	indices.sort()
	print("house 5478984: doodads=%d failures=%d first=%s last=%s" % [doodads.size(), state.failures, indices.slice(0, 1), indices.slice(-1)])
	if doodads.size() < 100 or indices.has(0) or indices.min() < 1 or indices.max() > 253 or state.failures != 0:
		fail("Expected the Training Hall set (MODD 1..=253) without MODD 0; got %d doodads %s, %d failures" % [doodads.size(), indices, state.failures])
		return
	print("PASS: the MWDS placement spawns its listed doodad sets")
	quit(0)
