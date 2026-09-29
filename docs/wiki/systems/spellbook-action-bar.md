# Spellbook, action bar and casting (Godot)

Contract: [spec](../../specs/spellbook-action-bar.md). Code: `godot/rust/src/spells.rs` (host), `godot/rust/src/player_spells.rs` (server spell state), shared rsx `src/ui/screens/spellbook_frame_component.rs` and `main_action_bar_component.rs`, the Bevy casting bar `casting_bar_frame_component.rs`, text `src/ui/cast_failed_text.rs`, data [[spell-catalog]].

## Flow

- `godot/network` subscribes to the spell messages and snapshots `CastState`, `UnitPowers`, `UnitAuras` into `UnitSnapshot`. `Account` keeps `PlayerSpells` (known spells, spec, 120 slots, cooldowns + GCD) and the last 64 `CombatLogEvent`s with a sequence counter; `CastFailed` becomes `AccountEvent::CastFailed`.
- `GameClient::update_spells` runs after targeting each frame: keys `ActionSlot1..12` and bar clicks send `SpellCastIntent { spell_id, spell: name, target_entity: targeting.target }` on `CombatChannel`; P toggles the book; then the bar, cast bar and book re-sync. Errors close the spell UI; they do not end the session.
- Cooldown shown on a button = max(spell cooldown, GCD if the spell's `StartRecoveryTime` > 0).
- Icons and chrome are copied from local CASC into `data/textures/{fdid}.blp` on first use; missing chrome is an error, a missing icon an empty slot.
- Combat text is a fixed-size `Label3D` under the client root (unit nodes carry model scale), in three lanes so auto-attack and ability numbers do not stack.

## Server behaviour seen live (dev server e489332, 2026-09-28)

- Level 1 warrior: known `[137047, 1464 Slam, 3127 Parry, 88163 Attack, 123829, 325446]`, bar Slam/Attack.
- Level 10 Arms (Fbcamera): class line to Execute/Heroic Throw/Battle Shout; the running server still grants Arms spells above the level (Seasoned Soldier 11, Plate Specialization 27) until it runs game-server `c252ccf` (spec spells gated by SpellLevel, learned spells pushed to the bar).
- Training dummies (`npc_training_dummy`, e.g. 44548 at Northshire -8970,-150) take hits but never lose health, so the target frame stays full; damage shows in the combat log/text. Auto-attack builds rage (Slam costs 200 tenths).
- Warrior spells up to level 10 are all instant: the casting bar has no live proof yet.

## Fixture

`godot/tests/spellbook_cast.gd`, env `SPELL_ACCOUNT`, `SPELL_CHARACTER`, `SPELL_EXPECT_LEVEL`, `SPELL_CAST=1`, `SPELL_SHOTS`. Place the character at the dummies while offline: `game-server-admin set-position Fbworldmap -8967.3 -146.5 81.7`. A Movie Maker run (`--write-movie cast.avi --fixed-fps 30`) records the cast sequence.
