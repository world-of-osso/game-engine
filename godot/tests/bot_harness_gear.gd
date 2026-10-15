extends RefCounted
## Explicit normal native bag-to-paperdoll cursor swaps; no admin equip bypass.

func find_control(tree: SceneTree, name: String) -> Control:
    for attempt in range(100):
        var control := tree.root.find_child(name, true, false)
        if control is Control and control.is_visible_in_tree():
            return control
        await tree.create_timer(0.1).timeout
    return null

func equipped(client: Node, slot: String, item: int) -> bool:
    for entry: Dictionary in client.merchant_state().get("equipment", []):
        if entry["slot"] == slot and int(entry["item_id"]) == item:
            return true
    return false

func bag_item(client: Node, item: int) -> Dictionary:
    for entry: Dictionary in client.merchant_state().get("bags", []):
        if int(entry["item_id"]) == item:
            return entry
    return {}

func equip_one(tree: SceneTree, client: Node, input: RefCounted, item: Dictionary) -> Dictionary:
    var identity := int(item["item_id"])
    var slot := str(item["slot"])
    if equipped(client, slot, identity):
        return {"slot": slot, "item_id": identity, "outcome": "already equipped"}
    var location := bag_item(client, identity)
    if location.is_empty():
        return {"error": "Sourced item missing from owned bags", "slot": slot, "item_id": identity}
    var frame: String
    match slot:
        "OffHand": frame = "CharacterSecondaryHandSlot"
        "Head", "Shoulder", "Chest", "Waist", "Legs", "Feet", "Wrist", "Hands", "Back", "MainHand":
            frame = "Character" + slot + "Slot"
        _: return {"error": "Unsupported fixture equipment slot", "slot": slot}
    var bag_button := await find_control(tree, "ContainerFrame%dSlot%d" % [int(location["bag"]), int(location["slot"])])
    if bag_button == null:
        return {"error": "Native source bag slot not visible", "location": location}
    # Pickup is client-local CursorItem state; it does not remove the bag item.
    # The second ordinary click sends the validated SwapItem request.
    await input.click_control(tree, bag_button)
    var target := await find_control(tree, frame)
    if target == null:
        return {"error": "Native source paperdoll slot not visible after pickup", "target": frame}
    await input.click_control(tree, target)
    for attempt in range(100):
        if equipped(client, slot, identity):
            return {"slot": slot, "item_id": identity, "outcome": "authoritative equipment snapshot confirmed"}
        await tree.create_timer(0.1).timeout
    return {"error": "Ordinary native equip was not confirmed; no alternate equipment path", "slot": slot, "item_id": identity}

func free_bag_location(tree: SceneTree, client: Node) -> Dictionary:
    var occupied: Dictionary = {}
    for entry: Dictionary in client.merchant_state().get("bags", []):
        occupied["ContainerFrame%dSlot%d" % [int(entry["bag"]), int(entry["slot"])]] = true
    for control: Control in tree.root.find_children("ContainerFrame*Slot*", "Control", true, false):
        var frame := str(control.name)
        var parts := frame.trim_prefix("ContainerFrame").split("Slot")
        if parts.size() != 2 or not parts[0].is_valid_int() or not parts[1].is_valid_int():
            continue
        if control.is_visible_in_tree() and not occupied.has(frame):
            return {"bag": int(parts[0]), "slot": int(parts[1]), "frame": frame}
    return {}

func removal_confirmed(state: Dictionary, slot: String, item: int, location: Dictionary) -> bool:
    for entry: Dictionary in state.get("equipment", []):
        if entry["slot"] == slot:
            return false
    for entry: Dictionary in state.get("bags", []):
        var at_destination := int(entry["bag"]) == location["bag"] and int(entry["slot"]) == location["slot"]
        if at_destination and int(entry["item_id"]) == item:
            return true
    return false

func unequip_one(tree: SceneTree, client: Node, input: RefCounted, slot: String) -> Dictionary:
    var original: Dictionary = {}
    for entry: Dictionary in client.merchant_state().get("equipment", []):
        if entry["slot"] == slot:
            original = entry
            break
    if original.is_empty():
        return {"slot": slot, "outcome": "already empty"}
    var frame: String
    match slot:
        "Neck": frame = "CharacterNeckSlot"
        "Finger1": frame = "CharacterFinger0Slot"
        "Finger2": frame = "CharacterFinger1Slot"
        "Trinket1": frame = "CharacterTrinket0Slot"
        "Trinket2": frame = "CharacterTrinket1Slot"
        _: return {"error": "Unsupported empty fixture slot", "slot": slot}
    var location := free_bag_location(tree, client)
    if location.is_empty():
        return {"error": "No visible empty native bag slot for jewelry", "slot": slot}
    var source := await find_control(tree, frame)
    if source == null:
        return {"error": "Native jewelry paperdoll slot not visible", "slot": slot}
    await input.click_control(tree, source)
    var target := await find_control(tree, location["frame"])
    if target == null:
        return {"error": "Native empty bag slot not visible after pickup", "slot": slot, "location": location}
    await input.click_control(tree, target)
    for attempt in range(100):
        if removal_confirmed(client.merchant_state(), slot, int(original["item_id"]), location):
            return {"slot": slot, "item_id": original["item_id"], "item_guid": original["item_guid"],
                "location": location, "outcome": "authoritative empty equipment and owned bag snapshot confirmed"}
        await tree.create_timer(0.1).timeout
    return {"error": "Ordinary native jewelry removal was not confirmed", "slot": slot, "location": location}

func apply_plan(tree: SceneTree, client: Node, plan: Dictionary) -> Dictionary:
    var input = load("res://tests/bot_harness_talents.gd").new()
    var before: Dictionary = client.merchant_state()
    var outcomes: Array[Dictionary] = []
    await input.press_key(tree, KEY_B)
    if not client.character_frame_state()["open"]:
        await input.press_key(tree, KEY_C)
    for item: Dictionary in plan["items"]:
        var outcome := await equip_one(tree, client, input, item)
        outcomes.append(outcome)
        if outcome.has("error"):
            return {"error": outcome["error"], "steps": outcomes, "before": before,
                "after": client.merchant_state()}
    for slot: String in plan["empty_slots"]:
        var outcome := await unequip_one(tree, client, input, slot)
        outcomes.append(outcome)
        if outcome.has("error"):
            return {"error": outcome["error"], "steps": outcomes, "before": before,
                "after": client.merchant_state()}
    await input.press_key(tree, KEY_C)
    await input.press_key(tree, KEY_B)
    return {"spec": plan["spec"], "steps": outcomes, "before": before,
        "after": client.merchant_state(), "proof": "normal native bag input and authoritative owner equipment snapshots"}
