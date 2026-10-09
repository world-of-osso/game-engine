# Game Menu logout

## Behavior

- [x] Game Menu Log Out returns to character select without credential entry; Exit Game quits.
- [x] Retain combat blocking, rest-area instant logout, the 20-second countdown, movement cancellation and repeated-request behavior.
- [x] Release the character world and its transport. Keep the authenticated account token; use the existing token LoginRequest to obtain a fresh character roster on a new transport. Never auto-enter the previous character. Loading covers the roster request; success opens CharacterSelect. Authentication errors retain existing login feedback.
- [x] No new logout or roster protocol messages.

## Retail grounding

Paths below are relative to `~/.cache/wow-ui-sim/blizzard-ui/retail/AddOns/`:

- `Blizzard_GameMenu/Shared/GameMenuFrame.lua:214-216`: Log Out callback is `Logout`; Exit Game callback is `Quit`.
- `Blizzard_APIDocumentationGenerated/ConnectionDocumentation.lua:86-93`: both callbacks are native functions. `SystemDocumentation.lua:134-138` declares synchronous `PLAYER_LOGOUT`. There is no separate `LOGOUT` glue event in this checkout; native logout dispatch itself is not Lua source.
- `Blizzard_GlueParent/Mainline/GlueParent.lua:157-159,201-225,380-389`: `LOGIN_STATE_CHANGED` validates the glue screen; a connected account's best screen is `charselect`, not `login`. Lines 1-6 and 439-466 map/show `CharacterSelect`.
- `Blizzard_GlueXML/Mainline/CharacterSelect.lua:208-213`: exceptional disconnect-on-logout policy is handled after arriving at character select.

## Proof

`godot/network/examples/native_input_fixture/logout.rs` + `godot/tests/world_logout_flow.gd`: real loopback UDP auth/combat/rest, physical Game Menu clicks, countdown/cancellation/repeated request, detached world, automatic token-only authentication, populated character list, roster click and world re-entry, rest-area instant logout. Existing session logout tests cover exact countdown rules.

DEATH Escape behavior belongs to [death flow](death-flow.md). Skin changes are out of scope.
