# Retail Lua loadout fixtures

Generated2026-10-15 by executing the installed Retail `Blizzard_PlayerSpells/ClassTalents/Blizzard_ClassTalentImportExport.lua` and `Blizzard_SharedXMLBase/ExportUtil.lua` unchanged through system `liblua5.4`. Harness supplies Lua5.4 bit helpers, `tInvert`, and the C API inputs; not a second Rust/Python codec.

`C_Traits.GetTreeNodes(658)` receives all208 sorted nodeIDs from installed `data/db2/active/TraitNode.csv`. `GetNodeInfo` returns grants62121,94647,94664 and purchased62122/80181 rank1; other nodes unselected. Header version2/spec62, zero hash or bytes1..16. The approved installed version sources and their SHA256 are in [talent-loadouts](../../../../docs/specs/talent-loadouts.md).

Adversarial fixtures change one API input: `overrank` encodes purchased2 where real62122 allows1; `choice` encodes index3 where62087 has only two real entries; `prerequisite` selects62084 without its path; `otherspec` selects62117 under Arcane62. These deliberately invalid input states exercise import validation, not valid Retail character builds.

Full executed Lua harnesses and command receipts: canonical `data/diagnostics/talent-import-export-2026-10-15/lua-*-harness.lua`. The zero fixture is compared byte-for-byte against Rust export. Both hash fixtures must decode identically; nonzero hash comparison remains unavailable, explicitly documented in the spec.
