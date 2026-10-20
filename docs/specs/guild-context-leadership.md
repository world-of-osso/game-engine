# Guild roster leadership context action

Mainline UnitPopupSharedMenus.lua257–286 and UnitPopupSharedButtonMixins.lua883–900 expose GUILD_PROMOTE for a current Guild Leader's other guild member, online or offline. GameDialogDefs.lua872–882 captures that member for Accept/Cancel/Escape. Reuse the landed target-only TransferLeadership route; [guild system](../wiki/systems/guilds.md) owns implementation/proof details.

## What it must do

- [ ] Both skins expose `Promote to Guildmaster` for an authoritative current-master/nonself roster target, including offline members. Nonleader/self/missing-authority/waiting contexts must not dispatch the action. Existing ordinary Promote/Demote/Remove stay unchanged.
- [ ] Opening the entry only captures the original target and shows `Really promote %s to Guildmaster?`; selection, sort and benign roster/profile refresh must not retarget acceptance. Authenticated local-CASC70448 GlobalStrings15965/15967 supplies the exact English strings.
- [ ] Accept dispatches exactly one existing target-only TransferLeadership request. Existing server reply/profile owns actual leader/rank changes; no optimistic mutation or new backend authority.
- [ ] Cancel and the existing native Escape dismissal boundary send no transfer, close only the confirmation and preserve the guild window. Closing/reopening must not resurrect a stale confirmation.
- [ ] Use source ordinary popup/menu primitives, labels and geometry in both skins, with no generic popup redesign. CONFIRM_GUILD_PROMOTE does not request fullScreenCover; do not add an unrelated full-screen cover. Native physical input/rendering remains a separately granted acceptance boundary.

## How it works

- [Guild system](../wiki/systems/guilds.md).
- [Existing voluntary leadership authority](guild-leadership-transfer.md).
- [Shared menu primitives](menu-primitives.md).

## Implementation inventory

- `godot/ui-model/src/guild_rank_frame.rs`: existing guild screen/context menu.
- `godot/ui-model/src/guild_member_detail.rs`: existing source-style confirmations.
- `godot/ui-model/src/guild_ranks.rs`: existing reducer/authoritative session and subdialog dismissal.
- `godot/rust/src/guild_ranks.rs`: existing generic request dispatch and native Escape→dismiss_subdialog boundary, not a new backend.
- `godot/ui-model/tests/guild_context_leadership_tests.rs`: observable menu/confirmation/request/cancellation/identity controls.

## Tests asserting this spec

- `guild_context_leadership_menu_eligibility_and_ordinary_rank_controls_both_skins`.
- `guild_context_leadership_accept_keeps_captured_member_after_sort_selection_and_refresh`.
- `guild_context_leadership_cancel_escape_and_reopen_never_send_transfer_both_skins`.
- `guild_context_leadership_popup_has_source_labels_and_bounded_dialog_geometry_both_skins`.

## Known gaps (current cycle)

- [ ] Ignored behavioral tests/spec pushed first, actual missing-menu/confirmation/dismissal RED before production, committed minimal GREEN, enabled proven tests and independent gate.
- [ ] After gate/landing: exact-cohort/startup and granted private-copy/window, real authenticated two-member both-skin physical Cancel/Escape/Accept and authoritative refresh. Model snapshots are not native guild proof.

## Out of scope

No new protocol/server/persistence, ordinary rank-action replacement, disband/dethroning, rank reordering, bank semantics, realm/Club-ID metadata, native name-order claims, Communities text or professions. Existing self-transfer refusal remains named project policy, not a native Retail self-transfer assertion.
