# Godot audio bus-details cleanup

With 40 players casting at the Stormwind dummies, `AudioServer::_cleanup_lists()` was
the largest main-thread symbol: 12.7 % and 24.2 % of main-thread samples in two profiled
Battle Shout runs (symbolized `template_release`, release extension;
`data/diagnostics/fps-120-close-2026-11-06/crowd-combat.md`).

## Cause

Each active `AudioStreamPlayer3D`, on every physics tick in which the audio mix count
moved, calls `AudioServer::set_playback_bus_volumes_linear()`. That swaps in a new
`AudioStreamPlaybackBusDetails` and puts the old one in `bus_details_graveyard`, a
`SafeList`. `AudioServer::update()` then runs `_cleanup_lists()`, which moved every entry to
`bus_details_graveyard_frame_old` and deleted the previous ones there, erasing each entry
by value. `SafeList::erase` searches from the head, and erased nodes stay linked until
`maybe_cleanup()`, so retiring k entries cost O(k²). k grows with the number of playing 3D
sounds: about three spell sounds per cast with 40 casters.

## Fix

`audio-bus-graveyard.patch` (in `scripts/godot/`, pin suffix `-audiobus`) keeps both
graveyards as `LocalVector`s; the current one is guarded by a mutex because playback bus
volumes can be set from any thread. `_cleanup_lists()` deletes the old generation and
swaps the current one in. An entry is still deleted one `update()` after it was retired,
as before, so the mix never reads freed details and every bus volume the mix applies is
the same: audio output is unchanged.

## Measurement

`godot/tests/_diag_audio_players.gd` (diagnostic, uncommitted): N looping
`AudioStreamPlayer3D` nodes, 20 fps, main-thread CPU per frame from
`/proc/thread-self/schedstat`, headless, old (`-present`) and new (`-audiobus`) editor
interleaved, host load 18-32 (`data/diagnostics/fps-120-close-2026-11-06/audio-bus-ab.txt`):

| players | old ms per frame | new ms per frame |
|---|---|---|
| 200 | 5.14-5.68 | 1.38-1.74 |
| 400 | 16.1-21.9 | 2.65-4.76 |

Doubling the players multiplies the old cost by about 3.5 and the new one by about 2.3.
The ground drape field dump on the new editor keeps all 25 hashes and array bytes.
The 40-player crowd A/B (11 interleaved runs, `profile/crowd-audio-*`) is dominated by host
load (20-152): Battle Shout main-thread CPU spans 25-51 ms on the old editor and 23-81 ms on
the new one, and the extension's own animation timer, which the patch cannot change, moves
6-40 ms with it. Relative to that timer the main thread does less elsewhere on the new
editor (median ratio about 2.6 against 3.5), but the crowd runs alone do not prove the gain.

Retire the patch with the pin once an official Godot release removes the quadratic
cleanup.
