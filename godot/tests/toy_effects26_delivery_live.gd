extends "res://tests/toy_effects26_bauble_live.gd"
# Diagnostic producer→receipt ordering.90067 is learned, never activated.
func prove_toy(sample: Array) -> void:
	record_delivery("before-learning")
	await learn_toy(90067)
	if failed: return
	if not learned_toy(90067): fail("source necklace ownership missing"); return
	record_delivery("after-learning")
	await super.prove_toy(sample)

func record_delivery(label: String) -> void:
	receipts.append({"phase": label, "ticks_ms": Time.get_ticks_msec(), "frame": Engine.get_process_frames(),
		"toys": client.toybox_state(), "bags": client.merchant_state().bags, "spells": client.spells_state(),
		"auras": client.aura_state(), "visuals": client.spell_visuals_state()})
	write_receipts()

func fail(message: String) -> void:
	if client != null: record_delivery("failure: " + message)
	super.fail(message)
