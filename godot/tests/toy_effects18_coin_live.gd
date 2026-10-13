extends "res://tests/toy_effects14_dice_live.gd"
# Actual bag learning and journal use; native chat shows the authored toss then one catch.
const TOSS := "casually flips"
const HEADS := "catches the coin heads up!"
const TAILS := "finds the coin face down for tails!"

func proof_cases() -> Array:
	return [[44430, 60458, 0, "Titanium Seal of Dalaran", 0]]

func prove_toy(sample: Array) -> void:
	await learn_toy(int(sample[0]))
	if failed: return
	var before := coin_rows()
	await click("CollectionsMicroButton")
	await click("CollectionsJournalTab3")
	await click("ToyBoxSearchBox")
	var edit := control("ToyBoxSearchBox") as LineEdit
	edit.select_all()
	await type_text(str(sample[3]))
	if not await wait_until(func(): return matching_toy_button(str(sample[3])) != "", "source coin search"): return
	await click(matching_toy_button(str(sample[3])))
	await click("CollectionsMicroButton")
	var tossed_at := Time.get_ticks_msec()
	if not await wait_until(func(): return count_rows(TOSS) == before[0] + 1, "authored coin toss emote", 15000): return
	if not await wait_until(func(): return count_rows(HEADS) + count_rows(TAILS) == before[1] + 1, "authored coin catch emote", 15000): return
	var caught_ms := Time.get_ticks_msec() - tossed_at
	if caught_ms < 1500: fail("catch arrived before source flight"); return
	await create_timer(3.0).timeout
	if count_rows(HEADS) + count_rows(TAILS) != before[1] + 1: fail("more than one catch per toss"); return
	var chat := " ".join(chat_rows())
	if chat.contains("|H") or chat.contains("|c"): fail("raw control tokens in native chat"); return
	if not chat.contains(str(sample[3])): fail("authored coin item label absent"); return
	receipts.append({"phase": "native-coin-chat", "item": sample[0], "spell": sample[1], "catch_after_click_ms": caught_ms, "chat_rows": chat_rows()})
	print("TOYFX_COIN_CHAT ", chat_rows())
	await capture("toy44430-toss-catch")

func coin_rows() -> Array:
	return [count_rows(TOSS), count_rows(HEADS) + count_rows(TAILS)]

func count_rows(text: String) -> int:
	var count := 0
	for row in chat_rows():
		if row.contains(text): count += 1
	return count
