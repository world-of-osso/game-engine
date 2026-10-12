# Dungeon objectives and native achievements

Native Retail Mainline AchievementUI and dungeon objectives, using authoritative server catalog/state and local extracted art. Forever changes art, not feature availability. [State and sources](../wiki/systems/dungeon-achievements.md).

## What it must do

### Dungeon and window lifecycle

- Show the active dungeon name and ordered boss objectives before watched quests, without moving the approved tracker anchor, in Modern and Forever.
- Replace boss state from server snapshots; clear on transfer/leave/disconnect and reject other-map/copy snapshots. Label only verified optional bosses.
- Open from launcher, micro-menu and configured ToggleAchievements binding; close with its button or Escape's CloseAllWindows.
- [x] Request cursor pages on demand, cache replies and refresh on live updates; preserve unknown dates and unevaluated criteria.

### Achievement presentation

- [x] Open Summary by default with full authoritative points/completion totals, latest three earned rows and authored summary-category bars. Never sum loaded pages as account totals.
- [x] Provide All/Account/Character scope tabs using the authentic account flag.
- [x] Browse category ancestry/order, icons/names/descriptions/points/earned dates and criterion progress, including continuation pages.
- [x] Show bare thousands-separated earned points beside the Retail shield in both skins.
- [x] Use Retail's 768×500 AchievementFrame, styled collapsible category tree with native wheel/thumb scrolling, header shield/search/clear art and authored bottom tabs. Forever changes art only.
- [x] Achievement row buttons expand criteria in place, pushing following rows down; include icon frames, point shields, centered titles/descriptions, completion dates and authentic progress-bar art. Native row scrolling fetches further cursor pages at the bottom without collapsing selection; category/filter changes start at the top. Statistics use Retail striped rows. Summary uses carved category status-bar art; fills must not cover captions.

### Earned links and tracking

- [x] Show earned alerts with authoritative icon/name/points, Mainline plate/icon/shield elements and click-to-open own-state details.
- [x] Add a local earned chat line with a clickable achievement name; clicking it requests authoritative own-state details. Shift-click inserts the real ID/name link at the current chat caret.
- [x] Show only the achievement label in chat drafts while retaining its real ID for transport and sent clickable chat. Unicode caret/selection replacement, surrounding edits and history recall preserve unedited links; editing a label makes it plain text.
- [x] Never fabricate GUIDs, dates, other-character completion or comparison payloads in links.
- [x] Track unearned achievements in the existing objective tracker after quests. Refresh tracked rows while the window is closed; stop tracking earned rows.
- [x] Persist character-scoped watch IDs under authenticated account/realm preferences; restore rows from the server, not serialized progress.
- Tracking follows Retail's ten-achievement cap and refuses completed achievements.

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
- `godot/rust/src/achievements.rs::tracking_tests` — persisted IDs and corrupt preference rejection.
- `godot/network/src/dungeonclient_wire_tests.rs` — ephemeral UDP AchievementChannel ordering.
- Server `achievement_window_tests.rs` — real catalog totals, faction/scope, hidden-copy exclusion, raw statistic counters, multi-page search, Unicode correlation and authored earned icon.
- Native private proof and inspected Modern/Forever PNGs are indexed in the linked wiki.

## Known gaps (current cycle)

- [ ] Server-unevaluated criterion/statistic types have no trustworthy quantity to display; do not invent zero/completion.
- Native art/draft proof: private UDP5730, `fb_achievementsui`/Fbachui, both skins at082d6ffa6. Label-only drafts send clickable real633 links; native stepper scroll loads24→25 statistic rows. Evidence and exact scope live in the linked wiki; no full-suite/CI/comparison certification.

## Out of scope

- Comparison: no authoritative comparison catalog endpoint; InspectSnapshot exposes equipment/talents, not achievements.
- Gameplay-leveling certification: private live earned events use explicitly documented admin character setup.

## Player comparison

- Compare another online player’s authoritative account/character achievements and statistics from the Retail player-unit menu. Keep viewer and target progress separate and correlate all replies by target.
- Browse Summary/categories, expand criteria, continue cursor pages and display target completion/dates without inventing unavailable values. Forever changes art only.
- Offline, unknown, self and non-player targets fail explicitly; changing targets rejects late replies.
