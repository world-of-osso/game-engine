extends "res://tests/toy_effects14_visual_live.gd"
# Actual Toy Box target marker and authored looping ocean audio, private5911.
func proof_cases() -> Array:
	return [[168824, 298869, 2021674, "Ocean Simulator"], [122120, 179869, 165877, "Gaze of the Darkmoon"]]

func prove_toy(sample: Array) -> void:
	await learn_toy(int(sample[0]))
	if failed: return
	if int(sample[0]) == 122120:
		await prove_enemy_marker(sample)
	else:
		await prove_ocean(sample)

func activate_named(title: String) -> void:
	await click("CollectionsMicroButton")
	await click("CollectionsJournalTab3")
	await click("ToyBoxSearchBox")
	var edit := control("ToyBoxSearchBox") as LineEdit
	edit.select_all()
	await type_text(title)
	if not await wait_until(func(): return control("ToySpellButton1Name") != null and control("ToySpellButton1Name").text == title, "source toy search"): return
	await click("ToySpellButton1")
	await click("CollectionsMicroButton")

func prove_enemy_marker(sample: Array) -> void:
	for unit in client.find_children(character, "Node3D", true, false):
		if unit.get_node_or_null("PlayerModel") != null:
			actor = unit
			break
	if actor == null: fail("native player missing"); return
	if not await wait_until(func(): return visible_invader() != null, "authored hostile visual readiness", 120000): return
	var closest := visible_invader()
	var forward := -root.get_camera_3d().global_basis.z
	forward.y = 0.0
	var destination := closest.position - forward.normalized() * 12.0
	if not command(OS.get_environment("TOYFX_ADMIN"), ["set-position", character, str(destination.x), str(-destination.z), str(destination.y)]): return
	if not await wait_until(func(): return actor.position.distance_to(destination) < 3.0, "private hostile range"): return
	var target := 0
	for attempt in range(60):
		await tap(KEY_TAB)
		var selected = client.target_state().target
		if selected == null: continue
		var rules: Dictionary = client.nameplate_rules(int(selected))
		if str(rules.get("reaction", "")) == "Hostile" and bool(rules.get("alive", false)) and float(rules.get("distance", 1000.0)) <= 30.0:
			target = int(selected)
			break
	if target == 0: fail("living source hostile target absent"); return
	if not await wait_until(func(): return client.target_state().server_target == target, "authoritative hostile selection"): return
	# The 12yd forest viewpoint can be occluded by authored trees. Observe from the
	# selected live unit's own source position, without bypassing server LoS checks.
	var target_transform: Transform3D = client.unit_transform(target)
	var near := target_transform.origin + Vector3(2.0, 0.0, 0.0)
	if not command(OS.get_environment("TOYFX_ADMIN"), ["set-position", character, str(near.x), str(-near.z), str(near.y)]): return
	if not await wait_until(func(): return actor.global_position.distance_to(near) < 1.0, "unoccluded near-target observation"): return
	await activate_named(str(sample[3]))
	if failed: return
	await create_timer(1.0).timeout
	receipts.append({"phase": "enemy-request-diagnostic", "expected_target": target, "target_display": client.unit_display(target), "target": client.target_state(), "toys": client.toybox_state(), "visuals": client.spell_visuals_state(), "spells": client.spells_state()})
	write_receipts()
	if not await wait_until(func(): return target_marker(target), "source marker on selected enemy", 45000): return
	if not native_display(-1): fail("enemy marker replaced the player"); return
	receipts.append({"phase": "enemy-marker", "target": client.target_state(), "rules": client.nameplate_rules(target)})
	await capture("enemy-toy122120-source165877")

func visible_invader() -> Node3D:
	var closest: Node3D
	for unit in client.find_children("Blackrock Invader", "Node3D", true, false):
		if unit.get_node_or_null("NpcVisualRoot") == null: continue
		if closest == null or actor.position.distance_to(unit.position) < actor.position.distance_to(closest.position): closest = unit
	return closest

func target_marker(target: int) -> bool:
	for active in client.spell_visuals_state().active:
		if int(active.spell) == 179869 and int(active.unit) == target and int(active.model) in [165877, 165878]: return true
	return false

func ocean_loop() -> Dictionary:
	for sound in client.spell_visuals_state().sounds:
		if int(sound.spell) == 298869 and int(sound.sound_kit) == 138396 and bool(sound.looping) and float(sound.stopped_at) < 0:
			return sound
	return {}

func ocean_cast_started() -> bool:
	for cast in client.spell_visuals_state().casts:
		if int(cast.spell) == 298869 and bool(cast.go): return true
	return false

func prove_ocean(sample: Array) -> void:
	await activate_named(str(sample[3]))
	if failed: return
	if not await wait_until(func(): return ocean_cast_started() and has_aura(298869) and not ocean_loop().is_empty(), "actual cast and authored ocean loop138396", 60000): return
	var sound := ocean_loop()
	var nodes := client.find_children("SpellSound" + str(sound.fdid), "AudioStreamPlayer3D", true, false)
	if nodes.is_empty(): fail("ocean stream not playing"); return
	var player := nodes[0] as AudioStreamPlayer3D
	if not player.playing or player.volume_linear <= 0: fail("muted or stopped source ocean stream"); return
	# Inspect only this existing source stream's mix without altering gain or bytes.
	AudioServer.add_bus()
	var bus := AudioServer.get_bus_count() - 1
	AudioServer.set_bus_name(bus, "ToySourceProof")
	var meter := AudioEffectCapture.new()
	AudioServer.add_bus_effect(bus, meter)
	player.bus = "ToySourceProof"
	await create_timer(0.3).timeout
	var samples := meter.get_buffer(meter.get_frames_available())
	var peak := 0.0
	for frame in samples: peak = max(peak, max(abs(frame.x), abs(frame.y)))
	receipts.append({"phase": "ocean-source-audio", "sound": sound, "volume_linear": player.volume_linear, "mix_frames": samples.size(), "mix_peak": peak})
	player.bus = "Master"
	AudioServer.remove_bus(bus)
	if samples.is_empty() or peak <= 0.000001: fail("source ocean audio mixed no signal"); return
	await capture("ocean-toy168824-audio138396")
	await cancel_aura(298869)
	if not await wait_until(func(): return not has_aura(298869) and ocean_loop().is_empty(), "source ocean loop stops with aura", 30000): return
	await capture("ocean-toy168824-removed")

func capture(label: String) -> void:
	await super.capture("target-sound-" + label)
