# Achievement bottom tabs

## Source contract

Retail Mainline `Blizzard_AchievementUI.xml:246–342` defines `AchievementFrameTabButtonTemplate` directly; it does not inherit `PanelTabButtonTemplate`. Reuse the shared tab builder with a bounded Achievement variant, not generic Panel art.

- `UI-Achievement-Header` (FDID235402): left21/right18, x−4/+4, middle between caps; selected height59, inactive49, inactive tint0.6.
- GameFontNormalSmall/HighlightSmall: Friz10, centered; `AchievementFrame_UpdateTabs` Lua364–372 centers text at y−5 selected/−3 inactive.
- `OnShow PanelTemplates_TabResize(self,30)`: shared Lua403–435 adds side padding20, so width is measured text+50, minimum39.
- First tab XML2555–2558: TOPLEFT to frame BOTTOMLEFT x17/y+3. Shared Lua475–480 anchors next tab +3; Achievement Lua354–361 overrides Statistics to previous RIGHT−5.
- Existing Achievements/Guild/Statistics actions and UI state stay unchanged. Other shared Panel/Minimal consumers keep their existing art.

## Other boxed-tab audit

Read-only source search covered `panel_button`, defaultbutton atlas attributes, tab builder functions, and tab-named/action-bearing RSX buttons across `godot/ui-model/src/`. No additional confirmed instance of this Achievement LEFT-caption/fixed-width tab implementation was found.

Inspected existing tab families:

- Adventure Guide, Merchant, auction bottom tabs, Guild Bank modes, Collections, Follower Dungeons and standard Character tabs use the shared tab builder.
- Professions and crafting-order tabs, Transmog collection tabs use authored TabSystem strips with centered captions and default button skin disabled.
- Communities/Guild, Cooldown Settings, Character's existing Forever side tabs use explicit side-tab art, not boxed panel-button replacements.
- Search candidates in Guild Tabard rotation buttons and Auction Cancel/Bid controls are ordinary action buttons, not tabs; do not convert them.

This is a source audit, not a claim that every client panel was natively rendered. No unrelated frame was changed. Raw search candidate evidence: `bottom-tabs-{boxed-candidates,default-button-candidates}.txt` under the existing achievement proof directory.

## Proof

Source behavioral RED and native capture fixtures are committed before implementation. Initial attempt using manifest protocol revision74e67cf could not compile because that checkout lacks the engine's required `vendor/lightyear_udp`; this setup failure is not behavioral RED. Subsequent proofs use owned frozen protocol5660814657bc4e651c357304ceb4e5c08ad04248 and the accepted toolkit core. Final results live in the proof ledger; do not infer PASS from this handoff.
