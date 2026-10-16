extends "res://tests/toy_effects47_scarab_release_live.gd"
# Actual source Podium at the authored spawn; no photo camera or actor repositioning.

func proof_cases() -> Array:
	return [[239018, 1227063, 127527, "Winner's Podium", 1.0, 240220, false, 1200000]]

func prove_toy(_sample: Array) -> void:
	await learn_toy(239018)
	if failed: return
	var local := int(client.account_state().local_player_id)
	var owner: Transform3D = client.unit_transform(local)
	var health := float(client.account_state().local_player_health)
	await click("CollectionsMicroButton")
	await click("CollectionsJournalTab3")
	await click("ToyBoxSearchBox")
	var edit := control("ToyBoxSearchBox") as LineEdit
	edit.select_all()
	await type_text("Winner's Podium")
	if not await wait_until(func(): return matching_toy_button("Winner's Podium") != "", "Podium journal search"): return
	await click(matching_toy_button("Winner's Podium"))
	await click("CollectionsMicroButton")
	if not await wait_until(func(): return not find_loaded_podia().is_empty(), "source127527 native Podium meshes", 120000): return
	var podia := find_loaded_podia()
	if podia.size() != 1: fail("source Podium count is not one"); return
	var podium := podia[0]
	var model := podium.find_children("NpcModel", "Node3D", true, false)[0] as Node3D
	var visual := model.get_parent() as Node3D
	if abs(podium.scale.x - 1.0) > 0.0001 or abs(visual.scale.x - 1.0) > 0.0001: fail("Podium source scale1 lost"); return
	if control("PetFrame") != null: fail("Podium acquired pet frame"); return
	if not native_display(-1): fail("Podium replaced native player"); return
	var offset := podium.global_position - owner.origin
	offset.y = 0.0
	var forward := owner.basis.x
	forward.y = 0.0
	if abs(offset.length() - 1.5) > 0.15 or offset.normalized().dot(forward.normalized()) < 0.99: fail("source47 minimum scaled reach1.5/front lost"); return
	for mesh in model.find_children("*", "MeshInstance3D", true, false):
		if abs((mesh as MeshInstance3D).transparency) > 0.000001: fail("Podium source alpha255 lost"); return
	receipts.append({"phase": "podium-source-model", "display": 127527, "source_model_fdid": 6392183, "position": str(podium.global_position), "owner": str(owner.origin), "offset": str(offset), "forward": str(forward), "root_scale": str(podium.scale), "visual_scale": str(visual.scale), "meshes": model.find_children("*", "MeshInstance3D", true, false).size(), "lifetime_scope": "1200000ms simulation expiry tested separately, not wall-time certified here"})
	await capture("toy239018-source-display127527")
	if failed: return
	var placed := podium.global_position
	var before: Vector3 = client.unit_transform(local).origin
	set_walking(true)
	await create_timer(2.0).timeout
	set_walking(false)
	var after: Vector3 = client.unit_transform(local).origin
	if after.distance_to(before) < 10.0: fail("static behavior stimulus did not move owner10yd"); return
	if not is_instance_valid(podium) or podium.global_position.distance_to(placed) > 0.05: fail("source static Podium moved/followed"); return
	if float(client.account_state().local_player_health) != health: fail("Podium changed owner health"); return
	receipts.append({"phase": "podium-static-after-real-owner-movement", "before": str(before), "after": str(after), "placed": str(placed), "health": health, "scope": "functional movement, not screenshot positioning"})
	write_receipts()

func capture(label: String) -> void:
	# Keep gameplay's camera unchanged, bypass inherited sign/model photo composition.
	await capture_native_frame(label)

func find_loaded_podia() -> Array[Node3D]:
	var result: Array[Node3D] = []
	var units := client.find_child("WorldUnits", true, false)
	if units == null: return result
	for unit in units.get_children():
		var models := unit.find_children("NpcModel", "Node3D", true, false)
		if models.is_empty(): continue
		var model := models[0] as Node3D
		var visual := model.get_parent() as Node3D
		if int(visual.get_meta("creature_display_id", 0)) == 127527 and not model.find_children("*", "MeshInstance3D", true, false).is_empty(): result.append(unit)
	return result
