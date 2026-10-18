# Dungeon objectives and native achievements

Native Retail Mainline AchievementUI and dungeon objectives, using authoritative server catalog/state and local extracted art. Forever changes art, not feature availability. [State and sources](../wiki/systems/dungeon-achievements.md).

## What it must do

### Dungeon and window lifecycle

- Show the active dungeon name and ordered boss objectives before watched quests, without moving the approved tracker anchor, in Modern and Forever.
- Replace boss state from server snapshots; clear on transfer/leave/disconnect and reject other-map/copy snapshots. Label only verified optional bosses.
- Open from launcher, micro-menu and configured ToggleAchievements binding; close with its button or Escape's CloseAllWindows.
- [x] Request cursor pages on demand, cache replies and refresh on live updates; preserve unknown dates and unevaluated criteria.
- [x] Keep the connection's earned set and progress from the latest full `AchievementStateUpdate` snapshot (initial login or reconnect); existing catalog queries remain unchanged. An `AchievementDelta` reduces to exactly that full snapshot's state: `earned_ids: Some` replaces the earned set in server order (including removals), `None` keeps it; progress rows are removed by ID, then upserted in ascending achievement-ID catalog order. It invalidates pages and requests refreshes exactly like a full snapshot, including an empty delta: raw statistics live in paginated views, not the grant snapshot. A delta without a full baseline on the current connection (first message, earned alert only, or after reconnect/reset) is an explicit dispatch error, never an empty fallback. Earned alerts stay ordered `snapshot: None` updates on AchievementChannel; deltas join their ordered relay.

### Achievement presentation

- [x] Use authored CriteriaTree.Flags, not quantity heuristics, for expanded criteria: ProgressBar0x1 from Mainline Constants.lua127 selects the XML510–569212×14 bar with centered authoritative current/required caption, even at required1. Unflagged rows use source completed checks or dash-prefixed text (AchievementUI.lua2009–2070), even at required100. Existing398-wide criteria allocation/row spacing remains unchanged; the source template specifically replaces only the old398×16 bar. Modern/Forever share behavior/geometry, with art/tint only differences. Unsupported progress remains explicitly Unevaluated without fake zero, completion or fill; pagination retains source flags. No new progression producer or comparison/toast/hidden/meta/tracker redesign. Native both-skin proof follows triple landing.

- [x] Open Summary by default with full authoritative points/completion totals, latest three earned rows and authored summary-category bars. Never sum loaded pages as account totals.
- [x] Use Retail Achievements, Guild and Statistics bottom tabs; Guild and Statistics roots never appear in the player achievement sidebar. Account/Character scope remains a query capability, not replacement tabs.
- [x] Use the requested Mainline top-level set: Characters, Quests, Exploration, Delves, Player vs. Player, Dungeons & Raids, Professions, Reputation, World Events, Pet Battles, Collections, Expansion Features, Legacy, Feats of Strength. Source child names/IDs/parents/order from DB2 12.1.0.69933 Achievement_Category. Housing is outside this requested set; Classic remains an authentic dungeon child.
- [x] Offer All/Completed/Incomplete filters independently of account scope. Statistics bypass completion filters; Feats shows earned entries and its explanation when empty.
- [x] Browse category ancestry/order, icons/names/descriptions/points/earned dates and criterion progress, including continuation pages.
- [x] Show bare thousands-separated earned points beside the Retail shield in both skins.
- [x] Use Retail's 768×500 AchievementFrame, styled collapsible category tree with native wheel/thumb scrolling, header shield/search/clear art and authored bottom tabs. Forever changes art only.
- [x] Keep the107×30 search template immediately left of the header filter with7px between control bounds, in ordinary and comparison views. Retail XML1675–1713 right-to-left/index1/index2 layout gives search x511/x631; preserve current y, sizes, filter behavior and every other UI element.
- [x] Achievement row buttons expand criteria in place, pushing following rows down; include icon frames, point shields, centered titles/descriptions, completion dates and authentic progress-bar art. Native row scrolling fetches further cursor pages at the bottom without collapsing selection; category/filter changes start at the top. Statistics use Retail striped rows. Summary uses carved category status-bar art; fills must not cover captions.

### Earned links and tracking

- [x] Show earned alerts with authoritative icon/name/points, Mainline plate/icon/shield elements and click-to-open own-state details.
- [x] Add a local earned chat line with a clickable achievement name; clicking it requests authoritative own-state details. Shift-click inserts the real ID/name link at the current chat caret.
- [x] Show only the achievement label in chat drafts while retaining its real ID for transport and sent clickable chat. Unicode caret/selection replacement, surrounding edits and history recall preserve unedited links; editing a label makes it plain text.
- [x] Never fabricate GUIDs, dates, other-character completion or comparison payloads in links.
- [x] Track unearned achievements in the existing objective tracker after quests. Refresh tracked rows while the window is closed; stop tracking earned rows.
- [x] Persist character-scoped watch IDs under authenticated account/realm preferences; restore rows from the server, not serialized progress.
- Tracking follows Retail's ten-achievement cap and refuses completed achievements.

- [x] AchievementSearchBox uses source TextInsets left16/right20/top0/bottom0 (InputBoxTemplates.xml245), allowing the authored107×30 native rectangle with its font family, position, art and filter behavior retained. Native geometry proof covers both skins × ordinary/comparison; separate post-landing real-peer physical search/clear/completion-filter retention passes in all four modes (2026-10-18).

- [x] AchievementSearchBox Roman font size is10, not the generic EditBox16 default. Source: AchievementUI.xml1709→InputBoxTemplates.xml206/202→FontStyles.xml92/56→Fonts.xml39–44 (SearchBoxTemplate→GameFontHighlightSmall→SystemFont_Shadow_Small/FRIZQT__.TTF height10). Use the same nominal10 as accepted criterion captions; the shared projection-root UI scale applies once, with no additional font conversion. This size-only change preserves family/color/shadow/insets/107×30/positions/art/filter/actions; separate post-landing both-skin normal/actual-online-comparison physical search/clear/three-filter retention passes (2026-10-18). Full font/template or cross-run pixel invariance is not claimed.

- [x] AchievementSearchBox typed text uses WHITE_FONT_COLOR (AchievementUI.xml1709→InputBoxTemplates.xml206/202→FontStyles.xml92–94). Only this widget's font_color is explicit; global defaults stay unchanged. Existing native projection also colors its caret white: accepted coupled side effect, not a claim of Retail caret parity. Font family/size10,107×30/insets/x/y/art/actions/filter/caret width/blink stay unchanged; shadow and empty instruction hint remain outside this slice.

### Statistics and search

- [x] Browse authentic statistic rows and raw, unclamped supported counters; unsupported quantities remain explicitly unevaluated.
- [x] Search the full eligible server catalog by title/description, including statistics, with scoped ID-cursor pages rather than loaded-page filtering.
- [x] Correlate search replies by term/scope, preserve valid Unicode terms and navigate selected results to authoritative details/the appropriate tab.
- [x] Exclude hidden, guild and hidden tracking copies from player browsing and search. Statistics are separate from ordinary achievement category rows.

## How it works

- [State, rendering, sources and proof](../wiki/systems/dungeon-achievements.md).

## Implementation inventory

- `godot/ui-model/src/achievements.rs` — cache, navigation, scopes, tracking and both-skin screens.
- `godot/ui-model/src/ui/screens/objective_tracker_component.rs` — achievement module after watched quests.
- `godot/rust/src/achievements.rs` — native requests, input, alerts and watch persistence.
- `godot/rust/src/objective_tracker.rs` — tracked state and click routing.
- `godot/rust/src/chat.rs` and UI-model chat components — insertion and clickable link rendering/navigation.
- Protocol `achievement_catalog_messages.rs` / `protocol_snapshots.rs` and server `achievement_window.rs` — authoritative catalog, Summary, search, statistics and earned icon.

## Tests asserting this spec

- `godot/ui-model/tests/dungeonclient.rs` — concrete both-skin Summary/scopes, criteria continuation, search/statistics, tracking, links and authored art regions.
- `godot/rust/src/account_dungeonclient_tests.rs` — actual account dispatch/continuations/live updates.
- `godot/tests/achievement_search_geometry.gd` — actual offline production RegistryUi/LineEdit four-mode107×30, L16/R20/T0/B0 margins, unchanged16pt font/search/filter geometry; no received-comparison or pointer proof claim.
- `godot/rust/src/ui/layout.rs::tests::achievement_header_search_*` — both skins × ordinary/comparison source bounds and translated search art/clear, through actual native host layout; post-landing native pointer proof is separate.
- `godot/rust/src/achievements.rs::tracking_tests` — persisted IDs and corrupt preference rejection.
- `godot/network/src/dungeonclient_wire_tests.rs` — ephemeral UDP AchievementChannel ordering.
- `godot/rust/src/account_netdelta_tests.rs` — full-vs-delta state, invalidation and toast-order equivalence; earned replacement/removal, progress add/update/remove; full-snapshot replacement/reconnect baseline reset; missing-baseline failure.
- `godot/network/src/netdelta_wire_tests.rs` — UDP relay keeps deltas between full snapshots and earned alerts.
- `godot/tests/achievement_criteria_live.gd` with `native_achievements_criteria_peer` — private5898 physical paid purchase/item-use, current1→2 mount progress, three earned achievements and isolated online comparison, in either skin. Requires fresh fb_achcrit_* accounts, explicit level/gold/position fixture setup and relocated authentic vendors; never pre-seeded counters or known spells.
- Server `achievement_window_tests.rs` — real catalog totals, faction/scope, hidden-copy exclusion, raw statistic counters, multi-page search, Unicode correlation and authored earned icon.
- Native private proof and inspected Modern/Forever PNGs are indexed in the linked wiki.

## Known gaps (current cycle)

- [ ] Server-unevaluated criterion/statistic types have no trustworthy quantity to display; do not invent zero/completion.
- [ ] Guild progress has no authoritative window endpoint. Show the real Guild category tree and an unavailable notice, never player Summary/points as guild facts.
- Native art/draft proof: private UDP5730, `fb_achievementsui`/Fbachui, both skins at082d6ffa6. Label-only drafts send clickable real633 links; native stepper scroll loads24→25 statistic rows. Evidence and exact scope live in the linked wiki; no full-suite/CI/comparison certification.

## Out of scope

- Gameplay-leveling certification: private live earned events use explicitly documented admin character setup.

## Player comparison

- Compare another online player’s authoritative account/character achievements and statistics from the Retail player-unit menu. Keep viewer and target progress separate and correlate all replies by target.
- Browse Summary/categories, expand criteria, continue cursor pages and display target completion/dates without inventing unavailable values. Forever changes art only.
- Offline, unknown, self and non-player targets fail explicitly; changing targets rejects late replies.
