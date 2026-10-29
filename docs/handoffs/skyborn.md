# Skyborn takeover handoff

Verified: 2026-10-08. Recipient: existing `remote-agent-world-of-osso` terminal on agent-server, cwd `/home/osso/Projects/world-of-osso/game-engine`. This is a task/state handoff, not a claim that full Skyborn integration is complete.

## Immediate ownership

User asked the agent-server session to take over. Sender stops feature work after handing off. Preserve the desktop agent's separate merge/landing lane. Do not start duplicate feature agents or overwrite its edits. No production deployment, PR, osso-5000 mutation, protocol-pin change, or other-owner branch landing is authorized by this handoff.

## Git state to fetch

All code and earlier tracked docs are pushed. Use explicit branch refspecs when pushing.

| Repository | Branch | Current pushed revision before this handoff-doc commit |
|---|---|---|
| game-engine | skyborn | `8d0fb80bb1e682f73589e34c4b8b2cd6b2a79b34` |
| game-server | skyborn | `b32e197cec632e3824b59661d0a3cd5dcd07b9cf` |
| shared-protocol | master | producing/proved pin `5107483b61745a3378d1a88d1cf2cd960123e998` |
| ui-toolkit | master | producing/proved pin `29a9ebfd22947deed373e33b887b452bfda3082d` |
| ui-toolkit-macros | selected sibling | `9525d84f618997f3918de0681e94e5ab24b9970e` |
| asset-resolver | selected sibling | `80d16d05fa6210e8ef1bf5b9742144a844a5d464` |

Desktop's canonical protocol was `75979089d07167e349d0873cef56291975e50f42`; do not silently replace its pin or attribute our protocol510 proof to759.

Engine157 unique commits rebased onto master `13319752cca2169ee170d6bb9a8f22394989e328`; server90 onto `8b3819b0d9e14394526f3f6196889a5baebe7123`, no skipped unique commits. Backup refs remain. Both sender worktrees were clean before this document.

Desktop saved merge is `origin/skyborn-master-merge` at `ac8aae4fdaaf7620984a6bfda1cf18b3fa4ec7e7`. It is not a drop-in replacement for newer source-selected NPC policy. Its worktree `/home/osso-test/.worktrees/game-engine-hudfixes` later moved to `hudeditmode`; never assume ac8 is still checked out. Read-only comparison showed current skyborn and saved branch diverge from041fa0f54. Sender did not merge, reset, rebase, or land that branch. Its owner retains landing decisions.

## Delivered data — exact paths

Already transferred and independently hash-verified on Desktop / OssoBuild, user osso-test:

- world.db: `/home/osso-test/Projects/world-of-osso/game-engine/data/skyborn-handoff/2026-10-07-c1cdf6e/server/world.db`
- Item.csv: `/home/osso-test/Projects/world-of-osso/game-engine/data/skyborn-handoff/2026-10-07-c1cdf6e/engine/db2/1.60.1.70205/items/Item.csv`
- data README: `/home/osso-test/Projects/world-of-osso/game-engine/data/skyborn-handoff/2026-10-07-c1cdf6e/README.md`

Packet contains2186 payload files (~2.44GB): all2163 declared Godot fixture inputs, current Forever catalog/NPC caches, and stopped scratch reference world.db. NO player .redb or provider credentials included. World table counts: db2_row_source1229; skyborne_content_item30; skyborne_item30; skyborne_item_sparse30. World and seven cache integrity checks passed.

Final packet MANIFEST SHA256: `e9454740563756143f19d8ed19fbbd479a06e516eba8e2314761f38f9c55e157`. README SHA256: `5049b74920b1ff1fe754046600a1c713271351169b015497cdd7ea0d5047b987`. The2185 non-README payload hashes retain the independent full-transfer proof; subsequent updates changed only README/control metadata.

As of the latest read-only probe, Desktop canonical game-server/data/world.db still lacks the four source tables, while canonical engine's Item.csv exists (634 bytes). Live `/home/osso-test/data/osso-5000/data/world.db` also lacks them. Do not replace it to make tests pass. Observed osso-5000 remained active/PID1053927/cwd `/home/osso-test/data/osso-5000`. Data staging does not prove the remote30 failing tests consumed it or passed.

Data is deliberately not in source Git. If agent-server cannot reach Desktop, coordinate dataset transfer with the migration owner; do not invent source rows, download from Blizzard CDN, copy player storage, or claim the files are already on agent-server. Before scheduling jobs consult `/syncthing/Sync/KB/state/agent-server.md` on the laptop/migration owner's copy. Server access is `ssh osso@agent-server.local`; preserve Nouveau blacklist and existing host limits.

## Implemented corrections

- Retail replacement metadata loads only for owned Retail catalogs; Forever remains source-isolated. Indexed model columns use `(column_index, model_fdid, skin_fdids)`.
- Pinned Forever70205 RandPropPoints really lacks Retail spell-budget fields. Server `c1cdf6e` loads spell budgets only for Retail; normal Forever armor/weapon/stat tables remain mandatory. No fabricated fields, Retail borrowing or fallback. Forever item-spell support is not claimed.
- Fixed-record DB2 decoder preserves master read-decrypted/report-skip-zero-filled behavior; selected IDs remain mandatory. Sparse full encrypted rejection remains explicit. Test/docs aligned without changing decoder.
- Native Ailee bake7352105 avoids unused component-overlay mapping1102747, while required model/material and ordinary outfit/player checks remain strict.
- Selected Ailee61137→target38/material22855→cached3613861 provides authored type20 canvas256x256. NPC composition previously omitted it; source-driven separate-type composition fixes that.
- User explicitly selected “omit unselected pass.” Actual Forever layout202 has type7 target19/canvas512x256 but Ailee selects no target19 material. NPC-only absence state omits that pass; players carry None. No blank textures or hardcoded batch-ID hide.
- Initial absence guard incorrectly rejected already-hidden optional cape unit29/geoset1507/type2. Corrective `7429822c3` respects unchanged visibility first and enforces actual batch-local missing-file receipts before omission. Shared filesystem helper preserves read/decode semantics. Naming-only `8d0fb80bb` resolves one complex-condition readability finding; predicate/results are literally equivalent.
- Reported player-motion2049 was a fixture error, not physics. `a7616cd` changes lower terrain corner from99 (same cell3 as100) to adjacent corner100-100/3; player is cell2. Original ten probes were Unsupported; falling was accurate. New assertions prove Supported48, immediate landing/tracker/motion1 and all10 no-input ticks. Production physics/ABI/data unchanged; correction pushed.

## Proof ledger — do not rerun unchanged expensive scopes

| Scope | Exact proof level/result |
|---|---|
| Server creation/storage/source transfers/quests |60 distinct fresh tests passed,0 failed; one ground test ignored/not credited. Frozen ABI/migrations/save-reopen26; creation covers all ten race/class pairs; source commerce/requirements/quest gates covered. Native ten-pair UI creation not claimed. |
| Other current server gates | Loader3/3; combat/aura/group8/8; composed Python19 cases (18 unchanged retained +1 fresh changed case), scoped format/check0, no compiler warnings. |
| Motion fixture |1/1 GREEN retained; independent scoped rustfmt0 and `cargo check -p server --tests`0, no warnings; production hashes unchanged. |
| Current NPC pixel/pass helper |13/13 CPU; twelve authentic asset hashes unchanged. Real filesystem NotFound→production-shared reader→policy tested, including mixed-slot orders and visible/hidden states. Not an injected whole-GPU/publication failure. |
| Current engine format/check | Naming revision8d: scoped cargo fmt0, extension cargo check0/no warnings using SAME warm Depot cache. No direct extension Cargo/new bulk target cache. |
| Ailee/Grove native | Exact unchanged ailee-assets fixture DONE: original female7478494;15 visible textured batches; type20 Batch94/97; original shoulders7579617/18 Left6/Right5 identity transforms, active base256x256+second64x64 textures, front/back captures. Protected character30 remains40XP/log[]; no quest mutation. |
| Fresh supported starts/acceptance | Separate new player store, UDP15003: race95Skymage original Map2991 start +seven source-kit IDs +Ailee visible +native greeting/detail/title/Accept clicks→authoritative92460 log/chat/tracker; fixture DONE. Race96Skyshaman original start/seven IDs/Ventaari visible DONE; no level1 quest invented. Two scoped fixtures,14 source item queries,17 CLI operations,6 provisioning operations,8 screenshot hashes verified. |

Actual native extension SHA256: `2763a276c0383a9ca7184be74366f279ba010d4fccda8b6991e2f035dbe3a93a`, built742. Server: `c818f4541057feb805756106dd59d00719de1ad6e4d61dca4b31b6077e9daed0`, loaderc1 production. Fixture: `04e20ba7efe1448aeb6a422424a37081251d2323687aa818ac545f64876948fa`. Source8d naming-only behavior equivalence separately verified; do not claim the native artifact was built8d. Native OS exit codes uncaptured by asynchronous maintained cage helper: DONE is not exit0 or general shutdown proof.

Fresh race95 native position `(4088.501,976.1686,-1847.5)`, server `(4088.501,976.169,-1847.5)`. Race96 `(4051.417,977.6204,-1858.625)`. Race95 kit `[35,117,159,6948,271655,271658,271659]`; race96 `[117,159,6948,271661,271662,271663,280400]`. Accepted92460 is completed/zero-objective in reconstructed content; original scripted progression not proved.

## Cleanup and protected state

All sender-owned clients/cages and services `skyborn-e2e-server` (UDP15002) / `skyborn-fresh-acceptance` (UDP15003) stopped; MainPID0 and no listeners/pidfile. No build/test jobs retained or cancelled to accelerate handoff. Fresh fork `/home/osso/.worktrees/game-server-skyborn/target/skyborn-fresh-acceptance` had a new player store; numeric ID30 inside that fork is NOT protected original ID30.

Protected original `/home/osso/.worktrees/game-server-skyborn/target/skyborn-e2e/data/game.redb` SHA256 before/after fresh proofs is exactly `4b1b12bf8adf739c00ed291866db2ec1edc0f4e14fc23a12d7ad6402980c3aca`. Do not transfer/reset it. Earlier login trials changed its physical bytes but retained30/40XP/log[]; only the fresh-fork interval has exact file equality. Cold backups and Git backup refs remain on sender; no clean merged worktree deletion is justified for active unique branches.

## Remaining work / limits

- Confirm agent-server required datasets and exact repository/protocol refs before jobs; fetch pushed source, do not depend on sender target/ files or copy empty build caches.
- Remote data-dependent30-test/engine-wide consumption gates and its merge/landing remain the receiving/desktop integration owner's lane. Savedac8 is not automatically compatible with later NPC policy or latest upstream; reconcile only within authorized ownership.
- Full original scripts/phasing, race96level1 intro, Classic/thrown DPS, original-build asset byte equivalence and PBR/Retail parity remain unproved/excluded. AreaTable source build explicitly unverified;1 Criteria/8 CriteriaTree encrypted records unavailable.
- Native logs retain11 other NPC appearance errors, including display136974 Batch52 strict type2 rejection. Do not claim zero runtime errors or whole-scene NPC coverage. Independent image payloads were often omitted; source/material/fixture acceptance is not full independent visual parity.
- Protected historical quest0→40XP turn-in/restart and fresh acceptance are separate proofs; no continuous full storyline claim. GUID-level source provenance is not exposed by native snapshots.

Original detailed receipts/logs live only on sender under `target/skyborn-source-items/` and server target/; they are NOT needed to understand this summary and are not asserted accessible on agent-server. This tracked handoff and existing specs/wiki are available by fetching origin/skyborn. Continue the parent goal under these exact limits; do not declare full Skyborn complete merely because handoff/data delivery completed.
