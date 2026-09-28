extends SceneTree

const ROOT := "res://../data/diagnostics/native-sound"

func fail(message: String) -> void:
	push_error(message)
	quit(1)

func expect_playing(player: AudioStreamPlayer, name: String, volume: float) -> bool:
	if not player.is_playing() or player.stream == null or player.stream.resource_name != name or absf(player.volume_linear - volume) > 0.001:
		fail("Expected %s playing at %.3f, got %s playing=%s volume=%.3f" % [name, volume, player.stream.resource_name if player.stream != null else "none", player.is_playing(), player.volume_linear])
		return false
	return true

func _initialize() -> void:
	call_deferred("run_test")

func run_test() -> void:
	if not ClassDB.class_exists("NativeSound"):
		fail("NativeSound production node missing")
		return
	var rejected: Node = ClassDB.instantiate("NativeSound")
	get_root().add_child(rejected)
	if rejected.configure(ProjectSettings.globalize_path(ROOT + "/missing")):
		fail("Missing catalog unexpectedly accepted")
		return
	if rejected.get_node_or_null("Music") == null or rejected.get_node_or_null("Ambient") == null:
		fail("Failed load left manually allocated players unattached to their owner")
		return
	rejected.queue_free()
	await process_frame
	if is_instance_valid(rejected):
		fail("Failed-load NativeSound did not release its children")
		return
	var sound: Node = ClassDB.instantiate("NativeSound")
	get_root().add_child(sound)
	if not sound.configure(ProjectSettings.globalize_path(ROOT)):
		fail("NativeSound fixture catalog rejected")
		return
	var music := sound.get_node_or_null("Music") as AudioStreamPlayer
	var ambient := sound.get_node_or_null("Ambient") as AudioStreamPlayer
	if music == null or ambient == null:
		fail("NativeSound has no music and ambient child players")
		return
	if not sound.sync_options(5, 0.8, 0.5, 0.25, true, false):
		fail("Zone 5 sync failed")
		return
	if not expect_playing(music, "629319", 0.4) or not expect_playing(ambient, "2851182", 0.2):
		return
	if not (music.stream is AudioStreamMP3) or not (ambient.stream is AudioStreamOggVorbis):
		fail("Local bytes did not decode as MP3 and Ogg")
		return
	music.stop()
	if not sound.sync_options(5, 0.8, 0.5, 0.25, true, false) or not expect_playing(music, "629320", 0.4):
		return
	if not sound.sync_options(5, 0.8, 0.5, 0.25, true, true) or not expect_playing(music, "629320", 0.0) or not expect_playing(ambient, "2851182", 0.0):
		return
	if not sound.sync_options(5, 0.6, 0.3, 0.4, true, false) or not expect_playing(music, "629320", 0.18) or not expect_playing(ambient, "2851182", 0.24):
		return
	if not sound.sync_options(5, 0.6, 0.3, 0.4, false, false) or music.is_playing() or not ambient.is_playing():
		fail("Music disabled did not stop music while retaining ambient")
		return
	if not sound.sync_options(6, 0.6, 0.3, 0.4, true, false) or not expect_playing(music, "2109035", 0.18) or ambient.is_playing():
		return
	if not (music.stream is AudioStreamOggVorbis):
		fail("OggS data under .mp3 was not decoded as Ogg")
		return
	if not sound.sync_options(-1, 1.0, 1.0, 1.0, true, false) or music.is_playing() or ambient.is_playing():
		fail("Leaving zone did not stop both channels")
		return
	if sound.sync_options(7, 1.0, 1.0, 1.0, true, false) or music.is_playing():
		fail("Unsupported FLAC did not report a decode error")
		return
	if not sound.sync_options(7, 1.0, 1.0, 1.0, true, false) or music.is_playing():
		fail("Unsupported track retried each frame")
		return
	if not sound.sync_options(8, 1.0, 1.0, 1.0, true, false) or not expect_playing(music, "629322", 1.0) or not (music.stream is AudioStreamMP3):
		fail("ID3-less MPEG frame stream did not play")
		return
	if not sound.sync_options(5, 1.0, 1.0, 1.0, true, false) or not expect_playing(music, "629319", 1.0):
		return
	sound.queue_free()
	await process_frame
	print("PASS: local MP3/Ogg streams, per-zone sequence, live volumes/mute/disable, Ogg magic, zone changes, lifecycle")
	quit(0)
