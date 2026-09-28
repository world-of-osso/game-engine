# Native audio shutdown leaks

At `741344ca`, the bounded native `NativeSound` fixture proves local MP3/Ogg stream loading, natural music completion, per-zone sequencing, live volume/mute/music-enable changes, format detection, zone changes, and node lifecycle. It does not prove audible output, in-world wiring, or a clean final audio shutdown.

## Runtime implementation

`godot/rust/src/sound.rs` owns two `AudioStreamPlayer` children (`Music` and `Ambient`). It discovers local tracks, parses the shared music and ambient catalogs, removes ambient tracks from music, caches decoded Godot streams, selects per-zone tracks, and applies live `SoundOptionsFile` volumes. It detects Ogg by bytes, MP3 by ID3 or MPEG-frame header, WAV by RIFF/WAVE, and rejects FLAC explicitly. `exit_tree` stops both channels, clears their player streams, and clears the stream cache.

The retained headless run at `741344ca` exits 0 and prints its PASS marker. Its expected negative cases also log a missing catalog and unsupported FLAC. Verbose shutdown reports 16 leaked `AudioStream*`, `AudioStreamPlayback*`, and Ogg packet objects, but no `NativeSound` or `AudioStreamPlayer` node.

## Bounded shutdown comparison

The pure-GDScript control creates the same two player kinds, loads MP3/Ogg, plays them, waits for natural MP3 completion, switches track, changes volume, stops, clears streams, queues both nodes for free, waits one frame, then waits a real 0.5-second `SceneTree` timer before quitting. Its retained verbose result is exit 0 with zero leaked classes.

The immediate variant is otherwise the same but omits the 0.5-second timer. It exits 0 and prints the same PASS marker, while verbose shutdown reports six stream/playback/Ogg-packet leaks. This controls the timing difference: Rust-specific node ownership is not necessary to produce this class of shutdown leak. It does not establish that all sixteen native leaked objects have the same owner or lifetime.

A native fixture copy includes the same 0.5-second wait, but no retained result log is available. Its source is not independent timing proof and does not justify a production delay.

## Source diagnosis

The retained Godot 4.7.2 source snapshot shows `AudioStreamPlayerInternal` removes finished playback references during its process path after the audio server no longer reports them active. Stop/fade-out removal is processed through the audio mix path and deferred cleanup structures. The Dummy driver repeatedly calls the audio-server process while active; shutdown signals its thread and joins it, but does not force an additional final mix/cleanup pass. This explains why shutdown timing is a plausible common mechanism, not a proven owner-level root cause for every native leaked reference.

## Status

Independent 811 verification at `cda9a648` passes Godot `fmt --check`, native `cargo check -p game-engine-godot`, and root `cargo check --locked -p game-engine --bin game-engine`. Existing `NativeWmoGroup::fdid` and `skeleton_afid` warnings remain; the protected root `Cargo.lock` SHA-256 is unchanged. No production shutdown delay is proposed. Godot 4.7.2 remains pinned; the engine-level shutdown-timing issue is documented but will not be patched now, and client work continues. This is not a full-parity waiver. The targeted fixture establishes headless object/state behavior at `741344ca` only; actual `GameClient` zone-to-sound/GUI integration and audible output remain unproven.

## Sources

- [native sound implementation](../../../godot/rust/src/sound.rs) — `NativeSound` ownership, decoding, catalog, options, and teardown
- [native sound fixture](../../../godot/tests/native_sound.gd) — targeted behavior and lifecycle boundary
- [native verbose run](../../../data/diagnostics/audio-source/native-sound-leak-verbose.log) — exit-0 PASS and 16 non-node leaked audio objects
- [timed pure-GDScript control](../../../data/diagnostics/audio-leak-control/control.gd) — 0.5-second shutdown timing
- [timed control result](../../../data/diagnostics/audio-leak-control/result.txt) — exit 0 and zero leaked classes
- [immediate pure-GDScript control](../../../data/diagnostics/audio-leak-control/control_immediate.gd) — same control without timer
- [immediate control log](../../../data/diagnostics/audio-leak-control/immediate.log) — exit-0 PASS and six leaked audio objects
- [Godot 4.7.2 source snapshot](../../../data/diagnostics/audio-source/SOURCE.md) — upstream version and retained sources

## See Also

- [[sound]] — audio catalog and backend boundary
- [[godot-conversion]] — native-client conversion status
