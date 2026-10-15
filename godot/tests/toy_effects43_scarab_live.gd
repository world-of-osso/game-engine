extends "res://tests/toy_effects14_signs_framed_live.gd"
# Real learned UseToy, source models/scale, actual movement and natural24000ms removal.
# Open-ground renderer witness only: not full guardian AI, navigation or level proof.
var prior_scarabs: Array[int] = []
var selected_scarab: Node3D
var walking := false

func proof_cases() -> Array:
	return [[64881, 92146, 0, "Pendant of the Scarab Storm", 1.0, 51113, false, 24000]]

func prove_toy(_sample: Array) -> void:
	await learn_toy(64881)
	if failed: return
	for unit in scarab_units(): prior_scarabs.append(unit.get_instance_id())
	var activated_at := await activate_scarabs()
	if failed: return
	if not await wait_until(func(): return new_scarabs().size() >= 20, "source variable Scarab count", 10000): return
	await create_timer(1.0).timeout
	var units := new_scarabs()
	if units.size() < 20 or units.size() > 30: fail("source Scarab count outside variance interval"); return
	var displays := {}
	for unit in units:
		if not await wait_until(func(): return is_instance_valid(unit) and not unit.find_children("*", "MeshInstance3D", true, false).is_empty(), "source Scarab mesh", 10000): return
		var models := unit.find_children("NpcModel", "Node3D", true, false)
		if models.is_empty(): fail("Scarab has no loaded source model"); return
		var model := models[0] as Node3D
		var visual := model.get_parent() as Node3D
		var display := int(visual.get_meta("creature_display_id"))
		if display not in [36611, 36595]: fail("substituted Scarab display"); return
		if abs(visual.scale.x - 0.1) > 0.0001: fail("Scarab lost authored CDI scale0.1"); return
		displays[display] = unit
		receipts.append({"phase": "scarab-source-model", "display": display, "position": str(unit.global_position), "scale": str(visual.scale), "meshes": model.find_children("*", "MeshInstance3D", true, false).size()})
	if control("PetFrame") != null: fail("source guardians acquired combat pet frame"); return
	receipts.append({"phase": "scarab-source-count", "count": units.size(), "activated_at": activated_at, "observed_displays": displays.keys()})
	write_receipts()
	for display in displays:
		selected_scarab = displays[display]
		await capture("toy64881-source-display%d" % int(display))
		if failed: return
	await prove_owner_follow(units)
	if failed: return
	var following := new_scarabs()
	if following.is_empty(): fail("Scarabs removed before follow capture"); return
	selected_scarab = following[0]
	await capture("toy64881-following-owner")
	if failed: return
	if not await wait_until(func(): return new_scarabs().is_empty(), "source24000ms natural Scarab removal", 30000): return
	var elapsed := Time.get_ticks_msec() - activated_at
	# Fixed before the first run; never widen this band after a slow observation.
	if elapsed < 23000 or elapsed > 27000: fail("source Scarab expiry outside fixed wall window"); return
	receipts.append({"phase": "scarab-natural-expiry", "elapsed_ms": elapsed, "scope": "timing acceptance additionally requires pressure qualification"})
	write_receipts()

func activate_scarabs() -> int:
	await click("CollectionsMicroButton")
	await click("CollectionsJournalTab3")
	await click("ToyBoxSearchBox")
	var edit := control("ToyBoxSearchBox") as LineEdit
	edit.select_all()
	await type_text("Pendant of the Scarab Storm")
	if not await wait_until(func(): return matching_toy_button("Pendant of the Scarab Storm") != "", "source Scarab toy search"): return 0
	var started := Time.get_ticks_msec()
	await click(matching_toy_button("Pendant of the Scarab Storm"))
	await click("CollectionsMicroButton")
	return started

func prove_owner_follow(units: Array[Node3D]) -> void:
	var before := {}
	for unit in units: before[unit.get_instance_id()] = unit.global_position
	var local := int(client.account_state().local_player_id)
	var start: Vector3 = client.unit_transform(local).origin
	set_walking(true)
	await create_timer(3.5).timeout
	set_walking(false)
	var destination: Vector3 = client.unit_transform(local).origin
	# Concrete20yd stimulus, not an invented production follow offset.
	if destination.distance_to(start) < 20.0: fail("native W did not move owner20yd"); return
	if not await wait_until(func(): return all_approached_owner(units, before, destination), "source Scarabs follow actual moving owner", 5000): return
	receipts.append({"phase": "scarab-owner-follow", "owner_start": str(start), "owner_end": str(destination), "actors": units.size()})
	write_receipts()

func all_approached_owner(units: Array[Node3D], before: Dictionary, destination: Vector3) -> bool:
	for unit in units:
		if not is_instance_valid(unit): return false
		var original: Vector3 = before[unit.get_instance_id()]
		if unit.global_position.distance_to(destination) >= original.distance_to(destination) - 0.1: return false
	return true

func scenery(_title: String) -> Node3D:
	return selected_scarab if is_instance_valid(selected_scarab) else null

func scarab_units() -> Array[Node3D]:
	var result: Array[Node3D] = []
	var units := client.find_child("WorldUnits", true, false)
	if units == null: return result
	for unit in units.get_children():
		if unit is Node3D and unit.has_meta("unit_name") and str(unit.get_meta("unit_name")) == "Storm Scarab": result.append(unit)
	return result

func new_scarabs() -> Array[Node3D]:
	var result: Array[Node3D] = []
	for unit in scarab_units():
		if unit.get_instance_id() not in prior_scarabs: result.append(unit)
	return result

func set_walking(pressed: bool) -> void:
	walking = pressed
	var event := InputEventKey.new()
	event.keycode = KEY_W
	event.physical_keycode = KEY_W
	event.pressed = pressed
	root.push_input(event, true)

func fail(message: String) -> void:
	if walking: set_walking(false)
	super.fail(message)
