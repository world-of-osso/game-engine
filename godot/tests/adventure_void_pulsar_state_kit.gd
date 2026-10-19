extends "res://tests/adventure_zero_actor_display.gd"
# Genuine detached Journal selection, no kit/pose/model injection.
const PULSAR := [505,1319,2870,6132,141206,"Void Pulsar",1380,4131,1361,"EncounterJournalDungeonTab"]
const EFFECTS := [4861458,7455384]

func selected_model() -> Node3D:
	return control("EncounterJournalModelScene").find_child("NpcVisualRoot", true, false) as Node3D

func source_effects(model: Node3D) -> Dictionary:
	var found := {}
	for node in model.find_children("*", "Node3D", true, false):
		if not node.has_meta("m2_source_path"):
			continue
		var path: String = str(node.get_meta("m2_source_path"))
		for fdid in EFFECTS:
			if path.ends_with("/%s.m2" % fdid):
				if found.has(fdid):
					completion.fail("Duplicate source effect%s under one detached visual" % fdid)
				found[fdid] = node
	return found

func require_effects() -> Dictionary:
	var deadline := Time.get_ticks_msec() + 30000
	var found := source_effects(selected_model())
	while found.size() < EFFECTS.size() and Time.get_ticks_msec() < deadline:
		await process_frame
		found = source_effects(selected_model())
	print("JOURNAL_STATE_KIT_CHILDREN ", JSON.stringify({"display":141206,"kit":253973,"model_identity":selected_model().get_instance_id(),"found":found.keys()}))
	if not completion.require(found.size() == 2, "Journal detached CDI253973 children missing: expected4861458/7455384 actual" + str(found.keys())):
		return {}
	for fdid in EFFECTS:
		var node: Node3D = found[fdid]
		var parent_path := str(node.get_parent().get_path())
		if not completion.require(parent_path.ends_with("AttachmentBone34/Attachment34"), "Source effect not attached to authored34: " + parent_path):
			return {}
		var player: Node = node.find_child("M2Animation", true, false)
		if not completion.require(player != null and player.current_animation_id() == 158, "Source effect%s did not play158" % fdid):
			return {}
	return found

func check_clock_and_identity(found: Dictionary) -> bool:
	var before := {}
	for fdid in EFFECTS:
		var node: Node3D = found[fdid]
		var player: Node = node.find_child("M2Animation", true, false)
		before[fdid] = {"node":node.get_instance_id(),"player":player.get_instance_id(),"phase":player.animation_phase()}
	await create_timer(0.3).timeout
	var after := source_effects(selected_model())
	for fdid in EFFECTS:
		if not completion.require(after.has(fdid), "Normal sync removed state effect"):
			return false
		var node: Node3D = after[fdid]
		var player: Node = node.find_child("M2Animation", true, false)
		var phase: Dictionary = player.animation_phase()
		var prior: Dictionary = before[fdid]
		var advance := fposmod(float(phase.time_ms) - float(prior.phase.time_ms), float(phase.duration_ms))
		if not completion.require(node.get_instance_id() == prior.node and player.get_instance_id() == prior.player and advance > 5.0, "Normal Journal sync replaced/reset source effect%s" % fdid):
			return false
	return true

func run() -> void:
	completion = load("res://tests/completion.gd").new(self, 1)
	completion.finish(await run_state_kit())

func run_state_kit() -> bool:
	var server := OS.get_environment("ADVENTURE_GUIDE_SERVER")
	var account := OS.get_environment("ADVENTURE_GUIDE_ACCOUNT")
	shots = OS.get_environment("ADVENTURE_GUIDE_SHOTS")
	if not completion.require(server == "127.0.0.1:5771" and account.begins_with("fb_") and not shots.is_empty(), "Explicit private5771/fb account/shots required"):
		return false
	client = load("res://scenes/client.tscn").instantiate()
	root.add_child(client)
	var deadline := Time.get_ticks_msec() + 180000
	while client.account_state().assets_starting and Time.get_ticks_msec() < deadline:
		await process_frame
	var error: String = client.connect_account(server, account, OS.get_environment("ADVENTURE_GUIDE_PASSWORD"), false)
	if not completion.require(error.is_empty(), error) or not completion.step(await enter_world_when_ready(), "genuine world readiness"):
		return false
	await journal_key()
	deadline = Time.get_ticks_msec() + 90000
	while not client.adventure_guide_state().loaded and Time.get_ticks_msec() < deadline:
		await process_frame
	if not completion.require(client.adventure_guide_state().loaded, "Journal source not loaded") or not await select_zero_actor(PULSAR):
		return false
	var source := source_row("CreatureDisplayInfo", 141206)
	if not completion.require(int(source.StateSpellVisualKitID) == 253973, "CDI state source changed"):
		return false
	await shot("void-pulsar-state-kit-before-assertion")
	var found := await require_effects()
	if completion.failed or not await check_clock_and_identity(found):
		return false
	var old_model := selected_model().get_instance_id()
	if not completion.step(await click("JournalCreature6127"), "physical switch away from Pulsar"):
		return false
	await settle()
	if not completion.require(not is_instance_id_valid(old_model), "Switched-away detached visual remains alive"):
		return false
	if not completion.step(await click("JournalCreature6132"), "physical return to Pulsar") or not completion.step(await expect_model(6132,141206,"Void Pulsar"), "Pulsar reload"):
		return false
	if (await require_effects()).size() != 2:
		return false
	old_model = selected_model().get_instance_id()
	await journal_key()
	await settle()
	if not completion.require(not is_instance_id_valid(old_model), "Closing Journal retained detached visual"):
		return false
	await journal_key()
	if not completion.step(await expect_model(6132,141206,"Void Pulsar"), "Pulsar reopen") or (await require_effects()).size() != 2:
		return false
	if not completion.require(selected_model().get_instance_id() != old_model, "Reopen reused stale detached visual"):
		return false
	return completion.case_done()
