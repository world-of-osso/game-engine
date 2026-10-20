# RSX justify_v consumer audit

Coordinated engine/toolkit alignment fix: toolkit `attrs.rs` omitted `justify_v` dispatch while the typed FontStringData and native projection already supported Top/Middle/Bottom. Unknown attribute silently fell through. Fix normal dispatch, not a postsetup workaround; defaults and renderer unchanged.

Gitignore-aware Rust search of the full engine and coordinated toolkit found15 literal RSX uses, no dynamic RSX expressions. Typed properties and generated snapshot strings excluded because they do not pass through the broken parser.

## Changed effective alignment

- `godot/ui-model/src/achievements.rs`, ordinary Achievement<ID>Description: approved TOP instead of inherited Middle; only the new RSX attribute changes presentation.
- `godot/ui-model/src/ui/screens/options_menu_sections.rs::info_detail`, every InfoDetail{key}: existing authored TOP now honored instead of Middle. Lead explicitly accepted this second effect; bounds/text remain unchanged. Native fixture `justify_v_options_capture.gd` checks Social/Addons info rows in both skins and can save panel PNGs with JUSTIFY_V_OPTIONS_CAPTURES. Native pixels are separate proof, not established by this audit.

## Parsed MIDDLE, effective behavior unchanged

- `achievements.rs`: criterion progress-bar caption; checklist caption; toast title, achievement name and points (five sites).
- `achievement_art.rs`: empty-search instructions (one).
- `achievement_search.rs`: preview name and Show All label (two).
- `achievement_full_search.rs`: name/path/status text helper (one).
- `minimap.rs`: grouped blip number (one).
- `ui/screens/objective_tracker_component.rs`: POI number (one).
- `ui/screens/quest_art.rs`: Launcher title (one).
- `ui/screens/ready_check_frame_component.rs`: ReadyCheck body (one).

Thirteen MIDDLE use-sites formerly relied on defaultMiddle and remain Middle. No literal BOTTOM consumer found; unit test covers TOP/BOTTOM/MIDDLE transitions on the same FontString. Shared helper instances are not invented per-frame counts. This is not an audit of every unknown attribute.

Source/pinned file:line inventory and raw records: `/home/osso/.worktrees/logs/achievements-criteria-2026-10-12/description-alignment-{justify-v-audit.md,rsx-audit.json}`. [Achievement contract](../specs/dungeon-achievements.md#ordinary-description-vertical-alignment).
