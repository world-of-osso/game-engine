# Character Preview Double Free

The paperdoll teardown at `d7c05ba7` reproducibly calls `Gd::free()` on a child already destroyed by its UI parent. This matches the reported `gd.rs:911` panic after suspend/reconnect, but the original report has no full stack or record of whether the character preview existed; attribution of that particular incident remains conditional.

## Ownership and reconnect path

`CharacterFrame::reset` originally freed `self.ui` at `character_frame.rs:57`, then called `self.preview.reset()` at line 59. `Scene::new` attaches `CharacterModelView` under a Control owned by that RegistryUi (`character_frame/preview.rs:156`). Freeing the UI recursively destroys the view, viewport, scene root, camera and model. `ModelPreview::reset` then takes the retained scene and calls `scene.view.free()` (`character_frame/preview.rs:48`).

`GameClient::process` → `run_frame` → `update_character_frame` invokes that reset whenever the screen is not InWorld (`character_frame.rs:89-95`). Token reconnect initially retains InWorld, but successful authentication with auto-selection transitions to Loading (`godot/session/src/lib.rs:254-265`); the Character frame step then tears down the previous paperdoll. Login, logout and world-loading transitions reach the same boundary. The frame need not still be visible if its model scene has not yet been cleared.

The fix resets the preview before freeing either UI parent. `ModelPreview::reset` consumes its scene handle; subsequent resets do nothing. No instance-validity guard, fallback, or deferred free was added. Tab changes already reset the preview before changing tabs; UI allocation-error frees do not own a stored preview.

## Free-site audit

Audit baseline: `d7c05ba7`, all `.free()` calls in `godot/rust/src`, including allocation/error paths reached through frame actions, account events, replication, world rebuilds and native IPC. Line numbers below refer to that baseline; debug scenes with their own process callbacks were inspected separately, not treated as GameClient world state.

| Sites (Rust paths relative to `godot/rust/src`) | Ownership result |
|---|---|
| `character_frame.rs:57,318,348`; `character_frame/preview.rs:48,132,182` | Parent-first reset was unsafe; fixed. Allocation failures free unpublished nodes; model replacement consumes its old handle. |
| `lib.rs:2308,2311,2314,2353,2384,2406,2410,2421,2425,2441,2445,2476,2485` | Screen fields are taken/replaced before free; failed allocations are unpublished; imported model container consumes its previous handle. |
| `world.rs:129,1342,1362`; `world_mount.rs:104`; `world_models.rs:505`; `game_objects.rs:139,212,236,241` | Unit/object removal consumes entries. World reset clears unit handles and resets particles before freeing the world root. Dismount takes the old mount and reparents the rider before free. Failed detached models are unpublished. |
| `terrain/objects.rs:959,1195`; `terrain/material.rs:139`; `terrain/ground_detail.rs:166,220`; `terrain/horizon.rs:106`; `wmo/global.rs:135`; `wmo/collision.rs:101`; `wmo/scene.rs:273` | Roots are taken, descendant maps cleared. Unfinished WMO builds remain detached until completed, so abandon does not refree a root child. Object particle pools attach to the supplied parent (`objects.rs:572`), making their root a sibling of the object root, not its child. |
| `particles.rs:650`; `spell_effects.rs:556,564,1270,1310,1316` | World and spell particle pools reset before their enclosing root. Missiles belong to the spell root, not targets. Removed missiles leave the vector. Unit-attached effects already handle externally destroyed models; reset drops their observer handles rather than freeing those children again. |
| `lighting/mod.rs:278,417`; `camera.rs:122`; `sky_model.rs:227,278` | Sky replacements remove map entries; lighting reset clears child observers before its root. Camera reset takes its own root-level node. Sky skeleton/player removals use freshly looked-up children. |
| `character_select.rs:53,230`; `character_select/background.rs:247`; `char_create/scene.rs:68,145,172` | Selection/creation swaps take/replace child owners. Preview background resets before its parent; whole-scene reset takes the scene, dropping remaining child handles without freeing them again. |
| `targeting.rs:152,158,894`; `unit_portraits.rs:119,170,193,310`; `nameplates.rs:607,641,675`; `quests.rs:633`; `loot.rs:195` | Target UI reset clears portraits before its parent; portrait observers already handle projection/foreign-parent destruction. Nameplates belong to a root-level CanvasLayer and removed entries are discarded. Quest failed models are unpublished; loot sparkle is freshly looked up. |
| `sound_footsteps.rs:144,155`; `spell_sounds.rs:253,281,295` | Footstep players belong to NativeSound, not unit nodes; stop drains them. NativeSound stops on exit_tree before child destruction. Spell sounds already observe parent-owned lifetime and check for lost unit/missile parents. |
| `auction.rs:27,82,125`; `auras.rs:87,305,324`; `bag_cursor.rs:79,454`; `bags.rs:80,375`; `bank.rs:65,71,515,554`; `chat.rs:286`; `damage_meter.rs:43,100`; `entrance_bar.rs:89,286,346`; `game_menu.rs:56,64,111`; `launcher.rs:66,222`; `loot.rs:50,181`; `mail.rs:41,189`; `merchant.rs:77,530`; `minimap.rs:127,487`; `mirror_timers.rs:62`; `objective_tracker.rs:25,86`; `party_frames.rs:58,322`; `pet_bar.rs:71,335`; `quests.rs:106,811,831`; `spells.rs:168,172,590,646,672`; `tooltips.rs:58,237`; `trade.rs:39,198`; `vigor.rs:73`; `world_map.rs:426,446`; `xp_bar.rs:21,82`; `ui/mod.rs:543` | HUD roots are direct GameClient children, taken/replaced before free; allocation errors precede storage. Floating combat text belongs directly to GameClient; expiry queues free and removes its handle immediately. No second unguarded preview-parent teardown found. |
| `assets/mod.rs:392,406,468`; `assets/player.rs:303`; `assets/creature.rs:98`; `assets/equipment.rs:113,226,370,373` | Failed model allocations are not published to owning fields. Equipment conversion looks up the animation/skeleton under the item being converted, not retained world-owner fields. |

Separate debug scene callbacks: `particle_debug.rs:532`, `selection_debug.rs:247`, `m2_debug.rs:273`, `debug_character.rs:298`, `eula.rs:136`, `skybox_debug.rs:213`. Their whole-scene/swap paths consume scene options or freshly created models. `queue_free()` in UI projection removes frame handles from its node map before deferred deletion; transient art controls are detached before queuing. Quest marker handles are looked up, not retained for a later explicit free.

Existing validity checks on spell effects, sounds, targeting rings and portraits were not expanded or rewritten: those components intentionally observe nodes owned by other independently changing parents. No other matching unguarded double-free path was established by this audit.

## Regression and proof

`godot/tests/character_frame_teardown.gd` uses `WowCharacterFrameLifecycleProbe` to construct the actual Rust ModelPreview scene under RegistryUi and execute the actual `CharacterFrame::reset`. It checks destruction of UI and all nested scene nodes, a second reset, and three reattachment cycles separated by process frames. No model assets, account, server, or graphical display are needed.

RED: baseline plus probe compiled through the locked local Depot helper. Headless Godot 4.7.2 / godot-rust 0.5.5 emitted exactly `gd.rs:911 called free() on already destroyed object` from the first reset. A Rust panic aborts the GDScript call; the initial fixture lacked a watchdog and was bounded by the runner timeout. The retained fixture adds a watchdog so a future panic terminates the test explicitly.

GREEN: pending verification after the fix commit. The three-hour suspend and live network reconnect are not replayed: only the proven native ownership failure boundary is reproduced. This fixture uses the available trial Godot binary (`4.7.2.stable.pr123946`), not the unavailable doubly patched launcher pin; neither rendering nor compositor behavior is part of this test.

## Sources

- [Character frame teardown](../../../godot/rust/src/character_frame.rs)
- [Preview ownership and engine probe](../../../godot/rust/src/character_frame/preview.rs)
- [Session reconnect transitions](../../../godot/session/src/lib.rs)
- [Frame processing and world reset](../../../godot/rust/src/lib.rs)
- [Regression fixture](../../../godot/tests/character_frame_teardown.gd)

## See Also

- [[godot-conversion]] — native screen/frame integration
- [[networking]] — reconnect lifecycle
