extends RefCounted

# Character selection stays hidden while the selected character preview loads.
# A card click can start that gate again for the newly selected character.
static func wait_for_reveal(tree: SceneTree, client: Node, card_name: String, selected_name: String = "", timeout_ms: int = 180000) -> bool:
	var deadline := Time.get_ticks_msec() + timeout_ms
	while Time.get_ticks_msec() < deadline:
		await tree.process_frame
		var ui := client.get_node_or_null("CharacterSelectUI")
		if ui == null:
			continue
		var card := ui.find_child(card_name, true, false) as Control
		var enter := ui.find_child("EnterWorld", true, false) as Control
		var selected := ui.find_child("CharSelectCharacterName", true, false) as Label
		if card == null or enter == null or selected == null:
			continue
		if card.is_visible_in_tree() and enter.is_visible_in_tree() and (selected_name.is_empty() or selected.text == selected_name):
			return true
	return false
