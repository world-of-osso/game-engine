# Native running-battleground invite proof — 2026-10-18

**PASS, bounded direct-Arathi proof; accepted by Lead.** One rendered native client in both skins plus19 real UDP support accounts. DEBUG builds, not release; not20 rendered windows.

## Frozen inputs

| Input | Revision / SHA-256 |
|---|---|
| Engine | `4776bd744df9071e5708c59475d8d49c1337410f` |
| Server | `ec0863223e07821859a122eb9727d34aad4bf0b1` |
| Protocol | `019cd75bbc9320e40430a9ec6594d37ccfee5752` |
| Loaded native DEBUG library | `e09c57f34d1bd51e93f7ed1ceda1493a13ce79f6ad9dbf559f1238551db0112d` |
| DEBUG UDP peers | `7d0b78f47907922dea935cfe057d5994bc29e6a41bcc340fb66681ccf4b79d3f` |
| DEBUG server | `fced00e83c640447672bb5af3063fc8a8352f0c67a762aeaac338acea88ea78d` |
| DEBUG admin | `a49cc0bf4c78d5a571c0af70c12ea0c447a80ef7883ad1e2bbebf73b041037df` |

All five built artifacts, including the IPC CLI, matched their receipts. `native-loaded-debug.json` records the actual process mapping and matching library hash. Documentation commits do not change these frozen build inputs.

## Observed behavior

1. Nineteen actual level39 support accounts formed ordinary parties5/5/5/4 through real invitations/accepts. Direct Arathi1018 queue requests formed match1/map2107,10v9, without splitting parties. Both teams exceeded the unchanged source8/player minimum; normal acceptance naturally reached InProgress.
2. Native account20 remained solo on origin map0, then queued through the visible specific-Arathi selector. Its actual received invitation was match1, InProgress, accepted=false, arena=None. Both existing ready-dialog actions appeared.
3. Forever and Modern each exercised physical **Leave Queue**, stayed on origin, queued again, then physically **Enter Battle** into the same running match1/map2107. Ordinary match-ID-bound leave returned to map0 with cleared battleground state. No queue, roster, phase, score, timer, outcome or reward was injected.
4. All19 supports also left normally and returned to origin. Owned processes stopped, UDP5941 freed and the Lead-managed pvp window cleared at00:33:50 CDT on2026-10-18.
5. Ten original1920×1080 PNGs were inspected through1280px previews and copied to `/syncthing/AgentShared/pvp-2026-10-18-{forever,modern}-running-invite-{decline,declined,accept,entered,returned}.png`.

## Evidence and retained failures

Evidence root: `data/diagnostics/pvp-native-running-invite-2026-10-17/` (directory named for preparation date; runtime completed October18).

- `proof-ledger.json` binds exact source/artifact hashes, actual loaded mappings, PNG paths/hashes, capability assertions and cleanup. Per-stage `*-pvp.json`, `*-native.json` and `*-peers.json` retain received authority and native state; input ledgers retain observed physical targets and IPC requests.
- The first peer/native connection timed out after5s because UDP became bound before the server's first ticking frame. Initial peer state and native logs remain under `peers-initial-startup-timeout/` and `client-startup-timeout/`; one bounded retry after actual readiness succeeded. UDP bind alone is not application readiness. This proof does not certify cold-start reliability.
- The first leave IPC omitted required `--match-id` and was rejected without transition. The diagnostic driver was corrected to pass the actually received match ID. No product code changed.
- Headless editor import exited0 with ShaderFile thread-access/dialog-parent errors and an ObjectDB exit warning. Actual rendered proof completed; do not call import/runtime warning-free.

## Limits

Direct1018 only: no Random selector, full-match outcome, new reward or20-rendered-client claim. Historical scoreboard rows are not a count of currently attached players. The PvP frame partly overlaps the ready dialog's top-left; both actions remain readable and functional. Placement/stacking parity requires a separate sourced proposal, not an inference from this lifecycle proof. Broader [PvP authority gaps](../specs/pvp-progression.md) remain parked.
