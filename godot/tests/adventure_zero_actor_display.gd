extends "res://tests/adventure_model_scenes.gd"
# Real installed Journal joins; no injected actors, scenes, defaults or assets.
const CASES := [
	[514, 1296, 2644, 5771, 122886, "One-Armed Bandit", 978, 2418, 960, "EncounterJournalRaidTab"],
	[505, 1319, 2870, 6132, 141206, "Void Pulsar", 1380, 4131, 1361, "EncounterJournalDungeonTab"],
	[503, 1200, 2499, 5385, 103255, "Raszageth the Storm-Eater", 609, 1696, 595, "EncounterJournalRaidTab"],
]

func run() -> void:
	completion = load("res://tests/completion.gd").new(self, CASES.size())
	completion.finish(await run_zero_actors())

func source_actor_display(actor: Dictionary) -> Dictionary:
	if int(actor.UiModelSceneActorDisplayID) != 0:
		return super.source_actor_display(actor)
	# ModelSceneActorMixin ApplyFromModelSceneActorInfo's explicit no-display arm.
	return {"Scale":"1", "Alpha":"1", "AnimationID":"0"}

func reveal_source_boss(name: String) -> bool:
	for attempt in 20:
		var boss := control(name)
		var list := control("EncounterJournalBosses")
		if boss != null and list != null and list.get_global_rect().has_point(boss.get_global_rect().get_center()):
			return true
		if not completion.step(await wheel("EncounterJournalBosses", MOUSE_BUTTON_WHEEL_DOWN), "physical boss-list scroll"):
			return false
	return completion.fail("Source boss was not revealed: " + name)

func select_zero_actor(row: Array) -> bool:
	if control("EncounterJournalBack") != null and client.adventure_guide_state().instance > 0:
		if not completion.step(await click("EncounterJournalBack"), "physical Journal Home"):
			return false
	if not completion.step(await click(row[9]), "physical source instance-type catalog") or not completion.step(await click("EncounterJournalTierDropdown"), "physical tier menu") or not completion.step(await click("EncounterJournalTier%s" % row[0]), "physical source tier"):
		return false
	var name := "JournalInstance%s" % row[1]
	if not completion.step(await reveal_instance(name), "source instance visible") or not completion.step(await click(name), "physical source instance"):
		return false
	var boss := "JournalBoss%s" % row[2]
	if not await reveal_source_boss(boss) or not completion.step(await click(boss), "physical source encounter"):
		return false
	if not completion.require(client.adventure_guide_state().encounter == row[2], "Physical encounter selection did not activate the source encounter"):
		return false
	if not completion.step(await click("EncounterJournalPage3"), "physical Model tab"):
		return false
	if not completion.require(client.adventure_guide_state().page == "Model", "Physical Model tab did not activate"):
		return false
	if client.adventure_guide_state().creature != row[3]:
		if not completion.step(await click("JournalCreature%s" % row[3]), "physical source creature selector"):
			return false
	if not completion.require(client.adventure_guide_state().creature == row[3], "Physical source creature selection did not activate"):
		return false
	var deadline := Time.get_ticks_msec() + 90000
	while client.adventure_guide_state().model_display != row[4] and Time.get_ticks_msec() < deadline:
		await process_frame
	var state: Dictionary = client.adventure_guide_state()
	if not completion.require(state.creature == row[3] and state.model_display == row[4] and label_text("EncounterJournalModelTitle") == row[5], "Journal zero-display source actor was refused: creature=%s display=%s page=%s" % [state.creature, state.model_display, state.page]):
		return false
	await settle()
	var host := control("EncounterJournalModelScene")
	var model := host.find_child("NpcVisualRoot", true, false) as Node3D
	if not completion.require(model != null, "Journal zero-display loaded model missing"):
		return false
	var player := model.find_child("M2Animation", true, false)
	if player != null and not completion.require(player.current_animation_id() == 0, "Journal no-display default idle animation differs from0"):
		return false
	for mesh in model.find_children("*", "MeshInstance3D", true, false):
		if not completion.require(mesh.transparency == 0.0, "Journal no-display default alpha differs from1"):
			return false
	print("JOURNAL_ZERO_DISPLAY_DEFAULTS creature=%s scene=%s scale1 alpha1 animation0" % [row[3], row[6]])
	return true

func run_zero_actors() -> bool:
	var server := OS.get_environment("ADVENTURE_GUIDE_SERVER")
	var account := OS.get_environment("ADVENTURE_GUIDE_ACCOUNT")
	shots = OS.get_environment("ADVENTURE_GUIDE_SHOTS")
	if not completion.require(server == "127.0.0.1:5771" and account.begins_with("fb_") and not shots.is_empty(), "Explicit private5771/fb_* account and shots required"):
		return false
	client = load("res://scenes/client.tscn").instantiate()
	root.add_child(client)
	var deadline := Time.get_ticks_msec() + 180000
	while client.account_state().assets_starting and Time.get_ticks_msec() < deadline:
		await process_frame
	if not completion.require(not client.account_state().assets_starting, "Asset startup timeout"):
		return false
	var error: String = client.connect_account(server, account, OS.get_environment("ADVENTURE_GUIDE_PASSWORD"), false)
	if not completion.require(error.is_empty(), error) or not completion.step(await enter_world_when_ready(), "genuine world readiness"):
		return false
	await journal_key()
	deadline = Time.get_ticks_msec() + 90000
	while not client.adventure_guide_state().loaded and Time.get_ticks_msec() < deadline:
		await process_frame
	if not completion.require(client.adventure_guide_state().loaded, "Journal source catalog not ready"):
		return false
	if not completion.require(client.account_state().ui_skin == OS.get_environment("ADVENTURE_GUIDE_EXPECTED_SKIN"), "Expected skin mismatch"):
		return false
	for row in CASES:
		var actor := source_row("UiModelSceneActor", row[7])
		if not completion.require(int(actor.UiModelSceneID) == row[6] and int(actor.UiModelSceneActorDisplayID) == 0, "Wrong zero-display source join"):
			return false
		if not await select_zero_actor(row) or not await verify_scene(row[3], row[6], row[7], row[8]):
			return false
	print("JOURNAL_ZERO_DISPLAY_ACTORS_PASS skin=%s source609/978/1380 retainedFOV_not_retail_certified" % client.account_state().ui_skin)
	return true
