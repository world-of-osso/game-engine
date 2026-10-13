extends "res://tests/toy_effects14_live.gd"
# Actual bag learning and journal use; one authored Personalized Weather aura, natural expiry.
const WEATHER := [46740, 46739, 46738, 46736]

func proof_cases() -> Array:
	return [[35227, 46203, 0, "Goblin Weather Machine - Prototype 01-B"]]

func prove_toy(sample: Array) -> void:
	await learn_toy(int(sample[0]))
	if failed: return
	await click("CollectionsMicroButton")
	await click("CollectionsJournalTab3")
	await click("ToyBoxSearchBox")
	var edit := control("ToyBoxSearchBox") as LineEdit
	edit.select_all()
	await type_text("Goblin Weather Machine")
	if not await wait_until(func(): return control("ToySpellButton1Name") != null and control("ToySpellButton1Name").text.begins_with("Goblin Weather Machine"), "source weather search"): return
	await click("ToySpellButton1")
	await click("CollectionsMicroButton")
	if not await wait_until(func(): return active_weather().size() == 1, "one authored personal weather aura", 20000): return
	var weather: int = active_weather()[0]
	var started := Time.get_ticks_msec()
	await capture("toy35227-weather%d" % weather)
	if not await wait_until(func(): return active_weather().is_empty(), "authored 30s weather expiry", 45000): return
	var lasted := Time.get_ticks_msec() - started
	if lasted < 25000: fail("weather ended before its authored duration"); return
	receipts.append({"phase": "source-weather-expiry", "weather": weather, "observed_ms": lasted})
	await capture("toy35227-weather-expired")

func active_weather() -> Array:
	var found: Array = []
	for spell in WEATHER:
		if has_aura(spell): found.append(spell)
	return found
