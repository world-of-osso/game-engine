extends "res://tests/questlog_fidelity_live.gd"

# Existing disposable account journal only; no staged pet health, selection or model.
# QF_STEPS=check_skin,check_owned_pet_camera, QF_SKIN=modern|forever, private UDP5801.
func check_owned_pet_camera() -> bool:
	await tap_collections()
	if not await wait_frames(func(): return client.find_child("CollectionsJournalTab2", true, false) != null, "Collections shell"):
		return false
	await click_control(client.find_child("CollectionsJournalTab2", true, false) as Control)
	if not await wait_frames(func(): return client.get_node_or_null("CollectionsJournalUI") != null, "Pet Journal"):
		return false
	var journal := client.get_node("CollectionsJournalUI")
	if not await wait_frames(func(): return not journal.find_children("pet_select_owned_*", "Control", true, false).is_empty(), "authoritative owned pets"):
		return false
	var owned: Control
	for candidate in journal.find_children("pet_select_owned_*", "Control", true, false):
		if String(candidate.name).trim_prefix("pet_select_owned_").is_valid_int():
			owned = candidate
			break
	if owned == null:
		fail("Owned pet row missing")
		return false
	await click_control(owned)
	if not await wait_frames(func(): return model_meshes("PetJournalPetCardModelScene") > 0, "card model", 60000):
		return false
	await frames(30)
	await capture("authored-card-before-assert.png")
	if not authored_camera("PetJournalPetCardModelScene", Vector3(12.124355, 0, 7)):
		return false
	for slot in [0, 1, 2]:
		var button := journal.find_child("PetJournalBattleSlot%d" % slot, true, false) as BaseButton
		if not await wait_frames(func(): return not button.disabled, "loadout ready"):
			return false
		await click_control(button)
		if not await wait_frames(func(): return model_meshes("PetJournalBattleSlot%dModelScene" % slot) > 0, "slot model", 60000):
			return false
		await frames(30)
		if not authored_camera("PetJournalBattleSlot%dModelScene" % slot, Vector3(6.928203, 0.25, -4)):
			return false
	await capture("authored-pet-camera.png")
	print("PASS PET_AUTHORED_CAMERA skin=", OS.get_environment("QF_SKIN"), " source scenes6/7; FOV unsourced")
	return true

func authored_camera(frame_name: String, expected_eye: Vector3) -> bool:
	var journal := client.get_node("CollectionsJournalUI")
	var host := journal.find_child(frame_name, true, false)
	var camera := host.find_children("*", "Camera3D", true, false)[0] as Camera3D
	if camera.position.distance_to(expected_eye) > 0.0001:
		fail("DB2 camera eye mismatch: " + str(camera.position))
		return false
	var bounds: AABB
	var have_bounds := false
	for mesh: MeshInstance3D in host.find_children("*", "MeshInstance3D", true, false):
		if not mesh.is_visible_in_tree():
			continue
		var mesh_bounds := mesh.global_transform * mesh.get_aabb()
		bounds = bounds.merge(mesh_bounds) if have_bounds else mesh_bounds
		have_bounds = true
	print("PET_AUTHORED_BOUNDS ", frame_name, " ", bounds, " camera=", camera.position)
	# Scene6/7 actor flags9 center only WoW Z (engine Y); actor PositionZ is zero.
	if not have_bounds or abs(bounds.get_center().y) > 0.001:
		fail("Authored actor origin not centered on engine Y: " + str(bounds))
		return false
	return true
