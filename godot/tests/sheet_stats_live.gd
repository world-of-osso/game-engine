extends "res://tests/tooltips_live.gd"

## The owner-only DerivedStats on the CharacterFrame and in a spell tooltip, against a
## private server (docs/specs/character-frame.md, docs/reference/spell-description-tokens.md).
## Environment:
##   GODOT_TEST_SERVER   a private test server (not :5000)
##   SHEET_ACCOUNT / SHEET_CHARACTER  account (password fbtest) and an Arcane mage (level
##                       10 or more) with Intellect / secondary-stat gear equipped
##   SHEET_SHOTS         screenshot directory
## C opens the CharacterFrame: Attributes show Intellect but not Strength or Agility, and
## Enhancements shows Critical Strike, Haste, Mastery and Versatility percentages. The
## spellbook's Arcane Blast tooltip shows its spell power damage, not the `{?...}` token.

func run_test() -> void:
	root.size = Vector2i(1600, 900)
	var server := OS.get_environment("GODOT_TEST_SERVER")
	var account := OS.get_environment("SHEET_ACCOUNT")
	character = OS.get_environment("SHEET_CHARACTER")
	shots = OS.get_environment("SHEET_SHOTS")
	if account == "" or character == "" or shots == "" or not server.begins_with("127.0.0.1:") or server == "127.0.0.1:5000":
		fail("Set SHEET_ACCOUNT, SHEET_CHARACTER, SHEET_SHOTS and a private GODOT_TEST_SERVER (not :5000)")
		return
	DirAccess.make_dir_recursive_absolute(shots)
	client = load("res://scenes/client.tscn").instantiate()
	root.add_child(client)
	var error = client.connect_account(server, account, PASSWORD, false)
	if error != "":
		fail("Fixture connection: " + error)
		return
	if not await enter_world():
		return
	if not await wait_until(func(): return spells().catalog_ready and spells().known.has(ARCANE_BLAST), 60000, "spell catalog with Arcane Blast"):
		return
	await wait_frames(30)
	if not await character_sheet():
		return
	if not await arcane_blast_tooltip():
		return
	print("FIXTURE SHEET_STATS_LIVE_DONE")
	client.free()
	quit(0)

func character_sheet() -> bool:
	await press(KEY_C)
	if not await wait_until(func(): return sheet_line("Critical Strike:") != "", 10000, "Enhancements on the CharacterFrame"):
		return false
	await wait_frames(10)
	var lines := sheet_lines()
	print("FIXTURE SHEET ", lines)
	if sheet_line("Intellect:") == "" or sheet_line("Strength:") != "" or sheet_line("Agility:") != "":
		fail("Arcane attributes should show Intellect only: " + str(lines))
		return false
	if title_text("CharacterStatsPaneEnhancementsCategoryTitle") != "Enhancements":
		fail("No Enhancements category: " + str(lines))
		return false
	for label in ["Critical Strike:", "Haste:", "Mastery:", "Versatility:"]:
		if not sheet_line(label).ends_with("%"):
			fail("%s is not a percentage: %s" % [label, lines])
			return false
	await capture("01-character-sheet.png")
	await press(KEY_C)
	return true

func arcane_blast_tooltip() -> bool:
	await press(KEY_P)
	if not await wait_until(func(): return spells().spellbook_open, 3000, "spellbook"):
		return false
	await wait_frames(10)
	var button := control("SpellBookUI", "SpellBookItem%dButton" % ARCANE_BLAST)
	if button == null:
		fail("No Arcane Blast in the spellbook")
		return false
	await hover(button)
	if not await wait_tooltip("Arcane Blast"):
		return false
	var state := tooltip()
	print("FIXTURE ARCANE_BLAST ", state)
	var damage := line_with(state, "dealing ")
	if damage == "" or has_line(state, "{?"):
		return fail_state("Arcane Blast damage line", state)
	var amount := damage.get_slice("dealing ", 1).get_slice(" ", 0)
	if not amount.is_valid_int() or int(amount) <= 0:
		return fail_state("Arcane Blast damage is not a number (%s)" % amount, state)
	await capture("02-arcane-blast-tooltip.png")
	return true

## Every shown stats pane line as "Label Value".
func sheet_lines() -> Array:
	var lines := []
	for index in range(1, 16):
		var label := sheet_label("CharacterStatsPaneStat%dLabel" % index)
		if label != null and label.is_visible_in_tree():
			lines.append(label.text + " " + sheet_label("CharacterStatsPaneStat%dValue" % index).text)
	return lines

## The value shown for a stats pane label, or "" when it is not shown.
func sheet_line(text: String) -> String:
	for index in range(1, 16):
		var label := sheet_label("CharacterStatsPaneStat%dLabel" % index)
		if label != null and label.text == text and label.is_visible_in_tree():
			return sheet_label("CharacterStatsPaneStat%dValue" % index).text
	return ""

func title_text(name: String) -> String:
	var label := sheet_label(name)
	return label.text if label != null and label.is_visible_in_tree() else ""

func sheet_label(name: String) -> Label:
	return control("CharacterFrameUI", name) as Label
