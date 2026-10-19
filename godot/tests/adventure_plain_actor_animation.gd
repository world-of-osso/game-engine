extends "res://tests/adventure_zero_actor_display.gd"
# Genuine source selections; no injected pose, animation, scene or model.
const ANIMATED_CASES := [
	[5481,112005,"Vakan",673,1848,659,239,25,503,1205,2531],
	[5482,112006,"Gholna",674,1849,660,239,25,503,1205,2531],
]

func animated_cases() -> Array:
	return ANIMATED_CASES

func run() -> void:
	completion = load("res://tests/completion.gd").new(self, animated_cases().size())
	completion.finish(await run_plain_animation())

func check_authored_animation(row: Array) -> bool:
	var actor := source_row("UiModelSceneActor", row[4])
	var display := source_row("UiModelSceneActorDisplay", int(actor.get("UiModelSceneActorDisplayID", "0")))
	if completion.failed:
		return false
	if not completion.require(int(actor.UiModelSceneID) == row[3] and int(actor.UiModelSceneActorDisplayID) == row[6] and int(display.AnimationID) == row[7] and int(display.AnimKitID) == 0 and int(display.SpellVisualKitID) == 0 and int(display.SequenceVariation) == 0 and float(display.AnimSpeed) == 1.0, "Plain authored-animation source contract changed"):
		return false
	var host := control("EncounterJournalModelScene")
	var model := host.find_child("NpcVisualRoot", true, false) as Node3D
	var player := model.find_child("M2Animation", true, false) if model != null else null
	if not completion.require(player != null, "Journal loaded per-model animation player missing"):
		return false
	var phase: Dictionary = player.animation_phase()
	print("JOURNAL_PLAIN_ANIMATION_OBSERVED ", JSON.stringify({"creature":row[0],"scene":row[3],"actor":row[4],"expected_animation":int(display.AnimationID),"actual_animation":player.current_animation_id(),"phase":phase}))
	if not completion.step(await move_to("EncounterJournalModelTitle"), "move away from selector") or not completion.step(await shot("plain-animation-scene%s-before-assertion" % row[3]), "source animation capture"):
		return false
	if not completion.require(player.current_animation_id() == int(display.AnimationID), "Journal plain authored animation mismatch: expected%s actual%s" % [row[7],player.current_animation_id()]):
		return false
	var identity := player.get_instance_id()
	phase = player.animation_phase()
	var initial_time := float(phase.get("time_ms", -1.0))
	var duration := float(phase.get("duration_ms", 0.0))
	if not completion.require(initial_time >= 0.0 and duration > 0.0, "Journal source clip phase missing"):
		return false
	var elapsed := minf(duration * 0.25 / 1000.0, 0.3)
	await create_timer(elapsed).timeout
	var later: Dictionary = player.animation_phase()
	var phase_advance := fposmod(float(later.get("time_ms", initial_time)) - initial_time, duration)
	print("JOURNAL_PLAIN_ANIMATION_CLOCK ", JSON.stringify({"creature":row[0],"initial_ms":initial_time,"later_ms":later.get("time_ms"),"duration_ms":duration,"modulo_advance_ms":phase_advance,"same_player":player.get_instance_id()==identity}))
	if not completion.require(player.get_instance_id() == identity and player.current_animation_id() == row[7] and phase_advance > 5.0, "Journal authored clip reset or failed to advance across normal sync frames"):
		return false
	return true

func open_animation_encounter(row: Array) -> bool:
	if client.adventure_guide_state().encounter == row[10]:
		return true
	if control("EncounterJournalBack") != null and client.adventure_guide_state().instance > 0:
		if not completion.step(await click("EncounterJournalBack"), "physical Journal Home"):
			return false
	if not completion.step(await click("EncounterJournalRaidTab"), "physical raid tab") or not completion.step(await click("EncounterJournalTierDropdown"), "physical tier menu") or not completion.step(await click("EncounterJournalTier%s" % row[8]), "physical authored tier"):
		return false
	var instance := "JournalInstance%s" % row[9]
	if not completion.step(await reveal_instance(instance), "source instance visible") or not completion.step(await click(instance), "physical source instance"):
		return false
	var boss := "JournalBoss%s" % row[10]
	if not await reveal_source_boss(boss) or not completion.step(await click(boss), "physical source encounter"):
		return false
	if not completion.require(client.adventure_guide_state().encounter == row[10], "Physical source encounter did not activate"):
		return false
	return completion.step(await click("EncounterJournalPage3"), "physical Model tab")

func run_plain_animation() -> bool:
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
	for row in animated_cases():
		if not await open_animation_encounter(row):
			return false
		if client.adventure_guide_state().creature != row[0]:
			if not completion.step(await click("JournalCreature%s" % row[0]), "physical authored creature selector"):
				return false
		if not completion.step(await expect_model(row[0],row[1],row[2]), "genuine source model loaded"):
			return false
		if not await check_authored_animation(row) or not await verify_scene(row[0],row[3],row[4],row[5]):
			return false
	print("JOURNAL_PLAIN_ACTOR_ANIMATION_PASS skin=%s cases=%s no_kits_variations_speed_effect_changes" % [client.account_state().ui_skin,animated_cases().size()])
	return true
