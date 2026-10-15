extends "res://tests/toy_effects47_scarab_release_live.gd"
# Source Bashiok model/destination/alpha/static lifetime; no fabricated actor or photograph move.
var bashiok: Node3D

func proof_cases() -> Array:
	return [[118222, 173895, 60051, "Spirit of Bashiok", 1.2, 86916, false, 60000]]

func prove_toy(_sample: Array) -> void:
	await learn_toy(118222)
	if failed: return
	var local := int(client.account_state().local_player_id)
	var owner: Transform3D = client.unit_transform(local)
	var health := float(client.account_state().local_player_health)
	await click("CollectionsMicroButton")
	await click("CollectionsJournalTab3")
	await click("ToyBoxSearchBox")
	var edit := control("ToyBoxSearchBox") as LineEdit
	edit.select_all()
	await type_text("Spirit of Bashiok")
	if not await wait_until(func(): return matching_toy_button("Spirit of Bashiok") != "", "Bashiok journal search"): return
	var started := Time.get_ticks_msec()
	await click(matching_toy_button("Spirit of Bashiok"))
	await click("CollectionsMicroButton")
	if not await wait_until(func(): return scenery("") != null, "source Bashiok60051 native mesh", 120000): return
	bashiok = scenery("")
	if find_loaded_bashioks().size() != 1: fail("Bashiok source actor count is not one"); return
	await create_timer(1.0).timeout
	var model := bashiok.find_children("NpcModel", "Node3D", true, false)[0] as Node3D
	var visual := model.get_parent() as Node3D
	if abs(visual.scale.x - 1.2) > 0.0001: fail("Bashiok lost source CDI scale1.2"); return
	if control("PetFrame") != null: fail("Bashiok acquired pet frame"); return
	if not native_display(-1): fail("Bashiok replaced native player"); return
	var offset := bashiok.global_position - owner.origin
	offset.y = 0.0
	var forward := owner.basis.z
	forward.y = 0.0
	var transparencies := []
	for mesh in model.find_children("*", "MeshInstance3D", true, false):
		transparencies.append((mesh as MeshInstance3D).transparency)
	receipts.append({"phase": "bashiok-source-model", "display": 60051, "scale": str(visual.scale), "position": str(bashiok.global_position), "owner": str(owner.origin), "offset": str(offset), "forward": str(forward), "alpha": 175, "mesh_transparencies": transparencies})
	write_receipts()
	if abs(offset.length() - 5.0) > 0.15 or offset.normalized().dot(forward.normalized()) < 0.99:
		fail("Bashiok lost source caster-front5yd destination"); return
	var expected := 1.0 - 175.0 / 255.0
	for transparency in transparencies:
		if abs(float(transparency) - expected) > 0.000001:
			fail("Bashiok lost source alpha175: observed transparency " + str(transparency)); return
	for unit in client.find_children(character, "Node3D", true, false):
		if unit.get_node_or_null("PlayerModel") == null: continue
		for mesh in unit.find_children("*", "MeshInstance3D", true, false):
			if abs((mesh as MeshInstance3D).transparency) > 0.000001: fail("Bashiok alpha leaked to owner"); return
	await capture("toy118222-source-display60051-alpha175")
	if failed: return
	var placed := bashiok.global_position
	var before: Vector3 = client.unit_transform(local).origin
	set_walking(true)
	await create_timer(3.5).timeout
	set_walking(false)
	var after: Vector3 = client.unit_transform(local).origin
	if after.distance_to(before) < 20.0: fail("Bashiok nonfollow stimulus did not move owner20yd"); return
	if not is_instance_valid(bashiok) or bashiok.global_position.distance_to(placed) > 0.05: fail("Bashiok followed or moved"); return
	if float(client.account_state().local_player_health) != health: fail("Bashiok changed owner health"); return
	receipts.append({"phase": "bashiok-static-after-owner-move", "before": str(before), "after": str(after), "placed": str(placed)})
	write_receipts()
	if not await wait_until(func(): return not is_instance_valid(bashiok), "source60000ms natural Bashiok removal", 80000): return
	var elapsed := Time.get_ticks_msec() - started
	if elapsed < 59000 or elapsed > 80000: fail("Bashiok source lifetime outside predeclared59..80s observation band"); return
	receipts.append({"phase": "bashiok-natural-expiry", "elapsed_ms": elapsed, "scope": "wall timing additionally requires pressure qualification"})
	write_receipts()

func scenery(_title: String) -> Node3D:
	var loaded := find_loaded_bashioks()
	return loaded[0] if not loaded.is_empty() else null

func find_loaded_bashioks() -> Array[Node3D]:
	var result: Array[Node3D] = []
	var units := client.find_child("WorldUnits", true, false)
	if units == null: return result
	for unit in units.get_children():
		var models := unit.find_children("NpcModel", "Node3D", true, false)
		if models.is_empty(): continue
		var visual := models[0].get_parent() as Node3D
		if int(visual.get_meta("creature_display_id")) == 60051 and not models[0].find_children("*", "MeshInstance3D", true, false).is_empty(): result.append(unit)
	return result
