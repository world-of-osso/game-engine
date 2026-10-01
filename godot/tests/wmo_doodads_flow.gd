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
# Along the Stockade cell block from the fixture spawn.
const STOCKADE_LOOK := Vector3(1.0, 0.0, 0.0)

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
	if not assert_material_animation(probe, wmo):
		return
	if not assert_portal_particles(probe, portal):
		return
	if not await assert_unculled_static_models():
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
	if not assert_global_portal_cull(probe, wmo, doodads):
		return
	if not assert_interior_doodad_light(wmo):
		return
	quit(0)

# Stockade MODD 100 (flags 0x2, colour (77, 78, 86), interior groups): every batch
# is lit as interior with the MODD colour as direct light and the MOHD ambient
# (25, 25, 25), as `wmo::doodad_light` derives from WebWowViewerCpp.
func assert_interior_doodad_light(wmo: Node3D) -> bool:
	var doodad := wmo.get_node("WmoDoodad100") as Node3D
	var direct := Vector3(77.0, 78.0, 86.0) / 255.0
	var ambient := Vector3(25.0, 25.0, 25.0) / 255.0
	var batches := doodad.find_children("*", "MeshInstance3D", true, false)
	for mesh in batches:
		var material := (mesh as MeshInstance3D).get_active_material(0) as ShaderMaterial
		var blend = material.get_shader_parameter("exterior_blend")
		var lit = material.get_shader_parameter("interior_direct")
		var amb = material.get_shader_parameter("interior_ambient")
		if blend != 0.0 or lit == null or not lit.is_equal_approx(direct) or amb == null or not amb.is_equal_approx(ambient):
			fail("MODD 100 %s: exterior_blend %s, interior_direct %s, interior_ambient %s" % [mesh.name, blend, lit, amb])
			return false
	print("PASS: Stockade MODD 100's %d batches take its interior light" % batches.size())
	return true

# The global WMO is portal-culled like ADT WMOs: from the owned fixture's Stockade
# spawn (engine (103, -34.5, -76), 2 yd up) only the groups its portals reach through
# the view are drawn. The scenery distance does not depend on the view direction, so
# doodads drawn looking one way and hidden looking the other way from the same eye
# are hidden by their groups' portal culling.
func assert_global_portal_cull(probe: Node, wmo: Node3D, doodads: Array) -> bool:
	var eye := Vector3(103.0, -32.5, -76.0)
	var views := []
	for look in [STOCKADE_LOOK, -STOCKADE_LOOK]:
		cull_from(probe, eye, eye + look)
		var hidden := {}
		for batch in wmo.find_children("Group*_Batch*", "", false, false):
			hidden[String(batch.name).get_slice("_", 0)] = hidden.get(String(batch.name).get_slice("_", 0), true) and not batch.visible
		var shown := {}
		for doodad in doodads:
			if doodad.visible:
				shown[doodad.name] = true
		views.append([hidden.keys().filter(func(group): return hidden[group]), shown])
	var turned_away := 0
	for name in views[0][1]:
		if not views[1][1].has(name):
			turned_away += 1
	print("stockade cull: hidden groups %d / %d, doodads shown %d / %d, %d drawn looking +X are hidden looking -X" % [views[0][0].size(), views[1][0].size(), views[0][1].size(), views[1][1].size(), turned_away])
	if views[0][0].is_empty() or views[1][0].is_empty() or turned_away == 0:
		fail("Stockade portal cull: hidden groups %s / %s, %d doodads hidden by turning" % [views[0][0], views[1][0], turned_away])
		return false
	print("PASS: the Stockade global WMO is portal-culled with its doodads")
	return true

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

# Drawn quads of each of the portal's six emitter pools (instanceportal.m2, 197007).
func portal_particles(probe: Node) -> Array:
	var drawn := []
	for index in 6:
		var pool := probe.find_child("Particles197007_%d" % index, true, false) as MultiMeshInstance3D
		drawn.append(-1 if pool == null else pool.multimesh.visible_instance_count)
	return drawn

# The portal's emitters update and draw only while it is drawn: nothing 20 yd above the
# district with Jail01 portal-culled, all six emitting at the trigger facing it.
func assert_portal_particles(probe: Node, portal: Node3D) -> bool:
	var above := portal.global_position + Vector3(0.0, 20.0, 0.0)
	for frame in 5:
		cull_from(probe, above, above + Vector3(1.0, 0.0, 0.0))
	var culled := portal_particles(probe)
	var eye := TRIGGER + Vector3(0.0, 2.0, 0.0)
	for frame in 10:
		cull_from(probe, eye, portal.global_position + Vector3(0.0, 1.0, 0.0))
	var drawn := portal_particles(probe)
	var state: Dictionary = probe.objects_state()
	print("portal particles: culled=%s drawn=%s pools=%s" % [culled, drawn, state.get("particles")])
	if culled != [0, 0, 0, 0, 0, 0] or drawn.any(func(count): return count <= 0):
		fail("Portal emitters culled=%s drawn=%s; expected none above and all six at the trigger" % [culled, drawn])
		return false
	return true

func material_state(doodad: Node3D) -> Array:
	var state := []
	for mesh in doodad.find_children("Batch*", "MeshInstance3D", false, false):
		var material := (mesh as MeshInstance3D).get_active_material(0) as ShaderMaterial
		state.append([material.get_shader_parameter("mesh_color"), material.get_shader_parameter("transparency"), material.get_shader_parameter("texture_matrix_1")])
	return state

# Ten cull frames from `eye` looking at `target`, each after the shared material clock
# advances 137 ms.
func animate_frames(probe: Node, eye: Vector3, target: Vector3) -> void:
	for step in 10:
		root.get_node("M2MaterialClock").advance_time_ms(137.0)
		cull_from(probe, eye, target)

func animation_processing(node: Node, child: String) -> bool:
	var animation := node.get_node_or_null(child)
	return animation != null and animation.is_processing()

# Jail01 lamp 199823 (MODD 32, 10 yd from the trigger) animates its batch colour but
# not its bones. Culled, its colour animation (M2MaterialAnimation) does not advance
# and its material does not change as the shared clock advances; drawn again at the
# trigger, it animates. Its bones and those of the Jail01 cobweb 199565 (MODD 1105),
# whose bone and material tracks are all constant, never advance or process.
func assert_material_animation(probe: Node, wmo: Node3D) -> bool:
	var lamp := wmo.get_node("WmoDoodad32") as Node3D
	var cobweb := wmo.get_node("WmoDoodad1105") as Node3D
	if lamp.get_node_or_null("M2MaterialAnimation") == null:
		fail("Lamp 199823 has no material animation")
		return false
	var target := lamp.global_position + Vector3(0.0, 0.5, 0.0)
	# 300 yd up, looking away: beyond the lamp's scenery distance and outside Jail01.
	var far := TRIGGER + Vector3(0.0, 300.0, 0.0)
	cull_from(probe, far, far + Vector3(1.0, 0.0, 0.0))
	var frozen := material_state(lamp)
	animate_frames(probe, far, far + Vector3(1.0, 0.0, 0.0))
	if lamp.visible or material_state(lamp) != frozen:
		fail("Culled lamp: drawn=%s, material %s -> %s" % [lamp.visible, frozen, material_state(lamp)])
		return false
	var eye := TRIGGER + Vector3(0.0, 2.0, 0.0)
	var lamp_bones := bone_pose(lamp)
	animate_frames(probe, eye, target)
	if not lamp.visible or material_state(lamp) == frozen:
		fail("Drawn lamp: drawn=%s, material did not animate: %s" % [lamp.visible, frozen])
		return false
	if bone_pose(lamp) != lamp_bones:
		fail("Lamp 199823 has constant bone tracks but its pose changed")
		return false
	# The cobweb is out of view from the trigger; look at it from 3 yd.
	var near := cobweb.global_position + Vector3(0.0, 0.5, 0.0)
	var cobweb_eye := near + (near - eye).normalized() * 3.0
	var cobweb_bones := bone_pose(cobweb)
	var cobweb_material := material_state(cobweb)
	animate_frames(probe, cobweb_eye, near)
	if not cobweb.visible or bone_pose(cobweb) != cobweb_bones or material_state(cobweb) != cobweb_material:
		fail("Static cobweb: drawn=%s, pose or material changed" % cobweb.visible)
		return false
	for prop in [lamp, cobweb]:
		if animation_processing(prop, "M2Animation") or animation_processing(prop, "M2MaterialAnimation"):
			fail("Static-boned %s has a processing animation node" % prop.name)
			return false
	print("material animation: frozen on the culled lamp, animates when drawn; static bones and materials never advance or process")
	return true

# Outside the in-world cull (character select, previews) a model's own nodes animate:
# the cobweb 199565, whose tracks are all constant, has none processing, while the
# portal 197007 animates its bones.
func assert_unculled_static_models() -> bool:
	var loader: Object = ClassDB.instantiate("WowAssetLoader")
	var processing := {}
	for fdid in [199565, 197007]:
		var result: Dictionary = loader.load_m2("res://../data/models/%d.m2" % fdid)
		if result.has("error"):
			fail("Load %d: %s" % [fdid, result.error])
			return false
		var model: Node3D = result.node
		root.add_child(model)
		await process_frame
		processing[fdid] = [animation_processing(model, "M2Animation"), animation_processing(model, "M2MaterialAnimation")]
		model.free()
	if processing[199565] != [false, false] or not processing[197007][0]:
		fail("Unculled animation processing (bones, material): cobweb %s, portal %s" % [processing[199565], processing[197007]])
		return false
	print("unculled models: static cobweb processes nothing, portal animates")
	return true
