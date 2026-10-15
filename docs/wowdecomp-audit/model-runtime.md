# Model events and sequence scheduling

[Scope and pins](README.md). These are conditional source-semantic gaps, not proof of affected real assets. Existing action-event support and variation selection are not absent.

## A4: General authored events lose dispatch, timing context and payload

**Reference.** `W/Engine/Source/Model2/M2Model.cpp:5906–5978` traverses authored event timestamps, selects ordinary versus global timing, filters the relevant bone ancestry, evaluates the pose at an event boundary and queues identifier, data, bone identity, transformed position and event-relative time. The callback queue is invoked at `:6438–6453`, processed recursively at `:6457–6561`, with the scene reaching callback processing at `W/Engine/Source/Model2/M2Scene.cpp:62`.

**Current subset.** `E/godot/core/src/asset/m2_format/m2_event.rs:24–75` retains identifier, data, bone, position and timestamps, but not the track's global-sequence selector. Native `E/godot/rust/src/animation/action.rs:29–35,69–103` selects only `$SCD/$CSS/$CAH` action notifications and the first `$CSL/$CSR/$CST` missile-release time. `:244–256` reports passed action events as identifiers only. `E/godot/rust/src/world_combat.rs:303–312` carries unit identity plus identifier to `E/godot/rust/src/spell_melee.rs:99–112`; this is real partial support, not a general event record dispatch. Ordinary sequence advancement at `E/godot/rust/src/animation/mod.rs:648–706` has no corresponding authored-event traversal.

**Required change.** Preserve and dispatch the supported authored event contract over the actually traversed sequence/global interval, including event data and bone-transformed event position/time. Extend beyond the selected action notifications without duplicating the existing missile/melee/voice behavior. Do not invent effects for unknown event IDs: general dispatch and individual event-effect handlers are separate coverage boundaries.

**Acceptance.** A looping ordinary sequence has an event at 100 ms with a known identifier, payload, bone 2 and a nonzero bone-relative position. Advance 90→110 ms and observe exactly one event with its authored fields and the event-time world position. Repeat across a wrap, several timestamps in one update, sequence replacement and a global-sequence event; events outside the traversed interval must not fire. Include the existing six action/missile IDs as regression controls. This proposes an observable event fixture, not proof that every event ID has a gameplay handler or that an affected asset was observed.

## A5: Replay bounds do not control sequence residence

**Reference.** `W/Engine/Source/Model2/M2Model.cpp:1572–1614` chooses an authored replay count, promotes zero to one, and stores both that count and an end time based on `repeats × sequence duration / speed`. Preserve the actual integer selection formula; its upper-bound arithmetic must not be replaced with an assumed inclusive random range.

**Current.** `E/godot/core/src/asset/m2_format/m2_anim.rs:112–122` retains replay bounds. `E/godot/rust/src/animation/mod.rs:648–706` clamps a non-looping clip after one duration, wraps an ordinary loop by modulo, or chooses a variation at each single-duration boundary. No replay budget delays completion or variation reselection in this path.

**Required change.** Let authored repeats control how long a selected sequence remains active before the next applicable completion/variation decision, while sampling within the repeated clip correctly. Preserve default zero-bounds behavior, playback speed, continuous crossfades and already implemented variation-family probabilities. This is missing repeat scheduling, not missing variation parsing.

**Acceptance.** Use duration 100 ms and deterministic bounds `[3,3]` at speed 1: the selected sequence's residence budget is 300 ms, not 100 ms. Its pose cycles at 100/200 ms but must not complete or reselect a different variation early. Check 299→300 ms, `[0,0]` minimum-one behavior, non-unit playback speed and a large elapsed interval. Use fixed equal bounds first so the reference's random-range endpoints cannot invalidate the test. Real-asset incidence and native proof remain unknown.
