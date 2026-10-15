extends RefCounted
## Apply only installed starter-loadout choices through actual native UI input.

func press_key(tree: SceneTree, code: Key) -> void:
    var event := InputEventKey.new()
    event.keycode = code
    event.physical_keycode = code
    event.pressed = true
    tree.root.push_input(event)
    await tree.process_frame
    event.pressed = false
    tree.root.push_input(event)
    await tree.create_timer(0.25).timeout

func click_control(tree: SceneTree, control: Control) -> void:
    var position := control.get_global_rect().get_center()
    var motion := InputEventMouseMotion.new()
    motion.position = position
    motion.global_position = position
    tree.root.push_input(motion)
    await tree.process_frame
    var event := InputEventMouseButton.new()
    event.position = position
    event.global_position = position
    event.button_index = MOUSE_BUTTON_LEFT
    event.pressed = true
    tree.root.push_input(event)
    await tree.process_frame
    event.pressed = false
    tree.root.push_input(event)
    await tree.create_timer(0.2).timeout

func rank_text(tree: SceneTree, node: int) -> String:
    var label := tree.root.find_child("TalentNode" + str(node) + "Ranks", true, false)
    if label is Label:
        return label.text
    return ""

func apply_plan(tree: SceneTree, plan: Dictionary) -> Dictionary:
    await press_key(tree, KEY_N)
    var panel: Node
    for attempt in range(100):
        panel = tree.root.find_child("TalentLoadoutDropDown", true, false)
        if panel is CanvasItem and panel.is_visible_in_tree():
            break
        await tree.create_timer(0.1).timeout
    if not panel is CanvasItem or not panel.is_visible_in_tree():
        return {"error": "Native Talents panel did not open", "loadout": plan["loadout"]}
    var observations: Array[Dictionary] = []
    for step: Dictionary in plan["steps"]:
        var record: Dictionary = step.duplicate()
        var before := rank_text(tree, int(step["node"]))
        record["before"] = before
        var button := tree.root.find_child(str(step["frame"]), true, false)
        if before.is_empty() or not button is BaseButton or not button.is_visible_in_tree():
            record["outcome"] = "not available in native tree"
        elif before.get_slice("/", 0).to_int() >= int(step["rank"]):
            record["outcome"] = "target rank already present"
        elif button.disabled:
            record["outcome"] = "native purchase disabled"
        else:
            await click_control(tree, button)
            record["after"] = rank_text(tree, int(step["node"]))
            record["outcome"] = "native click requested"
        observations.append(record)
    var apply_button := tree.root.find_child("TalentApply", true, false)
    var clicked_apply := false
    if apply_button is BaseButton and not apply_button.disabled:
        await click_control(tree, apply_button)
        clicked_apply = true
        await tree.create_timer(2.0).timeout
    var realized := {}
    for step: Dictionary in plan["steps"]:
        realized[str(step["node"])] = rank_text(tree, int(step["node"]))
    await press_key(tree, KEY_N)
    var sheet := await capture_sheet(tree)
    var receipt := {"loadout": plan["loadout"], "spec": plan["spec"], "steps": observations,
        "clicked_apply": clicked_apply, "native_ranks_after_apply": realized, "sheet_stats": sheet,
        "proof": "native input and UI ranks only; server persistence requires fresh-login receipt"}
    if sheet.is_empty():
        receipt["error"] = "Native CharacterFrame stats were not observed"
    return receipt

func capture_sheet(tree: SceneTree) -> Dictionary:
    await press_key(tree, KEY_C)
    var result := {}
    for attempt in range(100):
        for index in range(1, 16):
            var label := tree.root.find_child("CharacterStatsPaneStat%dLabel" % index, true, false)
            var value := tree.root.find_child("CharacterStatsPaneStat%dValue" % index, true, false)
            if label is Label and value is Label and label.is_visible_in_tree():
                result[label.text] = value.text
        if not result.is_empty():
            break
        await tree.create_timer(0.1).timeout
    await press_key(tree, KEY_C)
    return result
