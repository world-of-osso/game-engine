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

Retire the patch with the pin once an official Godot release removes the quadratic
cleanup.
