# Native Mythic Keystone UI

Verified: 2026-10-11. [Client contract](../../specs/mythic-plus.md) owns UI scope; [server runtime/proof](../../../../game-server/docs/wiki/systems/mythic-plus.md) owns authority, persistence, numeric gates and private run outcomes.

## Content

Account decodes ChallengeSnapshot over the headless transport. One shared screen builds the Mainline398x548 Font frame and completion banner; the existing objective tracker hosts ChallengeModeFrame ahead of quests. Skin changes resync the same elements. Font interaction travels through the normal physical game-object right-click path; bag pickup/slot click or drag emits the authenticated Slot request. No native fixture injects a challenge state.

Two native defects were reproduced during live proof: a rendered Font did not route through use_game_object, and generic button chrome obscured the slotted item icon. The dispatch now calls use_font_of_power; the48x48 clickable slot uses the existing transparent item-slot host with the authentic icon as its child. Both fixes have physical/PNG proof, not source-shape assertions.

`tests/mythic_plus_live.gd` connects only private5800 fb_* accounts, drives viewport input, exports real snapshots and captures1280x720 PNGs. Require an actual adminPing before login, not merely an existing UDP socket; startup SQL can outlast the legitimate5s transport timeout. The populated dungeon can kill a test actor while inspecting UI; private Revive/Damage use is explicitly recorded in the server proof. Neither native screenshot nor that harness certifies natural-party combat or full Azure visual receipts.

## Sources

- [UI model](../../../godot/ui-model/src/challenge.rs), [native host](../../../godot/rust/src/challenge.rs), [right-click dispatch](../../../godot/rust/src/merchant.rs).
- [Live fixture](../../../godot/tests/mythic_plus_live.gd), [client contract](../../specs/mythic-plus.md), [authoritative server/proof](../../../../game-server/docs/wiki/systems/mythic-plus.md).

## See Also

- [Server boss scripts](../../../../game-server/docs/wiki/systems/boss-scripts.md) — encounter frame/progress producers.
- [[native-hud-edit-mode]] — supported skin/layout ownership.
