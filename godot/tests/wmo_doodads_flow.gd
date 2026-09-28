extends SceneTree

# The Stockade instance portal is a WMO doodad: `sw_magicdistrict` (root FDID 321999)
# placed by azeroth_30_48's `_obj0` MODF places MODD 1112 (instanceportal.m2, FDID
# 197007) in its Jail01 doorway. The in-world object path must spawn it as a child of
# the WMO node at area trigger 101, WoW (-8761.85, 848.56, 87.81), engine
# (x, z, -y), with its MODD scale, within the in-world per-frame object budget. A
# WMO-only map's global WMO places its doodads the same way.

const MAGIC_DISTRICT := 321999
const TRIGGER := Vector3(-8761.85, 87.81, -848.56)
const PORTAL_SCALE := 1.3597486

func fail(message: String) -> void:
	push_error(message)
	quit(1)

func _initialize() -> void:
	if not ClassDB.class_exists("WowWmoPlacementProbe"):
		fail("WowWmoPlacementProbe not registered")
		return
	run.call_deferred()

func run() -> void:
	var probe = ClassDB.instantiate("WowWmoPlacementProbe")
	root.add_child(probe)
	var error: String = probe.load("azeroth", 30, 48, MAGIC_DISTRICT)
	if error != "":
		fail("Load failed: " + error)
		return
	var frames := 0
	var deadline := Time.get_ticks_msec() + 300000
	var state: Dictionary = probe.objects_state()
	while Time.get_ticks_msec() < deadline:
		await process_frame
		frames += 1
		state = probe.objects_state()
		if state.error != "" or state.get("map_error", "") != "" or state.get("terrain_errors", "") != "":
			fail("Probe failed: " + str(state))
			return
		if state.get("parsed", false) and state.spawned > 0 and state.pending == 0:
			break
	if state.get("pending", 1) != 0 or state.get("spawned", 0) != 1:
		fail("sw_magicdistrict did not finish spawning: " + str(state))
		return
	var wmo := probe.find_child("Wmo*", true, false) as Node3D
	if wmo == null:
		fail("No WMO node spawned")
		return
	var doodads := wmo.find_children("WmoDoodad*", "", false, false)
	print("sw_magicdistrict: doodads=%d failures=%d frames=%d" % [doodads.size(), state.failures, frames])
	# Incremental: 1177 default-set doodads cannot all spawn in one 8 ms frame.
	if frames < 3:
		fail("Doodads spawned in %d frames; the object budget was not applied" % frames)
		return
	var portal := wmo.get_node_or_null("WmoDoodad1112") as Node3D
	if portal == null:
		fail("MODD 1112 (instance portal) not spawned")
		return
	var world := portal.global_transform
	var scale := world.basis.get_scale()
	print("portal at %s scale %s" % [world.origin, scale])
	if world.origin.distance_to(TRIGGER) > 5.0:
		fail("Portal at %s, area trigger 101 at %s" % [world.origin, TRIGGER])
		return
	if absf(scale.x - PORTAL_SCALE) > 1e-3 or absf(scale.y - PORTAL_SCALE) > 1e-3 or absf(scale.z - PORTAL_SCALE) > 1e-3:
		fail("Portal scale %s, MODD scale %f" % [scale, PORTAL_SCALE])
		return
	# MODD 1112 has an identity rotation: model up is world up.
	if world.basis.y.normalized().dot(Vector3.UP) < 0.999:
		fail("Portal up axis %s is not world up" % world.basis.y)
		return
	if portal.find_children("*", "MeshInstance3D", true, false).is_empty():
		fail("Portal has no meshes")
		return
	# Every MODD entry of `Set_$DefaultGlobal` (0..1177) is referenced by some group's MODR.
	if doodads.size() != 1177 or state.failures != 0:
		fail("Expected all 1177 default-set doodads without failures; got %d, %d failures" % [doodads.size(), state.failures])
		return
	if not assert_scenery_distance(probe, doodads, portal):
		return
	if not assert_group_cull(probe, wmo, portal):
		return
	print("PASS: sw_magicdistrict places MODD 1112 instanceportal at area trigger 101 within the object budget")
	probe.free()
	run_global()

# The Stockade (`stormwindjail`) is a WMO-only map: its WDT global WMO (FDID 108631)
# places all 748 MODD entries of `Set_$DefaultGlobal`, each referenced by a group MODR.
func run_global() -> void:
	var probe = ClassDB.instantiate("WowWmoPlacementProbe")
	root.add_child(probe)
	var error: String = probe.load("stormwindjail", 32, 32, 0)
	if error != "":
		fail("Load failed: " + error)
		return
	var deadline := Time.get_ticks_msec() + 300000
	var state: Dictionary = probe.objects_state()
	var wmo: Node3D = null
	while Time.get_ticks_msec() < deadline:
		await process_frame
		state = probe.objects_state()
		if state.error != "" or state.get("map_error", "") != "" or state.get("terrain_errors", "") != "":
			fail("Probe failed: " + str(state))
			return
		wmo = probe.find_child("GlobalWmo108631", true, false) as Node3D
		if wmo != null and state.pending == 0:
			break
	if wmo == null or state.pending != 0:
		fail("Stockade global WMO did not finish spawning: " + str(state))
		return
	var doodads := wmo.find_children("WmoDoodad*", "", false, false)
	print("stormwindjail: doodads=%d failures=%d" % [doodads.size(), state.failures])
	if doodads.size() != 748 or state.failures != 0:
		fail("Expected all 748 Stockade doodads without failures; got %d, %d failures" % [doodads.size(), state.failures])
		return
	print("PASS: the Stockade global WMO places its 748 default-set doodads")
	quit(0)

# Retail gates WMO doodads by the ADT doodad scenery distance and by their groups'
# portal visibility: from a camera at the trigger (looking +X down the tunnel) the
# portal is drawn; the rest of the district is mostly hidden.
func assert_scenery_distance(probe: Node, doodads: Array, portal: Node3D) -> bool:
	var eye := TRIGGER + Vector3(0.0, 2.0, 0.0)
	cull_from(probe, eye, eye + Vector3(1.0, 0.0, 0.0))
	var shown := 0
	for doodad in doodads:
		shown += int(doodad.visible)
	var hidden := doodads.size() - shown
	print("cull from the trigger: shown=%d hidden=%d" % [shown, hidden])
	if not portal.visible or shown == 0 or hidden == 0:
		fail("The cull must draw the portal and near doodads and hide far ones: portal=%s shown=%d hidden=%d" % [portal.visible, shown, hidden])
		return false
	return true

# One in-world cull frame, 100 ms after the previous one.
func cull_from(probe: Node, eye: Vector3, target: Vector3) -> void:
	var camera := Camera3D.new()
	root.add_child(camera)
	camera.look_at_from_position(eye, target, Vector3.UP)
	probe.cull_from(camera, 100.0)
	camera.free()

func bone_pose(doodad: Node3D) -> Array:
	var skeleton := doodad.get_node("Skeleton3D") as Skeleton3D
	var pose := []
	for bone in skeleton.get_bone_count():
		pose.append([skeleton.get_bone_pose_position(bone), skeleton.get_bone_pose_rotation(bone), skeleton.get_bone_pose_scale(bone)])
	return pose

# The portal's bone pose after five cull frames from `eye` looking at `target`.
func pose_after_frames(probe: Node, portal: Node3D, eye: Vector3, target: Vector3) -> Array:
	var before := bone_pose(portal)
	for frame in 5:
		cull_from(probe, eye, target)
	return [before, bone_pose(portal)]

# A WMO doodad is drawn only while a group referencing it (Jail01, group 59, for
# MODD 1112) is drawn. 20 yd above the portal, looking away from the district, the
# portal is within every scenery distance but Jail01 is portal-culled, so the portal
# is hidden and its bones do not advance; at the trigger, inside Jail01 and facing
# it, it is drawn and animates.
func assert_group_cull(probe: Node, wmo: Node3D, portal: Node3D) -> bool:
	var jail := wmo.find_children("Group59_Batch*", "", false, false)
	if jail.is_empty():
		fail("Jail01 (group 59) has no batches")
		return false
	var above := portal.global_position + Vector3(0.0, 20.0, 0.0)
	var culled := pose_after_frames(probe, portal, above, above + Vector3(1.0, 0.0, 0.0))
	if jail[0].visible or portal.visible or culled[0] != culled[1]:
		fail("Above the district Jail01 drawn=%s, portal drawn=%s, animated=%s; expected all culled" % [jail[0].visible, portal.visible, culled[0] != culled[1]])
		return false
	var eye := TRIGGER + Vector3(0.0, 2.0, 0.0)
	var drawn := pose_after_frames(probe, portal, eye, portal.global_position + Vector3(0.0, 1.0, 0.0))
	if not jail[0].visible or not portal.visible or drawn[0] == drawn[1]:
		fail("At the trigger Jail01 drawn=%s, portal drawn=%s, animated=%s; expected all drawn" % [jail[0].visible, portal.visible, drawn[0] != drawn[1]])
		return false
	print("group cull: portal hidden and still with Jail01 above the district, drawn and animating at the trigger")
	return true
