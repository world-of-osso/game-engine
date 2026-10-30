extends "res://tests/quest_overflow_capture.gd"

# Offline rendering/clip proof only. Relog and party authority need the live fixture.
func run_rewards_case(forever: bool, long_text: bool) -> void:
	var probe = ClassDB.instantiate("UiAuditProbe")
	root.add_child(probe)
	require(probe.mount_quest_overflow(forever, "log", long_text).is_empty())
	await settle()
	var clip := node(probe, "QuestMapRewardsFrameContainer")
	var rewards := node(probe, "QuestMapRewardsFrame")
	var scroll := node(probe, "QuestMapDetailsScrollFrame")
	var money := node(probe, "QuestInfoMoneyText") as Label
	require(money.get_theme_color("font_color") == Color.WHITE, "Map money must use GameFontHighlight, not dark parchment text")
	require(node(probe, "QuestInfoMoneyIconTexture").size == Vector2(30, 30), "Money must use SmallItemButtonTemplate icon")
	require(clip.clip_contents, "Reward container must clip native children")
	var scale: float = clip.get_global_transform().get_scale().y
	require(abs(clip.size.x - 307.0) < 1, "Reward width differs from XML")
	var background := node(probe, "QuestMapDetailsBackground")
	require(background.size.y <= 440.0, "Parchment exceeds source height cap")
	var skin := "forever" if forever else "modern"
	var prefix := "%s-rewards-%s" % [skin, "long" if long_text else "short"]
	var initial_bottom := clip.get_global_rect().end.y
	await image(prefix + "-top.png")
	if long_text:
		require(abs(clip.size.y - 124.0) < 1, "Multiple rows must begin at source minimum reveal")
		await drag_thumb(probe, "QuestMapDetailsScrollFrame", 1.0)
	else:
		require(probe.quest_scroll_metrics("QuestMapDetailsScrollFrame").y == 0, "Short story should not scroll")
	require(abs(clip.get_global_rect().end.y - initial_bottom) < scale, "Bottom reward anchor moved")
	require(abs(clip.size.y - rewards.size.y) < 1, "End must reveal whole reward frame")
	require(clip.get_global_rect().grow(1).encloses(money.get_global_rect()), "Money crosses reward container")
	var final_image := await image(prefix + "-bottom.png")
	rewards.hide()
	await settle()
	assert_clipped_pixels(final_image, root.get_texture().get_image(), clip.get_global_rect())
	rewards.show()
	await settle()
	print("PASS QUEST_REWARD_CONTAINER skin=", skin, " long=", long_text, " clip=", clip.get_global_rect(), " money=", money.get_global_rect(), " scroll=", scroll.get_global_rect())
	probe.free()
	await process_frame

func run_test() -> void:
	root.size = Vector2i(1920, 1080)
	shots = OS.get_environment("QUEST_OVERFLOW_SHOTS")
	require(not shots.is_empty())
	DirAccess.make_dir_recursive_absolute(shots)
	for forever in [true]:
		for long_text in [false, true]:
			await run_rewards_case(forever, long_text)
	print("QUEST_REWARDS_CAPTURE_PASS cases=4")
	quit(0)
