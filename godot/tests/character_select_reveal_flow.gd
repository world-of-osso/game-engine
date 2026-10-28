extends "res://tests/world_units_flow.gd"

# Exercises the real two-character roster, preview reveal and server response.
func select_second_character(client: Node) -> void:
	var ui := client.get_node("CharacterSelectUI")
	var card := ui.find_child("CharCard_1", true, false) as Control
	var name_label := ui.find_child("CharCard_1Name", true, false) as Label
	var selected := ui.find_child("CharSelectCharacterName", true, false) as Label
	var enter := ui.find_child("EnterWorld", true, false) as Control
	if card == null or name_label == null or selected == null or enter == null:
		fail("Character selection controls are missing")
		return
	var expected := name_label.text
	if expected.is_empty() or selected.text == expected or not card.is_visible_in_tree():
		fail("The second card must be visible and initially unselected")
		return
	await click_control(card)
	if not await CharacterSelectReveal.wait_for_reveal(self, client, "CharCard_1", expected):
		fail("The second character did not become visible after selection")
		return
	if client.account_state().selected_character_id != null:
		fail("Preview selection must not establish a server character")
		return
	await click_control(enter)
	var deadline := Time.get_ticks_msec() + 60000
	while Time.get_ticks_msec() < deadline:
		await process_frame
		var state: Dictionary = client.account_state()
		if state.selected_character_name == expected and state.selected_character_id != null and state.world_attached:
			print("PASS: second roster card reveals, selects, and enters the world")
			client.free()
			quit(0)
			return
	fail("Selected character did not enter the world: " + str(client.account_state()))
