extends "res://tests/toy_effects14_warning_signs_live.gd"
# Genuine sign cooldowns matured before this capture; dice use actual journal/UI chat.
func proof_cases() -> Array:
	var cases := super.proof_cases()
	cases.append([36863, 47770, 100, "Decahedral Dwarven Dice", 1])
	cases.append([36862, 47776, 6, "Worn Troll Dice", 2])
	return cases

func prove_toy(sample: Array) -> void:
	if int(sample[0]) == 36863 or int(sample[0]) == 36862:
		await prove_dice(sample)
	else:
		await super.prove_toy(sample)

func prove_dice(sample: Array) -> void:
	await learn_toy(int(sample[0]))
	if failed: return
	var before := dice_rolls(int(sample[2])).size()
	await click("CollectionsMicroButton")
	await click("CollectionsJournalTab3")
	await click("ToyBoxSearchBox")
	var edit := control("ToyBoxSearchBox") as LineEdit
	edit.select_all()
	await type_text(str(sample[3]))
	if not await wait_until(func(): return matching_toy_button(str(sample[3])) != "", "source dice search"): return
	await click(matching_toy_button(str(sample[3])))
	await click("CollectionsMicroButton")
	if not await wait_until(func(): return dice_rolls(int(sample[2])).size() == before + int(sample[4]), "source native dice rolls", 30000): return
	var rolls := dice_rolls(int(sample[2]))
	for value in rolls:
		if value < 1 or value > int(sample[2]): fail("native dice value outside source range"); return
	var chat := " ".join(chat_rows())
	if not chat.contains(str(sample[3])): fail("authored dice emote/item name absent from native chat"); return
	receipts.append({"phase": "native-dice-chat", "item": sample[0], "spell": sample[1], "maximum": sample[2], "new_count": sample[4], "rolls": rolls, "chat_rows": chat_rows()})
	await capture("toy%d-authored-emote-rolls" % int(sample[0]))

func dice_rolls(maximum: int) -> Array:
	var expression := RegEx.new()
	if expression.compile(character + " rolls (\\d+) \\(1-" + str(maximum) + "\\)") != OK:
		fail("dice row expression"); return []
	var rolls: Array = []
	for row in chat_rows():
		var found := expression.search(row)
		if found != null: rolls.append(int(found.get_string(1)))
	return rolls

func chat_rows() -> PackedStringArray:
	var rows := PackedStringArray()
	for row in client.find_children("ChatFrame1MessagesRow*", "Control", true, false):
		if str(row.name).contains("Run") or not row.is_visible_in_tree(): continue
		var parts := PackedStringArray()
		for label in row.find_children("*", "Label", true, false):
			if label.is_visible_in_tree(): parts.append(label.text)
		rows.append("".join(parts))
	return rows
