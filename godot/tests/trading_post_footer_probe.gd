extends "res://tests/trading_post_live.gd"

func layout_proof() -> void:
	if not await choose_category(1): return
	if not await select_first_available(int(client.trading_post_state().tender)): return
	var purchase := control("PerksProgramPurchaseButton")
	for signal_name in ["button_down", "button_up", "pressed"]:
		purchase.connect(signal_name, func(): print("TP_PURCHASE_SIGNAL ", signal_name))
	await click("PerksProgramPurchaseButton")
	print("TP_PURCHASE_STATE ", client.trading_post_state())
	if not await wait_until(func(): return control("PerksProgramConfirmDialog") != null, "source popup"): return
	await capture("purchase-confirm")
	await tap(KEY_ESCAPE)
	finish("footer-probe")
