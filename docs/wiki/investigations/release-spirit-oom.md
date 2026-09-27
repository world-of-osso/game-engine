# Release Spirit OOM Kill

The client "terminated" when the player released spirit in The Stockade (map 34). It was no panic: `earlyoom` sent SIGTERM at 06:31:57Z, 25 s after the server's 34 → 0 graveyard transfer, with the client at VmRSS 10348 MiB and no free swap (`journalctl`). Evidence: `../../../data/diagnostics/releasecrash-20260927/`.

## Root cause: per-instance animation data

A jemalloc heap profile (`LD_PRELOAD=libjemalloc.so.2`, `MALLOC_CONF=prof:true`, `prof.dump` through gdb) of a client in the Stockade after Stormwind had 9.6 GB live:
- 3.8 GB in `bevy_curves::build_clip` from `bind_m2_animation_players`. Each animated model instance built one `AnimationClip` per sequence. Display 2989, spawned 23 times in the Stockade, has 422 sequences.
- 3.0 GB in `load_m2_cached`. Each spawn cloned the cached `M2Model`, including every bone track.

The Stockade has 93 NPCs, mostly on the same HD humanoid models. So the dungeon cost more memory than Stormwind. A map transfer loads the new map while the old map's memory is still held, so it made the peak higher.

## Fix

- `M2Model::bone_tracks` and `M2AnimData::bone_tracks` are `Arc<[BoneAnimTracks]>`, so the per-spawn model clone shares them.
- `bind_m2_animation_players` keeps a `Local<SequenceClipCache>`. It is keyed by the tracks' address and the sequence durations, and it stores only clip asset ids. Instances share one set of clips, and the clips drop with the last instance.
- `load_m2_cached` keeps the first cache entry when two loads of the same model race.

Live (glibc, `--screen inworld`, same server data): Stockade after Stormwind went from 12.4–13.0 GB RSS to 5.1 GB. Across the release transfer to the Stormwind graveyard the peak is now 5.3 GB, where the old client was SIGTERMed at 8.7 GB.
