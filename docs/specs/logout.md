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

Current proof (2026-10-09): original code RED times out on Login. Private live server master `5b04aa9`, UDP5470, fresh diagnostic database: retained-token logout reaches CharacterSelect with zero units and detached world across five samples spanning 18 seconds. Evidence: canonical `data/diagnostics/logoutfix-2026-10-09/a-after-{5..9}.json`, `client-a.log`, `server.log`.

The fixture RED at `642931906` re-creates both units one frame after reset. Replicon's ConnectedClient defaults to visible before LoginRequest; delaying ReplicationSender does not gate that packet path. Real server `interest::on_replication_client_added` hides existing world entities from new connections before selection. The logout fixture now matches that boundary instead of broadcasting the old world to the unauthenticated replacement. The unchanged world-detachment assertions pass at `4ffe04328` and again with the DEATH Escape regression at `8a44570de`. Both countdown and rest logout authenticate the roster, detach the world and retain the token; explicit roster selection re-enters the world.

Final private live re-check at `8a44570de` also passes: five timestamped samples over 18 seconds retain CharacterSelect, zero units, connected account and detached world. Inspected 1920×1080 PNG: `/syncthing/AgentShared/2026-10-09/logoutfix/character-list-after-logout.png`; diagnostic `b-after-{4..8}.json`, `client-b.log`, `server-b.log` live under the same canonical diagnostics directory. No UDP5000 access or shared-account mutation.

Final regressions, run once per actual selected test: session logout 7, UI death 10, popup stack 7, popup component 2, account death/resurrection over UDP 3, network death over UDP 1 (30 passing). Logs: `/home/osso/.worktrees/logs/logoutfix2-final2-{ui-death,ui-popup-component,network-death}.log` and `logoutfix2-final3-{session-logout,ui-popup,account-death}.log`. Initial invocations blocked before Cargo by missing newly merged game tables; three zero-test filters were corrected, not counted as proof. No passing tests rerun.

DEATH Escape behavior belongs to [death flow](death-flow.md). Skin changes are out of scope.
