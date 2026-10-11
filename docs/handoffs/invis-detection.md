# invis-detection — native quest proof

Server behavior and rollout: [typed invisibility](../../../game-server/docs/specs/invisibility-detection.md), [incremental spell-area import](../../../game-server/docs/invisibility-import.md).

Private UDP5496, fresh `fb_invis_detection_final_{m,f}` Orc Warrior characters: Modern and Forever each accepted Jaga's29401, picked five authored Windswept Balloon chests, clicked real loot rows, received item credit, turned in for1400XP/20silver/Horde Balloon, and asserted Jaga leaves the scene after reward removes eligible detection. Runtime server05e698c and fixtureeb0badf5c; later server commits add tests/docs only. No objective/item grants, aura stripping, canonical world.db writes or shared5000.

Evidence: canonical engine `data/diagnostics/invis-detection-2026-10-11/{live-final-modern.log,live-final-forever.log,live-proof-ledger.json}`. Ten final loot/object/reward/turned-in/ineligible PNGs inspected under `/syncthing/AgentShared/2026-10-11/quest-objectives/invis-final-*`. Blank roster captures are diagnostic-only, not character-select proof.

Fixture changes: keep picks off HUD edges; stand sideways so the player cannot occlude child-sized Jaga; re-aim real right-clicks after transfer readiness; select a visible LootFrame and scope item lookup to it rather than an unqualified first name match; assert post-reward giver removal. These are native click/selector fixes, not direct objective or reward messages.

Missing authored eligibility for Vin/Rendan and unhosted530 remain explicit in the server rollout doc. Core Fragment's current authored normal-copy row passes server spawn/interest proof; no new native dungeon loot/turn-in claim. Parent owns independent verification/integration; neither branch merges master.
