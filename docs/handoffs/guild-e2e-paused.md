# guild-e2e (paused 2026-10-29 by user: not a priority yet)

Landed: bank moves, Guild Control rights, social rights; server rewards/rep (ee23984b3d, dbaf3bb7a1); protocol 1660f5b.
Unlanded: engine branch origin/guild-e2e @ 6dc12c2a8 (rewards list/bar, chat lines, hover label, fixtures).

Resume:
1. Find why `depot-build --test -p game-engine-ui-model --test integration guild_rewards_list` hangs (suspect wait_for_item_catalog); run it once.
2. Rebuild engine (`depot-build.py --root <engine> --cli`).
3. `/home/osso/.worktrees/logs/guild-e2e-rewards-server.sh`, then `guild-e2e-rewards-clients.sh Modern run2`, then `Forever run1`.
4. After both skins pass: push engine to master; update capability matrix and guild perks specs (5.0.4 rule, boss/PvP gain gap, Cost price rule).
5. News fixture (not started): craft Enchanted Elementium Bar 22967 (Mining 2575; reagents 18562x1, 12360x10, 17010x1, 18567x3); loot via Stormwind Rat at (-8850.29,575.33,99.96) + `stage-corpse-loot <lead> 17771`; both clients read ItemCrafted/ItemLooted in News.
