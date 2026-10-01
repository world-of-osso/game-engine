extends "res://tests/world_auction_flow.gd"

# Read-only native paging/category proof against main's CLI-verified large market.
func run_test() -> void:
	root.size = Vector2i(1280, 720)
	npc_name = OS.get_environment("GODOT_AUCTION_NPC")
	if OS.get_environment("GODOT_AUCTION_CLI_PROVED") != "1" or not OS.get_environment("GODOT_TEST_SERVER").begins_with("127.0.0.1:") or npc_name.is_empty():
		fail("Large-market fixture requires prior CLI proof and owned loopback")
		return
	client = load("res://scenes/client.tscn").instantiate()
	root.add_child(client)
	if not await wait_until(func(): return client.account_state().screen == "InWorld", "world", WAIT_MS) or not await open_auction():
		return
	var revision: int = client.auction_state().search_revision
	if not await click_action("auction_search") or not await wait_until(func(): return client.auction_state().search_revision > revision, "global browse"):
		return
	var first: Array = client.auction_state().groups.map(func(row): return row.item_id)
	var total: int = client.auction_state().search_total
	if first.size() != 50 or total <= 50:
		fail("Large fixture must contain a real second item page")
		return
	if not await click_name("AuctionRowsNext") or client.auction_state().row_page != 1:
		fail("Next visible rows did not advance")
		return
	await capture_auction("auction-browse-rows.png")
	revision = client.auction_state().search_revision
	if not await click_name("AuctionPageNext") or not await wait_until(func(): return client.auction_state().search_revision > revision, "second server page"):
		return
	var second: Array = client.auction_state().groups
	if second.size() != 50 or client.auction_state().search_page != 1 or second.any(func(row): return row.item_id in first):
		fail("Distinct-item pagination overlapped or missed its second page")
		return
	await capture_auction("auction-browse-page2.png")
	revision = client.auction_state().search_revision
	if not await click_action("auction_category:7") or not await wait_until(func(): return client.auction_state().search_revision > revision, "server category"):
		return
	if client.auction_state().search_total <= 0 or client.auction_state().search_total >= total or client.auction_state().search_page != 0:
		fail("Category did not filter the full catalog and reset its page")
		return
	await capture_auction("auction-browse-category.png")
	print("FIXTURE NATIVE_AUCTION_BROWSE_DONE total=", total, " category_total=", client.auction_state().search_total)
	client.free()
	quit(0)
