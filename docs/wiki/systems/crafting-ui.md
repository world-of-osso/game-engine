# Crafting UI

Verified: 2026-10-11 — private both-skin Leatherworking specialization and cross-character fixed-output order/mail proof. [Contract](../../specs/crafting-ui.md) owns requirements; [existing professions](native-professions-crafting.md) owns recipe/trainer behavior. Retail Mainline defines both skins' elements; Forever changes art.

## Customer control fidelity — verified 2026-10-13

CustomerOrders.xml67-77 supplies Place Order (`BROWSE_ORDERS` in GlobalStrings.csv) BOTTOMLEFT(20,-28) and My Orders LEFT at Browse RIGHT(-10,0); shared PanelTabButtonTemplate measures widths. Form.xml supplies recipient TOPRIGHT(-10,-11); Form.lua530/588 overrides WowStyle1 widths to136/143px. Form origin(5,60), Payment origin(510,155), Duration label110×40 at y345 resolves dropdown(620,352.5). Right-justified labels retain source RIGHT edges and use measured font extents inside the right pane. Commission/input centerline is325; Personal Order To/input centerline109.5. Back uses SecureUIPanelTemplates sheet art, not defaultbutton art. CustomerOrders.lua86 binds the interacting NPC through the existing native masked portrait renderer. Pending disables dropdown actions; recipient/duration reducers remain unchanged.

Review RED caught missing Order To and120px vs136px; eight targeted order tests pass after caption fix005037ac2 (master merged932412df5), including pane bounds and input centerlines. Native1920×1080 captures: original `before/`, corrected recipient/duration/personal in `after-review/`, under `/syncthing/AgentShared/2026-10-13/ui-gameplay-panels/`. Both skins inspected against XML/Lua. The fixture asserts a populated portrait using a concrete offline human clerk; live NPC binding follows the active interaction, not the profession icon. Build and refreshed captures pass at005037ac2; helper check and fmt pass at33cfee500. Cage reports unavailable XWayland/libdecor/FIFO/activation/VSync capabilities but captures pass. These prove control geometry/art and portrait rendering, not new gameplay authority.

## Customer tab selection — native regression

The private modern-crafting fixture placed a Public order and observed commission escrow, then could not click My Orders. The form was visible while `snapshot.mine=true` selected and disabled My Orders. Customer catalog/placement replies carry owned snapshots; this flag describes query scope, not the visible page. Mainline CustomerOrders.lua123–138 selects the tab from the visible mode. Tab selection now follows `orders.creating`; a both-skin regression covers the owned snapshot on both pages. Current native revalidation is pending; earlier public/mail proof does not cover this regression.

## Projection and authority

`ProfessionView` selects Recipes, Specializations, Orders or the disabled recraft schematic. Customer orders use the separate825×568 PortraitFrame; crafter orders use942×658 ProfessionsFrame; specialization page is1144px. Backgrounds, tabs, paths and dial art use the authored Retail atlases and profession kit.

Specialization snapshots retain every authored node entry. One displayed path chooses the first unfinished entry in source order; drafts key tree/node/entry. Displayed knowledge ranks exclude the unlock tier. Currency records map TraitCurrencyIDs to wallet CurrencyTypesIDs or derived unlock tokens. Mainline Specializations.lua432-578 supplies radial positions when source coordinates are zero.

Apply sends the **complete selected-tree purchased configuration**: positive committed ranks plus staged overrides, in authored entry order. Granted-only ranks are not purchases. Root unlock uses the same full configuration. First live Modern Apply exposed the old staged-only payload removing committed unlock36338; server refused it. Engine650e9249a preserves unlock36338 and knowledge36327; native Undo after successful Apply cannot erase the committed point.

Dial perks filter source `path_node_id`; the major bonus bit is independent of subpath unlock. Templates.lua23-26 supplies the1.4-radian gap; Specializations.lua724-749 supplies177/189px milestone radii. Unearned/pending use the first90px flipbook cell; committed uses the last. Pending milestones use the authored glow. Native hover resolves path/perk ownership, source descriptions, requirements and unearned/pending/earned state.

Customer service opens only from authoritative InteractionOpened/Closed. Source-scoped recipes do not require the customer to own the profession. Exact requirements and supplied counts stay separate; commission uses copper. Claim/release/fulfill/cancel follow server eligibility. Completion removes the row from open orders and reports `Order completed`; the cast/effect24 backend delivers result and commission through system mail, never optimistic client inventory changes.

Recraft exposes its Retail owned-item input/output element but submission is disabled: no supported operation evaluator/owned-instance lineage exists. No CREATE_ITEM24 substitution or recraft wire message is invented.

## Native acceptance

One private UDP5871 server lineage: initial integrated07b23ad9, then owner restart atd0593dad for the skill_line0 customer catalog fix; protocol2a48bdc. No UDP5000, instance map or canonical world/cache mutation.

| Scope | Concrete proof |
|---|---|
| Specialization, both skins | Craftmodern/Craftforever, Dragon Isles LW2830, tree350/node28546: confirmed unlock, stage/Undo, Apply0→1/30, knowledge25→24, post-Apply Undo preserves rank; resolved+1Skill path and+5Ingenuity perk tooltips |
| Public customer orders, both skins | Authentic Clerk Gretal215258/menu30243/OptionNpc50; Glacial Cloak28208→22658×1; supplied22682×5,14048×4,7080×2,14227×4;1g escrow100000→90000 per customer |
| Crafter, both skins | Separate Craftworker, Classic Tailoring2540rank300, knows28208; native claim→Create→authoritative completion/removal; no output item fabricated in crafter bags |
| Result/commission mail, both skins | Actual Northshire mailbox202578; each customer claims22658×1; worker claims two10000-copper system commissions, wallet100000→110000→120000 |
| Acquisition/quality/recraft | Skills, knowledge25 and achievement15876 were fixture-primed, **not acquisition proof**. No variable-quality/concentration outcome or recraft operation claim |

`godot/tests/crafting_ui_live.gd` drives normal character selection and physical controls. It samples actual posed model triangles, not unreliable bounds centers; startup graphics overrides the initial FPS cap, so input is paced after entering world. Initial completed-row and mail-row selector assumptions were corrected to the actual server/UI contract. Original failed logs remain; no failed run is relabeled as a passing fixture.

Broker source appearance and authenticated local-CASC closure were staged privately by the server owner. Strict receipt loading exposed duplicates; publication retained79 missing identities after85 matching duplicates were excluded. Canonical appearance cache remained unchanged; engine-s6's temporary cache link was restored after proof. Unrelated NPC1052435 receipt gaps did not block these cases.

## Proof ledger and evidence

SSOT: canonical `data/diagnostics/crafting-ui-2026-10-11/`. Inspected PNGs only are published to `/syncthing/AgentShared/2026-10-11/crafting-ui/`; logs, configuration and tokens are not published there.

- Model/screen21+20 at9c46abd33/protocol2a48bdc; native extension/CLI and helper-backed scoped check passed. Full-config RED at96894163f, eight specialization tests GREEN at650e9249a.
- Native logs: `live/modern-customer/spec-attempt2.log`, `live/forever-customer/spec.log`, `live/modern-customer/order-posed.log`, `live/forever-customer/order.log`, `live/forever-worker/craft.log`, customer/worker receiving-mail logs in those skin directories.
- Modern cast did complete; its first fixture incorrectly waited for a retained Completed detail (`live/modern-worker/craft.log`). Actual completion PNG and subsequent result/commission claims prove its outcome; corrected Forever cast fixture passes.
- Final master integration3f8d83357:21 model+20 screen+5 native+1 wire tests passed; extension/CLI build, helper-backed scoped check and fmt-check passed with no warnings (`final-*.log`). Later acceptance-checkbox edits are docs-only and do not invalidate that code proof. No full suite or CI certification is implied. Independent AskClaude verification was unavailable because OAuth expired.

## Sources

- [Crafting UI contract](../../specs/crafting-ui.md) — behavior and explicit gaps.
- `godot/ui-model/src/professions_specializations*.rs`, `professions_frame*.rs`, `professions_orders.rs` — state and Retail projection.
- `godot/rust/src/professions*.rs`, `account.rs`, `tooltip_sources.rs` — native lifecycle/input/transport/hover.
- Local Retail `Blizzard_Professions*`; `Blizzard_ProfessionsCustomerOrdersForm.xml` supplies the shared auctionhouse-background-index panels.

## See Also

- [[native-professions-crafting]] — existing recipe/trainer pipeline.
- [[forever-preset]] — authenticated character skin selection.
