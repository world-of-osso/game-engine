extends "res://tests/toy_effects14_live.gd"
# Source-audited cast-event kits: the authored model plays on the caster with no aura.
func proof_cases() -> Array:
	return [
		[122119, 103740, [1627561], "Everlasting Darkmoon Firework"],
		[127668, 187150, [192714, 896706], "Jewel of Hellfire"],
	]

func prove_toy(sample: Array) -> void:
	var item := int(sample[0])
	var spell := int(sample[1])
	await learn_toy(item)
	if failed or not learned_toy(item): return
	await click("CollectionsMicroButton")
	await click("CollectionsJournalTab3")
	if not await wait_until(func(): return control("ToyBoxSearchBox") != null, "Toy Box"): return
	await click("ToyBoxSearchBox")
	var edit := control("ToyBoxSearchBox") as LineEdit
	edit.select_all()
	await type_text(str(sample[3]))
	if not await wait_until(func(): return control("ToySpellButton1Name") != null and control("ToySpellButton1Name").text == str(sample[3]), "search " + str(item)): return
	await click("ToySpellButton1")
	await click("CollectionsMicroButton")
	if not await wait_until(func(): return kit_model(spell, sample[2]) != 0, "source cast kit model", 30000): return
	if has_aura(spell): fail("cast kit created an aura"); return
	if not native_display(-1): fail("cast kit replaced native player model"); return
	receipts.append({"phase": "cast-kit", "item": item, "spell": spell, "model": kit_model(spell, sample[2])})
	await RenderingServer.frame_post_draw
	var path := OS.get_environment("TOYFX_SHOTS").path_join("%s-castkit-toy%d.png" % [skin, item])
	var image := root.get_texture().get_image()
	image.resize(1280, 720, Image.INTERPOLATE_LANCZOS)
	if image.save_png(path) != OK: fail("PNG capture"); return
	await create_timer(4.0).timeout

func kit_model(spell: int, models: Array) -> int:
	var local_id := int(client.account_state().local_player_id)
	for active in client.spell_visuals_state().active:
		if int(active.spell) == spell and models.has(int(active.model)) and int(active.unit) == local_id:
			return int(active.model)
	return 0
