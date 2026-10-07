# Death and resurrection UI

> Historical Bevy contract: root `src/` paths below name files deleted with the [retired Bevy client](godot-conversion.md#retired-bevy-client-user-decision-2026-10-02). Current native contract and final per-step PASS/FAIL table live in [Player death flow](death-flow.md#live-acceptance--2026-10-07). Native re-proof passes ghost grading/corpse pin, spirit-healer sickness and local corpse animation; end-to-end logout/relog remains blocked by server `f25258b` death-state restoration.

Retail-style death flow on the live `DeathStateUpdate` stream (phase 3 of [in-game UI plan](../plans/2026-09-23-ingame-ui.md)). UI lives in `src/scenes/death_ui/`; network state and actions in `src/death.rs`.

## What it must do

- [x] Dead: StaticPopup `DEATH` "You have died. Release spirit to the nearest graveyard?" with one "Release Spirit" button and no cancel. Escape/cancel does not dismiss it; it stays until released or resurrected.
- [x] Accepting `DEATH` sends `ReleaseSpirit`. The popup stays closed until the server answers; a server error reopens it and shows the error in `UIErrorsFrame`.
- [x] Ghost: world desaturated (`ColorGrading.post_saturation` 0.2 on the `WowCamera`), restored when no longer a ghost. The ghost player model is not changed: the server replicates no ghost display/aura.
- [x] Ghost out of corpse range: top-center gold hint "Return to your corpse to resurrect (N yds)".
- [x] Ghost within `shared::death::CORPSE_RESURRECT_RANGE` (30 yd) of the corpse: StaticPopup `RECOVER_CORPSE` "Resurrect now?" → `ResurrectAtCorpse`. Range is checked client-side against the local player transform because the server does not push updates as the ghost moves; the server re-validates. The server has no resurrection delay, so no timer shows. Leaving range hides it without an answer.
- [x] Resurrection (state no longer Dead/Ghost) and leaving the world clear both popups; leaving the world also resets the death snapshot until the server resends it.
- [ ] Spirit healer resurrection from UI. The server accepts `AcceptSpiritHealerResurrection` by range alone, but Retail triggers it through spirit-healer gossip; blocked on NPC interaction (phase 5). IPC `DeathCmd::AcceptSpiritHealer` works.
- [ ] `RESURRECT` popup for a resurrection offered by another player. Blocked: the protocol has no resurrect-offer message (only `shared::death` range helpers).
- [ ] Retail minimap corpse arrow / world-map corpse pin.

## How it works

- [UI system](../wiki/systems/ui-system.md)
- [Networking](../wiki/systems/networking.md)

## Implementation inventory

- `src/scenes/death_ui/mod.rs` — popup sync, results → death actions, ghost grading, hint screen, world-exit reset.
- `src/ui/screens/ghost_hint_component.rs` — corpse-run hint frame.
- `godot/ui-model/src/ui/popup.rs` — `PopupStack::hide`/`contains` (`StaticPopup_Hide`).
- `src/death.rs` — `DeathRuntimeState::request_release_spirit` / `request_resurrect_at_corpse`.
