extends "res://tests/guild_lifecycle_live.gd"

var last_news: Dictionary = {}

func run_test() -> void:
	root.size = Vector2i(1920, 1080)
	Engine.max_fps = 30
	shots = OS.get_environment("BANK_SHOTS")
	diagnostics = OS.get_environment("GUILD_NEWS_DIAGNOSTICS")
	assert(OS.get_environment("GODOT_TEST_SERVER") == "127.0.0.1:5891", "Private5891 only")
	assert(OS.get_environment("BANK_ACCOUNT") == "fb_guildcompletion_newslead", "Private news lead only")
	DirAccess.make_dir_recursive_absolute(shots)
	client = load("res://scenes/client.tscn").instantiate()
	root.add_child(client)
	await wait_frames(30)
	if not await connect_private_account() or not await enter_world("Gcnewslead"):
		return
	var mode := OS.get_environment("GUILD_NEWS_MODE")
	if mode == "produce":
		if not await produce_news():
			return
	elif mode == "overflow":
		if not await overflow_news():
			return
	elif mode == "readback":
		if not await readback_news():
			return
	else:
		fail("Unknown news proof mode")
		return
	print("PASS GUILD_NEWS_LIVE mode=", mode, " retained=", last_news.entries.size())
	client.free()
	quit(0)

func envelope() -> Dictionary:
	var ui := client.get_node_or_null("GuildRanksUI")
	if ui == null:
		return {}
	var raw := str(ui.call("guild_news_observation"))
	return {} if raw.is_empty() else JSON.parse_string(raw)

func news() -> Dictionary:
	var value := envelope()
	if value.is_empty() or value.waiting or value.reply == null:
		return {}
	return value.reply

func open_news() -> bool:
	if not await open_guild():
		return false
	await press("GuildInfoTab")
	if not await wait_until(func(): return not news().is_empty(), "actual authenticated news query"):
		return false
	last_news = news()
	return last_news.error == null or fail("News query refused: " + str(last_news))

func produce_news() -> bool:
	if not await wait_until(func(): return FileAccess.file_exists(diagnostics.path_join("initial.json")), "real buyer committed acquisitions"):
		return false
	for skin in ["Modern", "Forever"]:
		if not await select_skin(skin) or not await open_news():
			return false
		role = skin.to_lower() + "-news"
		if not await wait_until(func(): return news().entries.size() >= 2, "actual news acquisitions"):
			return false
		last_news = news()
		await capture("entries")
		await press("CommunitiesGuildNewsSetFilters")
		if not await wait_until(func(): return shown("GuildNewsFiltersFrameTitle"), "source filters"):
			return false
		await capture("filters")
		await press("GuildNewsFiltersCheck6")
		if not await wait_until(purchases_hidden, "purchase category hidden while genuine achievements remain"): 
			return false
		await capture("filtered")
		await press("GuildNewsFiltersCheck6")
		await press("GuildNewsFiltersFrameCloseButton")
		var entry: Dictionary = news().entries.back()
		var row := "CommunitiesGuildNewsEntry%d" % int(entry.id)
		await click_at(control(row).get_global_rect().get_center(), MOUSE_BUTTON_RIGHT)
		if not await wait_until(func(): return shown("GuildNewsMakeSticky"), "unclipped source context menu"):
			return false
		await capture("context")
		await press("GuildNewsMakeSticky")
		if not await wait_until(func(): return envelope().pending_sticky == null and bool(news().entries.back().sticky) != bool(entry.sticky), "server sticky ACK"):
			return false
		await capture("sticky")
		await close_guild()
	mark("overflow.go")
	return await overflow_news()

func overflow_news() -> bool:
	if not await wait_file("overflow.json"):
		return false
	for skin in ["Modern", "Forever"]:
		if not await select_skin(skin) or not await open_news():
			return false
		role = skin.to_lower() + "-news"
		if not await wait_until(func(): return news().entries.size() == 250, "authoritative retained250"):
			return false
		if skin == "Modern":
			var latest: Dictionary = news().entries.back()
			await click_at(control("CommunitiesGuildNewsEntry%d" % int(latest.id)).get_global_rect().get_center(), MOUSE_BUTTON_RIGHT)
			if not await wait_until(func(): return shown("GuildNewsMakeSticky"), "retained row menu"):
				return false
			await press("GuildNewsMakeSticky")
			if not await wait_until(func(): return envelope().pending_sticky == null and news().entries.back().sticky, "persisted real sticky"):
				return false
		await capture("overflow-top")
		if not await oldest_visible():
			return false
		await capture("overflow-bottom")
		last_news = news()
		write_json("news-history.json", last_news)
		await close_guild()
	mark("privacy.go")
	if not await wait_until(func(): return FileAccess.file_exists(diagnostics.path_join("privacy.json")), "former member news refusal"):
		return false
	if read_json("permission-refusal.json").error != "Permissions" or read_json("privacy.json").error != "NotInGuild":
		return fail("Missing actual UDP refusal receipts")
	return true

func purchases_hidden() -> bool:
	var value := envelope()
	if value.is_empty() or value.visible_ids.is_empty():
		return false # Actual Level50 news remains under its separate checked category.
	for entry in news().entries:
		if entry.kind.has("ItemPurchased") and not entry.sticky and int(entry.id) in value.visible_ids:
			return false
	return true

func wait_file(filename: String) -> bool:
	var deadline := Time.get_ticks_msec() + 1800000
	while Time.get_ticks_msec() < deadline:
		await process_frame
		await RenderingServer.frame_post_draw
		if FileAccess.file_exists(diagnostics.path_join(filename)):
			return true
	return fail("Timed out waiting for actual peer receipt " + filename)

func oldest_visible() -> bool:
	var entries: Array = news().entries
	var id := int(entries.front().id)
	var row := "CommunitiesGuildNewsEntry%d" % id
	var viewport := control("CommunitiesGuildNewsScrollBox").get_global_rect()
	if viewport.intersects(control(row).get_global_rect()):
		return fail("Overflow requires a genuine second page")
	for turn in range(200):
		var bounds := control(row).get_global_rect()
		if bounds.position.y >= viewport.position.y and bounds.end.y <= viewport.end.y:
			return true
		await click_at(viewport.get_center(), MOUSE_BUTTON_WHEEL_DOWN)
	return fail("Real news wheel never revealed oldest retained entry")

func readback_news() -> bool:
	var saved := read_json("news-history.json")
	if saved.is_empty() or saved.entries.size() != 250:
		return fail("Original real retained250 missing")
	for skin in ["Modern", "Forever"]:
		if not await select_skin(skin) or not await open_news():
			return false
		role = skin.to_lower() + "-news"
		last_news = news()
		if last_news.entries != saved.entries:
			return fail("Fresh server/client changed retained news IDs/timestamps/stickies")
		await capture("restart")
		await close_guild()
	return true
