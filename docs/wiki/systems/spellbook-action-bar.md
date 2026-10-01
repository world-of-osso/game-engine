# Spellbook, action bar and casting (Godot)

Contract: [spec](../../specs/spellbook-action-bar.md). Code: `godot/rust/src/spells.rs` (host), `godot/rust/src/player_spells.rs` (server spell state), shared rsx `src/ui/screens/spellbook_frame_component.rs` and `main_action_bar_component.rs`, the Bevy casting bar `casting_bar_frame_component.rs`, text `src/ui/cast_failed_text.rs`, data [[spell-catalog]].

## Flow

- `godot/network` subscribes to the spell messages and replicates `CastState`, `UnitPowers`, `UnitAuras` into the host `Replica` ([[godot-replication]]). `Account` keeps `PlayerSpells` (known spells, spec, 120 slots, cooldowns + GCD) and the last 64 `CombatLogEvent`s with a sequence counter; `CastFailed` becomes `AccountEvent::CastFailed`.
- `GameClient::update_spells` runs after targeting each frame: keys `ActionSlot1..12` and bar clicks send `SpellCastIntent { spell_id, spell: name, target_entity: targeting.target }` on `CombatChannel`; P toggles the book; then the bar, cast bar and book re-sync. Errors close the spell UI; they do not end the session. Committed `hud.show_action_bars` sets visibility on the cached `MainActionBarUI` during sync, leaving slot snapshots and key casts intact. Hidden bar hover is ignored; restoring visibility reuses the same UI and makes pointer casts available again.
- Cooldown shown on a button = max(spell cooldown, GCD if the spell's `StartRecoveryTime` > 0).
- **Auto-attack** (`godot/rust/src/auto_attack.rs`). Selecting a unit never attacks it. The client sends `AttackSwing` (`CMSG_ATTACK_SWING`) in three cases:
  - a right-click on an attackable unit. Attackable is the server's rule: alive, selectable, `shared::faction_reaction::can_attack`.
  - Auto Attack 6603. It is never sent as a cast; its `SPELL_EFFECT_ATTACK` is unused server-side (TrinityCore SpellEffects.cpp:170).
  - a cast of an auto-attack spell, per the catalog's `SpellAutoAttack` ([[spell-catalog]]). `OnCast` is `SpellMisc.Attributes_1 & 0x200` (SPELL_ATTR1_INITIATES_COMBAT_ENABLES_AUTO_ATTACK, "client only", TrinityCore SharedDefines.h:482); it covers Slam, Mortal Strike and the Attack action 88163. `PostCast` is `Attributes_2 & 0x100000` alone (SharedDefines.h:530, e.g. Smite); it attacks on the local player's `SpellGo`. Frostbolt has neither.
- While attacking, a new attackable selection becomes the victim, and a friendly or empty selection sends `AttackStop`. The server's `AttackStart`/`AttackStopped` echoes keep the victim in step with server-side stops (death, evade, refusals). `target_state().auto_attack` exposes the victim.
- Icons and chrome are copied from local CASC into `data/textures/{fdid}.blp` on first use; missing chrome is an error, a missing icon an empty slot.
- Combat text is a fixed-size `Label3D` under the client root (unit nodes carry model scale), in three lanes so auto-attack and ability numbers do not stack.

## Server behaviour seen live (dev server e489332, 2026-09-28)

- Level 1 warrior: known `[137047, 1464 Slam, 3127 Parry, 88163 Attack, 123829, 325446]`, bar Slam/Attack.
- Level 10 Arms (Fbcamera): class line to Execute/Heroic Throw/Battle Shout; the running server still grants Arms spells above the level (Seasoned Soldier 11, Plate Specialization 27) until it runs game-server `c252ccf` (spec spells gated by SpellLevel, learned spells pushed to the bar).
- Training dummies (`npc_training_dummy`, e.g. 44548 at Northshire -8970,-150) take hits but never lose health, so the target frame stays full; damage shows in the combat log/text. Auto-attack builds rage (Slam costs 200 tenths).
- Warrior spells up to level 10 are all instant: the casting bar has no live proof yet.

## Native managed placement

`SpellBookRoot` (formerly authored `SpellBookFrame`) is the Panel root and canonical `ui_layout.ron` key. The native client reads the selected server character's saved logical top-left on open, otherwise places it at (16, 104), clamps to its logical viewport, moves from the top 24 logical units excluding `SpellBookCloseButton`, and writes on left release. A reopened or newly launched client reads the same file; Options Reset Window Positions clears the open book's cache alongside the map. Body/tab/paging clicks are not title drag input. The downloaded Depot-built `reset-windows` fixture passes these assertions in three authenticated processes (code `c66bdfef`, script `e0d02e78`; `data/diagnostics/spellbook-placement-fixture-e0d02e78.log`). Native merchant coexistence and two-panel slot stacking are not implemented. Source: `godot/rust/src/spells.rs`, `godot/core/src/ui_layout_data.rs`, `godot/tests/options_reset_windows.gd`; contract: [window manager](../../specs/window-manager.md). See also [[world-map]].

## Fixture

`godot/tests/spellbook_cast.gd`, env `SPELL_ACCOUNT`, `SPELL_CHARACTER`, `SPELL_EXPECT_LEVEL`, `SPELL_CAST=1`, `SPELL_SHOTS`. Place the character at the dummies while offline: `game-server-admin set-position Fbworldmap -8967.3 -146.5 81.7`. A Movie Maker run (`--write-movie cast.avi --fixed-fps 30`) records the cast sequence.

Owned `native_input_fixture sound-click` uses loopback UDP to prove committed whole-main-bar off/on visibility, unchanged cached node identity and slot, a hidden bound-key `SpellCastIntent`, and restored pointer casting. The intentional restored left-down is followed by quiet right/release/keyboard/reopen pointer-effect checks; CastStart request/repeat/inactive/reset/mute/removal stages also pass. Scoped pass: `/tmp/claude/actionbar-consumer-targeted-green.log` (2026-09-29, commit `67e6e430`); verifier941 is pending, so this is not independent proof.
