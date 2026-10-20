extends "res://tests/trading_post_monthly_clock.gd"
# Offline startup only: no authentication, character entry or server interaction.
func run_completion() -> void:
	completion = CompletionContract.new(self, 1)
	var result: bool = await startup_case()
	if client != null:
		client.free()
	completion.finish(result)

func startup_case() -> bool:
	if not completion.step(await initialize_monthly_client(), "native startup"): return false
	if not completion.step(await wait_until(login_ready, "asset-backed LoginUI", 180000), "LoginUI ready"): return false
	printerr("CASE_STARTED trading_post_monthly_startup")
	await RenderingServer.frame_post_draw
	var image := root.get_texture().get_image()
	if not completion.require(not image.is_empty() and image.get_size() == Vector2i(1920, 1080), "rendered native startup frame"): return false
	if not completion.require(image.save_png(shots.path_join("startup-login.png")) == OK, "native startup PNG"): return false
	var file := FileAccess.open(receipts.path_join("startup.json"), FileAccess.WRITE)
	if not completion.require(file != null, "startup receipt"): return false
	file.store_string(JSON.stringify({"account": client.account_state(), "LoginUI_visible": true, "size": [1920,1080]}, "\t"))
	file.close()
	return completion.case_done()

func login_ready() -> bool:
	var login := client.get_node_or_null("LoginUI") as CanvasLayer
	return login != null and login.visible and str(client.account_state().screen) == "Login"
