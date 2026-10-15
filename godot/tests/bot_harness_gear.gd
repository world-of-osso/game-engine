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
        "MainHand": frame = "CharacterMainHandSlot"
        "OffHand": frame = "CharacterSecondaryHandSlot"
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
    await input.press_key(tree, KEY_C)
    await input.press_key(tree, KEY_B)
    return {"spec": plan["spec"], "steps": outcomes, "before": before,
        "after": client.merchant_state(), "proof": "normal native bag input and authoritative owner equipment snapshots"}
