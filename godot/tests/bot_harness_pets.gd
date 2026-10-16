extends RefCounted
## One ordinary pet-bar Assist click, completed only by received native PetSpells.

const REACT_ASSIST := 3
const ACT_REACTION := 6

func same_owned_pet(state: Dictionary, pet: int) -> bool:
    return state.get("local_pet") != null and state.get("pet") != null \
        and int(state["local_pet"]) == pet and int(state["pet"]) == pet

func apply_assist(tree: SceneTree, client: Node, request: Dictionary) -> Dictionary:
    var pet := int(str(request["pet"]))
    var before: Dictionary = client.pet_bar_state()
    if not same_owned_pet(before, pet):
        return {"error": "Requested native owned pet is not replicated with its received bar"}
    if int(before["react_state"]) == REACT_ASSIST:
        return {"before": before, "after": before, "input": "native Assist already received"}
    var name := str(request["name"])
    var slot := name.trim_prefix("PetActionButton").to_int() - 1
    var buttons: Variant = before["buttons"]
    if slot < 0 or slot >= buttons.size():
        return {"error": "Requested native pet button is absent"}
    var packed := int(buttons[slot])
    if (packed >> 24) != ACT_REACTION or (packed & 0xffffff) != REACT_ASSIST:
        return {"error": "Requested native pet button is not the received Assist reaction"}
    var button := tree.root.find_child(name, true, false)
    if not button is Control or not button.is_visible_in_tree():
        return {"error": "Native Assist pet button is not visible"}
    if button is BaseButton and button.disabled:
        return {"error": "Native Assist pet button is disabled"}
    await click_button(tree, button)
    var deadline := Time.get_ticks_msec() + 8000
    while Time.get_ticks_msec() < deadline:
        var after: Dictionary = client.pet_bar_state()
        if not same_owned_pet(after, pet):
            return {"error": "Owned pet changed while awaiting native Assist acknowledgement"}
        if int(after["react_state"]) == REACT_ASSIST:
            return {"before": before, "after": after, "name": name,
                "input": "one native viewport mouse press/release; received PetSpells react_state3"}
        await tree.process_frame
    return {"error": "Native Assist acknowledgement missing after one click; input not repeated"}

func click_button(tree: SceneTree, button: Control) -> void:
    var point := button.get_global_rect().get_center()
    var motion := InputEventMouseMotion.new()
    motion.position = point
    motion.global_position = point
    tree.root.push_input(motion)
    await tree.process_frame
    var event := InputEventMouseButton.new()
    event.position = point
    event.global_position = point
    event.button_index = MOUSE_BUTTON_LEFT
    event.pressed = true
    tree.root.push_input(event)
    await tree.process_frame
    event.pressed = false
    tree.root.push_input(event)
    await tree.process_frame
